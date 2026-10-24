use super::artwork;
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Mp3,
    M4a,
    Wav,
    Flac,
}
impl Format {
    pub const ALL: [Self; 4] = [Self::Mp3, Self::M4a, Self::Wav, Self::Flac];
    pub fn extension(self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::M4a => "m4a",
            Self::Wav => "wav",
            Self::Flac => "flac",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Mp3 => "MP3",
            Self::M4a => "M4A / AAC",
            Self::Wav => "WAV",
            Self::Flac => "FLAC",
        }
    }
    fn args(self) -> &'static [&'static str] {
        match self {
            Self::Mp3 => &["-c:a", "libmp3lame", "-b:a", "192k"],
            Self::M4a => &["-c:a", "aac", "-b:a", "192k"],
            Self::Wav => &["-c:a", "pcm_s16le"],
            Self::Flac => &["-c:a", "flac"],
        }
    }
}
struct Temp(PathBuf);
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(self.0.join("audio.tmp"));
        let _ = std::fs::remove_file(self.0.join("cover.jpg"));
        let _ = std::fs::remove_dir(&self.0);
    }
}
static SEQUENCE: AtomicU64 = AtomicU64::new(0);
pub fn extract(input: &Path, output: &Path, format: Format) -> Result<u64, String> {
    if !input.is_file() {
        return Err("请选择有效的视频文件".into());
    }
    if output.exists() {
        return Err("输出文件已存在，请选择其他路径（不会覆盖原文件）".into());
    }
    if output
        .extension()
        .and_then(|v| v.to_str())
        .map(|v| v.eq_ignore_ascii_case(format.extension()))
        != Some(true)
    {
        return Err(format!("输出文件扩展名应为 .{}", format.extension()));
    }
    let parent = output
        .parent()
        .filter(|v| !v.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    if !parent.is_dir() {
        return Err("输出目录不存在".into());
    }
    let temp = loop {
        let p = parent.join(format!(
            ".ktb-audio-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        match std::fs::create_dir(&p) {
            Ok(()) => break Temp(p),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("无法写入输出目录：{e}")),
        }
    };
    let file = temp.0.join("audio.tmp");
    let cover = artwork::stream(input)?;
    let sidecar = (cover.is_some() && format == Format::Wav).then(|| artwork::sidecar(output));
    if sidecar.as_ref().is_some_and(|p| p.exists()) {
        return Err("封面图片已存在，请更换输出文件名".into());
    }
    let mut command = Command::new("ffmpeg");
    command
        .args(["-hide_banner", "-loglevel", "error", "-nostdin", "-n", "-i"])
        .arg(input)
        .args(["-map", "0:a:0", "-map_metadata", "0"])
        .args(format.args());
    if let Some(index) = cover.filter(|_| format != Format::Wav) {
        command.args([
            "-map",
            &format!("0:{index}"),
            "-c:v",
            "mjpeg",
            "-disposition:v:0",
            "attached_pic",
            "-metadata:s:v:0",
            "title=Cover",
            "-metadata:s:v:0",
            "comment=Cover (front)",
        ]);
        if format == Format::Mp3 {
            command.args(["-id3v2_version", "3"]);
        }
    } else {
        command.arg("-vn");
    }
    let result = command
        .args([
            "-f",
            match format {
                Format::Mp3 => "mp3",
                Format::M4a => "ipod",
                Format::Wav => "wav",
                Format::Flac => "flac",
            },
        ])
        .arg(&file)
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "未找到 FFmpeg，请安装并将 ffmpeg 加入 PATH 后重启应用".into()
            } else {
                format!("无法启动 FFmpeg：{e}")
            }
        })?;
    if !result.status.success() {
        let detail = String::from_utf8_lossy(&result.stderr);
        return Err(format!(
            "提取失败（请确认视频含音轨）：{}",
            detail.chars().take(2000).collect::<String>()
        ));
    }
    let size = std::fs::metadata(&file).map_err(|e| e.to_string())?.len();
    if size == 0 {
        return Err("未生成有效的音频文件".into());
    }
    if let Some(index) = cover {
        if format != Format::Wav && artwork::stream(&file)?.is_none() {
            return Err("音频未成功保留封面，已取消保存".into());
        }
        if let Some(sidecar) = &sidecar {
            let image = temp.0.join("cover.jpg");
            artwork::save_image(input, index, &image)?;
            std::fs::hard_link(&image, sidecar).map_err(|e| format!("无法保存封面图片：{e}"))?;
        }
    }
    // Publish completed files only; never overwrite an existing destination.
    if let Err(error) = std::fs::hard_link(&file, output) {
        if let Some(path) = sidecar {
            let _ = std::fs::remove_file(path);
        }
        return Err(format!("无法保存音频（目标不能已存在）：{error}"));
    }
    Ok(size)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn run(args: &[&str], output: &Path) {
        let result = Command::new("ffmpeg")
            .args(["-v", "error", "-y"])
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
    #[test]
    fn retains_embedded_artwork_and_exports_wav_sidecar() {
        if Command::new("ffmpeg").arg("-version").output().is_err()
            || Command::new("ffprobe").arg("-version").output().is_err()
        {
            return;
        }
        let dir = std::env::temp_dir().join(format!(
            "ktb-extract-cover-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&dir).unwrap();
        let video = dir.join("video.mp4");
        let image = dir.join("cover.png");
        let covered = dir.join("covered.mp4");
        run(
            &[
                "-f",
                "lavfi",
                "-i",
                "color=green:size=16x16:rate=5",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440",
                "-t",
                "1.2",
                "-c:v",
                "mpeg4",
                "-c:a",
                "aac",
            ],
            &video,
        );
        run(
            &[
                "-f",
                "lavfi",
                "-i",
                "color=red:size=32x32",
                "-frames:v",
                "1",
            ],
            &image,
        );
        let result = Command::new("ffmpeg")
            .args(["-v", "error", "-i"])
            .arg(&video)
            .arg("-i")
            .arg(&image)
            .args([
                "-map",
                "0",
                "-map",
                "1:v:0",
                "-c",
                "copy",
                "-c:v:1",
                "mjpeg",
                "-disposition:v:1",
                "attached_pic",
            ])
            .arg(&covered)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(artwork::stream(&video).unwrap().is_none());
        for format in Format::ALL {
            let output = dir.join(format!("audio.{}", format.extension()));
            extract(&covered, &output, format).unwrap();
            let result = Command::new("ffprobe")
                .args(["-v", "error", "-show_streams", "-of", "json"])
                .arg(&output)
                .output()
                .unwrap();
            let info: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
            let streams = info["streams"].as_array().unwrap();
            let audio = streams.iter().find(|s| s["codec_type"] == "audio").unwrap();
            assert!(
                audio["duration"]
                    .as_str()
                    .unwrap_or("1.2")
                    .parse::<f64>()
                    .unwrap()
                    >= 1.1,
                "audio truncated: {info}"
            );
            if format == Format::Wav {
                assert!(artwork::stream(&output).unwrap().is_none());
                let sidecar = artwork::sidecar(&output);
                let image = std::fs::read(&sidecar).unwrap();
                assert!(image.starts_with(&[0xff, 0xd8]));
                std::fs::remove_file(&output).unwrap();
                assert!(extract(&covered, &output, format).is_err());
                assert_eq!(std::fs::read(sidecar).unwrap(), image);
                assert!(!output.exists());
            } else {
                let covers: Vec<_> = streams
                    .iter()
                    .filter(|s| s["codec_type"] == "video")
                    .collect();
                assert_eq!(covers.len(), 1);
                assert_eq!(covers[0]["disposition"]["attached_pic"], 1);
                assert_eq!(covers[0]["width"], 32);
                assert_eq!(covers[0]["height"], 32);
            }
        }
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn extracts_real_video_audio_and_preserves_existing_files() {
        if Command::new("ffmpeg").arg("-version").output().is_err() {
            eprintln!("FFmpeg unavailable; skipping integration test");
            return;
        }
        let dir = std::env::temp_dir().join(format!("ktb-test-audio-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let video = dir.join("video.mkv");
        let result = Command::new("ffmpeg")
            .args([
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "color=size=16x16:rate=1",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440",
                "-t",
                "0.2",
                "-c:v",
                "mpeg4",
                "-c:a",
                "pcm_s16le",
            ])
            .arg(&video)
            .output()
            .unwrap();
        assert!(result.status.success(), "{:?}", result.stderr);
        for format in Format::ALL {
            let output = dir.join(format!("audio.{}", format.extension()));
            assert!(extract(&video, &output, format).unwrap() > 0);
            let bytes = std::fs::read(&output).unwrap();
            assert!(extract(&video, &output, format).is_err());
            assert_eq!(std::fs::read(&output).unwrap(), bytes);
        }
        let silent = dir.join("silent.mkv");
        assert!(
            Command::new("ffmpeg")
                .args(["-loglevel", "error", "-y", "-i"])
                .arg(&video)
                .args(["-an", "-c:v", "copy"])
                .arg(&silent)
                .status()
                .unwrap()
                .success()
        );
        let missing = dir.join("missing.mp3");
        assert!(extract(&silent, &missing, Format::Mp3).is_err());
        assert!(!missing.exists());
        let batch_dir = dir.join("batch");
        std::fs::create_dir(&batch_dir).unwrap();
        let jobs = super::super::queue::plan(
            vec![(0, video.clone()), (1, silent.clone()), (2, video.clone())],
            &batch_dir,
            Format::Mp3,
        )
        .unwrap();
        let outcomes: Vec<_> = jobs
            .into_iter()
            .map(|(_, input, output)| extract(&input, &output, Format::Mp3))
            .collect();
        assert!(outcomes[0].is_ok());
        assert!(outcomes[1].is_err());
        assert!(outcomes[2].is_ok());
        assert_eq!(std::fs::read_dir(&batch_dir).unwrap().count(), 2);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
