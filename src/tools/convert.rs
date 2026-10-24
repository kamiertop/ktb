use super::*;
pub fn convert(tool: Tool, text: &str, reverse: bool) -> Result<String, String> {
    match tool {
        Tool::Timestamp => timestamp::convert(text, reverse),
        Tool::Base64 => base64::convert(text, reverse),
        Tool::Chinese => chinese::convert(text, reverse),
        Tool::Md5 => md5::convert(text, reverse),
        Tool::Sha256 => sha256::convert(text, reverse),
        Tool::Json => json::convert(text, reverse),
        Tool::Url => url::convert(text, reverse),
        Tool::Audio => Err("音频标签使用文件操作".into()),
        Tool::Qrcode => Err("二维码使用独立页面".into()),
        Tool::Video => Err("视频工具使用文件操作".into()),
    }
}
