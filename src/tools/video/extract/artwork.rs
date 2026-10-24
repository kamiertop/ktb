use std::{
    path::{Path, PathBuf},
    process::Command,
};

/// Inspect embedded artwork only; ordinary video streams are not covers.
pub fn stream(input: &Path) -> Result<Option<u64>, String> {
    let probe = Command::new("ffprobe")
        .args(["-v", "error", "-show_streams", "-of", "json"])
        .arg(input)
        .output()
        .map_err(|e| format!("无法检查视频封面，请确认 ffprobe 已加入 PATH：{e}"))?;
    if !probe.status.success() {
        return Err("无法读取视频文件".into());
    }
    let info: serde_json::Value =
        serde_json::from_slice(&probe.stdout).map_err(|e| e.to_string())?;
    Ok(info["streams"]
        .as_array()
        .and_then(|items| items.iter().find(|s| s["disposition"]["attached_pic"] == 1))
        .and_then(|s| s["index"].as_u64()))
}
pub fn sidecar(output: &Path) -> PathBuf {
    let mut name = output.as_os_str().to_os_string();
    name.push(".cover.jpg");
    PathBuf::from(name)
}
pub fn save_image(input: &Path, index: u64, file: &Path) -> Result<(), String> {
    let result = Command::new("ffmpeg")
        .args(["-v", "error", "-nostdin", "-n", "-i"])
        .arg(input)
        .args([
            "-map",
            &format!("0:{index}"),
            "-frames:v",
            "1",
            "-c:v",
            "mjpeg",
            "-f",
            "image2",
        ])
        .arg(file)
        .output()
        .map_err(|e| e.to_string())?;
    if !result.status.success() {
        return Err(format!(
            "无法保存封面：{}",
            String::from_utf8_lossy(&result.stderr)
        ));
    }
    Ok(())
}
