use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::SystemTime,
};
pub const FIELDS: [(&str, &str); 10] = [
    ("title", "标题"),
    ("artist", "歌手"),
    ("album", "专辑"),
    ("album_artist", "专辑歌手"),
    ("date", "年份 / 日期"),
    ("genre", "流派"),
    ("track", "音轨序号"),
    ("disc", "碟片序号"),
    ("composer", "作曲者"),
    ("comment", "备注"),
];
pub struct Info {
    pub path: PathBuf,
    pub bytes: u64,
    pub modified: Option<SystemTime>,
    pub duration: String,
    pub codec: String,
    pub tags: BTreeMap<String, String>,
    pub cover: Option<Vec<u8>>,
    pub streams: usize,
    pub cover_streams: usize,
}
fn ext(path: &Path) -> Result<String, String> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    if matches!(ext.as_str(), "mp3" | "m4a" | "flac") {
        Ok(ext)
    } else {
        Err("支持 MP3、M4A、FLAC 音频".into())
    }
}
fn probe(path: &Path) -> Result<serde_json::Value, String> {
    let result = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_format",
            "-show_streams",
            "-of",
            "json",
        ])
        .arg(path)
        .output()
        .map_err(|e| format!("请检查 ffprobe：{e}"))?;
    if !result.status.success() {
        return Err("无法读取音频文件".into());
    }
    serde_json::from_slice(&result.stdout).map_err(|e| e.to_string())
}
fn aliases(key: &str) -> &[&str] {
    match key {
        "composer" => &["composer", "tcom", "©wrt"],
        "date" => &["date", "year"],
        "album_artist" => &["album_artist", "albumartist"],
        _ => &[],
    }
}
fn normalize(raw: &serde_json::Value) -> BTreeMap<String, String> {
    raw.as_object()
        .into_iter()
        .flatten()
        .filter_map(|(key, value)| {
            value
                .as_str()
                .map(|value| (key.to_lowercase(), value.to_owned()))
        })
        .collect()
}
fn tags(info: &serde_json::Value) -> BTreeMap<String, String> {
    let mut result = normalize(&info["format"]["tags"]);
    // Containers may store descriptive fields on the audio stream instead of the file.
    // Only import editable fields, not stream-specific technical tags.
    if let Some(audio) = info["streams"]
        .as_array()
        .and_then(|streams| streams.iter().find(|s| s["codec_type"] == "audio"))
    {
        let stream_tags = normalize(&audio["tags"]);
        for (key, _) in FIELDS {
            if value(&result, key).is_empty() {
                let fallback = value(&stream_tags, key);
                if !fallback.is_empty() {
                    result.insert(key.into(), fallback);
                }
            }
        }
    }
    result
}
pub fn value(tags: &BTreeMap<String, String>, key: &str) -> String {
    tags.get(key)
        .filter(|v| !v.trim().is_empty())
        .or_else(|| {
            aliases(key)
                .iter()
                .filter_map(|alias| tags.get(*alias))
                .find(|v| !v.trim().is_empty())
        })
        .cloned()
        .unwrap_or_default()
}
pub fn read(path: &Path) -> Result<Info, String> {
    ext(path)?;
    let metadata = std::fs::metadata(path).map_err(|e| e.to_string())?;
    let info = probe(path)?;
    let streams = info["streams"].as_array().ok_or("文件缺少媒体轨道")?;
    let audio = streams
        .iter()
        .find(|s| s["codec_type"] == "audio")
        .ok_or("文件没有音轨")?;
    let seconds = info["format"]["duration"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.)
        .max(0.) as u64;
    Ok(Info {
        path: path.to_path_buf(),
        bytes: metadata.len(),
        modified: metadata.modified().ok(),
        duration: format!("{}:{:02}", seconds / 60, seconds % 60),
        codec: audio["codec_name"]
            .as_str()
            .unwrap_or("未知编码")
            .to_uppercase(),
        tags: tags(&info),
        streams: streams.len(),
        cover_streams: streams
            .iter()
            .filter(|s| s["disposition"]["attached_pic"] == 1)
            .count(),
        cover: crate::tools::video::read_cover(path)?,
    })
}
pub fn suggested(path: &Path) -> PathBuf {
    let mut stem = path.file_stem().unwrap_or_default().to_os_string();
    stem.push("-tags");
    path.with_file_name(stem)
        .with_extension(path.extension().unwrap_or_default())
}
static SEQ: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(self.0.join("output.tmp"));
        let _ = std::fs::remove_dir(&self.0);
    }
}
fn unchanged(info: &Info) -> Result<(), String> {
    let now = std::fs::metadata(&info.path).map_err(|e| e.to_string())?;
    if now.len() != info.bytes || now.modified().ok() != info.modified {
        return Err("源文件已变化，请重新读取后编辑".into());
    }
    Ok(())
}
fn validate(edits: &BTreeMap<String, String>) -> Result<(), String> {
    for key in ["track", "disc"] {
        if let Some(value) = edits.get(key).filter(|v| !v.is_empty()) {
            let numbers: Vec<_> = value.split('/').map(str::parse::<u32>).collect();
            if numbers.is_empty()
                || numbers.len() > 2
                || numbers.iter().any(|v| !matches!(v, Ok(1..)))
                || (numbers.len() == 2
                    && numbers[0].as_ref().unwrap() > numbers[1].as_ref().unwrap())
            {
                return Err("音轨/碟片序号请填写正整数或 序号/总数，例如 1/12".into());
            }
        }
    }
    Ok(())
}
pub fn save(
    info: &Info,
    edits: &BTreeMap<String, String>,
    output: &Path,
    overwrite: bool,
    cover: Option<&Path>,
) -> Result<(), String> {
    validate(edits)?;
    if let Some(cover) = cover {
        let extension = cover
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        if !cover.is_file() || !matches!(extension.as_str(), "jpg" | "jpeg" | "png") {
            return Err("请选择有效的 JPG / PNG 封面图片".into());
        }
    }
    unchanged(info)?;
    let extension = ext(&info.path)?;
    if ext(output)? != extension {
        return Err("输出格式必须与原文件相同".into());
    }
    if overwrite {
        if output != info.path {
            return Err("覆盖目标必须为原文件".into());
        }
        if std::fs::symlink_metadata(output)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
        {
            return Err("请选择原始文件，不能覆盖符号链接".into());
        }
    } else if output.exists() {
        return Err("输出文件已存在，请更换文件名".into());
    }
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let temp = loop {
        let dir = parent.join(format!(
            ".ktb-tags-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        match std::fs::create_dir(&dir) {
            Ok(()) => break Temp(dir),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.to_string()),
        }
    };
    let file = temp.0.join("output.tmp");
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-v", "error", "-nostdin", "-n", "-i"])
        .arg(&info.path);
    if let Some(cover) = cover {
        cmd.arg("-i").arg(cover);
        let original = probe(&info.path)?;
        let streams = original["streams"].as_array().ok_or("文件缺少媒体轨道")?;
        let retained: Vec<_> = streams
            .iter()
            .filter(|s| s["disposition"]["attached_pic"] != 1)
            .collect();
        let video_index = retained
            .iter()
            .filter(|s| s["codec_type"] == "video")
            .count();
        for stream in retained {
            let index = stream["index"].as_u64().ok_or("无效媒体流索引")?;
            cmd.args(["-map", &format!("0:{index}")]);
        }
        cmd.args(["-map", "1:v:0", "-c", "copy"])
            .args([&format!("-c:v:{video_index}"), "mjpeg"])
            .args([&format!("-disposition:v:{video_index}"), "attached_pic"])
            .args([&format!("-metadata:s:v:{video_index}"), "title=Cover"])
            .args([
                &format!("-metadata:s:v:{video_index}"),
                "comment=Cover (front)",
            ]);
    } else {
        cmd.args(["-map", "0", "-c", "copy"]);
    }
    cmd.args(["-map_metadata", "0", "-map_chapters", "0"]);
    for (key, _) in FIELDS {
        if let Some(v) = edits.get(key)
            && *v != value(&info.tags, key)
        {
            for alias in aliases(key)
                .iter()
                .filter(|alias| **alias != key && info.tags.contains_key(**alias))
            {
                cmd.args(["-metadata", &format!("{alias}=")]);
            }
            cmd.args(["-metadata", &format!("{key}={v}")]);
        }
    }
    if extension == "mp3" {
        cmd.args(["-id3v2_version", "3"]);
    }
    let muxer = if extension == "m4a" {
        "ipod"
    } else {
        &extension
    };
    let result = cmd
        .args(["-f", muxer])
        .arg(&file)
        .output()
        .map_err(|e| format!("无法启动 FFmpeg：{e}"))?;
    if !result.status.success() {
        return Err(format!(
            "标签写入失败：{}",
            String::from_utf8_lossy(&result.stderr)
                .chars()
                .take(1800)
                .collect::<String>()
        ));
    }
    let written = probe(&file)?;
    let new_tags = tags(&written);
    let expected_streams = if cover.is_some() {
        info.streams - info.cover_streams + 1
    } else {
        info.streams
    };
    if cover.is_some()
        && written["streams"]
            .as_array()
            .ok_or("生成文件缺少媒体流")?
            .iter()
            .filter(|s| s["disposition"]["attached_pic"] == 1)
            .count()
            != 1
    {
        return Err("封面未能保存，已取消保存".into());
    }
    if written["streams"].as_array().map(Vec::len) != Some(expected_streams) {
        return Err("轨道数量变化，已取消保存".into());
    }
    for (key, _) in FIELDS {
        let expected = edits
            .get(key)
            .cloned()
            .unwrap_or_else(|| value(&info.tags, key));
        if value(&new_tags, key) != expected {
            return Err(format!("该格式无法完整保存字段 {key}，原文件未改动"));
        }
    }
    for (key, old) in &info.tags {
        if FIELDS
            .iter()
            .any(|(k, _)| *k == key || aliases(k).contains(&key.as_str()))
            || matches!(
                key.as_str(),
                "encoder"
                    | "major_brand"
                    | "minor_version"
                    | "compatible_brands"
                    | "year"
                    | "albumartist"
            )
        {
            continue;
        }
        if new_tags.get(key) != Some(old) {
            return Err(format!("其他标签 {key} 无法保留，已取消保存"));
        }
    }
    unchanged(info)?;
    if overwrite {
        std::fs::set_permissions(
            &file,
            std::fs::metadata(&info.path)
                .map_err(|e| e.to_string())?
                .permissions(),
        )
        .map_err(|e| e.to_string())?;
        std::fs::File::open(&file)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())?;
        std::fs::rename(&file, output).map_err(|e| e.to_string())?;
    } else {
        std::fs::hard_link(&file, output).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn run(args: &[&str], path: &Path) {
        let result = Command::new("ffmpeg")
            .args(["-v", "error", "-y"])
            .args(args)
            .arg(path)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    fn hashes(path: &Path) -> Vec<String> {
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
        let info: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        info["packets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["data_hash"].as_str().unwrap().to_owned())
            .collect()
    }
    #[test]
    fn composer_locations_aliases_and_missing_values() {
        use serde_json::json;
        let file = json!({"format":{"tags":{"COMPOSER":"File composer"}},"streams":[{"codec_type":"audio","tags":{"composer":"Stream composer"}}]});
        assert_eq!(value(&tags(&file), "composer"), "File composer");
        for alias in ["COMPOSER", "TCOM", "©wrt"] {
            let mut raw = serde_json::Map::new();
            raw.insert(alias.into(), json!("Actual composer"));
            let stream = json!({"format":{"tags":{"composer":""}},"streams":[{"codec_type":"video","tags":{"composer":"Wrong stream"}},{"codec_type":"audio","tags":raw}]});
            assert_eq!(value(&tags(&stream), "composer"), "Actual composer");
        }
        let missing = json!({"format":{"tags":{"artist":"Singer","author":"Writer","lyricist":"Lyricist","comment":"作曲：Somebody"}},"streams":[{"codec_type":"audio"}]});
        assert_eq!(value(&tags(&missing), "composer"), "");
    }
    #[test]
    fn edit_tags_without_changing_audio_cover_or_other_tags() {
        if Command::new("ffmpeg").arg("-version").output().is_err()
            || Command::new("ffprobe").arg("-version").output().is_err()
        {
            return;
        }
        let dir = std::env::temp_dir().join(format!(
            "ktb-tags-test-{}",
            SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&dir).unwrap();
        let image = dir.join("cover.png");
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
        for extension in ["mp3", "m4a", "flac"] {
            let bare = dir.join(format!("bare.{extension}"));
            run(
                &[
                    "-f",
                    "lavfi",
                    "-i",
                    "sine=frequency=440",
                    "-t",
                    "1",
                    "-metadata",
                    "title=Old title",
                    "-metadata",
                    "artist=Old artist",
                    "-metadata",
                    "composer=Original composer",
                    "-metadata",
                    "copyright=Keep this",
                    "-metadata",
                    "ktb_private=Keep custom tag",
                ],
                &bare,
            );
            let source = dir.join(format!("source.{extension}"));
            let result = Command::new("ffmpeg")
                .args(["-v", "error", "-i"])
                .arg(&bare)
                .arg("-i")
                .arg(&image)
                .args([
                    "-map",
                    "0:a:0",
                    "-map",
                    "1:v:0",
                    "-map_metadata",
                    "0",
                    "-c",
                    "copy",
                    "-c:v",
                    "mjpeg",
                    "-disposition:v:0",
                    "attached_pic",
                ])
                .arg(&source)
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            let info = read(&source).unwrap();
            assert!(info.cover.is_some(), "source cover: {extension}");
            assert_eq!(
                value(&info.tags, "composer"),
                "Original composer",
                "{extension}"
            );
            if extension != "m4a" {
                assert_eq!(info.tags.get("ktb_private").unwrap(), "Keep custom tag");
            }
            let original = std::fs::read(&source).unwrap();
            let edits: BTreeMap<_, _> = FIELDS
                .iter()
                .map(|(k, _)| {
                    (
                        k.to_string(),
                        match *k {
                            "title" => "新标题",
                            "artist" => "新歌手",
                            "album" => "测试专辑",
                            "album_artist" => "专辑歌手",
                            "date" => "2026",
                            "genre" => "Rock",
                            "track" => "2/12",
                            "disc" => "1/2",
                            "composer" => "作曲者",
                            "comment" => "测试备注",
                            _ => "",
                        }
                        .to_owned(),
                    )
                })
                .collect();
            let output = dir.join(format!("edited.{extension}"));
            save(&info, &edits, &output, false, None)
                .unwrap_or_else(|error| panic!("{extension}: {error}"));
            assert_eq!(std::fs::read(&source).unwrap(), original);
            assert_eq!(hashes(&source), hashes(&output));
            let written = read(&output).unwrap();
            assert_eq!(written.cover, info.cover);
            assert_eq!(written.tags.get("copyright"), info.tags.get("copyright"));
            for (k, v) in &edits {
                assert_eq!(value(&written.tags, k), *v, "{extension}: {k}");
            }
            let mut cleared = edits.clone();
            cleared.insert("comment".into(), String::new());
            save(&written, &cleared, &output, true, None).unwrap();
            let updated = read(&output).unwrap();
            assert_eq!(value(&updated.tags, "comment"), "");
            assert_eq!(hashes(&source), hashes(&output));
            let unchanged_bytes = std::fs::read(&output).unwrap();
            cleared.insert("track".into(), "0/12".into());
            assert!(save(&updated, &cleared, &output, true, None).is_err());
            assert_eq!(std::fs::read(&output).unwrap(), unchanged_bytes);
            assert!(save(&info, &edits, &output, false, None).is_err());
            // Tags and replacement artwork are published together without re-encoding audio.
            let replacement = dir.join("replacement.png");
            run(
                &[
                    "-f",
                    "lavfi",
                    "-i",
                    "color=blue:size=48x48",
                    "-frames:v",
                    "1",
                ],
                &replacement,
            );
            let mut cover_edits = edits.clone();
            cover_edits.insert("title".into(), "封面和标签一起保存".into());
            let combined = dir.join(format!("combined.{extension}"));
            save(&updated, &cover_edits, &combined, false, Some(&replacement)).unwrap();
            let combined_info = read(&combined).unwrap();
            assert_eq!(value(&combined_info.tags, "title"), "封面和标签一起保存");
            assert_eq!(combined_info.cover_streams, 1);
            assert_ne!(combined_info.cover, info.cover);
            assert_eq!(hashes(&source), hashes(&combined));
            assert_eq!(std::fs::read(&output).unwrap(), unchanged_bytes);
            save(&combined_info, &edits, &combined, true, Some(&image)).unwrap();
            let overwritten = read(&combined).unwrap();
            assert_eq!(overwritten.cover, info.cover);
            assert_eq!(hashes(&source), hashes(&combined));
            assert_eq!(value(&overwritten.tags, "title"), edits["title"]);
            let stable = std::fs::read(&combined).unwrap();
            let broken = dir.join("broken.png");
            std::fs::write(&broken, b"invalid image").unwrap();
            assert!(save(&overwritten, &cover_edits, &combined, true, Some(&broken)).is_err());
            assert_eq!(std::fs::read(&combined).unwrap(), stable);
            let bare_info = read(&bare).unwrap();
            let added = dir.join(format!("added.{extension}"));
            save(&bare_info, &edits, &added, false, Some(&replacement)).unwrap();
            assert_eq!(read(&added).unwrap().cover_streams, 1);
            assert_eq!(hashes(&bare), hashes(&added));
        }
        std::fs::remove_dir_all(dir).unwrap();
    }
}
