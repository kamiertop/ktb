use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::*;

/// Video operations keep independent forms and background tasks when switching tabs.
pub struct Page {
    selected: usize,
    extraction: Entity<super::extract::Page>,
    cover: Entity<super::cover::Page>,
}
impl Page {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            selected: 0,
            extraction: cx.new(|cx| super::extract::Page::new(window, cx)),
            cover: cx.new(|cx| super::cover::Page::new(window, cx)),
        }
    }
}
impl Render for Page {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                TabBar::new("video-operation")
                    .underline()
                    .selected_index(self.selected)
                    .child(Tab::new().label("提取音频"))
                    .child(Tab::new().label("修改视频封面"))
                    .on_click(cx.listener(|this, index: &usize, _, cx| {
                        this.selected = *index;
                        cx.notify();
                    })),
            )
            .child(if self.selected == 0 {
                AnyView::from(self.extraction.clone())
            } else {
                AnyView::from(self.cover.clone())
            })
    }
}
