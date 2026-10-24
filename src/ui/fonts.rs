use gpui_kit::App;
use std::borrow::Cow;

pub const BOLD_FONT: &[u8] = include_bytes!("../../assets/fonts/NotoSansCJKsc-Bold.otf");
pub const CODE_FONT: &[u8] = include_bytes!("../../assets/fonts/LiberationMono-Regular.ttf");

pub fn init(cx: &mut App) {
    cx.text_system()
        .add_fonts(vec![Cow::Borrowed(BOLD_FONT), Cow::Borrowed(CODE_FONT)])
        .expect("无法加载内置界面及等宽代码字体");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_font_spaces_match_the_width_of_code_characters() {
        let face = ttf_parser::Face::parse(CODE_FONT, 0).unwrap();
        assert!(face.is_monospaced());
        assert!(!face.is_variable());
        assert_eq!(face.weight().to_number(), 400);
        assert!(face.names().into_iter().any(|name| {
            name.name_id == ttf_parser::name_id::FAMILY
                && name.to_string().as_deref() == Some(crate::ui::config::CODE_FONT_FAMILY)
        }));
        let width = |ch| {
            face.glyph_hor_advance(face.glyph_index(ch).unwrap())
                .unwrap()
        };
        let space = width(' ');
        for ch in "iWm0\"{}[]:".chars() {
            assert_eq!(width(ch), space, "代码字符与空格宽度不一致：{ch}");
        }
    }

    #[test]
    fn bundled_font_is_bold_and_contains_ui_glyphs() {
        let face = ttf_parser::Face::parse(BOLD_FONT, 0).unwrap();
        assert_eq!(face.weight().to_number(), 700);
        assert!(!face.is_variable());
        for ch in "时间戳转换输入结果ABC123".chars() {
            assert!(face.glyph_index(ch).is_some(), "缺少字形：{ch}");
        }
    }
}
