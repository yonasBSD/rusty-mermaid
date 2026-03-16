pub mod bridge;
pub mod ir;
pub mod parser;

use rusty_mermaid_core::{
    BBox, Color, CurveType, Point, Primitive, Scene, Style, TextAnchor, TextStyle, interpolate,
};

use bridge::LayoutResult;

fn node_style() -> Style {
    Style {
        fill: Some(Color::WHITE),
        stroke: Some(Color::rgb(51, 51, 51)),
        stroke_width: Some(1.5),
        ..Default::default()
    }
}

fn label_style() -> TextStyle {
    TextStyle {
        fill: Some(Color::rgb(51, 51, 51)),
        ..Default::default()
    }
}

/// Convert a state diagram layout result into a Scene of drawing primitives.
pub fn to_scene(layout: &LayoutResult) -> Scene {
    let mut scene = Scene::new(layout.width, layout.height);
    layout_to_scene(layout, &mut scene);
    scene
}

fn edge_label_style() -> TextStyle {
    TextStyle {
        font_size: 12.0,
        fill: Some(Color::rgb(51, 51, 51)),
        ..Default::default()
    }
}

fn layout_to_scene(layout: &LayoutResult, scene: &mut Scene) {
    for node in &layout.nodes {
        let bbox = BBox::new(node.x, node.y, node.width, node.height);
        scene.push(Primitive::Rect {
            bbox,
            rx: 5.0,
            ry: 5.0,
            style: node_style(),
        });
        scene.push(Primitive::Text {
            position: Point::new(node.x, node.y),
            content: node.label.clone(),
            anchor: TextAnchor::Middle,
            style: label_style(),
        });
    }
    for edge in &layout.edges {
        if edge.points.len() >= 2 {
            let points: Vec<Point> = edge.points.iter().map(|&(x, y)| Point::new(x, y)).collect();
            let segments = interpolate(&points, CurveType::Basis);
            scene.push(Primitive::Path {
                segments,
                style: Style::default(),
                marker_start: None,
                marker_end: Some(rusty_mermaid_core::MarkerType::ArrowPoint),
            });
            if let Some(label) = &edge.label {
                let mid = &points[points.len() / 2];
                scene.push(Primitive::Text {
                    position: *mid,
                    content: label.clone(),
                    anchor: TextAnchor::Middle,
                    style: edge_label_style(),
                });
            }
        }
    }
}
