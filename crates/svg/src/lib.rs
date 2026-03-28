//! SVG rendering backend for rusty-mermaid.
//!
//! Converts a [`Scene`] into an SVG string by
//! walking each primitive and emitting the corresponding SVG element. Marker
//! definitions are generated per-color so arrow heads match their edge stroke.
//!
//! Implements the [`Renderer`] trait from core
//! (`Output = String`).
//!
//! # Key types
//!
//! * [`SvgRenderer`] -- the rendering backend.
//! * [`SvgConfig`] -- padding, default stroke color/width.
//!   Use [`SvgConfig::from_theme`] to derive settings from a
//!   [`Theme`].
//!
//! [`SvgRenderer::render_themed`] adds a background `<rect>` when the theme
//! background is not white (e.g. dark themes).
//!
//! # Examples
//!
//! ```
//! use rusty_mermaid_core::{Renderer, Scene};
//! use rusty_mermaid_svg::SvgRenderer;
//!
//! let scene = Scene::new(200.0, 100.0);
//! let svg: String = SvgRenderer::new().render(&scene);
//! assert!(svg.contains("<svg"));
//! ```

pub mod document;
pub mod markers;
pub mod path;
pub mod primitive;
pub mod style;

use rusty_mermaid_core::{Color, Renderer, Scene, Theme};

use document::SvgDocument;
use markers::marker_defs;
use primitive::{collect_marker_colors, render_primitive};

/// SVG-specific rendering configuration.
#[derive(Debug, Clone)]
pub struct SvgConfig {
    /// Padding around the diagram (pixels on each side).
    pub padding: f64,
    /// Default stroke color for paths/arcs that don't specify one.
    pub default_stroke: Color,
    /// Default stroke width for paths that don't specify one.
    pub default_stroke_width: f64,
}

impl Default for SvgConfig {
    fn default() -> Self {
        Self::from_theme(&Theme::default())
    }
}

impl SvgConfig {
    pub fn from_theme(theme: &Theme) -> Self {
        Self {
            padding: theme.padding,
            default_stroke: theme.edge_stroke,
            default_stroke_width: theme.default_stroke_width,
        }
    }
}

/// SVG rendering backend. Converts a Scene to an SVG string.
pub struct SvgRenderer {
    pub config: SvgConfig,
}

impl SvgRenderer {
    pub fn new() -> Self {
        Self {
            config: SvgConfig::default(),
        }
    }

    pub fn with_config(config: SvgConfig) -> Self {
        Self { config }
    }

    pub fn with_theme(theme: &Theme) -> Self {
        Self {
            config: SvgConfig::from_theme(theme),
        }
    }

    /// Render a scene with theme-derived config and optional background rect.
    pub fn render_themed(&self, scene: &Scene, theme: &Theme) -> String {
        let padding = self.config.padding;
        let w = scene.width + padding * 2.0;
        let h = scene.height + padding * 2.0;

        let mut doc = SvgDocument::new(w, h);

        // Emit per-color marker defs
        let marker_colors = collect_marker_colors(scene.elements(), &self.config);
        if !marker_colors.is_empty() {
            doc.open_tag("defs", &[]);
            doc.raw(&marker_defs(&marker_colors));
            doc.close_tag("defs");
        }

        // Background rect (for dark theme or non-white backgrounds)
        if theme.background != Color::WHITE {
            let bg_hex = format!(
                "#{:02x}{:02x}{:02x}",
                theme.background.r, theme.background.g, theme.background.b
            );
            doc.open_tag(
                "rect",
                &[("width", "100%"), ("height", "100%"), ("fill", &bg_hex)],
            );
            doc.close_tag("rect");
        }

        let tx = document::fmt_f64(padding);
        let ty = document::fmt_f64(padding);
        let transform = format!("translate({tx}, {ty})");
        doc.open_tag("g", &[("transform", &transform)]);

        for elem in scene.elements() {
            render_primitive(&mut doc, &elem.primitive, &self.config);
        }

        doc.close_tag("g");
        doc.finish()
    }
}

impl Default for SvgRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer for SvgRenderer {
    type Output = String;

    fn render(&self, scene: &Scene) -> String {
        let padding = self.config.padding;
        let w = scene.width + padding * 2.0;
        let h = scene.height + padding * 2.0;

        let mut doc = SvgDocument::new(w, h);

        // Emit per-color marker defs
        let marker_colors = collect_marker_colors(scene.elements(), &self.config);
        if !marker_colors.is_empty() {
            doc.open_tag("defs", &[]);
            doc.raw(&marker_defs(&marker_colors));
            doc.close_tag("defs");
        }

        // Wrap everything in a group with padding offset
        let tx = document::fmt_f64(padding);
        let ty = document::fmt_f64(padding);
        let transform = format!("translate({tx}, {ty})");
        doc.open_tag("g", &[("transform", &transform)]);

        for elem in scene.elements() {
            render_primitive(&mut doc, &elem.primitive, &self.config);
        }

        doc.close_tag("g");
        doc.finish()
    }
}

