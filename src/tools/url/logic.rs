pub fn convert(text: &str, reverse: bool) -> Result<String, String> {
    match reverse {
        true => urlencoding::decode(text)
            .map(|v| v.into_owned())
            .map_err(|e| format!("解码失败：{e}")),
        false => Ok(urlencoding::encode(text).into_owned()),
    }
}
