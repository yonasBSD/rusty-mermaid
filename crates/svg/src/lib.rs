pub mod document;
pub mod markers;
pub mod path;
pub mod primitive;
pub mod style;

use rusty_mermaid_core::{Color, Renderer, Scene};

use document::SvgDocument;
use markers::marker_defs;
use primitive::{collect_markers, render_primitive};

/// SVG-specific rendering configuration.
#[derive(Debug, Clone)]
pub struct SvgConfig {
    /// Padding around the diagram (pixels on each side).
    pub padding: f64,
    /// Color for arrow markers. If None, derives from first path stroke or falls back to #333.
    pub marker_color: Option<Color>,
}

impl Default for SvgConfig {
    fn default() -> Self {
        Self {
            padding: 20.0,
            marker_color: None,
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

    /// Derive marker color from config, or from the first marker-bearing path's stroke.
    fn resolve_marker_color(&self, scene: &Scene) -> String {
        if let Some(c) = &self.config.marker_color {
            return c.to_string();
        }
        // Derive from the first path that actually uses markers.
        for prim in scene.primitives() {
            if let rusty_mermaid_core::Primitive::Path {
                style,
                marker_start,
                marker_end,
                ..
            } = prim
            {
                if marker_start.is_some() || marker_end.is_some() {
                    if let Some(c) = &style.stroke {
                        return c.to_string();
                    }
                }
            }
        }
        "#333333".to_string()
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

        // Emit marker defs if any paths use markers
        let markers = collect_markers(scene.primitives());
        if !markers.is_empty() {
            let color = self.resolve_marker_color(scene);
            doc.open_tag("defs", &[]);
            doc.raw(&marker_defs(&markers, &color));
            doc.close_tag("defs");
        }

        // Wrap everything in a group with padding offset
        let tx = document::fmt_f64(padding);
        let ty = document::fmt_f64(padding);
        let transform = format!("translate({tx}, {ty})");
        doc.open_tag("g", &[("transform", &transform)]);

        for prim in scene.primitives() {
            render_primitive(&mut doc, prim);
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
        assert!(svg.contains("arrow-point"));
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
    fn marker_color_from_config() {
        let mut scene = Scene::new(200.0, 100.0);
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
        let renderer = SvgRenderer::with_config(SvgConfig {
            marker_color: Some(Color::rgb(0, 128, 0)),
            ..Default::default()
        });
        let svg = renderer.render(&scene);
        assert!(svg.contains("#008000"), "marker should use config color");
    }

    #[test]
    fn marker_color_derived_from_path_stroke() {
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
        assert!(svg.contains("#9370db"), "marker should derive color from path stroke");
    }
}
