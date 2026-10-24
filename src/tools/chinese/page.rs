use super::convert;
use crate::ui::button::Button;
use crate::ui::{panels::eyebrow, theme};
use gpui_kit::component::input::{Textarea, TextareaState};
use gpui_kit::*;
pub struct Page {
    input: Entity<TextareaState>,
    output: Entity<TextareaState>,
    status: String,
    failed: bool,
}
impl Page {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            input: cx.new(|cx| {
                TextareaState::new(window, cx)
                    .rows(12)
                    .placeholder("输入内容")
            }),
            output: cx.new(|cx| TextareaState::new(window, cx).rows(12)),
            status: String::new(),
            failed: false,
        }
    }
    fn run(&mut self, reverse: bool, window: &mut Window, cx: &mut Context<Self>) {
        let value = self.input.read(cx).value().to_string();
        match convert(&value, reverse) {
            Ok(result) => {
                self.status.clear();
                self.failed = false;
                self.output
                    .update(cx, |state, cx| state.set_value(result, window, cx));
            }
            Err(error) => {
                self.status = error;
                self.failed = true;
                self.output
                    .update(cx, |state, cx| state.set_value("", window, cx));
            }
        }
        cx.notify();
    }

    fn workspace(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (primary, secondary) = ("转为繁体", "转为简体");
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .gap_4()
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .gap_4()
                    .items_stretch()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .gap_2()
                            .child(eyebrow("输入"))
                            .child(
                                Textarea::new(&self.input)
                                    .flex_1()
                                    .min_h_0()
                                    .text_size(px(theme::EDITOR_SIZE))
                                    .font_weight(theme::FONT_WEIGHT),
                            )
                            .child(div().flex().gap_2().child(Button::clear("clear").on_click(
                                cx.listener(|this, _, window, cx| {
                                    this.input.update(cx, |s, cx| s.set_value("", window, cx));
                                    this.output.update(cx, |s, cx| s.set_value("", window, cx));
                                    this.failed = false;
                                    this.status.clear();
                                    cx.notify();
                                }),
                            ))),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .gap_2()
                            .child(eyebrow("结果"))
                            .child(
                                Textarea::new(&self.output)
                                    .flex_1()
                                    .min_h_0()
                                    .text_size(px(theme::EDITOR_SIZE))
                                    .font_weight(theme::FONT_WEIGHT)
                                    .readonly(true),
                            )
                            .child(Button::new("copy", "复制结果").on_click(cx.listener(
                                |this, _, _, cx| {
                                    cx.write_to_clipboard(ClipboardItem::new_string(
                                        this.output.read(cx).value().to_string(),
                                    ));
                                    this.status = "结果已复制".into();
                                    this.failed = false;
                                    cx.notify();
                                },
                            ))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_shrink_0()
                    .gap_3()
                    .child(
                        Button::new("convert", primary)
                            .primary()
                            .on_click(cx.listener(|this, _, w, cx| this.run(false, w, cx))),
                    )
                    .child(
                        Button::new("reverse", secondary)
                            .on_click(cx.listener(|this, _, w, cx| this.run(true, w, cx))),
                    ),
            )
    }
}
impl Render for Page {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .gap_4()
            .child(self.workspace(cx))
            .children((!self.status.is_empty()).then(|| {
                div()
                    .px_4()
                    .flex_shrink_0()
                    .py_3()
                    .rounded_lg()
                    .bg(rgb(if self.failed {
                        theme::ERROR_BACKGROUND
                    } else {
                        theme::SUCCESS_BACKGROUND
                    }))
                    .text_size(px(theme::LABEL_SIZE))
                    .text_color(rgb(if self.failed {
                        theme::ERROR
                    } else {
                        theme::ACCENT
                    }))
                    .child(self.status.clone())
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::Page;
    use gpui_kit::{
        AppContext, Context, Entity, IntoElement, ParentElement, Render, Styled, TestAppContext,
        Window, div, px, size,
    };

    struct Preview(Entity<Page>);
    impl Render for Preview {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .flex()
                .flex_col()
                .child(div().h(px(120.)).flex_shrink_0())
                .child(self.0.clone())
        }
    }

    #[gpui_kit::test]
    fn both_textareas_grow_together_when_window_height_increases(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let (view, cx) =
            cx.add_window_view(|window, cx| Preview(cx.new(|cx| Page::new(window, cx))));
        let page = view.read_with(cx, |view, _| view.0.clone());
        let mut heights = Vec::new();
        for height in [800., 1200.] {
            cx.simulate_resize(size(px(1200.), px(height)));
            cx.update(|window, cx| {
                window.draw(cx).clear(cx);
            });
            let bounds = page.read_with(cx, |page, cx| {
                (
                    page.input.read(cx).input_bounds(),
                    page.output.read(cx).input_bounds(),
                )
            });
            assert!((bounds.0.size.height - bounds.1.size.height).abs() < px(1.));
            assert!(bounds.0.bottom() < px(height));
            heights.push(bounds.0.size.height);
        }
        assert!(
            (heights[1] - heights[0] - px(400.)).abs() < px(1.),
            "textareas must use the extra window height: {heights:?}"
        );
    }
}
