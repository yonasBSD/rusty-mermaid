mod paint;

pub use paint::paint_scene;

use std::rc::Rc;

use gpui::{App, Bounds, IntoElement, Pixels, Window, canvas};
use rusty_mermaid_core::{Scene, Theme};
use rusty_mermaid_viewport::ViewportState;

/// Paint a Scene directly into a gpui Window.
///
/// Accepts `Rc<Scene>` — cheap to clone into the canvas callback.
/// No deep copy of primitives on each frame.
pub fn render_element(scene: Rc<Scene>, theme: Theme, viewport: ViewportState) -> impl IntoElement {
    canvas(
        move |_bounds: Bounds<Pixels>, _window: &mut Window, _cx: &mut App| {
            (scene, theme, viewport)
        },
        move |bounds: Bounds<Pixels>,
              (scene, theme, viewport): (Rc<Scene>, Theme, ViewportState),
              window: &mut Window,
              cx: &mut App| {
            paint::paint_scene(&scene, &theme, &viewport, bounds, window, cx);
        },
    )
}
