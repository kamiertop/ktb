use crate::tools::{self, Tool};
use gpui_kit::*;
pub struct Toolbox {
    pub(super) focus: FocusHandle,
    pub(super) tool: Tool,
    pub(super) pages: Vec<AnyView>,
}
impl Toolbox {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        let pages = vec![
            cx.new(|cx| tools::timestamp::Page::new(window, cx)).into(),
            cx.new(|cx| tools::base64::Page::new(window, cx)).into(),
            cx.new(|cx| tools::chinese::Page::new(window, cx)).into(),
            cx.new(|cx| tools::md5::Page::new(window, cx)).into(),
            cx.new(|cx| tools::sha256::Page::new(window, cx)).into(),
            cx.new(|cx| tools::json::Page::new(window, cx)).into(),
            cx.new(|cx| tools::url::Page::new(window, cx)).into(),
            cx.new(|cx| tools::qrcode::Page::new(window, cx)).into(),
            cx.new(|cx| tools::audio::Page::new(window, cx)).into(),
            cx.new(|cx| tools::video::Page::new(window, cx)).into(),
        ];
        Self {
            focus,
            tool: Tool::Timestamp,
            pages,
        }
    }
    pub(super) fn select(&mut self, tool: Tool, _: &mut Window, cx: &mut Context<Self>) {
        self.tool = tool;
        cx.notify();
    }
    pub(super) fn page(&self) -> AnyView {
        self.pages[Tool::ALL.iter().position(|v| *v == self.tool).unwrap()].clone()
    }
}