#[cfg(test)]
mod tests {
    use rusty_mermaid_core::*;

    use super::*;

    #[test]
    fn render_empty_scene() {
        let scene = Scene::new(100.0, 100.0);
        let svg = SvgRenderer::new().render(&scene);
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("viewBox"));
    }

    #[test]
    fn render_scene_with_rect_and_text() {
        let mut scene = Scene::new(200.0, 100.0);
        scene.push(Primitive::Rect {
            bbox: BBox::new(100.0, 50.0, 80.0, 40.0),
            rx: 0.0,
            ry: 0.0,
            style: Style {
                fill: Some(Color::WHITE),
                stroke: Some(Color::BLACK),
                stroke_width: Some(1.0),
                ..Default::default()
            },
        });
        scene.push(Primitive::Text {
            position: Point::new(100.0, 50.0),
            content: "Hello".into(),
            anchor: TextAnchor::Middle,
            style: TextStyle::default(),
        });

        let svg = SvgRenderer::new().render(&scene);
        assert!(svg.contains("<rect"));
        assert!(svg.contains("<text"));
        assert!(svg.contains("Hello"));
    }

    #[test]
    fn render_scene_with_path_marker() {
        let mut scene = Scene::new(200.0, 100.0);
        scene.push(Primitive::Path {
            segments: vec![
                PathSegment::MoveTo(Point::new(10.0, 50.0)),
                PathSegment::LineTo(Point::new(190.0, 50.0)),
            ],
            style: Style {
                stroke: Some(Color::rgb(51, 51, 51)),
                ..Default::default()
            },
            marker_start: None,
            marker_end: Some(MarkerType::ArrowPoint),
        });

        let svg = SvgRenderer::new().render(&scene);
        assert!(svg.contains("<defs>"));
        assert!(svg.contains("arrow-point-333333"));
        assert!(svg.contains("marker-end"));
    }

    #[test]
    fn render_includes_padding() {
        let scene = Scene::new(100.0, 50.0);
        let svg = SvgRenderer::new().render(&scene);
        // Width should be 100 + 40 = 140, height 50 + 40 = 90
        assert!(svg.contains(r#"width="140""#));
        assert!(svg.contains(r#"height="90""#));
    }

    #[test]
    fn custom_padding() {
        let scene = Scene::new(100.0, 50.0);
        let renderer = SvgRenderer::with_config(SvgConfig {
            padding: 10.0,
            ..Default::default()
        });
        let svg = renderer.render(&scene);
        assert!(svg.contains(r#"width="120""#));
        assert!(svg.contains(r#"height="70""#));
    }

    #[test]
    fn per_color_marker_defs() {
        let mut scene = Scene::new(200.0, 200.0);
        scene.push(Primitive::Path {
            segments: vec![
                PathSegment::MoveTo(Point::new(0.0, 0.0)),
                PathSegment::LineTo(Point::new(100.0, 0.0)),
            ],
            style: Style {
                stroke: Some(Color::rgb(255, 0, 0)),
                ..Default::default()
            },
            marker_start: None,
            marker_end: Some(MarkerType::ArrowPoint),
        });
        scene.push(Primitive::Path {
            segments: vec![
                PathSegment::MoveTo(Point::new(0.0, 50.0)),
                PathSegment::LineTo(Point::new(100.0, 50.0)),
            ],
            style: Style {
                stroke: Some(Color::rgb(0, 128, 0)),
                ..Default::default()
            },
            marker_start: None,
            marker_end: Some(MarkerType::ArrowPoint),
        });
        let svg = SvgRenderer::new().render(&scene);
        assert!(svg.contains("arrow-point-ff0000"), "red marker def");
        assert!(svg.contains("arrow-point-008000"), "green marker def");
    }

    #[test]
    fn marker_color_matches_edge_stroke() {
        let mut scene = Scene::new(200.0, 100.0);
        scene.push(Primitive::Path {
            segments: vec![
                PathSegment::MoveTo(Point::new(0.0, 0.0)),
                PathSegment::LineTo(Point::new(100.0, 0.0)),
            ],
            style: Style {
                stroke: Some(Color::rgb(147, 112, 219)),
                ..Default::default()
            },
            marker_start: None,
            marker_end: Some(MarkerType::ArrowPoint),
        });
        let svg = SvgRenderer::new().render(&scene);
        assert!(
            svg.contains("arrow-point-9370db"),
            "marker ID should include edge color"
        );
        assert!(svg.contains(r#"marker-end="url(#arrow-point-9370db)""#));
    }
}
