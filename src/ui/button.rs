use gpui_kit::component::Disableable;
// Application buttons with explicit typography instead of library defaults.
use super::config;
#[derive(Clone, Copy)]
pub enum ButtonVariant {
    Primary,
    Outline,
    Ghost,
}
use gpui_kit::component::button::{Button as ComponentButton, ButtonVariants};
use gpui_kit::*;

#[derive(IntoElement)]
pub struct Button {
    inner: ComponentButton,
    primary: bool,
}
impl Button {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            inner: ComponentButton::new(id).label(label),
            primary: false,
        }
    }
    pub fn clear(id: impl Into<ElementId>) -> Self {
        let mut button = Self::new(id, "清空内容").variant(ButtonVariant::Outline);
        button.inner = button.inner.icon(gpui_kit::component::Icon::new(
            crate::ui::assets::IconName::Eraser,
        ));
        button
    }
    pub fn icon(mut self, icon: crate::ui::assets::IconName) -> Self {
        self.inner = self.inner.icon(gpui_kit::component::Icon::new(icon));
        self
    }
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.primary = matches!(variant, ButtonVariant::Primary);
        self.inner = match variant {
            ButtonVariant::Primary => self.inner.primary(),
            ButtonVariant::Outline => self.inner.outline(),
            ButtonVariant::Ghost => self.inner.ghost(),
        };
        self
    }
    pub fn primary(self) -> Self {
        self.variant(ButtonVariant::Primary)
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.inner = self.inner.disabled(disabled);
        self
    }
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.inner = self.inner.on_click(handler);
        self
    }
}
impl RenderOnce for Button {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.inner
            .font_family(config::FONT_FAMILY)
            .text_size(px(config::LABEL_SIZE))
            .font_weight(config::FONT_WEIGHT)
            .text_color(rgb(if self.primary {
                config::ON_ACCENT
            } else {
                config::INK
            }))
    }
}
