use super::logic;
use crate::ui::assets::IconName;
use crate::ui::{button::Button, panels::eyebrow, theme};
use gpui_kit::component::{
    Icon,
    input::{Input, InputEvent, InputState},
};
use gpui_kit::*;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

pub struct Page {
    audio: Entity<InputState>,
    cover: Entity<InputState>,
    output: Entity<InputState>,
    busy: bool,
    error: bool,
    message: String,
    saved: Option<PathBuf>,
    _subscriptions: Vec<Subscription>,
    original: Option<Arc<Image>>,
    original_status: String,
    preview_generation: u64,
}
impl Page {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let audio = cx.new(|cx| InputState::new(window, cx).placeholder("视频路径（MP4 / M4V）"));
        let cover = cx.new(|cx| InputState::new(window, cx).placeholder("封面路径（JPG / PNG）"));
        let subscriptions = vec![
            cx.subscribe(&audio, |this, _, event: &InputEvent, cx| {
                if matches!(
                    event,
                    InputEvent::Change | InputEvent::Blur | InputEvent::PressEnter { .. }
                ) && !this.busy
                {
                    this.saved = None;
                    this.load_original(cx);
                }
            }),
            cx.subscribe(&cover, |this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    this.saved = None;
                    cx.notify();
                }
            }),
        ];
        Self {
            original: None,
            original_status: "未选择文件".into(),
            preview_generation: 0,
            audio,
            cover,
            output: cx.new(|cx| InputState::new(window, cx).placeholder("另存为视频路径")),
            busy: false,
            error: false,
            message: String::new(),
            saved: None,
            _subscriptions: subscriptions,
        }
    }
    fn validate_video(path: &Path) -> Result<(), String> {
        match logic::extension(path)?.as_str() {
            "mp4" | "m4v" => Ok(()),
            _ => Err("请选择 MP4 / M4V 视频；音频封面请到音频工具中编辑".into()),
        }
    }
    fn load_original(&mut self, cx: &mut Context<Self>) {
        let audio = PathBuf::from(self.audio.read(cx).value().as_str());
        self.preview_generation += 1;
        let generation = self.preview_generation;
        self.original = None;
        if audio.as_os_str().is_empty() {
            self.original_status = "未选择文件".into();
            cx.notify();
            return;
        }
        if let Err(error) = Self::validate_video(&audio) {
            self.original_status = error;
            cx.notify();
            return;
        }
        self.original_status = "读取中…".into();
        cx.notify();
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(250))
                .await;
            if !this
                .update(cx, |this, _| this.preview_generation == generation)
                .unwrap_or(false)
            {
                return;
            }
            let result = cx
                .background_executor()
                .spawn(async move { logic::read_cover(&audio) })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.preview_generation != generation {
                    return;
                }
                match result {
                    Ok(Some(bytes)) => {
                        this.original = Some(Arc::new(Image::from_bytes(ImageFormat::Png, bytes)));
                        this.original_status.clear();
                    }
                    Ok(None) => this.original_status = "无内嵌封面".into(),
                    Err(error) => this.original_status = error,
                }
                cx.notify();
            });
        })
        .detach();
    }
    fn choose(&mut self, cover: bool, window: &mut Window, cx: &mut Context<Self>) {
        let prompt = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(
                if cover {
                    "选择封面图片"
                } else {
                    "选择视频"
                }
                .into(),
            ),
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = prompt.await;
            let _ = this.update_in(cx, |this, window, cx| {
                if this.busy {
                    return;
                }
                match result {
                    Ok(Ok(Some(paths))) => {
                        if let Some(path) = paths.first() {
                            if !cover {
                                if let Err(error) = Self::validate_video(path) {
                                    this.message = error;
                                    this.error = true;
                                    cx.notify();
                                    return;
                                }
                                let output = logic::suggested(path);
                                this.output.update(cx, |s, cx| {
                                    s.set_value(output.to_string_lossy().into_owned(), window, cx)
                                });
                            }
                            let state = if cover { &this.cover } else { &this.audio };
                            state.update(cx, |s, cx| {
                                s.set_value(path.to_string_lossy().into_owned(), window, cx)
                            });
                            if !cover {
                                this.load_original(cx);
                            }
                            this.saved = None;
                            this.message.clear();
                        }
                    }
                    Ok(Ok(None)) => {}
                    _ => {
                        this.error = true;
                        this.message = "文件选择器不可用，可直接输入路径".into();
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }
    fn choose_output(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut output = PathBuf::from(self.output.read(cx).value().as_str());
        if output.as_os_str().is_empty() {
            output = logic::suggested(Path::new(self.audio.read(cx).value().as_str()));
        }
        let prompt = cx.prompt_for_new_path(
            output.parent().unwrap_or(Path::new(".")),
            output.file_name().and_then(|v| v.to_str()),
        );
        cx.spawn_in(window, async move |this, cx| {
            let result = prompt.await;
            let _ = this.update_in(cx, |this, window, cx| {
                if this.busy {
                    return;
                }
                match result {
                    Ok(Ok(Some(path))) => {
                        let ext = PathBuf::from(this.audio.read(cx).value().as_str());
                        let path = path.with_extension(ext.extension().unwrap_or_default());
                        this.output.update(cx, |s, cx| {
                            s.set_value(path.to_string_lossy().into_owned(), window, cx)
                        });
                    }
                    Ok(Ok(None)) => {}
                    _ => {
                        this.message = "保存对话框不可用，可直接输入路径".into();
                        this.error = true;
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
        let audio = PathBuf::from(self.audio.read(cx).value().as_str());
        if let Err(error) = Self::validate_video(&audio) {
            self.error = true;
            self.message = error;
            cx.notify();
            return;
        }
        let cover = PathBuf::from(self.cover.read(cx).value().as_str());
        let output = if overwrite {
            audio.clone()
        } else {
            PathBuf::from(self.output.read(cx).value().as_str())
        };
        if output.as_os_str().is_empty() {
            self.message = "请选择另存位置".into();
            self.error = true;
            cx.notify();
            return;
        }
        self.busy = true;
        self.saved = None;
        self.error = false;
        self.message = "正在写入封面…".into();
        cx.notify();
        let work = cx.background_executor().spawn(async move {
            let result = if overwrite {
                logic::overwrite_cover(&audio, &cover)
            } else {
                logic::write_cover(&audio, &cover, &output)
            };
            (output, result)
        });
        cx.spawn(async move |this, cx| {
            let (output, result) = work.await;
            let _ = this.update(cx, |this, cx| {
                this.busy = false;
                match result {
                    Ok(_) => {
                        this.message = format!("已保存：{}", output.display());
                        this.saved = Some(output);
                        this.error = false;
                        if overwrite {
                            this.load_original(cx);
                        }
                    }
                    Err(error) => {
                        this.message = error;
                        this.error = true;
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }
}
impl Page {
    fn preview(
        &self,
        title: &'static str,
        height: Pixels,
        source: Option<ImageSource>,
        empty: String,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .gap_2()
            .child(eyebrow(title))
            .child(
                div()
                    .w_full()
                    .h(height)
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_lg()
                    .overflow_hidden()
                    .bg(rgb(theme::SIDEBAR))
                    .border_1()
                    .border_color(rgb(theme::LINE))
                    .children(
                        source
                            .clone()
                            .map(|source| img(source).size_full().object_fit(ObjectFit::Contain)),
                    )
                    .children(source.is_none().then(|| {
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_2()
                            .p_3()
                            .child(Icon::new(IconName::Image).size(px(36.)))
                            .child(div().text_size(px(theme::LABEL_SIZE)).child(empty))
                    })),
            )
    }
}
impl Render for Page {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let preview_height = px((f32::from(window.viewport_size().height)
            - theme::COVER_PAGE_CONTROLS_HEIGHT
            - 48.)
            .clamp(
                theme::COVER_PREVIEW_MIN_HEIGHT,
                theme::COVER_PREVIEW_MAX_HEIGHT,
            ));
        let cover = PathBuf::from(self.cover.read(cx).value().as_str());
        let media_path = PathBuf::from(self.audio.read(cx).value().as_str());
        let video = matches!(
            media_path
                .extension()
                .and_then(|v| v.to_str())
                .map(str::to_lowercase)
                .as_deref(),
            Some("mp4" | "m4v")
        );
        let has_audio = !self.audio.read(cx).value().is_empty();
        let has_cover = !cover.as_os_str().is_empty();
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .gap_2()
                    .items_center()
                    .child(Icon::new(if video {
                        IconName::FileVideoCamera
                    } else {
                        IconName::FileMusic
                    }))
                    .child(
                        div().flex_1().min_w_0().child(
                            Input::new(&self.audio)
                                .disabled(self.busy)
                                .font_weight(theme::FONT_WEIGHT),
                        ),
                    )
                    .child(
                        Button::new("audio", "选择视频")
                            .icon(IconName::FolderOpen)
                            .disabled(self.busy)
                            .on_click(cx.listener(|this, _, w, cx| this.choose(false, w, cx))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_4()
                    .child(self.preview(
                        "原封面",
                        preview_height,
                        self.original.clone().map(ImageSource::from),
                        self.original_status.clone(),
                    ))
                    .child(self.preview(
                        "新封面",
                        preview_height,
                        has_cover.then(|| ImageSource::from(cover)),
                        "未选择新封面".into(),
                    )),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div().flex_1().min_w_0().child(
                            Input::new(&self.cover)
                                .disabled(self.busy)
                                .font_weight(theme::FONT_WEIGHT),
                        ),
                    )
                    .child(
                        Button::new("cover", "选择新封面")
                            .icon(IconName::Image)
                            .disabled(self.busy)
                            .on_click(cx.listener(|this, _, w, cx| this.choose(true, w, cx))),
                    ),
            )
            .child(eyebrow("另存为"))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div().flex_1().min_w_0().child(
                            Input::new(&self.output)
                                .disabled(self.busy)
                                .font_weight(theme::FONT_WEIGHT),
                        ),
                    )
                    .child(
                        Button::new("output", "保存位置")
                            .icon(IconName::FolderOpen)
                            .disabled(self.busy)
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
                            .disabled(self.busy || !has_audio || !has_cover)
                            .on_click(cx.listener(|this, _, _, cx| this.save(false, cx))),
                    )
                    .child(
                        Button::new("overwrite", "覆盖原文件")
                            .disabled(self.busy || !has_audio || !has_cover)
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
                    .text_color(rgb(if self.error { theme::ERROR } else { theme::INK }))
                    .child(self.message.clone())
            }))
    }
}
