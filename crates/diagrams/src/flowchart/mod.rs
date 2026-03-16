pub mod bridge;
pub mod ir;
pub mod parser;

use rusty_mermaid_core::{
    BBox, Color, CurveType, Point, Primitive, Scene, Style, TextAnchor, TextStyle, interpolate,
};

use bridge::LayoutResult;
use ir::StrokeType;

fn node_style() -> Style {
    Style {
        fill: Some(Color::WHITE),
        stroke: Some(Color::rgb(51, 51, 51)),
        stroke_width: Some(1.5),
        ..Default::default()
    }
}

fn edge_style(stroke: StrokeType) -> Style {
    Style {
        stroke: Some(Color::rgb(51, 51, 51)),
        stroke_width: Some(1.5),
        stroke_dasharray: match stroke {
            StrokeType::Dotted => Some(vec![3.0, 3.0]),
            StrokeType::Thick => None,
            StrokeType::Normal => None,
        },
        ..Default::default()
    }
}

fn label_style() -> TextStyle {
    TextStyle {
        fill: Some(Color::rgb(51, 51, 51)),
        ..Default::default()
    }
}

fn edge_label_style() -> TextStyle {
    TextStyle {
        font_size: 12.0,
        fill: Some(Color::rgb(51, 51, 51)),
        ..Default::default()
    }
}

/// Convert a flowchart layout result into a Scene of drawing primitives.
pub fn to_scene(layout: &LayoutResult) -> Scene {
    let mut scene = Scene::new(layout.width, layout.height);
    layout_to_scene(layout, &mut scene);
    scene
}

fn subgraph_style() -> Style {
    Style {
        fill: Some(Color::rgb(236, 236, 236)),
        stroke: Some(Color::rgb(51, 51, 51)),
        stroke_width: Some(1.0),
        ..Default::default()
    }
}

fn subgraph_label_style() -> TextStyle {
    TextStyle {
        font_size: 13.0,
        fill: Some(Color::rgb(51, 51, 51)),
        font_weight: rusty_mermaid_core::FontWeight::Bold,
        ..Default::default()
    }
}

fn layout_to_scene(layout: &LayoutResult, scene: &mut Scene) {
    // Draw subgraph boundaries first (behind nodes)
    for sg in &layout.subgraphs {
        let bbox = BBox::new(sg.x, sg.y, sg.width, sg.height);
        scene.push(Primitive::Rect {
            bbox,
            rx: 5.0,
            ry: 5.0,
            style: subgraph_style(),
        });
        if let Some(label) = &sg.label {
            // Label at top-left of subgraph boundary
            let top_y = sg.y - sg.height / 2.0;
            let left_x = sg.x - sg.width / 2.0;
            scene.push(Primitive::Text {
                position: Point::new(left_x + 8.0, top_y + 12.0),
                content: label.clone(),
                anchor: TextAnchor::Start,
                style: subgraph_label_style(),
            });
        }
    }

    for node in &layout.nodes {
        let bbox = BBox::new(node.x, node.y, node.width, node.height);
        scene.push(Primitive::Rect {
            bbox,
            rx: 3.0,
            ry: 3.0,
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
                style: edge_style(edge.stroke),
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
