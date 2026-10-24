use super::logic::Format;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

#[derive(Clone)]
pub enum Status {
    Pending,
    Running,
    Done { output: PathBuf, bytes: u64 },
    Failed(String),
}
#[derive(Clone)]
pub struct Video {
    pub path: PathBuf,
    pub bytes: Option<u64>,
    pub status: Status,
}
pub fn inspect(paths: Vec<PathBuf>) -> Vec<Video> {
    paths
        .into_iter()
        .map(|path| {
            let path = path.canonicalize().unwrap_or(path);
            let meta = std::fs::metadata(&path);
            let bytes = meta.as_ref().ok().filter(|m| m.is_file()).map(|m| m.len());
            Video {
                path,
                bytes,
                status: Status::Pending,
            }
        })
        .collect()
}
pub fn size(bytes: u64) -> String {
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let (n, unit) = if bytes < 1024 * 1024 {
        (1024., "KiB")
    } else if bytes < 1024 * 1024 * 1024 {
        (1048576., "MiB")
    } else {
        (1073741824., "GiB")
    };
    format!("{:.1} {unit}", bytes as f64 / n)
}
/// Plan distinct outputs before running; never reuse an existing or reserved filename.
pub fn plan(
    paths: Vec<(usize, PathBuf)>,
    directory: &Path,
    format: Format,
) -> Result<Vec<(usize, PathBuf, PathBuf)>, String> {
    if !directory.is_dir() {
        return Err("请选择存在的输出目录".into());
    }
    let mut reserved = HashSet::new();
    let mut jobs = Vec::new();
    for (index, input) in paths {
        let stem = input.file_stem().ok_or("无效视频文件名")?.to_os_string();
        let mut counter = 0;
        let output = loop {
            let mut name = stem.clone();
            if counter > 0 {
                name.push(format!(" ({counter})"));
            }
            name.push(format!(".{}", format.extension()));
            let candidate = directory.join(name);
            let key = candidate.to_string_lossy().to_lowercase();
            if !candidate.exists()
                && !(format == Format::Wav && super::artwork::sidecar(&candidate).exists())
                && reserved.insert(key)
            {
                break candidate;
            }
            counter += 1;
        };
        jobs.push((index, input, output));
    }
    Ok(jobs)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn handles_duplicate_names_and_existing_outputs() {
        let dir = std::env::temp_dir().join(format!("ktb-audio-plan-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("video.mp3"), b"keep").unwrap();
        let jobs = plan(
            vec![
                (0, PathBuf::from("a/video.mp4")),
                (1, PathBuf::from("b/video.mov")),
            ],
            &dir,
            Format::Mp3,
        )
        .unwrap();
        assert_eq!(jobs[0].2, dir.join("video (1).mp3"));
        assert_eq!(jobs[1].2, dir.join("video (2).mp3"));
        assert_eq!(std::fs::read(dir.join("video.mp3")).unwrap(), b"keep");
        assert_eq!(size(1048576), "1.0 MiB");
        std::fs::write(dir.join("video.wav.cover.jpg"), b"existing artwork").unwrap();
        let wav = plan(vec![(0, PathBuf::from("video.mp4"))], &dir, Format::Wav).unwrap();
        assert_eq!(wav[0].2, dir.join("video (1).wav"));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
