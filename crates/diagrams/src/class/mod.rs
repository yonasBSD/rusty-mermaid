pub mod bridge;
pub mod ir;
pub mod parser;

use rusty_mermaid_core::{
    BBox, CurveType, MarkerType, PathSegment, Point, Primitive, Scene, Style, TextAnchor,
    TextStyle, Theme, interpolate,
};

use bridge::LayoutResult;
use ir::{ClassMember, RelationType};
use crate::common::rendering::{render_edge_label, shorten_path_for_markers};

/// Convert a class diagram layout result into a Scene with default theme.
pub fn to_scene(layout: &LayoutResult) -> Scene {
    to_scene_themed(layout, &Theme::default())
}

/// Convert a class diagram layout result into a themed Scene.
pub fn to_scene_themed(layout: &LayoutResult, theme: &Theme) -> Scene {
    let mut scene = Scene::new(layout.width, layout.height);
    layout_to_scene(layout, &mut scene, theme);
    scene
}

const CLASS_RX: f64 = 3.0;
const SEPARATOR_INSET: f64 = 0.0;

fn layout_to_scene(layout: &LayoutResult, scene: &mut Scene, theme: &Theme) {
    render_namespaces(layout, scene, theme);
    render_edges(layout, scene, theme);
    render_classes(layout, scene, theme);
}

// ── Namespace rendering ──

fn render_namespaces(layout: &LayoutResult, scene: &mut Scene, theme: &Theme) {
    for ns in &layout.namespaces {
        scene.push(Primitive::Rect {
            bbox: BBox::new(ns.x, ns.y, ns.width, ns.height),
            rx: 5.0,
            ry: 5.0,
            style: Style {
                fill: Some(theme.subgraph_fill),
                stroke: Some(theme.subgraph_stroke),
                stroke_width: Some(1.0),
                ..Default::default()
            },
        });
        // Namespace label at top-left
        let left = ns.x - ns.width / 2.0;
        let top = ns.y - ns.height / 2.0;
        scene.push(Primitive::Text {
            position: Point::new(left + 8.0, top + 12.0),
            content: ns.id.clone(),
            anchor: TextAnchor::Start,
            style: TextStyle {
                font_size: theme.font_size_label,
                fill: Some(theme.subgraph_label),
                font_weight: rusty_mermaid_core::FontWeight::Bold,
                ..Default::default()
            },
        });
    }
}

// ── Edge rendering ──

fn render_edges(layout: &LayoutResult, scene: &mut Scene, theme: &Theme) {
    for edge_layout in &layout.edges {
        let edge = &edge_layout.edge;
        if edge.points.len() < 2 { continue; }

        let mut segments = interpolate(&edge.points, CurveType::Basis);

        // Map per-side relation types to markers
        let marker_start = edge_layout.from_type.and_then(relation_to_marker);
        let marker_end = edge_layout.to_type.and_then(relation_to_marker);
        let sw = theme.default_stroke_width;
        shorten_path_for_markers(&mut segments, marker_start, marker_end, sw);

        let mut style = Style {
            stroke: Some(theme.edge_stroke),
            stroke_width: Some(sw),
            ..Default::default()
        };
        if edge_layout.edge.stroke == crate::common::layout::StrokeType::Dotted {
            style.stroke_dasharray = Some(vec![6.0, 4.0]);
        }

        scene.push(Primitive::Path {
            segments,
            style,
            marker_start,
            marker_end,
        });

        // Edge label
        if let Some(label) = &edge.label {
            let mid = edge.points[edge.points.len() / 2];
            render_edge_label(scene, mid, label, edge.label_size, theme);
        }

        // Cardinality text near endpoints. Both use the forward edge
        // direction for perpendicular offset (same side). The "to" end
        // reverses the along-offset to move inward from its endpoint.
        let fwd_dx = edge.points.last().unwrap().x - edge.points[0].x;
        let fwd_dy = edge.points.last().unwrap().y - edge.points[0].y;
        if let Some(card) = &edge_layout.cardinality_from {
            render_cardinality(scene, edge.points[0], fwd_dx, fwd_dy, 1.0, card, theme);
        }
        if let Some(card) = &edge_layout.cardinality_to {
            let n = edge.points.len();
            render_cardinality(scene, edge.points[n - 1], fwd_dx, fwd_dy, -1.0, card, theme);
        }
    }
}

