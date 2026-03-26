use std::rc::Rc;

use gpui::*;
use rusty_mermaid_core::{Point, Scene, Theme};
use rusty_mermaid_viewport::{ViewportAction, ViewportState, apply};

include!(concat!(env!("OUT_DIR"), "/diagrams.rs"));

struct CachedDiagram {
    name: String,
    scene: Rc<Scene>,
    width: f32,
    height: f32,
}

struct GalleryApp {
    diagrams: Vec<CachedDiagram>,
    list_state: ListState,
    /// Currently selected diagram for single-view (None = gallery mode).
    single_idx: Option<usize>,
    viewport: ViewportState,
    dragging: bool,
    last_mouse: Option<(f64, f64)>,
}

impl GalleryApp {
    fn load() -> Self {
        let theme = Theme::light();
        let padding = theme.padding;
        let mut diagrams = Vec::new();

        for (name, mmd) in DIAGRAMS {
            match rusty_mermaid_diagrams::render_to_scene(mmd) {
                Ok(scene) => {
                    let w = (scene.width + padding * 2.0) as f32;
                    let h = (scene.height + padding * 2.0) as f32;
                    diagrams.push(CachedDiagram {
                        name: name.to_string(),
                        scene: Rc::new(scene),
                        width: w,
                        height: h,
                    });
                }
                Err(e) => eprintln!("skip {name}: {e}"),
            }
        }

        let count = diagrams.len();
        eprintln!("Loaded {count} diagrams");

        Self {
            diagrams,
            list_state: ListState::new(count, ListAlignment::Top, px(200.0)),
            single_idx: None,
            viewport: ViewportState::default(),
            dragging: false,
            last_mouse: None,
        }
    }

    fn header(&self) -> Div {
        div()
            .px_4()
            .py_2()
            .bg(rgb(0xffffff))
            .border_b_1()
            .border_color(rgb(0xeeeeee))
            .flex_shrink_0()
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x9370db))
                    .child(format!(
                        "rusty-mermaid gpui gallery — {} diagrams",
                        self.diagrams.len()
                    )),
            )
    }

    fn render_gallery(&mut self, _cx: &mut Context<Self>) -> Div {
        let theme = Theme::light();
        let viewport = ViewportState::default();
        let diagrams: Vec<_> = self
            .diagrams
            .iter()
            .map(|d| (d.name.clone(), d.scene.clone(), d.width, d.height))
            .collect();

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0xf5f5f5))
            .font_family(rusty_mermaid_core::font_fallback::PRIMARY_FONT)
            .child(self.header())
            .child(
                list(self.list_state.clone(), {
                    let theme = theme.clone();
                    let viewport = viewport.clone();

                    move |idx, _window: &mut Window, _cx: &mut App| {
                        let (name, scene, w, h) = &diagrams[idx];
                        div()
                            .id(ElementId::NamedInteger("card".into(), idx as u64))
                            .p_4()
                            .overflow_x_scroll()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0x555555))
                                    .mb_1()
                                    .child(name.clone()),
                            )
                            .child(
                                div()
                                    .bg(rgb(0xffffff))
                                    .rounded_lg()
                                    .shadow_sm()
                                    .p_4()
                                    .w(px(w + 32.0))
                                    .h(px(h + 32.0))
                                    .child(rusty_mermaid_gpui::render_element(
                                        scene.clone(),
                                        theme.clone(),
                                        viewport.clone(),
                                    )),
                            )
                            .into_any_element()
                    }
                })
                .flex_1(),
            )
    }

    fn render_single(&self, idx: usize, cx: &mut Context<Self>) -> Div {
        let d = &self.diagrams[idx];
        let theme = Theme::light();
        let viewport = self.viewport.clone();
        let zoom_pct = self.viewport.zoom * 100.0;

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0xf5f5f5))
            .font_family(rusty_mermaid_core::font_fallback::PRIMARY_FONT)
            .child(self.header())
            // Toolbar
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_4()
                    .py_1()
                    .bg(rgb(0xffffff))
                    .border_b_1()
                    .border_color(rgb(0xeeeeee))
                    .child(
                        div()
                            .id("back-btn")
                            .text_sm()
                            .text_color(rgb(0x9370db))
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.single_idx = None;
                                this.viewport = ViewportState::default();
                                cx.notify();
                            }))
                            .child("← Gallery"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x555555))
                            .child(format!("{}  (zoom: {zoom_pct:.0}%)", d.name)),
                    ),
            )
            // Canvas with mouse events
            .child(
                div()
                    .id("viewport")
                    .flex_1()
                    .overflow_hidden()
                    .cursor(CursorStyle::OpenHand)
                    .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _, cx| {
                        let dy: f64 = event.delta.pixel_delta(px(1.0)).y.into();
                        let factor = if dy > 0.0 { 1.1 } else { 1.0 / 1.1 };
                        let center = Point::new(
                            event.position.x.into(),
                            event.position.y.into(),
                        );
                        this.viewport =
                            apply(&this.viewport, &ViewportAction::Zoom { factor, center });
                        cx.notify();
                    }))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, event: &MouseDownEvent, _, _cx| {
                            this.dragging = true;
                            this.last_mouse =
                                Some((event.position.x.into(), event.position.y.into()));
                        }),
                    )
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|this, _, _, _cx| {
                            this.dragging = false;
                            this.last_mouse = None;
                        }),
                    )
                    .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                        if this.dragging {
                            if let Some((lx, ly)) = this.last_mouse {
                                let mx: f64 = event.position.x.into();
                                let my: f64 = event.position.y.into();
                                let dx = mx - lx;
                                let dy = my - ly;
                                this.viewport =
                                    apply(&this.viewport, &ViewportAction::Pan { dx, dy });
                                cx.notify();
                            }
                            this.last_mouse =
                                Some((event.position.x.into(), event.position.y.into()));
                        }
                    }))
                    .child(
                        div()
                            .w(px(d.width + 64.0))
                            .h(px(d.height + 64.0))
                            .child(rusty_mermaid_gpui::render_element(
                                d.scene.clone(),
                                theme,
                                viewport,
                            )),
                    ),
            )
    }
}

impl Render for GalleryApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(idx) = self.single_idx {
            self.render_single(idx, cx)
        } else {
            self.render_gallery(cx)
        }
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1200.0), px(900.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("rusty-mermaid gpui gallery".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| cx.new(|_| GalleryApp::load()),
        )
        .unwrap();
    });
}
