mod paint;

pub use paint::paint_scene;

use gpui::{canvas, App, Bounds, IntoElement, Pixels, Window};
use rusty_mermaid_core::{Scene, Theme};
use rusty_mermaid_viewport::ViewportState;

/// Paint a Scene directly into a gpui Window.
///
/// Use this inside a `canvas()` paint callback or a custom Element's paint method.
/// gpui does not produce a document — it paints into a live GPU-backed window.
/// Therefore this crate does NOT implement the `Renderer` trait.
pub fn render_element(
    scene: Scene,
    theme: Theme,
    viewport: ViewportState,
) -> impl IntoElement {
    canvas(
        move |_bounds: Bounds<Pixels>, _window: &mut Window, _cx: &mut App| {
            (scene, theme, viewport)
        },
        move |bounds: Bounds<Pixels>,
              (scene, theme, viewport): (Scene, Theme, ViewportState),
              window: &mut Window,
              cx: &mut App| {
            paint::paint_scene(&scene, &theme, &viewport, bounds, window, cx);
        },
    )
}
