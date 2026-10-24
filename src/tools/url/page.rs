use super::logic::{self, Mode};
use crate::ui::{button::Button, panels::eyebrow, theme};
use gpui_kit::component::{
    input::{Textarea, TextareaState},
    tab::{Tab, TabBar},
};
use gpui_kit::*;

pub struct Page {
    input: Entity<TextareaState>,
    output: Entity<TextareaState>,
    mode: Mode,
    status: String,
    failed: bool,
}
impl Page {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            input: cx.new(|cx| {
                TextareaState::new(window, cx).placeholder("输入完整 URL 或需要编解码的文本")
            }),
            output: cx.new(|cx| TextareaState::new(window, cx)),
            mode: Mode::default(),
            status: String::new(),
            failed: false,
        }
    }

    fn transcode(&mut self, reverse: bool, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.input.read(cx).value().to_string();
        let result = if reverse {
            logic::decode(&text, self.mode)
        } else {
            Ok(logic::encode(&text, self.mode))
        };
        match result {
            Ok(result) => {
                self.output
                    .update(cx, |state, cx| state.set_value(result, window, cx));
                self.status = if reverse {
                    "解码完成"
                } else {
                    "编码完成"
                }
                .into();
                self.failed = false;
            }
            Err(error) => {
                self.output
                    .update(cx, |state, cx| state.set_value("", window, cx));
                self.status = error;
                self.failed = true;
            }
        }
        cx.notify();
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
            .child(
                div()
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .flex_wrap()
                    .child(
                        TabBar::new("url-mode")
                            .segmented()
                            .selected_index(
                                Mode::ALL
                                    .iter()
                                    .position(|mode| *mode == self.mode)
                                    .unwrap_or(0),
                            )
                            .children(
                                Mode::ALL
                                    .into_iter()
                                    .map(|mode| Tab::new().label(mode.label())),
                            )
                            .on_click(cx.listener(|this, index: &usize, _, cx| {
                                this.mode = Mode::ALL[*index];
                                this.status.clear();
                                cx.notify();
                            })),
                    )
                    .child(Button::clear("clear-url").on_click(cx.listener(
                        |this, _, window, cx| {
                            for state in [&this.input, &this.output] {
                                state.update(cx, |state, cx| state.set_value("", window, cx));
                            }
                            this.status.clear();
                            this.failed = false;
                            cx.notify();
                        },
                    ))),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .text_size(px(theme::LABEL_SIZE))
                    .text_color(theme::muted())
                    .child(self.mode.hint()),
            )
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .gap_4()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .gap_2()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .h_9()
                                    .flex_shrink_0()
                                    .child(eyebrow("输入")),
                            )
                            .child(
                                Textarea::new(&self.input)
                                    .flex_1()
                                    .min_h_0()
                                    .text_size(px(theme::EDITOR_SIZE)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .gap_2()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .h_9()
                                    .flex_shrink_0()
                                    .child(eyebrow("结果"))
                                    .child(Button::new("copy-url-result", "复制结果").on_click(
                                        cx.listener(|this, _, _, cx| {
                                            cx.write_to_clipboard(ClipboardItem::new_string(
                                                this.output.read(cx).value().to_string(),
                                            ));
                                        }),
                                    )),
                            )
                            .child(
                                Textarea::new(&self.output)
                                    .flex_1()
                                    .min_h_0()
                                    .readonly(true)
                                    .text_size(px(theme::EDITOR_SIZE)),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_shrink_0()
                    .gap_2()
                    .child(Button::new("url-encode", "编码").primary().on_click(
                        cx.listener(|this, _, window, cx| this.transcode(false, window, cx)),
                    ))
                    .child(Button::new("url-decode", "解码").on_click(
                        cx.listener(|this, _, window, cx| this.transcode(true, window, cx)),
                    )),
            )
            .children((!self.status.is_empty()).then(|| {
                div()
                    .flex_shrink_0()
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
    use super::{Mode, Page};
    use gpui_kit::{TestAppContext, px, size};

    #[gpui_kit::test]
    fn textareas_align_and_complete_urls_keep_delimiters_by_default(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let (page, cx) = cx.add_window_view(Page::new);
        cx.simulate_resize(size(px(1200.), px(900.)));
        cx.update(|window, cx| {
            page.update(cx, |page, cx| {
                let source = "http://www.kamier.top";
                page.input
                    .update(cx, |state, cx| state.set_value(source, window, cx));
                assert!(page.mode == Mode::Whole);
                page.transcode(false, window, cx);
                assert_eq!(page.output.read(cx).value(), source);
                page.mode = Mode::Component;
                page.transcode(false, window, cx);
                assert_eq!(page.output.read(cx).value(), "http%3A%2F%2Fwww.kamier.top");
            });
            window.draw(cx).clear(cx);
        });
        page.read_with(cx, |page, cx| {
            let input = page.input.read(cx).input_bounds();
            let output = page.output.read(cx).input_bounds();
            assert!(
                input.size.height > px(320.),
                "editor should fill available height: {input:?}"
            );
            assert!((input.top() - output.top()).abs() < px(1.));
            assert!((input.size.height - output.size.height).abs() < px(1.));
            assert!((input.size.width - output.size.width).abs() < px(1.));
        });
        let previous = page.read_with(cx, |page, cx| {
            page.input.read(cx).input_bounds().size.height
        });
        cx.simulate_resize(size(px(1200.), px(1100.)));
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        page.read_with(cx, |page, cx| {
            let input = page.input.read(cx).input_bounds();
            let output = page.output.read(cx).input_bounds();
            assert!((input.size.height - previous - px(200.)).abs() < px(1.));
            assert!((input.size.height - output.size.height).abs() < px(1.));
        });
    }
}
