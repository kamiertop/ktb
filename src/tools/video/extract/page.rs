use super::{
    logic::{self, Format},
    queue::{self, Status, Video},
};
use crate::ui::assets::IconName;
use crate::ui::{
    button::{Button, ButtonVariant},
    panels::eyebrow,
    theme,
};
use gpui_kit::component::{
    Icon,
    input::{Input, InputEvent, InputState},
};
use gpui_kit::*;
use std::path::PathBuf;

pub struct Page {
    videos: Vec<Video>,
    manual: Entity<InputState>,
    directory: Entity<InputState>,
    format: Format,
    busy: bool,
    message: String,
    _subscription: Subscription,
}
impl Page {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let directory = cx.new(|cx| InputState::new(window, cx).placeholder("音频输出目录"));
        let subscription = cx.subscribe(&directory, |this, _, event: &InputEvent, cx| {
            if !this.busy && matches!(event, InputEvent::Change) {
                this.reset();
                cx.notify();
            }
        });
        Self {
            _subscription: subscription,
            videos: Vec::new(),
            manual: cx
                .new(|cx| InputState::new(window, cx).placeholder("也可输入视频完整路径后添加")),
            directory,
            format: Format::Mp3,
            busy: false,
            message: String::new(),
        }
    }
    fn append(&mut self, videos: Vec<Video>, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        for video in videos {
            if !self.videos.iter().any(|v| v.path == video.path) {
                self.videos.push(video);
            }
        }
        if self.directory.read(cx).value().is_empty()
            && let Some(parent) = self.videos.first().and_then(|v| v.path.parent())
        {
            self.directory.update(cx, |s, cx| {
                s.set_value(parent.to_string_lossy().into_owned(), window, cx)
            });
        }
        self.message.clear();
        cx.notify();
    }
    fn add_paths(&mut self, paths: Vec<PathBuf>, window: &mut Window, cx: &mut Context<Self>) {
        let work = cx
            .background_executor()
            .spawn(async move { queue::inspect(paths) });
        cx.spawn_in(window, async move |this, cx| {
            let videos = work.await;
            let _ = this.update_in(cx, |this, window, cx| this.append(videos, window, cx));
        })
        .detach();
    }
    fn choose(&mut self, directory: bool, window: &mut Window, cx: &mut Context<Self>) {
        let prompt = cx.prompt_for_paths(PathPromptOptions {
            files: !directory,
            directories: directory,
            multiple: !directory,
            prompt: Some(
                if directory {
                    "选择输出目录"
                } else {
                    "选择视频（可多选）"
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
                    Ok(Ok(Some(paths))) if directory => {
                        if let Some(path) = paths.first() {
                            this.directory.update(cx, |s, cx| {
                                s.set_value(path.to_string_lossy().into_owned(), window, cx)
                            });
                            this.reset();
                        }
                    }
                    Ok(Ok(Some(paths))) => this.add_paths(paths, window, cx),
                    Ok(Ok(None)) => {}
                    _ => this.message = "无法打开文件选择器，可使用路径输入框".into(),
                }
                cx.notify();
            });
        })
        .detach();
    }
    fn reset(&mut self) {
        for video in &mut self.videos {
            video.status = Status::Pending;
        }
        self.message.clear();
    }
    fn start(&mut self, cx: &mut Context<Self>) {
        if self.busy || self.videos.is_empty() {
            return;
        }
        let paths: Vec<_> = self
            .videos
            .iter()
            .enumerate()
            .filter(|(_, v)| !matches!(v.status, Status::Done { .. }))
            .map(|(i, v)| (i, v.path.clone()))
            .collect();
        if paths.is_empty() {
            return;
        }
        let directory = PathBuf::from(self.directory.read(cx).value().as_str());
        if directory.as_os_str().is_empty() {
            self.message = "请选择输出目录".into();
            cx.notify();
            return;
        }
        let format = self.format;
        self.busy = true;
        self.message.clear();
        cx.notify();
        let executor = cx.background_executor().clone();
        cx.spawn(async move |this, cx| {
            let planned = executor
                .spawn(async move { queue::plan(paths, &directory, format) })
                .await;
            let jobs = match planned {
                Ok(jobs) => jobs,
                Err(error) => {
                    let _ = this.update(cx, |this, cx| {
                        this.busy = false;
                        this.message = error;
                        cx.notify();
                    });
                    return;
                }
            };
            let total = jobs.len();
            let mut succeeded = 0;
            for (position, (index, input, output)) in jobs.into_iter().enumerate() {
                if this
                    .update(cx, |this, cx| {
                        this.videos[index].status = Status::Running;
                        this.message = format!("正在提取 {}/{}", position + 1, total);
                        cx.notify();
                    })
                    .is_err()
                {
                    return;
                }
                let result = executor
                    .spawn(async move {
                        let result = logic::extract(&input, &output, format);
                        (output, result)
                    })
                    .await;
                let (output, result) = result;
                if result.is_ok() {
                    succeeded += 1;
                }
                if this
                    .update(cx, |this, cx| {
                        this.videos[index].status = match result {
                            Ok(bytes) => Status::Done { output, bytes },
                            Err(error) => Status::Failed(error),
                        };
                        cx.notify();
                    })
                    .is_err()
                {
                    return;
                }
            }
            let _ = this.update(cx, |this, cx| {
                this.busy = false;
                this.message = format!("完成：成功 {succeeded}，失败 {}", total - succeeded);
                cx.notify();
            });
        })
        .detach();
    }
}
impl Render for Page {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let total_bytes: u64 = self.videos.iter().filter_map(|v| v.bytes).sum();
        let remaining = self
            .videos
            .iter()
            .any(|v| !matches!(v.status, Status::Done { .. }));
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(eyebrow("视频列表"))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                Button::new("add-videos", "添加视频")
                                    .icon(IconName::FolderOpen)
                                    .disabled(self.busy)
                                    .on_click(
                                        cx.listener(|this, _, w, cx| this.choose(false, w, cx)),
                                    ),
                            )
                            .child(
                                Button::clear("clear-list")
                                    .disabled(self.busy || self.videos.is_empty())
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.videos.clear();
                                        this.message.clear();
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
            .child(
                div()
                    .id("video-list")
                    .flex()
                    .flex_col()
                    .max_h(px(320.))
                    .overflow_y_scroll()
                    .rounded_lg()
                    .border_1()
                    .border_color(rgb(theme::LINE))
                    .bg(rgb(theme::SURFACE))
                    .children(self.videos.is_empty().then(|| {
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .p_5()
                            .child(Icon::new(IconName::FileVideoCamera).size(px(28.)))
                            .child("选择一个或多个视频文件")
                    }))
                    .children(self.videos.iter().enumerate().map(|(index, video)| {
                        let name = video
                            .path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned();
                        let (label, color) = match &video.status {
                            Status::Pending => ("待提取".into(), theme::MUTED),
                            Status::Running => ("提取中…".into(), theme::NAV_SELECTED_TEXT),
                            Status::Done { bytes, .. } => {
                                (format!("已完成 · {}", queue::size(*bytes)), theme::ACCENT)
                            }
                            Status::Failed(_) => ("失败".into(), theme::ERROR),
                        };
                        div()
                            .flex()
                            .flex_col()
                            .p_3()
                            .gap_1()
                            .border_b_1()
                            .border_color(rgb(theme::LINE))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .child(Icon::new(IconName::FileVideoCamera))
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .flex()
                                            .flex_col()
                                            .gap_1()
                                            .child(div().text_ellipsis().child(name))
                                            .child(
                                                div()
                                                    .text_size(px(theme::LABEL_SIZE))
                                                    .text_color(theme::muted())
                                                    .text_ellipsis()
                                                    .child(
                                                        video.path.to_string_lossy().into_owned(),
                                                    ),
                                            ),
                                    )
                                    .child(
                                        div().w(px(90.)).text_size(px(theme::LABEL_SIZE)).child(
                                            video
                                                .bytes
                                                .map(queue::size)
                                                .unwrap_or("无法读取大小".into()),
                                        ),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(theme::LABEL_SIZE))
                                            .text_color(rgb(color))
                                            .child(label),
                                    )
                                    .children(if let Status::Done { output, .. } = &video.status {
                                        let path = output.clone();
                                        Some(
                                            Button::new(("reveal", index), "打开位置")
                                                .icon(IconName::FolderOpen)
                                                .on_click(move |_, _, cx| cx.reveal_path(&path)),
                                        )
                                    } else {
                                        None
                                    })
                                    .child(
                                        Button::new(("remove", index), "移除")
                                            .variant(ButtonVariant::Ghost)
                                            .disabled(self.busy)
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.videos.remove(index);
                                                cx.notify();
                                            })),
                                    ),
                            )
                            .children(if let Status::Failed(error) = &video.status {
                                Some(
                                    div()
                                        .text_size(px(theme::LABEL_SIZE))
                                        .text_color(rgb(theme::ERROR))
                                        .child(error.clone()),
                                )
                            } else {
                                None
                            })
                    })),
            )
            .child(
                div()
                    .text_size(px(theme::LABEL_SIZE))
                    .text_color(theme::muted())
                    .child(format!(
                        "{} 个视频 · 合计 {}",
                        self.videos.len(),
                        queue::size(total_bytes)
                    )),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div().flex_1().min_w_0().child(
                            Input::new(&self.manual)
                                .disabled(self.busy)
                                .font_weight(theme::FONT_WEIGHT),
                        ),
                    )
                    .child(
                        Button::new("add-path", "添加路径")
                            .disabled(self.busy)
                            .on_click(cx.listener(|this, _, w, cx| {
                                let value = this.manual.read(cx).value();
                                if !value.trim().is_empty() {
                                    this.add_paths(vec![PathBuf::from(value.as_str())], w, cx);
                                }
                            })),
                    ),
            )
            .child(eyebrow("输出格式"))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .children(Format::ALL.into_iter().enumerate().map(|(i, format)| {
                        Button::new(("format", i), format.label())
                            .disabled(self.busy)
                            .variant(if self.format == format {
                                ButtonVariant::Primary
                            } else {
                                ButtonVariant::Outline
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if this.format != format {
                                    this.format = format;
                                    this.reset();
                                }
                                cx.notify();
                            }))
                    })),
            )
            .child(
                div()
                    .text_size(px(theme::LABEL_SIZE))
                    .text_color(theme::muted())
                    .child(if self.format == Format::Wav {
                        "保留原内嵌封面：WAV 封面另存为 文件名.wav.cover.jpg"
                    } else {
                        "自动保留视频内嵌封面"
                    }),
            )
            .child(eyebrow("输出目录"))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div().flex_1().min_w_0().child(
                            Input::new(&self.directory)
                                .disabled(self.busy)
                                .font_weight(theme::FONT_WEIGHT),
                        ),
                    )
                    .child(
                        Button::new("directory", "选择目录")
                            .icon(IconName::FolderOpen)
                            .disabled(self.busy)
                            .on_click(cx.listener(|this, _, w, cx| this.choose(true, w, cx))),
                    ),
            )
            .child(
                Button::new(
                    "start",
                    if self.busy {
                        "正在提取…"
                    } else {
                        "开始批量提取"
                    },
                )
                .icon(IconName::Music)
                .primary()
                .disabled(self.busy || !remaining)
                .on_click(cx.listener(|this, _, _, cx| this.start(cx))),
            )
            .children((!self.message.is_empty()).then(|| {
                div()
                    .text_size(px(theme::LABEL_SIZE))
                    .child(self.message.clone())
            }))
    }
}
