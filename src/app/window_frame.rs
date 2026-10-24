//! Keep Kit's root surface and overlays, replacing only its stock square decoration.
//! The two public RootPlugin hooks bracket Component's decoration registration.
use gpui_kit::{
    base::{Root, RootPlugin},
    *,
};
use std::cell::RefCell;

#[derive(Default)]
struct ContentSurface(RefCell<Option<AnyElement>>);
impl Render for ContentSurface {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}
impl RootPlugin for ContentSurface {
    fn decorate(
        &self,
        surface: AnyElement,
        _: &Root,
        _: &mut Window,
        _: &mut App,
    ) -> impl IntoElement {
        *self.0.borrow_mut() = Some(surface);
        Empty
    }
}

struct WindowFrame;
impl Render for WindowFrame {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}
impl RootPlugin for WindowFrame {
    fn decorate(
        &self,
        stock_frame: AnyElement,
        root: &Root,
        _: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let content = root
            .plugin::<ContentSurface>()
            .and_then(|plugin| plugin.read(cx).0.borrow_mut().take());
        let Some(content) = content else {
            return stock_frame;
        };
        // Ely owns the rounded resize frame; Kit still owns the intact root surface.
        ely::shell::ResizeBorder::new()
            .glass()
            .child(content)
            .into_any_element()
    }
}

pub fn before_components(cx: &mut App) {
    Root::register_plugin(cx, |_, _| ContentSurface::default());
}
pub fn after_components(cx: &mut App) {
    Root::register_plugin(cx, |_, _| WindowFrame);
}

#[cfg(test)]
mod tests;
