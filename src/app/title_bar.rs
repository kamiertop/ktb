use crate::ui::assets::IconName;
use crate::ui::theme;
use gpui_kit::component::Icon;
use gpui_kit::component::InteractiveElementExt as _;
use gpui_kit::*;

pub(super) fn window_radius(window: &Window) -> Pixels {
    if window.is_maximized() || window.is_fullscreen() {
        px(0.)
    } else {
        px(theme::WINDOW_RADIUS)
    }
}

pub(super) fn title_bar(window: &Window) -> impl IntoElement {
    div()
        .id("title-bar")
        .flex()
        .items_center()
        .h(px(theme::TITLE_BAR_HEIGHT))
        .flex_shrink_0()
        .rounded_t(window_radius(window))
        .bg(rgb(theme::SIDEBAR))
        .text_color(rgb(theme::INK))
        .border_b_1()
        .border_color(rgb(theme::LINE))
        .child(
            div()
                .id("title-bar-drag")
                .flex()
                .items_center()
                .flex_1()
                .h_full()
                .pl_3()
                .window_control_area(WindowControlArea::Drag)
                .on_double_click(|_, window, _| window.zoom_window())
                .on_mouse_down(MouseButton::Left, |_, window, _| window.start_window_move())
                .on_mouse_down(MouseButton::Right, |event, window, _| {
                    window.show_window_menu(event.position)
                })
                .child(
                    div()
                        .text_size(px(theme::LABEL_SIZE))
                        .child("kamiertop · ToolBox"),
                ),
        )
        .child(control(
            "minimize",
            IconName::WindowMinimize,
            WindowControlArea::Min,
            px(0.),
        ))
        .child(control(
            "maximize",
            if window.is_maximized() {
                IconName::WindowRestore
            } else {
                IconName::WindowMaximize
            },
            WindowControlArea::Max,
            px(0.),
        ))
        .child(control(
            "close",
            IconName::WindowClose,
            WindowControlArea::Close,
            window_radius(window),
        ))
}

fn control(
    id: &'static str,
    icon: IconName,
    area: WindowControlArea,
    radius: Pixels,
) -> impl IntoElement {
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .w(px(theme::TITLE_BAR_HEIGHT))
        .h_full()
        .flex_shrink_0()
        .window_control_area(area)
        .rounded_tr(radius)
        .hover(|style| style.bg(rgb(theme::LINE)))
        .on_mouse_down(MouseButton::Left, |_, window, cx| {
            window.prevent_default();
            cx.stop_propagation();
        })
        .on_click(move |_, window, cx| {
            cx.stop_propagation();
            match id {
                "minimize" => window.minimize_window(),
                "maximize" => window.zoom_window(),
                _ => window.remove_window(),
            }
        })
        .child(Icon::new(icon).size(px(16.)))
}
