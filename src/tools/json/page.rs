use super::logic::{self, Indent};
use crate::ui::{button::Button, panels::eyebrow, theme};
use gpui_kit::component::{
    input::{Editor, EditorState},
    tab::{Tab, TabBar},
};
use gpui_kit::*;

pub struct Page {
    input: Entity<EditorState>,
    output: Entity<EditorState>,
    indent: Indent,
    compressed: bool,
    status: String,
    failed: bool,
}
impl Page {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut editor = || {
            cx.new(|cx| {
                EditorState::new(window, cx)
                    .language("json")
                    .line_number(true)
                    .folding(true)
                    .indent_guides(true)
                    .soft_wrap(false)
                    .tab_size(Indent::default().editor_tab())
            })
        };
        Self {
            input: editor(),
            output: editor(),
            indent: Indent::default(),
            compressed: false,
            status: String::new(),
            failed: false,
        }
    }
    fn run(&mut self, compress: bool, window: &mut Window, cx: &mut Context<Self>) {
        let value = self.input.read(cx).value().to_string();
        match logic::format(&value, compress, self.indent) {
            Ok(result) => {
                self.status.clear();
                self.failed = false;
                self.compressed = compress;
                self.output
                    .update(cx, |s, cx| s.set_value(result, window, cx));
            }
            Err(error) => {
                self.status = error;
                self.failed = true;
                self.output.update(cx, |s, cx| s.set_value("", window, cx));
            }
        }
        cx.notify();
    }
    fn set_indent(&mut self, indent: Indent, window: &mut Window, cx: &mut Context<Self>) {
        self.indent = indent;
        for editor in [&self.input, &self.output] {
            editor.update(cx, |s, cx| s.set_tab_size(indent.editor_tab(), cx));
        }
        if !self.compressed
            && let Ok(result) = logic::format(&self.output.read(cx).value(), false, indent)
        {
            self.output
                .update(cx, |s, cx| s.set_value(result, window, cx));
        }
        cx.notify();
    }
}
impl Render for Page {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let height = px((f32::from(window.viewport_size().height) - 285.).clamp(320., 900.));
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .flex_wrap()
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                Button::new("format", "格式化")
                                    .primary()
                                    .on_click(cx.listener(|this, _, w, cx| this.run(false, w, cx))),
                            )
                            .child(
                                Button::new("compress", "压缩")
                                    .on_click(cx.listener(|this, _, w, cx| this.run(true, w, cx))),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(eyebrow("缩进"))
                            .child(
                                TabBar::new("json-indent")
                                    .segmented()
                                    .selected_index(
                                        Indent::ALL
                                            .iter()
                                            .position(|v| *v == self.indent)
                                            .unwrap_or(0),
                                    )
                                    .children(
                                        Indent::ALL
                                            .into_iter()
                                            .map(|indent| Tab::new().label(indent.label())),
                                    )
                                    .on_click(cx.listener(|this, index: &usize, w, cx| {
                                        this.set_indent(Indent::ALL[*index], w, cx)
                                    })),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_4()
                    .items_start()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .gap_2()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(eyebrow("输入"))
                                    .child(Button::clear("clear").on_click(cx.listener(
                                        |this, _, w, cx| {
                                            this.input.update(cx, |s, cx| s.set_value("", w, cx));
                                            this.output.update(cx, |s, cx| s.set_value("", w, cx));
                                            this.status.clear();
                                            this.failed = false;
                                            cx.notify();
                                        },
                                    ))),
                            )
                            .child(json_editor(&self.input, height, false)),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .gap_2()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(eyebrow("结果"))
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
                            )
                            .child(json_editor(&self.output, height, true)),
                    ),
            )
            .children((!self.status.is_empty()).then(|| {
                div()
                    .text_size(px(theme::LABEL_SIZE))
                    .text_color(rgb(if self.failed {
                        theme::ERROR
                    } else {
                        theme::INK
                    }))
                    .child(self.status.clone())
            }))
    }
}

fn json_editor(state: &Entity<EditorState>, height: Pixels, readonly: bool) -> Editor {
    Editor::new(state)
        .h(height)
        .font_family(theme::CODE_FONT_FAMILY)
        .text_size(px(theme::JSON_EDITOR_SIZE))
        .font_weight(FontWeight::NORMAL)
        .bg(rgb(theme::JSON_EDITOR_BACKGROUND))
        .text_color(rgb(theme::JSON_EDITOR_FOREGROUND))
        .readonly(readonly)
}

#[cfg(test)]
mod tests {
    use gpui_kit::{
        AppContext, Context, Entity, IntoElement, Modifiers, ParentElement, Render, Styled,
        TestAppContext, Window,
        base::input::{EditorState, FoldRange, InputEditorStyle},
        canvas, div, point, px,
    };
    use std::{cell::Cell, rc::Rc};

    struct FoldPreview(Entity<EditorState>);
    impl Render for FoldPreview {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(self.0.clone())
        }
    }

    #[gpui_kit::test]
    fn expanded_fold_arrows_paint_without_hover_or_cursor(cx: &mut TestAppContext) {
        let paints = Rc::new(Cell::new(0));
        let counter = paints.clone();
        let (_, cx) = cx.add_window_view(|window, cx| {
            let editor = cx.new(|cx| EditorState::new(window, cx).line_number(true).folding(true));
            editor.update(cx, |editor, cx| {
                editor.set_value("{\n    \"a\": {\n        \"x\": 1\n    },\n    \"b\": {\n        \"y\": 2\n    }\n}", window, cx);
                editor.apply_highlighter_fold_candidates(
                    vec![FoldRange::new(1, 3), FoldRange::new(4, 6)], cx,
                );
                editor.set_editor_style(InputEditorStyle {
                    fold_icon_renderer: Some(Rc::new(move |_, _| {
                        let counter = counter.clone();
                        canvas(|_, _, _| {}, move |_, _, _, _| counter.set(counter.get() + 1))
                            .size(px(14.)).into_any_element()
                    })),
                    ..Default::default()
                });
            });
            FoldPreview(editor)
        });
        cx.simulate_mouse_move(point(px(-100.), px(-100.)), None, Modifiers::default());
        paints.set(0);
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        assert_eq!(
            paints.get(),
            2,
            "both expanded blocks must paint their arrows without hover"
        );
    }
}
