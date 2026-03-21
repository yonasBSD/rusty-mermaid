mod scene_builder;

pub use scene_builder::build_vello_scene;

use rusty_mermaid_core::{Scene, Theme};
use rusty_mermaid_viewport::ViewportState;

/// Convert a rusty-mermaid Scene into a vello Scene ready for GPU rendering.
///
/// The returned `vello::Scene` can be rendered to:
/// - A wgpu surface (windowed apps, browser WASM via WebGPU)
/// - A wgpu texture (headless/offscreen rendering)
///
/// The caller owns the wgpu device/queue and vello::Renderer.
/// This crate handles only the Scene→vello::Scene translation.
pub fn render(
    scene: &Scene,
    theme: &Theme,
    viewport: &ViewportState,
) -> vello::Scene {
    build_vello_scene(scene, theme, viewport)
}
