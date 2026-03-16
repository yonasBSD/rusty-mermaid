pub mod bridge;
pub mod ir;
pub mod parser;

use rusty_mermaid_core::{BBox, PathSegment, Point, Primitive, Scene, Style, TextAnchor, TextStyle};

use bridge::LayoutResult;

/// Convert a flowchart layout result into a Scene of drawing primitives.
pub fn to_scene(layout: &LayoutResult) -> Scene {
    let mut scene = Scene::new(layout.width, layout.height);
    layout_to_scene(layout, &mut scene);
    scene
}

fn layout_to_scene(layout: &LayoutResult, scene: &mut Scene) {
    for node in &layout.nodes {
        let bbox = BBox::new(
            node.x - node.width / 2.0,
            node.y - node.height / 2.0,
            node.width,
            node.height,
        );
        scene.push(Primitive::Rect {
            bbox,
            rx: 0.0,
            ry: 0.0,
            style: Style::default(),
        });
        scene.push(Primitive::Text {
            position: Point::new(node.x, node.y),
            content: node.id.clone(),
            anchor: TextAnchor::Middle,
            style: TextStyle::default(),
        });
    }
    for edge in &layout.edges {
        if edge.points.len() >= 2 {
            let mut segments = Vec::with_capacity(edge.points.len());
            segments.push(PathSegment::MoveTo(Point::new(edge.points[0].0, edge.points[0].1)));
            for &(x, y) in &edge.points[1..] {
                segments.push(PathSegment::LineTo(Point::new(x, y)));
            }
            scene.push(Primitive::Path {
                segments,
                style: Style::default(),
                marker_start: None,
                marker_end: Some(rusty_mermaid_core::MarkerType::ArrowPoint),
            });
        }
    }
}
