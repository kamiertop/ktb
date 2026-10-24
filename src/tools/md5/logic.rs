pub fn convert(text: &str, uppercase: bool) -> Result<String, String> {
    let digest = format!("{:x}", md5::compute(text));
    Ok(if uppercase {
        digest.to_uppercase()
    } else {
        digest
    })
}
