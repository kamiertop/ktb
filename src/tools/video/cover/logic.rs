use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
pub fn extension(path: &Path) -> Result<String, String> {
    let ext = path
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_lowercase();
    if matches!(ext.as_str(), "mp3" | "m4a" | "flac" | "mp4" | "m4v") {
        Ok(ext)
    } else {
        Err("支持 MP3、M4A、FLAC、MP4、M4V；MOV/MKV 等暂不支持".into())
    }
}
pub fn suggested(input: &Path) -> PathBuf {
    let mut name = input.file_stem().unwrap_or_default().to_os_string();
    name.push("-cover");
    input
        .with_file_name(name)
        .with_extension(input.extension().unwrap_or_default())
}
static SEQ: AtomicU64 = AtomicU64::new(0);
struct Temporary(PathBuf);
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(self.0.join("output.tmp"));
        let _ = std::fs::remove_dir(&self.0);
    }
}
pub fn write_cover(audio: &Path, image: &Path, output: &Path) -> Result<u64, String> {
    write_impl(audio, image, output, false)
}
pub fn overwrite_cover(audio: &Path, image: &Path) -> Result<u64, String> {
    if std::fs::symlink_metadata(audio)
        .map_err(|e| e.to_string())?
        .file_type()
        .is_symlink()
    {
        return Err("覆盖不支持符号链接，请选择原始媒体文件".into());
    }
    write_impl(audio, image, audio, true)
}
fn write_impl(audio: &Path, image: &Path, output: &Path, overwrite: bool) -> Result<u64, String> {
    let ext = extension(audio)?;
    if extension(output)? != ext {
        return Err("输出扩展名必须与原文件相同".into());
    }
    if !audio.is_file() || !image.is_file() {
        return Err("请选择存在的媒体文件和封面图片".into());
    }
    let image_ext = image
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !matches!(image_ext.as_str(), "jpg" | "jpeg" | "png") {
        return Err("封面支持 JPG 和 PNG 图片".into());
    }
    let original = std::fs::metadata(audio).map_err(|e| e.to_string())?;
    if !overwrite && output.exists() {
        return Err("输出文件已存在，请选择其他文件名；原文件不会被覆盖".into());
    }
    let parent = output
        .parent()
        .filter(|v| !v.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let temp = loop {
        let dir = parent.join(format!(
            ".ktb-cover-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        match std::fs::create_dir(&dir) {
            Ok(()) => break Temporary(dir),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("无法写入输出目录：{e}")),
        }
    };
    let file = temp.0.join("output.tmp");
    let info = probe(audio)?;
    let streams = info["streams"].as_array().ok_or("未找到媒体流")?;
    let is_video = matches!(ext.as_str(), "mp4" | "m4v");
    let retained: Vec<_> = streams
        .iter()
        .filter(|s| s["disposition"]["attached_pic"] != 1)
        .collect();
    let video_count = retained
        .iter()
        .filter(|s| s["codec_type"] == "video")
        .count();
    if is_video && video_count == 0 {
        return Err("文件中没有视频画面轨道".into());
    }
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-hide_banner", "-loglevel", "error", "-nostdin", "-n", "-i"])
        .arg(audio)
        .arg("-i")
        .arg(image);
    for stream in &retained {
        let index = stream["index"].as_u64().ok_or("无效媒体流索引")?;
        cmd.args(["-map", &format!("0:{index}")]);
    }
    let cover_index = video_count;
    cmd.args([
        "-map",
        "1:v:0",
        "-map_metadata",
        "0",
        "-map_chapters",
        "0",
        "-c",
        "copy",
    ])
    .args([&format!("-c:v:{cover_index}"), "mjpeg"])
    .args([&format!("-disposition:v:{cover_index}"), "attached_pic"])
    .args([&format!("-metadata:s:v:{cover_index}"), "title=Cover"])
    .args([
        &format!("-metadata:s:v:{cover_index}"),
        "comment=Cover (front)",
    ]);
    if ext == "mp3" {
        cmd.args(["-id3v2_version", "3"]);
    }
    let muxer = match ext.as_str() {
        "m4a" => "ipod",
        "m4v" => "mp4",
        _ => &ext,
    };
    cmd.args(["-f", muxer]).arg(&file);
    let result = cmd.output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "未找到 FFmpeg，请安装并加入 PATH".into()
        } else {
            format!("无法启动 FFmpeg：{e}")
        }
    })?;
    if !result.status.success() {
        return Err(format!(
            "封面写入失败：{}",
            String::from_utf8_lossy(&result.stderr)
                .chars()
                .take(2000)
                .collect::<String>()
        ));
    }
    // Some container/muxer combinations silently drop attached artwork.
    // Never publish or overwrite such an output.
    let written = probe(&file)?;
    let written_streams = written["streams"].as_array().ok_or("生成文件缺少媒体流")?;
    if written_streams
        .iter()
        .filter(|s| s["disposition"]["attached_pic"] == 1)
        .count()
        != 1
    {
        return Err("该容器未能保存内嵌封面，原文件保持不变".into());
    }
    if written_streams
        .iter()
        .filter(|s| s["disposition"]["attached_pic"] != 1)
        .count()
        != retained.len()
    {
        return Err("媒体轨道数量发生变化，已取消保存".into());
    }
    let bytes = std::fs::metadata(&file).map_err(|e| e.to_string())?.len();
    if bytes == 0 {
        return Err("未生成有效文件".into());
    }
    if overwrite {
        let current = std::fs::metadata(audio).map_err(|e| e.to_string())?;
        if current.len() != original.len() || current.modified().ok() != original.modified().ok() {
            return Err("原文件在处理期间发生变化，已取消覆盖".into());
        }
        std::fs::set_permissions(&file, original.permissions()).map_err(|e| e.to_string())?;
        std::fs::File::open(&file)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())?;
        std::fs::rename(&file, output).map_err(|e| format!("无法替换原文件：{e}"))?;
    } else {
        std::fs::hard_link(&file, output).map_err(|e| format!("无法保存文件：{e}"))?;
    }
    Ok(bytes)
}
fn probe(path: &Path) -> Result<serde_json::Value, String> {
    let result = Command::new("ffprobe")
        .args(["-v", "error", "-show_streams", "-of", "json"])
        .arg(path)
        .output()
        .map_err(|e| format!("无法读取媒体，请检查 ffprobe：{e}"))?;
    if !result.status.success() {
        return Err("无法读取媒体文件".into());
    }
    serde_json::from_slice(&result.stdout).map_err(|e| e.to_string())
}
/// Read only an embedded cover, never interpret a normal video track as cover art.
pub fn read_cover(audio: &Path) -> Result<Option<Vec<u8>>, String> {
    extension(audio)?;
    let info = probe(audio)?;
    let stream = info["streams"].as_array().and_then(|streams| {
        streams
            .iter()
            .find(|s| s["disposition"]["attached_pic"] == 1)
    });
    let Some(index) = stream.and_then(|s| s["index"].as_u64()) else {
        return Ok(None);
    };
    let result = Command::new("ffmpeg")
        .args(["-v", "error", "-nostdin", "-i"])
        .arg(audio)
        .args([
            "-map",
            &format!("0:{index}"),
            "-frames:v",
            "1",
            "-vf",
            "scale=w='min(1024,iw)':h='min(1024,ih)':force_original_aspect_ratio=decrease",
            "-c:v",
            "png",
            "-f",
            "image2pipe",
            "pipe:1",
        ])
        .output()
        .map_err(|e| e.to_string())?;
    if !result.status.success() || result.stdout.is_empty() {
        return Err("原封面解码失败".into());
    }
    Ok(Some(result.stdout))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn run(args: &[&str], output: &Path) {
        let result = Command::new("ffmpeg")
            .args(["-loglevel", "error", "-y"])
            .args(args)
            .arg(output)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    fn packets(path: &Path) -> serde_json::Value {
        let result = Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-select_streams",
                "a:0",
                "-show_packets",
                "-show_entries",
                "packet=data_hash",
                "-show_data_hash",
                "sha256",
                "-of",
                "json",
            ])
            .arg(path)
            .output()
            .unwrap();
        assert!(result.status.success());
        serde_json::from_slice(&result.stdout).unwrap()
    }
    fn media_hashes(path: &Path, selector: &str) -> Vec<String> {
        let result = Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-select_streams",
                selector,
                "-show_packets",
                "-show_entries",
                "packet=data_hash",
                "-show_data_hash",
                "sha256",
                "-of",
                "json",
            ])
            .arg(path)
            .output()
            .unwrap();
        assert!(result.status.success());
        let info: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        info["packets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["data_hash"].as_str().unwrap().to_owned())
            .collect()
    }
    #[test]
    fn video_cover_preserves_picture_and_all_audio_tracks() {
        if Command::new("ffmpeg").arg("-version").output().is_err()
            || Command::new("ffprobe").arg("-version").output().is_err()
        {
            return;
        }
        let dir = std::env::temp_dir().join(format!(
            "ktb-video-cover-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&dir).unwrap();
        let red = dir.join("red.png");
        let blue = dir.join("blue.png");
        run(
            &[
                "-f",
                "lavfi",
                "-i",
                "color=red:size=32x32",
                "-frames:v",
                "1",
            ],
            &red,
        );
        run(
            &[
                "-f",
                "lavfi",
                "-i",
                "color=blue:size=32x32",
                "-frames:v",
                "1",
            ],
            &blue,
        );
        for ext in ["mp4", "m4v"] {
            let source = dir.join(format!("source.{ext}"));
            run(
                &[
                    "-f",
                    "lavfi",
                    "-i",
                    "testsrc=size=32x32:rate=5",
                    "-f",
                    "lavfi",
                    "-i",
                    "sine=frequency=440",
                    "-f",
                    "lavfi",
                    "-i",
                    "sine=frequency=880",
                    "-t",
                    "1",
                    "-map",
                    "0:v",
                    "-map",
                    "1:a",
                    "-map",
                    "2:a",
                    "-c:v",
                    "mpeg4",
                    "-c:a",
                    "aac",
                    "-f",
                    if ext == "mov" { "mov" } else { "mp4" },
                ],
                &source,
            );
            assert!(read_cover(&source).unwrap().is_none());
            let original = std::fs::read(&source).unwrap();
            let output = dir.join(format!("covered.{ext}"));
            write_cover(&source, &red, &output).unwrap();
            assert_eq!(std::fs::read(&source).unwrap(), original);
            let old = read_cover(&output).unwrap().unwrap();
            overwrite_cover(&output, &blue).unwrap();
            assert_ne!(read_cover(&output).unwrap().unwrap(), old);
            for selector in ["v:0", "a:0", "a:1"] {
                assert_eq!(
                    media_hashes(&source, selector),
                    media_hashes(&output, selector),
                    "{ext}: {selector}"
                );
            }
            let info = probe(&output).unwrap();
            assert_eq!(info["streams"].as_array().unwrap().len(), 4);
        }
        let unsupported = dir.join("source.mov");
        run(
            &[
                "-f",
                "lavfi",
                "-i",
                "color=size=32x32:rate=5",
                "-t",
                "1",
                "-c:v",
                "mpeg4",
            ],
            &unsupported,
        );
        let untouched = std::fs::read(&unsupported).unwrap();
        assert!(overwrite_cover(&unsupported, &red).is_err());
        assert_eq!(std::fs::read(&unsupported).unwrap(), untouched);
        let silent = dir.join("silent.mp4");
        run(
            &[
                "-f",
                "lavfi",
                "-i",
                "color=size=32x32:rate=5",
                "-t",
                "1",
                "-c:v",
                "mpeg4",
            ],
            &silent,
        );
        let before = media_hashes(&silent, "v:0");
        overwrite_cover(&silent, &red).unwrap();
        assert_eq!(media_hashes(&silent, "v:0"), before);
        assert!(read_cover(&silent).unwrap().is_some());
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn adds_replaces_and_keeps_audio_packets() {
        if Command::new("ffmpeg").arg("-version").output().is_err()
            || Command::new("ffprobe").arg("-version").output().is_err()
        {
            eprintln!("FFmpeg/ffprobe unavailable; skipping");
            return;
        }
        let dir = std::env::temp_dir().join(format!(
            "ktb-cover-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let red = dir.join("red.png");
        let blue = dir.join("blue.png");
        run(
            &[
                "-f",
                "lavfi",
                "-i",
                "color=red:size=32x32",
                "-frames:v",
                "1",
            ],
            &red,
        );
        run(
            &[
                "-f",
                "lavfi",
                "-i",
                "color=blue:size=32x32",
                "-frames:v",
                "1",
            ],
            &blue,
        );
        for ext in ["mp3", "m4a", "flac"] {
            let audio = dir.join(format!("source.{ext}"));
            run(
                &[
                    "-f",
                    "lavfi",
                    "-i",
                    "sine=frequency=440",
                    "-t",
                    "0.2",
                    "-metadata",
                    "title=keep title",
                ],
                &audio,
            );
            let original = std::fs::read(&audio).unwrap();
            assert!(read_cover(&audio).unwrap().is_none());
            let first = dir.join(format!("first.{ext}"));
            let second = dir.join(format!("second.{ext}"));
            write_cover(&audio, &red, &first).unwrap();
            write_cover(&first, &blue, &second).unwrap();
            assert_eq!(packets(&audio), packets(&second));
            let probe = Command::new("ffprobe")
                .args([
                    "-v",
                    "error",
                    "-show_streams",
                    "-show_format",
                    "-of",
                    "json",
                ])
                .arg(&second)
                .output()
                .unwrap();
            let info: serde_json::Value = serde_json::from_slice(&probe.stdout).unwrap();
            let covers: Vec<_> = info["streams"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|s| s["disposition"]["attached_pic"] == 1)
                .collect();
            assert_eq!(covers.len(), 1, "{ext}: {info}");
            assert_eq!(info["format"]["tags"]["title"], "keep title");
            let mut hashes = Vec::new();
            for file in [&first, &second] {
                let result = Command::new("ffmpeg")
                    .args(["-v", "error", "-i"])
                    .arg(file)
                    .args(["-map", "0:v:0", "-f", "md5", "-"])
                    .output()
                    .unwrap();
                assert!(result.status.success());
                hashes.push(result.stdout);
            }
            assert_ne!(hashes[0], hashes[1]);
            assert!(write_cover(&audio, &red, &second).is_err());
            assert_eq!(std::fs::read(&audio).unwrap(), original);
            let old_preview = read_cover(&first).unwrap().unwrap();
            let new_preview = read_cover(&second).unwrap().unwrap();
            assert_ne!(old_preview, new_preview);
            let before_packets = packets(&audio);
            overwrite_cover(&audio, &blue).unwrap();
            assert_eq!(packets(&audio), before_packets);
            assert_eq!(read_cover(&audio).unwrap().unwrap(), new_preview);
            let updated = std::fs::read(&audio).unwrap();
            let broken = dir.join("broken.png");
            std::fs::write(&broken, b"not an image").unwrap();
            assert!(overwrite_cover(&audio, &broken).is_err());
            assert_eq!(std::fs::read(&audio).unwrap(), updated);
        }
        assert!(
            write_cover(
                &dir.join("source.mp3"),
                &dir.join("missing.png"),
                &dir.join("bad.mp3")
            )
            .is_err()
        );
        assert!(!dir.join("bad.mp3").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
