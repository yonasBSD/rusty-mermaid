pub mod document;
pub mod markers;
pub mod path;
pub mod primitive;
pub mod style;

use rusty_mermaid_core::{Renderer, Scene};

use document::SvgDocument;
use markers::marker_defs;
use primitive::{collect_markers, render_primitive};

/// SVG rendering backend. Converts a Scene to an SVG string.
pub struct SvgRenderer;

impl Renderer for SvgRenderer {
    type Output = String;

    fn render(&self, scene: &Scene) -> String {
        let padding = 20.0;
        let w = scene.width + padding * 2.0;
        let h = scene.height + padding * 2.0;

        let mut doc = SvgDocument::new(w, h);

        // Emit marker defs if any paths use markers
        let markers = collect_markers(&scene.primitives);
        if !markers.is_empty() {
            doc.open_tag("defs", &[]);
            doc.raw(&marker_defs(&markers));
            doc.close_tag("defs");
        }

        // Wrap everything in a group with padding offset
        let tx = document::fmt_f64(padding);
        let ty = document::fmt_f64(padding);
        let transform = format!("translate({tx}, {ty})");
        doc.open_tag("g", &[("transform", &transform)]);

        for prim in &scene.primitives {
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
        let svg = SvgRenderer.render(&scene);
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

        let svg = SvgRenderer.render(&scene);
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
            style: Style::default(),
            marker_start: None,
            marker_end: Some(MarkerType::ArrowPoint),
        });

        let svg = SvgRenderer.render(&scene);
        assert!(svg.contains("<defs>"));
        assert!(svg.contains("arrow-point"));
        assert!(svg.contains("marker-end"));
    }

    #[test]
    fn render_includes_padding() {
        let scene = Scene::new(100.0, 50.0);
        let svg = SvgRenderer.render(&scene);
        // Width should be 100 + 40 = 140, height 50 + 40 = 90
        assert!(svg.contains(r#"width="140""#));
        assert!(svg.contains(r#"height="90""#));
    }
}
