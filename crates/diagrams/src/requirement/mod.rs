pub mod bridge;
pub mod ir;
pub mod parser;

use rusty_mermaid_core::{
    BBox, CurveType, MarkerType, Point, Primitive, Scene, Style, TextAnchor, TextStyle, Theme,
    interpolate,
};

use crate::common::palette::DOTTED_PATTERN;
use crate::common::rendering::{render_edge_label, shorten_path_for_markers};
use bridge::LayoutResult;

pub fn to_scene(layout: &LayoutResult, theme: &Theme) -> Scene {
    let mut scene = Scene::new(layout.width, layout.height);
    render_edges(layout, &mut scene, theme);
    render_nodes(layout, &mut scene, theme);
    scene
}

fn render_nodes(layout: &LayoutResult, scene: &mut Scene, theme: &Theme) {
    for node in &layout.nodes {
        let style = node.custom_style.clone().unwrap_or_else(|| Style {
            fill: Some(theme.node_fill),
            stroke: Some(theme.node_stroke),
            stroke_width: Some(theme.default_stroke_width),
            ..Default::default()
        });
        scene.push(Primitive::Rect {
            bbox: BBox::new(node.x, node.y, node.width, node.height),
            rx: 3.0,
            ry: 3.0,
            style,
        });

        let top = node.y - node.height / 2.0;
        let line_height =
            theme.font_size_node * rusty_mermaid_core::constants::LINE_HEIGHT_MULTIPLIER;
        let mut y = top + 8.0 + line_height * 0.7;

        // Node name (bold)
        scene.push(Primitive::Text {
            position: Point::new(node.x, y),
            content: node.name.clone(),
            anchor: TextAnchor::Middle,
            style: TextStyle {
                font_size: theme.font_size_node,
                fill: Some(theme.node_text),
                font_weight: rusty_mermaid_core::FontWeight::Bold,
                ..Default::default()
            },
        });
        y += line_height;

        // Info lines (type label, id, text, risk, verify)
        for line in &node.lines {
            scene.push(Primitive::Text {
                position: Point::new(node.x, y),
                content: line.clone(),
                anchor: TextAnchor::Middle,
                style: TextStyle {
                    font_size: theme.font_size_edge_label,
                    fill: Some(theme.node_text),
                    ..Default::default()
                },
            });
            y += line_height * 0.85;
        }
    }
}

fn render_edges(layout: &LayoutResult, scene: &mut Scene, theme: &Theme) {
    for edge_layout in &layout.edges {
        let edge = &edge_layout.edge;
        if edge.points.len() < 2 {
            continue;
        }

        let mut segments = interpolate(&edge.points, CurveType::Basis);
        let marker_end = Some(MarkerType::ArrowPoint);
        let sw = theme.default_stroke_width;
        shorten_path_for_markers(&mut segments, None, marker_end, sw);

        let mut style = Style {
            stroke: Some(theme.edge_stroke),
            stroke_width: Some(sw),
            ..Default::default()
        };
        if edge_layout.rel_type.is_dashed() {
            style.stroke_dasharray = Some(DOTTED_PATTERN.to_vec());
        }

        scene.push(Primitive::Path {
            segments,
            style,
            marker_start: None,
            marker_end,
        });

        if let Some(label) = &edge.label {
            let mid = edge.points[edge.points.len() / 2];
            render_edge_label(scene, mid, label, edge.label_size, theme);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(input: &str) -> Scene {
        let diagram = super::parser::parse(input).unwrap();
        let layout = bridge::layout(&diagram);
        to_scene(&layout, &Theme::default())
    }

    #[test]
    fn scene_has_primitives() {
        let scene = render(
            "requirementDiagram\n    requirement REQ {\n        id: R1\n        text: \"Test\"\n    }",
        );
        assert!(scene.len() >= 3, "rect + name + info lines");
    }

    #[test]
    fn scene_with_edge() {
        let scene = render(
            "requirementDiagram\n    requirement A {\n        id: A\n    }\n    requirement B {\n        id: B\n    }\n    A - contains -> B",
        );
        assert!(scene.len() >= 5);
    }

    #[test]
    fn dashed_edge_has_dasharray() {
        let scene = render(
            "requirementDiagram\n    requirement A {\n        id: A\n    }\n    requirement B {\n        id: B\n    }\n    A - satisfies -> B",
        );
        let has_dashed = scene.elements().iter().any(|e| {
            if let Primitive::Path { style, .. } = &e.primitive {
                style.stroke_dasharray.is_some()
            } else {
                false
            }
        });
        assert!(has_dashed);
    }
}
