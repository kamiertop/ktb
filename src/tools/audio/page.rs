use super::logic::{self, FIELDS, Info};
use crate::ui::assets::IconName;
use crate::ui::{button::Button, panels::eyebrow, theme};
use gpui_kit::component::{
    Icon,
    input::{Input, InputState},
};
use gpui_kit::*;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};
pub struct Page {
    info: Option<Arc<Info>>,
    cover: Option<Arc<Image>>,
    new_cover: Option<PathBuf>,
    fields: Vec<Entity<InputState>>,
    output: Entity<InputState>,
    busy: bool,
    failed: bool,
    message: String,
    saved: Option<PathBuf>,
}
impl Page {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let fields = FIELDS
            .iter()
            .map(|(_, label)| cx.new(|cx| InputState::new(window, cx).placeholder(*label)))
            .collect();
        Self {
            info: None,
            cover: None,
            new_cover: None,
            fields,
            output: cx.new(|cx| InputState::new(window, cx).placeholder("另存为路径")),
            busy: false,
            failed: false,
            message: String::new(),
            saved: None,
        }
    }
    fn choose(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.busy = true;
        cx.notify();
        let prompt = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("选择音频（MP3 / M4A / FLAC）".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = match prompt.await {
                Ok(Ok(Some(paths))) => {
                    if let Some(path) = paths.into_iter().next() {
                        Some(
                            cx.background_executor()
                                .spawn(async move { logic::read(&path) })
                                .await,
                        )
                    } else {
                        None
                    }
                }
                Ok(Ok(None)) => None,
                _ => Some(Err("无法打开文件选择器".into())),
            };
            let _ = this.update_in(cx, |this, window, cx| {
                this.busy = false;
                if let Some(result) = result {
                    match result {
                        Ok(info) => {
                            for ((key, _), state) in FIELDS.iter().zip(&this.fields) {
                                state.update(cx, |s, cx| {
                                    s.set_value(logic::value(&info.tags, key), window, cx)
                                });
                            }
                            this.output.update(cx, |s, cx| {
                                s.set_value(
                                    logic::suggested(&info.path).to_string_lossy().into_owned(),
                                    window,
                                    cx,
                                )
                            });
                            this.new_cover = None;
                            this.cover = info
                                .cover
                                .clone()
                                .map(|bytes| Arc::new(Image::from_bytes(ImageFormat::Png, bytes)));
                            this.info = Some(Arc::new(info));
                            this.failed = false;
                            this.message.clear();
                            this.saved = None;
                        }
                        Err(error) => {
                            this.failed = true;
                            this.message = error;
                        }
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }
    fn choose_cover(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy || self.info.is_none() {
            return;
        }
        let prompt = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("选择音频封面（JPG / PNG）".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = prompt.await;
            let _ = this.update(cx, |this, cx| {
                if this.busy {
                    return;
                }
                match result {
                    Ok(Ok(Some(paths))) => {
                        if let Some(path) = paths.into_iter().next() {
                            let ext = path
                                .extension()
                                .and_then(|s| s.to_str())
                                .unwrap_or("")
                                .to_lowercase();
                            if matches!(ext.as_str(), "jpg" | "jpeg" | "png") && path.is_file() {
                                this.new_cover = Some(path);
                                this.message.clear();
                                this.saved = None;
                            } else {
                                this.failed = true;
                                this.message = "封面支持 JPG / PNG 图片".into();
                            }
                        }
                    }
                    Ok(Ok(None)) => {}
                    _ => {
                        this.failed = true;
                        this.message = "无法打开文件选择器".into();
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }
    fn choose_output(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let path = PathBuf::from(self.output.read(cx).value().as_str());
        let prompt = cx.prompt_for_new_path(
            path.parent().unwrap_or(Path::new(".")),
            path.file_name().and_then(|v| v.to_str()),
        );
        cx.spawn_in(window, async move |this, cx| {
            let result = prompt.await;
            let _ = this.update_in(cx, |this, window, cx| {
                if this.busy {
                    return;
                }
                match result {
                    Ok(Ok(Some(path))) => {
                        if let Some(info) = &this.info {
                            let path =
                                path.with_extension(info.path.extension().unwrap_or_default());
                            this.output.update(cx, |s, cx| {
                                s.set_value(path.to_string_lossy().into_owned(), window, cx)
                            });
                        }
                    }
                    Ok(Ok(None)) => {}
                    _ => {
                        this.failed = true;
                        this.message = "无法打开保存对话框".into();
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }
    fn save(&mut self, overwrite: bool, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let Some(info) = self.info.clone() else {
            return;
        };
        let edits: BTreeMap<_, _> = FIELDS
            .iter()
            .zip(&self.fields)
            .map(|((key, _), state)| (key.to_string(), state.read(cx).value().to_string()))
            .collect();
        let output = if overwrite {
            info.path.clone()
        } else {
            PathBuf::from(self.output.read(cx).value().as_str())
        };
        if output.as_os_str().is_empty() {
            self.failed = true;
            self.message = "请选择另存位置".into();
            cx.notify();
            return;
        }
        self.busy = true;
        self.saved = None;
        self.failed = false;
        self.message = "正在保存音频…".into();
        let cover = self.new_cover.clone();
        cx.notify();
        let work = cx.background_executor().spawn(async move {
            let result = logic::save(&info, &edits, &output, overwrite, cover.as_deref());
            let refreshed = if result.is_ok() && overwrite {
                logic::read(&output).ok()
            } else {
                None
            };
            (output, result, refreshed)
        });
        cx.spawn(async move |this, cx| {
            let (output, result, refreshed) = work.await;
            let _ = this.update(cx, |this, cx| {
                this.busy = false;
                match result {
                    Ok(()) => {
                        this.message = format!("已保存：{}", output.display());
                        this.saved = Some(output);
                        this.failed = false;
                        if let Some(info) = refreshed {
                            this.cover = info
                                .cover
                                .clone()
                                .map(|bytes| Arc::new(Image::from_bytes(ImageFormat::Png, bytes)));
                            this.new_cover = None;
                            this.info = Some(Arc::new(info));
                        }
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
}
impl Render for Page {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let disabled = self.busy || self.info.is_none();
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .gap_2()
                            .items_center()
                            .child(Icon::new(IconName::FileMusic))
                            .child(
                                div().min_w_0().text_ellipsis().child(
                                    self.info
                                        .as_ref()
                                        .map(|i| {
                                            i.path
                                                .file_name()
                                                .unwrap_or_default()
                                                .to_string_lossy()
                                                .into_owned()
                                        })
                                        .unwrap_or("未选择音频".into()),
                                ),
                            ),
                    )
                    .child(
                        Button::new("choose", "选择音频")
                            .icon(IconName::FolderOpen)
                            .disabled(self.busy)
                            .on_click(cx.listener(|this, _, w, cx| this.choose(w, cx))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap_6()
                    .child(
                        div()
                            .w(px(216.))
                            .flex_shrink_0()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(eyebrow("封面 / 文件信息"))
                            .child(
                                div()
                                    .size(px(216.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .bg(rgb(theme::SIDEBAR))
                                    .border_1()
                                    .border_color(rgb(theme::LINE))
                                    .rounded_lg()
                                    .overflow_hidden()
                                    .children(
                                        self.new_cover
                                            .clone()
                                            .map(ImageSource::from)
                                            .or_else(|| self.cover.clone().map(ImageSource::from))
                                            .map(|image| {
                                                img(image)
                                                    .size_full()
                                                    .object_fit(ObjectFit::Contain)
                                            }),
                                    )
                                    .children(
                                        (self.cover.is_none() && self.new_cover.is_none()).then(
                                            || {
                                                div()
                                                    .flex()
                                                    .flex_col()
                                                    .items_center()
                                                    .gap_2()
                                                    .child(Icon::new(IconName::Music).size(px(36.)))
                                                    .child(eyebrow("无内嵌封面"))
                                            },
                                        ),
                                    ),
                            )
                            .child(eyebrow(if self.new_cover.is_some() {
                                "新封面（待保存）"
                            } else {
                                "原封面"
                            }))
                            .child(
                                Button::new("choose-cover", "选择新封面")
                                    .icon(IconName::Image)
                                    .disabled(disabled)
                                    .on_click(
                                        cx.listener(|this, _, w, cx| this.choose_cover(w, cx)),
                                    ),
                            )
                            .children(self.new_cover.is_some().then(|| {
                                Button::new("reset-cover", "恢复原封面")
                                    .disabled(disabled)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.new_cover = None;
                                        cx.notify();
                                    }))
                            }))
                            .children(self.info.as_ref().map(|info| {
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .text_size(px(theme::LABEL_SIZE))
                                    .text_color(theme::muted())
                                    .child(format!(
                                        "{} · {} · {:.1} MiB",
                                        info.codec,
                                        info.duration,
                                        info.bytes as f64 / 1048576.
                                    ))
                                    .child(
                                        div()
                                            .text_ellipsis()
                                            .child(info.path.to_string_lossy().into_owned()),
                                    )
                            })),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .grid()
                            .grid_cols(2)
                            .gap_3()
                            .children(FIELDS.iter().zip(&self.fields).map(
                                |((_, label), state)| {
                                    div()
                                        .flex()
                                        .flex_col()
                                        .min_w_0()
                                        .gap_2()
                                        .child(eyebrow(label))
                                        .child(
                                            Input::new(state)
                                                .disabled(disabled)
                                                .font_weight(theme::FONT_WEIGHT)
                                                .text_size(px(theme::EDITOR_SIZE)),
                                        )
                                },
                            )),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .pt_3()
                    .border_t_1()
                    .border_color(rgb(theme::LINE))
                    .child(eyebrow("另存为"))
                    .child(
                        div().flex_1().min_w_0().child(
                            Input::new(&self.output)
                                .disabled(disabled)
                                .font_weight(theme::FONT_WEIGHT),
                        ),
                    )
                    .child(
                        Button::new("destination", "保存位置")
                            .icon(IconName::FolderOpen)
                            .disabled(disabled)
                            .on_click(cx.listener(|this, _, w, cx| this.choose_output(w, cx))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        Button::new("save-as", "另存为新文件")
                            .primary()
                            .disabled(disabled)
                            .on_click(cx.listener(|this, _, _, cx| this.save(false, cx))),
                    )
                    .child(
                        Button::new("overwrite", "覆盖原文件")
                            .disabled(disabled)
                            .on_click(cx.listener(|this, _, _, cx| this.save(true, cx))),
                    )
                    .children(self.saved.clone().map(|path| {
                        Button::new("reveal", "打开位置")
                            .icon(IconName::FolderOpen)
                            .on_click(move |_, _, cx| cx.reveal_path(&path))
                    })),
            )
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
