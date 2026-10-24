mod sidebar;
mod state;
mod title_bar;
mod view;
mod window_frame;
pub use state::Toolbox;

use gpui_kit::*;
pub fn launch() {
    gpui_kit::application()
        .with_assets(crate::ui::assets::source())
        .run(|cx| {
            ely::init(cx).expect("无法初始化 Ely 组件资源");
            crate::ui::fonts::init(cx);
            window_frame::before_components(cx);
            init(cx);
            window_frame::after_components(cx);
            component::theme::Theme::change(component::theme::ThemeMode::Light, None, cx);
            crate::ui::theme::init(cx);
            crate::ui::theme::init_ely(cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(1200.), px(800.)),
                        cx,
                    ))),
                    window_min_size: Some(size(px(960.), px(650.))),
                    window_decorations: Some(WindowDecorations::Client),
                    window_background: WindowBackgroundAppearance::Transparent,
                    ..component::TitleBar::window_options()
                },
                |window, cx| {
                    let view = cx.new(|cx| Toolbox::new(window, cx));
                    // Override the theme's opaque root surface so rounded corners reveal the desktop.
                    cx.new(|cx| component::Root::new(view, window, cx).bg(transparent_black()))
                },
            )
            .expect("无法打开工具箱窗口");
            cx.activate(true);
        });
}
