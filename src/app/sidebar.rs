use super::Toolbox;
use crate::ui::assets::IconName;
use crate::{tools::Tool, ui::theme};
use gpui_kit::component::{
    Icon,
    sidebar::{Sidebar, SidebarMenu, SidebarMenuItem},
};
use gpui_kit::*;

impl Toolbox {
    pub(super) fn sidebar(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        Sidebar::new("tools-sidebar")
            .collapsible(false)
            .w(px(theme::NAV_WIDTH))
            .bg(rgb(theme::SIDEBAR))
            .rounded_bl(super::title_bar::window_radius(window))
            .border_r_1()
            .border_color(rgb(theme::LINE))
            .header(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .w_full()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(theme::LINE))
                    .child(
                        div()
                            .id("github-link")
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(theme::SIDEBAR_IDENTITY_SIZE))
                            .cursor_pointer()
                            .text_color(rgb(theme::INK))
                            .hover(|style| style.text_color(rgb(theme::ACCENT)))
                            .on_click(|_, _, cx| cx.open_url(theme::GITHUB_URL))
                            .child(
                                Icon::new(IconName::Github).size(px(theme::SIDEBAR_IDENTITY_SIZE)),
                            ),
                    ),
            )
            .child(
                SidebarMenu::new().children(Tool::ALL.into_iter().map(|tool| {
                    let icon = match tool {
                        Tool::Timestamp => IconName::Clock,
                        Tool::Base64 => IconName::Binary,
                        Tool::Chinese => IconName::Languages,
                        Tool::Md5 => IconName::Hash,
                        Tool::Sha256 => IconName::Shield,
                        Tool::Json => IconName::Braces,
                        Tool::Url => IconName::Link,
                        Tool::Video => IconName::FileVideoCamera,
                        Tool::Qrcode => IconName::QrCode,
                        Tool::Audio => IconName::Music,
                    };
                    SidebarMenuItem::new(tool.title())
                        .icon(icon)
                        .active(self.tool == tool)
                        .label_style(
                            StyleRefinement::default()
                                .font_weight(theme::FONT_WEIGHT)
                                .text_size(px(theme::LABEL_SIZE)),
                        )
                        .suffix(|_, _| Icon::new(IconName::ChevronRight))
                        .on_click(
                            cx.listener(move |this, _, window, cx| this.select(tool, window, cx)),
                        )
                })),
            )
    }
}
