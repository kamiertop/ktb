use base64::{Engine, engine::general_purpose::STANDARD};
pub fn convert(text: &str, reverse: bool) -> Result<String, String> {
    match reverse {
        true => String::from_utf8(
            STANDARD
                .decode(text.trim())
                .map_err(|e| format!("无效 Base64：{e}"))?,
        )
        .map_err(|_| "解码结果不是 UTF-8 文本".into()),
        false => Ok(STANDARD.encode(text)),
    }
}
