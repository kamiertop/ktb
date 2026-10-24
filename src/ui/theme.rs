pub use super::config::*;
use gpui_kit::*;
pub fn muted() -> Hsla {
    rgb(MUTED).into()
}

/// Apply shared typography and contrast to GPUI Component.
pub fn init(cx: &mut App) {
    cx.set_reduce_motion(REDUCE_MOTION);
    let theme = gpui_kit::component::theme::Theme::global_mut(cx);
    theme.sidebar = rgb(SIDEBAR).into();
    theme.sidebar_foreground = rgb(INK).into();
    theme.sidebar_border = rgb(LINE).into();
    theme.sidebar_accent = rgb(NAV_SELECTED).into();
    theme.sidebar_accent_foreground = rgb(NAV_SELECTED_TEXT).into();
    theme.tokens.sidebar_accent = Hsla::from(rgb(NAV_SELECTED)).into();
    theme.tokens.tab_bar_segmented = Hsla::from(rgb(SEGMENTED_BACKGROUND)).into();
    theme.highlight_theme = gpui_kit::component::highlighter::HighlightTheme::default_light();
    theme.caret = rgb(0x569cd6).into();
    theme.font_size = px(BODY_SIZE);
    theme.font_family = FONT_FAMILY.into();
    theme.mono_font_family = CODE_FONT_FAMILY.into();
    theme.foreground = rgb(INK).into();
    theme.primary_foreground = rgb(ON_ACCENT).into();
    theme.muted_foreground = rgb(MUTED).into();
}

/// Keep Ely's frame and focus surfaces aligned with the existing Kit theme.
pub fn init_ely(cx: &mut App) {
    ely::theme::Theme::set_mode_now(ely::theme::Mode::Light, cx);
    ely::theme::Theme::update(cx, |theme| {
        theme.radius_scale = WINDOW_RADIUS / 8.0;
        theme.font_family = FONT_FAMILY.into();
        theme.mono_family = CODE_FONT_FAMILY.into();
        theme.reduced_motion = REDUCE_MOTION;
        // ResizeBorder retains its geometry but draws no extra hairline around content.
        theme.colors.border_strong = transparent_black();
    });
}
