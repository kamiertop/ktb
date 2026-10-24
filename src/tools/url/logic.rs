#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Whole,
    Component,
    Form,
}
impl Mode {
    pub const ALL: [Self; 3] = [Self::Whole, Self::Component, Self::Form];
    pub fn label(self) -> &'static str {
        match self {
            Self::Whole => "整个 URL",
            Self::Component => "参数值",
            Self::Form => "表单编码",
        }
    }
    pub fn hint(self) -> &'static str {
        match self {
            Self::Whole => {
                "保留 : / ? & = # 等 URL 分隔符；解码时保留被编码的分隔符，避免改变链接结构。"
            }
            Self::Component => {
                "用于单个参数名或参数值；空格编码为 %20，: / 等分隔符也会编码；解码时 + 不转为空格。"
            }
            Self::Form => "用于查询参数或表单字段；空格编码为 +，原有加号编码为 %2B。",
        }
    }
}
const RESERVED: &[u8] = b":/?#[]@!$&'()*+,;=";

pub fn encode(text: &str, mode: Mode) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        let safe = byte.is_ascii_alphanumeric()
            || match mode {
                Mode::Form => b"*-._".contains(&byte),
                Mode::Component => b"-._~".contains(&byte),
                Mode::Whole => b"-._~".contains(&byte) || RESERVED.contains(&byte),
            };
        if safe {
            out.push(byte as char);
        } else if mode == Mode::Form && byte == b' ' {
            out.push('+');
        } else {
            use std::fmt::Write;
            write!(out, "%{byte:02X}").unwrap();
        }
    }
    out
}

fn hex(byte: u8) -> Option<u8> {
    (byte as char).to_digit(16).map(|n| n as u8)
}

pub fn decode(text: &str, mode: Mode) -> Result<String, String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut ix = 0;
    while ix < bytes.len() {
        match bytes[ix] {
            b'%' => {
                let value = bytes
                    .get(ix + 1)
                    .and_then(|b| hex(*b))
                    .zip(bytes.get(ix + 2).and_then(|b| hex(*b)))
                    .map(|(a, b)| a * 16 + b)
                    .ok_or_else(|| format!("第 {} 字节处不是有效的 %XX 编码", ix + 1))?;
                if mode == Mode::Whole && RESERVED.contains(&value) {
                    out.extend_from_slice(&bytes[ix..ix + 3]);
                } else {
                    out.push(value);
                }
                ix += 3;
            }
            b'+' if mode == Mode::Form => {
                out.push(b' ');
                ix += 1;
            }
            b => {
                out.push(b);
                ix += 1;
            }
        }
    }
    String::from_utf8(out).map_err(|_| "解码失败：编码内容不是有效 UTF-8 文本".into())
}

// Keep the original converter entry point for the toolbox's shared conversion tests.
#[cfg(test)]
pub fn convert(text: &str, reverse: bool) -> Result<String, String> {
    if reverse {
        decode(text, Mode::Component)
    } else {
        Ok(encode(text, Mode::Component))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn modes_distinguish_plus_space_and_url_delimiters() {
        for mode in Mode::ALL {
            let text = "中文 a+b&x=1/#?%";
            assert_eq!(decode(&encode(text, mode), mode).unwrap(), text);
        }
        assert_eq!(encode("a b+c&d", Mode::Component), "a%20b%2Bc%26d");
        assert_eq!(encode("a b+c&d", Mode::Form), "a+b%2Bc%26d");
        assert_eq!(
            encode("https://x.test/中文?q=a b#片段", Mode::Whole),
            "https://x.test/%E4%B8%AD%E6%96%87?q=a%20b#%E7%89%87%E6%AE%B5"
        );
        assert_eq!(
            decode("%26%3F%23%E4%B8%AD", Mode::Whole).unwrap(),
            "%26%3F%23中"
        );
        for text in ["%", "%GG", "%A", "%FF", "%E4%B8"] {
            assert!(decode(text, Mode::Component).is_err());
        }
    }
}
