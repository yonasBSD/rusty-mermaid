use std::rc::Rc;

use gpui::*;
use rusty_mermaid_core::{Scene, Theme};
use rusty_mermaid_viewport::ViewportState;

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
    max_width: f32,
}

impl GalleryApp {
    fn load() -> Self {
        let theme = Theme::light();
        let padding = theme.padding;
        let mut diagrams = Vec::new();
        let mut max_width: f32 = 0.0;

        for (name, mmd) in DIAGRAMS {
            match rusty_mermaid_diagrams::render_to_scene(mmd) {
                Ok(scene) => {
                    let w = (scene.width + padding * 2.0) as f32;
                    let h = (scene.height + padding * 2.0) as f32;
                    max_width = max_width.max(w);
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
        eprintln!("Loaded {count} diagrams, max width {max_width:.0}px");

        Self {
            diagrams,
            list_state: ListState::new(
                count,
                ListAlignment::Top,
                px(200.0), // overdraw: render 200px beyond visible area
            ),
            max_width,
        }
    }
}

impl Render for GalleryApp {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::light();
        let viewport = ViewportState::default();
        let max_w = self.max_width;
        let diagrams = &self.diagrams;

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0xf5f5f5))
            .font_family(rusty_mermaid_core::font_fallback::PRIMARY_FONT)
            .child(
                div()
                    .px_4().py_2()
                    .bg(rgb(0xffffff))
                    .border_b_1()
                    .border_color(rgb(0xeeeeee))
                    .flex_shrink_0()
                    .child(
                        div().text_sm().text_color(rgb(0x9370db))
                            .child(format!("rusty-mermaid gpui gallery — {} diagrams", diagrams.len()))
                    )
            )
            .child(
                list(self.list_state.clone(), {
                    let diagrams: Vec<_> = diagrams.iter().map(|d| {
                        (d.name.clone(), d.scene.clone(), d.width, d.height)
                    }).collect();
                    let theme = theme.clone();
                    let viewport = viewport.clone();

                    move |idx, _window: &mut Window, _cx: &mut App| {
                        let (name, scene, w, h) = &diagrams[idx];
                        div()
                            .id(ElementId::NamedInteger("card".into(), idx as u64))
                            .p_4()
                            .overflow_x_scroll()
                            .child(
                                div().text_sm().text_color(rgb(0x555555)).mb_1()
                                    .child(name.clone())
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
                                    ))
                            )
                            .into_any_element()
                    }
                })
                .flex_1()
            )
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
