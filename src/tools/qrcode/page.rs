use super::logic::{self, Correction};
use crate::ui::assets::IconName;
use crate::ui::{button::Button, panels::eyebrow, theme};
use gpui_kit::component::{
    Icon,
    input::{InputEvent, Textarea, TextareaState},
    tab::{Tab, TabBar},
};
use gpui_kit::*;
use std::{path::Path, sync::Arc};
pub struct Page {
    input: Entity<TextareaState>,
    output: Entity<TextareaState>,
    reading: bool,
    size: u32,
    correction: Correction,
    busy: bool,
    generated: Option<Vec<u8>>,
    preview: Option<Arc<Image>>,
    decoded_preview: Option<Arc<Image>>,
    message: String,
    failed: bool,
    _subscriptions: Vec<Subscription>,
}
impl Page {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| {
            TextareaState::new(window, cx)
                .rows(8)
                .placeholder("输入文字、网址或多行内容")
        });
        let subscription = cx.subscribe(&input, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.invalidate();
                cx.notify();
            }
        });
        Self {
            input,
            output: cx.new(|cx| TextareaState::new(window, cx).rows(7)),
            reading: false,
            size: 512,
            correction: Correction::Medium,
            busy: false,
            generated: None,
            preview: None,
            decoded_preview: None,
            message: String::new(),
            failed: false,
            _subscriptions: vec![subscription],
        }
    }
    fn invalidate(&mut self) {
        self.generated = None;
        self.preview = None;
        self.message.clear();
        self.failed = false;
    }
    fn generate(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let text = self.input.read(cx).value().to_string();
        let size = self.size;
        let correction = self.correction;
        self.busy = true;
        self.invalidate();
        cx.notify();
        let work = cx
            .background_executor()
            .spawn(async move { logic::generate(&text, size, correction) });
        cx.spawn_in(window, async move |this, cx| {
            let result = work.await;
            let _ = this.update_in(cx, |this, _, cx| {
                this.busy = false;
                match result {
                    Ok(bytes) => {
                        this.preview =
                            Some(Arc::new(Image::from_bytes(ImageFormat::Png, bytes.clone())));
                        this.generated = Some(bytes);
                    }
                    Err(error) => {
                        this.failed = true;
                        this.message = error;
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }
    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let Some(bytes) = self.generated.clone() else {
            return;
        };
        self.busy = true;
        cx.notify();
        let prompt = cx.prompt_for_new_path(Path::new("."), Some("qrcode.png"));
        cx.spawn_in(window, async move |this, cx| {
            let result = match prompt.await {
                Ok(Ok(Some(path))) => {
                    let path = path.with_extension("png");
                    Some(
                        cx.background_executor()
                            .spawn(async move {
                                let result = logic::save(&path, &bytes);
                                (path, result)
                            })
                            .await,
                    )
                }
                Ok(Ok(None)) => None,
                _ => Some((Default::default(), Err("无法打开保存对话框".into()))),
            };
            let _ = this.update_in(cx, |this, _, cx| {
                this.busy = false;
                if let Some((path, result)) = result {
                    this.failed = result.is_err();
                    this.message = result
                        .err()
                        .unwrap_or_else(|| format!("已保存：{}", path.display()));
                }
                cx.notify();
            });
        })
        .detach();
    }
    fn decode(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.busy = true;
        self.message.clear();
        cx.notify();
        let prompt = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("选择二维码图片（PNG / JPG / WebP）".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = match prompt.await {
                Ok(Ok(Some(paths))) => {
                    if let Some(path) = paths.into_iter().next() {
                        Some(
                            cx.background_executor()
                                .spawn(async move { logic::decode(&path) })
                                .await,
                        )
                    } else {
                        None
                    }
                }
                Ok(Ok(None)) => None,
                _ => Some(Err("无法打开图片选择器".into())),
            };
            let _ = this.update_in(cx, |this, window, cx| {
                this.busy = false;
                if let Some(result) = result {
                    match result {
                        Ok(decoded) => {
                            this.message = format!("识别到 {} 个二维码", decoded.texts.len());
                            this.failed = false;
                            this.decoded_preview = Some(Arc::new(Image::from_bytes(
                                ImageFormat::Png,
                                decoded.preview,
                            )));
                            this.output.update(cx, |s, cx| {
                                s.set_value(decoded.texts.join("\n\n"), window, cx)
                            });
                        }
                        Err(error) => {
                            this.failed = true;
                            this.message = error;
                            this.decoded_preview = None;
                            this.output.update(cx, |s, cx| s.set_value("", window, cx));
                        }
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }
    fn image(&self, preview: &Option<Arc<Image>>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_center()
            .size(px(256.))
            .bg(rgb(theme::SURFACE))
            .border_1()
            .border_color(rgb(theme::LINE))
            .rounded_md()
            .overflow_hidden()
            .children(
                preview
                    .clone()
                    .map(|image| img(image).size_full().object_fit(ObjectFit::Contain)),
            )
            .children(preview.is_none().then(|| {
                Icon::new(IconName::QrCode)
                    .size(px(48.))
                    .text_color(theme::muted())
            }))
    }
}
impl Page {
    fn section_title(label: &'static str) -> Div {
        div().h(px(32.)).flex().items_center().child(eyebrow(label))
    }
    fn generation_form(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap_6()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .gap_2()
                            .child(Self::section_title("内容"))
                            .child(
                                Textarea::new(&self.input)
                                    .h(px(224.))
                                    .disabled(self.busy)
                                    .font_weight(theme::FONT_WEIGHT)
                                    .text_size(px(theme::EDITOR_SIZE)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_3()
                                    .mt_2()
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_2()
                                            .child(eyebrow("图片尺寸"))
                                            .child(
                                                TabBar::new("qr-size")
                                                    .segmented()
                                                    .selected_index(
                                                        [256, 512, 1024]
                                                            .iter()
                                                            .position(|size| *size == self.size)
                                                            .unwrap_or(1),
                                                    )
                                                    .children([256, 512, 1024].into_iter().map(
                                                        |size| {
                                                            Tab::new()
                                                                .label(format!("{size} px"))
                                                                .disabled(self.busy)
                                                        },
                                                    ))
                                                    .on_click(cx.listener(
                                                        |this, index: &usize, _, cx| {
                                                            let size = [256, 512, 1024][*index];
                                                            if this.size != size {
                                                                this.size = size;
                                                                this.invalidate();
                                                                cx.notify();
                                                            }
                                                        },
                                                    )),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_2()
                                            .child(eyebrow("纠错级别"))
                                            .child(
                                                TabBar::new("qr-correction")
                                                    .segmented()
                                                    .selected_index(
                                                        Correction::ALL
                                                            .iter()
                                                            .position(|level| {
                                                                *level == self.correction
                                                            })
                                                            .unwrap_or(1),
                                                    )
                                                    .children(Correction::ALL.into_iter().map(
                                                        |level| {
                                                            Tab::new()
                                                                .label(level.label())
                                                                .disabled(self.busy)
                                                        },
                                                    ))
                                                    .on_click(cx.listener(
                                                        |this, index: &usize, _, cx| {
                                                            let correction =
                                                                Correction::ALL[*index];
                                                            if this.correction != correction {
                                                                this.correction = correction;
                                                                this.invalidate();
                                                                cx.notify();
                                                            }
                                                        },
                                                    )),
                                            ),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .w(px(256.))
                            .flex_shrink_0()
                            .gap_2()
                            .child(Self::section_title("预览"))
                            .child(self.image(&self.preview)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .pt_4()
                    .border_t_1()
                    .border_color(rgb(theme::LINE))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                Button::new(
                                    "generate",
                                    if self.busy {
                                        "处理中…"
                                    } else {
                                        "生成二维码"
                                    },
                                )
                                .primary()
                                .icon(IconName::QrCode)
                                .disabled(self.busy)
                                .on_click(cx.listener(|this, _, w, cx| this.generate(w, cx))),
                            )
                            .child(
                                Button::new("save", "保存 PNG")
                                    .disabled(self.busy || self.generated.is_none())
                                    .on_click(cx.listener(|this, _, w, cx| this.save(w, cx))),
                            ),
                    )
                    .child(
                        Button::clear("clear")
                            .disabled(self.busy)
                            .on_click(cx.listener(|this, _, w, cx| {
                                this.input.update(cx, |s, cx| s.set_value("", w, cx));
                                this.invalidate();
                                cx.notify();
                            })),
                    ),
            )
    }
    fn recognition_form(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap_6()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .gap_2()
                            .child(Self::section_title("识别结果"))
                            .child(
                                Textarea::new(&self.output)
                                    .h(px(256.))
                                    .readonly(true)
                                    .font_weight(theme::FONT_WEIGHT)
                                    .text_size(px(theme::EDITOR_SIZE)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .w(px(256.))
                            .flex_shrink_0()
                            .gap_2()
                            .child(Self::section_title("图片预览"))
                            .child(self.image(&self.decoded_preview)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .pt_4()
                    .border_t_1()
                    .border_color(rgb(theme::LINE))
                    .child(
                        Button::new(
                            "decode",
                            if self.busy {
                                "识别中…"
                            } else {
                                "选择图片识别"
                            },
                        )
                        .icon(IconName::Image)
                        .primary()
                        .disabled(self.busy)
                        .on_click(cx.listener(|this, _, w, cx| this.decode(w, cx))),
                    )
                    .child(
                        Button::new("copy", "复制结果")
                            .disabled(self.output.read(cx).value().is_empty())
                            .on_click(cx.listener(|this, _, _, cx| {
                                cx.write_to_clipboard(ClipboardItem::new_string(
                                    this.output.read(cx).value().to_string(),
                                ));
                                this.message = "已复制".into();
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
            .child(
                TabBar::new("qr-mode")
                    .underline()
                    .selected_index(usize::from(self.reading))
                    .child(Tab::new().label("生成二维码").disabled(self.busy))
                    .child(Tab::new().label("识别二维码").disabled(self.busy))
                    .on_click(cx.listener(|this, index: &usize, _, cx| {
                        this.reading = *index == 1;
                        this.message.clear();
                        cx.notify();
                    })),
            )
            .children((!self.reading).then(|| self.generation_form(cx)))
            .children(self.reading.then(|| self.recognition_form(cx)))
            .children((!self.message.is_empty()).then(|| {
                div()
                    .text_size(px(theme::LABEL_SIZE))
                    .text_color(rgb(if self.failed {
                        theme::ERROR
                    } else {
                        theme::INK
                    }))
                    .child(self.message.clone())
            }))
    }
}
