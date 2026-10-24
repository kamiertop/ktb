use image::{ImageFormat, Luma};
use qrcode::{Color, EcLevel, QrCode};
use std::{
    io::{Cursor, Write},
    path::Path,
};
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Correction {
    Low,
    Medium,
    Quartile,
    High,
}
impl Correction {
    pub const ALL: [Self; 4] = [Self::Low, Self::Medium, Self::Quartile, Self::High];
    pub fn label(self) -> &'static str {
        match self {
            Self::Low => "L · 7%",
            Self::Medium => "M · 15%",
            Self::Quartile => "Q · 25%",
            Self::High => "H · 30%",
        }
    }
    fn level(self) -> EcLevel {
        match self {
            Self::Low => EcLevel::L,
            Self::Medium => EcLevel::M,
            Self::Quartile => EcLevel::Q,
            Self::High => EcLevel::H,
        }
    }
}
pub fn generate(text: &str, size: u32, correction: Correction) -> Result<Vec<u8>, String> {
    if text.is_empty() {
        return Err("请输入二维码内容".into());
    }
    if ![256, 512, 1024].contains(&size) {
        return Err("不支持的图片尺寸".into());
    }
    let code = QrCode::with_error_correction_level(text.as_bytes(), correction.level())
        .map_err(|_| "内容超过二维码容量，请缩短文字或降低纠错级别")?;
    let width = code.width() as u32;
    let scale = size / (width + 8);
    if scale == 0 {
        return Err("图片尺寸不足".into());
    }
    let margin = (size - width * scale) / 2;
    let mut image = image::GrayImage::from_pixel(size, size, Luma([255]));
    for y in 0..width {
        for x in 0..width {
            if code[(x as usize, y as usize)] == Color::Dark {
                for dy in 0..scale {
                    for dx in 0..scale {
                        image.put_pixel(
                            margin + x * scale + dx,
                            margin + y * scale + dy,
                            Luma([0]),
                        );
                    }
                }
            }
        }
    }
    let mut bytes = Cursor::new(Vec::new());
    image
        .write_to(&mut bytes, ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(bytes.into_inner())
}
pub struct Decoded {
    pub texts: Vec<String>,
    pub preview: Vec<u8>,
}
pub fn decode(path: &Path) -> Result<Decoded, String> {
    if std::fs::metadata(path)
        .map_err(|e| format!("无法读取图片：{e}"))?
        .len()
        > 32 * 1024 * 1024
    {
        return Err("图片请小于 32 MiB".into());
    }
    let mut reader = image::ImageReader::open(path)
        .map_err(|e| e.to_string())?
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    let image = reader
        .decode()
        .map_err(|e| format!("图片无法解码或尺寸过大：{e}"))?;
    let mut prepared = rqrr::PreparedImage::prepare(image.to_luma8());
    let mut texts = Vec::new();
    for grid in prepared.detect_grids() {
        if let Ok((_, text)) = grid.decode() {
            texts.push(text);
        }
    }
    if texts.is_empty() {
        return Err("没有识别到有效的文本二维码，请使用清晰且完整的图片".into());
    }
    let mut preview = Cursor::new(Vec::new());
    image
        .thumbnail(400, 400)
        .write_to(&mut preview, ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(Decoded {
        texts,
        preview: preview.into_inner(),
    })
}
pub fn save(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("无法保存（不会覆盖已有文件）：{e}"))?;
    if let Err(error) = file.write_all(bytes).and_then(|_| file.sync_all()) {
        drop(file);
        let _ = std::fs::remove_file(path);
        return Err(format!("保存失败：{error}"));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn utf8_roundtrip_sizes_and_levels() {
        let dir = std::env::temp_dir().join(format!(
            "ktb-qr-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&dir).unwrap();
        for size in [256, 512, 1024] {
            for level in Correction::ALL {
                let text = "你好，KTB！https://example.com/?q=二维码\nsecond line";
                let bytes = generate(text, size, level).unwrap();
                let decoded_image = image::load_from_memory(&bytes).unwrap();
                assert_eq!(
                    (decoded_image.width(), decoded_image.height()),
                    (size, size)
                );
                let path = dir.join(format!("{size}-{}.png", level.label().as_bytes()[0]));
                save(&path, &bytes).unwrap();
                assert_eq!(decode(&path).unwrap().texts, vec![text]);
                assert!(save(&path, b"overwrite").is_err());
                assert_eq!(std::fs::read(path).unwrap(), bytes);
            }
        }
        let mut combined = image::GrayImage::from_pixel(576, 288, Luma([255]));
        let left = image::load_from_memory(&generate("left QR", 256, Correction::Medium).unwrap())
            .unwrap()
            .to_luma8();
        let right =
            image::load_from_memory(&generate("右边二维码", 256, Correction::Medium).unwrap())
                .unwrap()
                .to_luma8();
        image::imageops::replace(&mut combined, &left, 16, 16);
        image::imageops::replace(&mut combined, &right, 304, 16);
        let multiple = dir.join("multiple.png");
        combined.save(&multiple).unwrap();
        let mut texts = decode(&multiple).unwrap().texts;
        texts.sort();
        assert_eq!(texts, vec!["left QR", "右边二维码"]);
        assert!(generate("", 512, Correction::Medium).is_err());
        assert!(generate(&"x".repeat(10000), 512, Correction::High).is_err());
        let blank = dir.join("blank.png");
        image::GrayImage::from_pixel(100, 100, Luma([255]))
            .save(&blank)
            .unwrap();
        assert!(decode(&blank).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
