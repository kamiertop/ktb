use super::theme;
use gpui_kit::*;
pub fn eyebrow(text: &'static str) -> Div {
    div()
        .font_weight(theme::FONT_WEIGHT)
        .text_size(px(theme::LABEL_SIZE))
        .text_color(theme::muted())
        .child(text)
}
