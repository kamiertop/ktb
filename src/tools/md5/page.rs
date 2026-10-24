use super::convert;
use crate::ui::button::Button;
use crate::ui::{panels::eyebrow, theme};
use gpui_kit::component::input::{Input, InputState, Textarea, TextareaState};
use gpui_kit::*;
pub struct Page {
    input: Entity<TextareaState>,
    output: Entity<InputState>,
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
            output: cx.new(|cx| InputState::new(window, cx)),
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
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(eyebrow("输入"))
                    .child(
                        Button::clear("clear").on_click(cx.listener(|this, _, window, cx| {
                            this.input.update(cx, |s, cx| s.set_value("", window, cx));
                            this.output.update(cx, |s, cx| s.set_value("", window, cx));
                            this.status.clear();
                            this.failed = false;
                            cx.notify();
                        })),
                    ),
            )
            .child(
                Textarea::new(&self.input)
                    .h(px(theme::TEXT_EDITOR_HEIGHT))
                    .text_size(px(theme::EDITOR_SIZE))
                    .font_weight(theme::FONT_WEIGHT),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        Button::new("generate", "生成摘要")
                            .primary()
                            .on_click(cx.listener(|this, _, w, cx| this.run(false, w, cx))),
                    )
                    .child(
                        Button::new("uppercase", "生成大写摘要")
                            .on_click(cx.listener(|this, _, w, cx| this.run(true, w, cx))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .mt_2()
                    .child(eyebrow("结果"))
                    .child(
                        div().flex_1().min_w_0().child(
                            Input::new(&self.output)
                                .readonly(true)
                                .text_size(px(theme::EDITOR_SIZE))
                                .font_weight(theme::FONT_WEIGHT),
                        ),
                    )
                    .child(
                        Button::new("copy", "复制").on_click(cx.listener(|this, _, _, cx| {
                            cx.write_to_clipboard(ClipboardItem::new_string(
                                this.output.read(cx).value().to_string(),
                            ));
                            this.status = "已复制".into();
                            this.failed = false;
                            cx.notify();
                        })),
                    ),
            )
    }
}

impl Render for Page {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(self.workspace(cx))
            .children((!self.status.is_empty()).then(|| {
                div()
                    .px_4()
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