fn relation_to_marker(rt: RelationType) -> Option<MarkerType> {
    match rt {
        RelationType::Extension => Some(MarkerType::ArrowBarb),
        RelationType::Composition => Some(MarkerType::Composition),
        RelationType::Aggregation => Some(MarkerType::Aggregation),
        RelationType::Dependency => Some(MarkerType::ArrowPoint),
        RelationType::Lollipop => Some(MarkerType::Circle),
        RelationType::Association => None,
    }
}

/// Perpendicular offset from the edge line (right side of forward direction).
const CARDINALITY_PERP: f64 = 9.0;
/// Offset along the edge toward midpoint (pulls label away from node boundary).
const CARDINALITY_INWARD: f64 = 9.0;

fn render_cardinality(scene: &mut Scene, endpoint: Point, fwd_dx: f64, fwd_dy: f64, inward_sign: f64, text: &str, theme: &Theme) {
    let len = (fwd_dx * fwd_dx + fwd_dy * fwd_dy).sqrt();
    if len < 1e-6 { return; }

    let tx = fwd_dx / len;
    let ty = fwd_dy / len;
    let nx = fwd_dy / len;
    let ny = -fwd_dx / len;

    // Along-edge: "from" goes forward (+1), "to" goes backward (-1) — both toward midpoint
    let pos = Point::new(
        endpoint.x + tx * CARDINALITY_INWARD * inward_sign + nx * CARDINALITY_PERP,
        endpoint.y + ty * CARDINALITY_INWARD * inward_sign + ny * CARDINALITY_PERP,
    );

    scene.push(Primitive::Text {
        position: pos,
        content: text.to_string(),
        anchor: TextAnchor::Start,
        style: TextStyle {
            font_size: theme.font_size_small,
            fill: Some(theme.edge_label_text),
            ..Default::default()
        },
    });
}

// ── Class box rendering ──

fn render_classes(layout: &LayoutResult, scene: &mut Scene, theme: &Theme) {
    for class in &layout.classes {
        render_class_box(class, scene, theme);
    }
}

fn render_class_box(class: &bridge::ClassLayout, scene: &mut Scene, theme: &Theme) {
    let left = class.x - class.width / 2.0;
    let top = class.y - class.height / 2.0;

    // Background rect
    let style = class.custom_style.clone().unwrap_or_else(|| Style {
        fill: Some(theme.node_fill),
        stroke: Some(theme.node_stroke),
        stroke_width: Some(theme.default_stroke_width),
        ..Default::default()
    });
    scene.push(Primitive::Rect {
        bbox: BBox::new(class.x, class.y, class.width, class.height),
        rx: CLASS_RX,
        ry: CLASS_RX,
        style,
    });

    let mut y_cursor = top;

    // Title section
    render_title_section(class, scene, theme, left, &mut y_cursor);

    // Separator after title
    render_separator(scene, left, left + class.width, y_cursor, theme);

    // Members section
    render_member_section(&class.members, scene, theme, left, &mut y_cursor);

    // Separator after members
    render_separator(scene, left, left + class.width, y_cursor, theme);

    // Methods section
    render_member_section(&class.methods, scene, theme, left, &mut y_cursor);
}

fn render_title_section(
    class: &bridge::ClassLayout,
    scene: &mut Scene,
    theme: &Theme,
    _left: f64,
    y_cursor: &mut f64,
) {
    let center_x = class.x;

    // Annotation above title: <<interface>>
    if let Some(ann) = class.annotations.first() {
        *y_cursor += theme.font_size_small * 0.8;
        scene.push(Primitive::Text {
            position: Point::new(center_x, *y_cursor),
            content: format!("<<{ann}>>"),
            anchor: TextAnchor::Middle,
            style: TextStyle {
                font_size: theme.font_size_small,
                fill: Some(theme.node_text),
                font_weight: rusty_mermaid_core::FontWeight::Normal,
                ..Default::default()
            },
        });
        *y_cursor += theme.font_size_small * 0.5;
    }

    // Class name (bold) + optional generic
    let mut title = class.label.clone();
    if let Some(g) = &class.generic_type {
        title.push_str(&format!("<{g}>"));
    }
    *y_cursor += theme.font_size_node * 0.8;
    scene.push(Primitive::Text {
        position: Point::new(center_x, *y_cursor),
        content: title,
        anchor: TextAnchor::Middle,
        style: TextStyle {
            font_size: theme.font_size_node,
            fill: Some(theme.node_text),
            font_weight: rusty_mermaid_core::FontWeight::Bold,
            ..Default::default()
        },
    });
    *y_cursor += theme.font_size_node * 0.5;
}

