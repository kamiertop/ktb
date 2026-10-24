pub fn convert(text: &str, reverse: bool) -> Result<String, String> {
    Ok(zhconv::zhconv(
        text,
        if reverse {
            zhconv::Variant::ZhHans
        } else {
            zhconv::Variant::ZhTW
        },
    ))
}
