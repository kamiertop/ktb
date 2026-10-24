use super::{ContentSurface, after_components, before_components};
use gpui_kit::{
    AnyElement, App, AppContext, Context, Empty, Global, IntoElement, ParentElement, Render,
    RenderOnce, Styled, TestAppContext, Window,
    base::{Root, RootPlugin},
    div, transparent_black,
};
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Default)]
struct Counters {
    content: Rc<Cell<usize>>,
    overlay: Rc<Cell<usize>>,
    stock: Rc<Cell<usize>>,
}
impl Global for Counters {}
struct Content;
impl Render for Content {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let count = &cx.global::<Counters>().content;
        count.set(count.get() + 1);
        div().size_full()
    }
}
struct Overlay;
impl Render for Overlay {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let count = &cx.global::<Counters>().overlay;
        count.set(count.get() + 1);
        Empty
    }
}
impl RootPlugin for Overlay {}
#[derive(IntoElement)]
struct StockPaint(Rc<Cell<usize>>);
impl RenderOnce for StockPaint {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.0.set(self.0.get() + 1);
        Empty
    }
}
struct StockDecoration;
impl Render for StockDecoration {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}
impl RootPlugin for StockDecoration {
    fn decorate(
        &self,
        surface: AnyElement,
        _: &Root,
        _: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        div()
            .child(StockPaint(cx.global::<Counters>().stock.clone()))
            .child(surface)
    }
}

#[gpui_kit::test]
fn keeps_root_content_and_overlays_without_laying_out_stock_decorations(cx: &mut TestAppContext) {
    let counts = Counters::default();
    cx.update(|cx| {
        cx.set_global(counts.clone());
        before_components(cx);
        ely::theme::Theme::init(cx);
        gpui_kit::init(cx);
        Root::register_plugin(cx, |_, _| Overlay);
        Root::register_plugin(cx, |_, _| StockDecoration);
        after_components(cx);
    });
    let (root, cx) = cx.add_window_view(|window, cx| {
        let content = cx.new(|_| Content);
        Root::new(content, window, cx).bg(transparent_black())
    });
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
    });
    assert!(counts.content.get() > 0, "application content was lost");
    assert!(counts.overlay.get() > 0, "root overlays were lost");
    assert_eq!(counts.stock.get(), 0, "stock decoration was laid out");
    root.read_with(cx, |root, cx| {
        assert!(
            root.plugin::<ContentSurface>()
                .unwrap()
                .read(cx)
                .0
                .borrow()
                .is_none()
        );
    });
}
