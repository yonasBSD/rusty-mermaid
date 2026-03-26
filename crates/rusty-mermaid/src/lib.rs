//! # rusty-mermaid
//!
//! Mermaid diagram rendering in pure Rust.
//!
//! The base crate parses mermaid syntax and produces a `Scene` — a backend-agnostic
//! intermediate representation of primitives (rects, paths, text, etc.).
//!
//! Enable features to add rendering backends:
//!
//! | Feature    | What you get                          |
//! |------------|---------------------------------------|
//! | `svg`      | `to_svg()` → SVG string               |
//! | `raster`   | `to_png()` → PNG bytes                 |
//! | `wgpu`     | vello/WebGPU scene builder             |
//! | `gpui`     | gpui canvas element (Zed)              |
//! | `viewport` | Pan/zoom state + coordinate transforms |

// ── Always available: parse → Scene ──

pub use rusty_mermaid_core::{
    BBox, Color, Direction, Point, Primitive, Scene, Style, TextAnchor, TextStyle, Theme,
};
pub use rusty_mermaid_diagrams::{DiagramKind, ParseError, detect, render_to_scene, render_to_scene_themed};

/// Parse and render a mermaid diagram to a `Scene`.
pub fn render(input: &str) -> Result<Scene, ParseError> {
    render_to_scene(input)
}

/// Parse and render with a custom theme.
pub fn render_themed(input: &str, theme: &Theme) -> Result<Scene, ParseError> {
    render_to_scene_themed(input, theme)
}

// ── Feature: svg ──

#[cfg(feature = "svg")]
pub fn to_svg(input: &str) -> Result<String, ParseError> {
    use rusty_mermaid_core::Renderer;
    let scene = render(input)?;
    Ok(rusty_mermaid_svg::SvgRenderer::new().render(&scene))
}

#[cfg(feature = "svg")]
pub fn to_svg_themed(input: &str, theme: &Theme) -> Result<String, ParseError> {
    let scene = render_themed(input, theme)?;
    Ok(rusty_mermaid_svg::SvgRenderer::with_theme(theme).render_themed(&scene, theme))
}

#[cfg(feature = "svg")]
pub mod svg {
    pub use rusty_mermaid_svg::*;
}

// ── Feature: raster ──

#[cfg(feature = "raster")]
pub fn to_png(input: &str, dpi: f64) -> Result<Vec<u8>, ParseError> {
    use rusty_mermaid_core::Renderer;
    let scene = render(input)?;
    let config = rusty_mermaid_raster::RasterConfig { scale: dpi, ..Default::default() };
    Ok(rusty_mermaid_raster::RasterRenderer::with_config(config).render(&scene))
}

#[cfg(feature = "raster")]
pub fn to_png_themed(input: &str, theme: &Theme, dpi: f64) -> Result<Vec<u8>, ParseError> {
    use rusty_mermaid_core::Renderer;
    let scene = render_themed(input, theme)?;
    let config = rusty_mermaid_raster::RasterConfig { scale: dpi, theme: theme.clone() };
    Ok(rusty_mermaid_raster::RasterRenderer::with_config(config).render(&scene))
}

#[cfg(feature = "raster")]
pub mod raster {
    pub use rusty_mermaid_raster::*;
}

// ── Feature: viewport ──

#[cfg(feature = "viewport")]
pub mod viewport {
    pub use rusty_mermaid_viewport::*;
}

// ── Feature: wgpu ──

#[cfg(feature = "wgpu")]
pub mod wgpu {
    pub use rusty_mermaid_wgpu::*;
}

// ── Feature: gpui ──

#[cfg(feature = "gpui")]
pub mod gpui {
    pub use rusty_mermaid_gpui::*;
}
