use sha2::{Digest, Sha256};

pub fn convert(text: &str, uppercase: bool) -> Result<String, String> {
    let bytes = Sha256::digest(text.as_bytes());
    let alphabet = if uppercase {
        b"0123456789ABCDEF"
    } else {
        b"0123456789abcdef"
    };
    let mut digest = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        digest.push(alphabet[(byte >> 4) as usize] as char);
        digest.push(alphabet[(byte & 0x0f) as usize] as char);
    }
    Ok(digest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_vectors_and_letter_case() {
        for (text, expected) in [
            (
                "",
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            ),
            (
                "abc",
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            ),
        ] {
            assert_eq!(convert(text, false).unwrap(), expected);
            assert_eq!(convert(text, true).unwrap(), expected.to_uppercase());
        }
    }
}