fn render_member_section(
    members: &[ClassMember],
    scene: &mut Scene,
    theme: &Theme,
    left: f64,
    y_cursor: &mut f64,
) {
    let x = left + 8.0;
    if members.is_empty() {
        *y_cursor += 4.0;
        return;
    }

    *y_cursor += 4.0;
    let line_height = theme.font_size_node * rusty_mermaid_core::constants::LINE_HEIGHT_MULTIPLIER;
    for member in members {
        *y_cursor += line_height * 0.7;
        scene.push(Primitive::Text {
            position: Point::new(x, *y_cursor),
            content: member.display_text(),
            anchor: TextAnchor::Start,
            style: TextStyle {
                font_size: theme.font_size_node,
                fill: Some(theme.node_text),
                ..Default::default()
            },
        });
        *y_cursor += line_height * 0.3;
    }
    *y_cursor += 4.0;
}

fn render_separator(scene: &mut Scene, x1: f64, x2: f64, y: f64, theme: &Theme) {
    scene.push(Primitive::Path {
        segments: vec![
            PathSegment::MoveTo(Point::new(x1 + SEPARATOR_INSET, y)),
            PathSegment::LineTo(Point::new(x2 - SEPARATOR_INSET, y)),
        ],
        style: Style {
            stroke: Some(theme.node_stroke),
            stroke_width: Some(0.5),
            ..Default::default()
        },
        marker_start: None,
        marker_end: None,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(input: &str) -> Scene {
        let diagram = super::parser::parse(input).unwrap();
        let layout = bridge::layout(&diagram);
        to_scene(&layout)
    }

    #[test]
    fn scene_has_primitives() {
        let scene = render("classDiagram\n    class Animal {\n        +String name\n        +makeSound()\n    }");
        assert!(scene.len() > 0, "scene should have primitives");
        // Should have: rect + annotation? + title text + separator + member text + separator + method text
        assert!(scene.len() >= 5, "class box needs rect + title + separators + members");
    }

    #[test]
    fn scene_with_relationship() {
        let scene = render("classDiagram\n    Animal <|-- Dog");
        // 2 class boxes + 1 edge path (with markers)
        assert!(scene.len() >= 5);
    }

    #[test]
    fn scene_with_namespace() {
        let scene = render("classDiagram\n    namespace MyApp {\n        class User\n    }");
        // Namespace rect + label + class box
        assert!(scene.len() >= 4);
    }

    #[test]
    fn annotation_rendered() {
        let scene = render("classDiagram\n    class Shape {\n        <<interface>>\n        +draw()\n    }");
        let texts: Vec<_> = scene.elements().iter().filter_map(|e| {
            if let Primitive::Text { content, .. } = &e.primitive { Some(content.as_str()) } else { None }
        }).collect();
        assert!(texts.iter().any(|t| t.contains("<<interface>>")), "annotation text should be rendered");
    }

    #[test]
    fn generic_in_title() {
        let scene = render("classDiagram\n    class List~T~");
        let texts: Vec<_> = scene.elements().iter().filter_map(|e| {
            if let Primitive::Text { content, .. } = &e.primitive { Some(content.as_str()) } else { None }
        }).collect();
        assert!(texts.iter().any(|t| t.contains("<T>")), "generic type should appear in title");
    }

    #[test]
    fn members_rendered_with_visibility() {
        let scene = render("classDiagram\n    class Foo {\n        +publicField\n        -privateField\n    }");
        let texts: Vec<_> = scene.elements().iter().filter_map(|e| {
            if let Primitive::Text { content, .. } = &e.primitive { Some(content.clone()) } else { None }
        }).collect();
        assert!(texts.iter().any(|t| t.starts_with('+')), "should render + visibility");
        assert!(texts.iter().any(|t| t.starts_with('-')), "should render - visibility");
    }

    #[test]
    fn dotted_relationship_has_dasharray() {
        let scene = render("classDiagram\n    A ..|> B");
        let has_dashed = scene.elements().iter().any(|e| {
            if let Primitive::Path { style, .. } = &e.primitive {
                style.stroke_dasharray.is_some()
            } else { false }
        });
        assert!(has_dashed, "dotted relationship should have stroke_dasharray");
    }

    #[test]
    fn extension_marker_on_edge() {
        let scene = render("classDiagram\n    A <|-- B");
        let has_extension = scene.elements().iter().any(|e| {
            if let Primitive::Path { marker_start, .. } = &e.primitive {
                *marker_start == Some(MarkerType::ArrowBarb)
            } else { false }
        });
        assert!(has_extension, "<|-- should place open arrow marker at from (left) end");
    }
}
