pub mod bridge;
pub mod ir;
pub mod parser;

use rusty_mermaid_core::{
    BBox, CurveType, PathSegment, Point, Primitive, Scene, Style, TextAnchor, TextStyle, Theme,
    interpolate,
};

use bridge::LayoutResult;
use ir::{Cardinality, Identification};
use crate::common::rendering::{render_edge_label, shorten_path_for_markers};

/// Convert an ER diagram layout result into a Scene with default theme.
pub fn to_scene(layout: &LayoutResult) -> Scene {
    to_scene_themed(layout, &Theme::default())
}

/// Convert an ER diagram layout result into a themed Scene.
pub fn to_scene_themed(layout: &LayoutResult, theme: &Theme) -> Scene {
    let mut scene = Scene::new(layout.width, layout.height);
    render_edges(layout, &mut scene, theme);
    render_entities(layout, &mut scene, theme);
    scene
}

// ── Entity rendering ──

fn render_entities(layout: &LayoutResult, scene: &mut Scene, theme: &Theme) {
    for entity in &layout.entities {
        render_entity(entity, scene, theme);
    }
}

fn render_entity(entity: &bridge::EntityLayout, scene: &mut Scene, theme: &Theme) {
    let left = entity.x - entity.width / 2.0;
    let top = entity.y - entity.height / 2.0;

    // Background rect
    let style = entity.custom_style.clone().unwrap_or_else(|| Style {
        fill: Some(theme.node_fill),
        stroke: Some(theme.node_stroke),
        stroke_width: Some(theme.default_stroke_width),
        ..Default::default()
    });
    scene.push(Primitive::Rect {
        bbox: BBox::new(entity.x, entity.y, entity.width, entity.height),
        rx: 0.0,
        ry: 0.0,
        style,
    });

    // Title
    let title_cy = top + entity.title_height / 2.0;
    scene.push(Primitive::Text {
        position: Point::new(entity.x, title_cy),
        content: entity.display_name.clone(),
        anchor: TextAnchor::Middle,
        style: TextStyle {
            font_size: theme.font_size_node,
            fill: Some(theme.node_text),
            font_weight: rusty_mermaid_core::FontWeight::Bold,
            ..Default::default()
        },
    });

    // Separator below title
    if !entity.attributes.is_empty() {
        let sep_y = top + entity.title_height;
        scene.push(Primitive::Path {
            segments: vec![
                PathSegment::MoveTo(Point::new(left, sep_y)),
                PathSegment::LineTo(Point::new(left + entity.width, sep_y)),
            ],
            style: Style {
                stroke: Some(theme.node_stroke),
                stroke_width: Some(0.5),
                ..Default::default()
            },
            marker_start: None,
            marker_end: None,
        });

        // Attribute rows with alternating backgrounds
        let attr_font_size = theme.font_size_node * 0.85;
        let border_w = theme.default_stroke_width;
        let row_inset = border_w / 2.0;
        let row_fill_width = entity.width - border_w;

        for (i, attr) in entity.attributes.iter().enumerate() {
            let row_y = top + entity.title_height + i as f64 * entity.row_height;
            let is_last = i == entity.attributes.len() - 1;

            // Every row gets a fill rect — even rows use entity fill, odd use alternate
            let fill = if i % 2 == 1 { theme.composite_fill } else { theme.node_fill };
            let row_h = if is_last { entity.row_height - row_inset } else { entity.row_height };
            scene.push(Primitive::Rect {
                bbox: BBox::new(
                    entity.x,
                    row_y + row_h / 2.0,
                    row_fill_width,
                    row_h,
                ),
                rx: 0.0,
                ry: 0.0,
                style: Style {
                    fill: Some(fill),
                    ..Default::default()
                },
            });

            // Attribute text: type name [PK,FK] ["comment"]
            let mut text = format!("{} {}", attr.attr_type, attr.name);
            if !attr.keys.is_empty() {
                let keys: Vec<&str> = attr.keys.iter().map(|k| k.label()).collect();
                text.push_str(&format!(" {}", keys.join(",")));
            }
            if let Some(c) = &attr.comment {
                text.push_str(&format!(" \"{}\"", c));
            }

            scene.push(Primitive::Text {
                position: Point::new(left + 8.0, row_y + entity.row_height / 2.0),
                content: text,
                anchor: TextAnchor::Start,
                style: TextStyle {
                    font_size: attr_font_size,
                    fill: Some(theme.node_text),
                    ..Default::default()
                },
            });
        }
    }
}

// ── Edge rendering with crow's foot markers ──

/// Size of cardinality marker symbols in scene units.
const MARKER_SIZE: f64 = 8.0;
/// Gap between the node boundary and the start of the marker.
const MARKER_GAP: f64 = 2.0;

fn render_edges(layout: &LayoutResult, scene: &mut Scene, theme: &Theme) {
    for edge_layout in &layout.edges {
        let edge = &edge_layout.edge;
        if edge.points.len() < 2 { continue; }

        // Shorten edge to leave room for crow's foot markers
        let mut segments = interpolate(&edge.points, CurveType::Basis);
        let sw = theme.default_stroke_width;

        let mut style = Style {
            stroke: Some(theme.edge_stroke),
            stroke_width: Some(sw),
            ..Default::default()
        };
        if edge_layout.identification == Identification::NonIdentifying {
            style.stroke_dasharray = Some(vec![6.0, 4.0]);
        }

        scene.push(Primitive::Path {
            segments,
            style,
            marker_start: None,
            marker_end: None,
        });

        // Edge label
        if let Some(label) = &edge.label {
            let mid = edge.points[edge.points.len() / 2];
            render_edge_label(scene, mid, label, edge.label_size, theme);
        }

        // Crow's foot markers at endpoints
        if edge.points.len() >= 2 {
            let p0 = edge.points[0];
            let p1 = edge.points[1];
            render_crowsfoot(scene, p0, p1, edge_layout.cardinality_a, theme);

            let n = edge.points.len();
            let pn = edge.points[n - 1];
            let pn1 = edge.points[n - 2];
            render_crowsfoot(scene, pn, pn1, edge_layout.cardinality_b, theme);
        }
    }
}

/// Render a crow's foot cardinality marker at an edge endpoint.
///
/// `endpoint` is the point on the node boundary.
/// `neighbor` is the next point along the edge (toward the interior).
fn render_crowsfoot(scene: &mut Scene, endpoint: Point, neighbor: Point, card: Cardinality, theme: &Theme) {
    let dx = neighbor.x - endpoint.x;
    let dy = neighbor.y - endpoint.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-6 { return; }

    // Unit tangent (pointing inward along edge) and perpendicular
    let tx = dx / len;
    let ty = dy / len;
    let nx = -ty;
    let ny = tx;

    let stroke_style = Style {
        stroke: Some(theme.edge_stroke),
        stroke_width: Some(theme.default_stroke_width),
        ..Default::default()
    };

    let base = Point::new(
        endpoint.x + tx * MARKER_GAP,
        endpoint.y + ty * MARKER_GAP,
    );

    match card {
        Cardinality::ExactlyOne => {
            // Two parallel lines perpendicular to edge
            let line1 = MARKER_GAP;
            let line2 = MARKER_GAP + MARKER_SIZE * 0.4;
            for offset in [line1, line2] {
                let cx = endpoint.x + tx * offset;
                let cy = endpoint.y + ty * offset;
                scene.push(Primitive::Path {
                    segments: vec![
                        PathSegment::MoveTo(Point::new(cx + nx * MARKER_SIZE * 0.5, cy + ny * MARKER_SIZE * 0.5)),
                        PathSegment::LineTo(Point::new(cx - nx * MARKER_SIZE * 0.5, cy - ny * MARKER_SIZE * 0.5)),
                    ],
                    style: stroke_style.clone(),
                    marker_start: None,
                    marker_end: None,
                });
            }
        }
        Cardinality::ZeroOrOne => {
            // Circle + single line
            let circle_center = Point::new(
                endpoint.x + tx * (MARKER_GAP + MARKER_SIZE * 0.3),
                endpoint.y + ty * (MARKER_GAP + MARKER_SIZE * 0.3),
            );
            scene.push(Primitive::Circle {
                center: circle_center,
                radius: MARKER_SIZE * 0.25,
                style: Style {
                    fill: Some(theme.background),
                    stroke: Some(theme.edge_stroke),
                    stroke_width: Some(theme.default_stroke_width),
                    ..Default::default()
                },
            });
            let line_offset = MARKER_GAP + MARKER_SIZE * 0.65;
            let cx = endpoint.x + tx * line_offset;
            let cy = endpoint.y + ty * line_offset;
            scene.push(Primitive::Path {
                segments: vec![
                    PathSegment::MoveTo(Point::new(cx + nx * MARKER_SIZE * 0.5, cy + ny * MARKER_SIZE * 0.5)),
                    PathSegment::LineTo(Point::new(cx - nx * MARKER_SIZE * 0.5, cy - ny * MARKER_SIZE * 0.5)),
                ],
                style: stroke_style,
                marker_start: None,
                marker_end: None,
            });
        }
        Cardinality::OneOrMore => {
            let fork_tip = render_fork(scene, endpoint, tx, ty, nx, ny, &stroke_style);
            // Single line behind fork
            let line_offset = fork_tip + MARKER_SIZE * 0.3;
            let cx = endpoint.x + tx * line_offset;
            let cy = endpoint.y + ty * line_offset;
            scene.push(Primitive::Path {
                segments: vec![
                    PathSegment::MoveTo(Point::new(cx + nx * MARKER_SIZE * 0.5, cy + ny * MARKER_SIZE * 0.5)),
                    PathSegment::LineTo(Point::new(cx - nx * MARKER_SIZE * 0.5, cy - ny * MARKER_SIZE * 0.5)),
                ],
                style: stroke_style,
                marker_start: None,
                marker_end: None,
            });
        }
        Cardinality::ZeroOrMore => {
            let fork_tip = render_fork(scene, endpoint, tx, ty, nx, ny, &stroke_style);
            // Circle behind fork
            let circle_center = Point::new(
                endpoint.x + tx * (fork_tip + MARKER_SIZE * 0.35),
                endpoint.y + ty * (fork_tip + MARKER_SIZE * 0.35),
            );
            scene.push(Primitive::Circle {
                center: circle_center,
                radius: MARKER_SIZE * 0.25,
                style: Style {
                    fill: Some(theme.background),
                    stroke: Some(theme.edge_stroke),
                    stroke_width: Some(theme.default_stroke_width),
                    ..Default::default()
                },
            });
        }
    }
}

/// Render the crow's foot fork (three prongs converging). Returns the fork tip offset.
fn render_fork(scene: &mut Scene, endpoint: Point, tx: f64, ty: f64, nx: f64, ny: f64, style: &Style) -> f64 {
    let fork_base = MARKER_GAP;
    let fork_tip = MARKER_GAP + MARKER_SIZE * 0.6;
    let base_pt = Point::new(endpoint.x + tx * fork_tip, endpoint.y + ty * fork_tip);
    for spread in [-0.5, 0.0, 0.5] {
        let prong_end = Point::new(
            endpoint.x + tx * fork_base + nx * MARKER_SIZE * spread,
            endpoint.y + ty * fork_base + ny * MARKER_SIZE * spread,
        );
        scene.push(Primitive::Path {
            segments: vec![
                PathSegment::MoveTo(base_pt),
                PathSegment::LineTo(prong_end),
            ],
            style: style.clone(),
            marker_start: None,
            marker_end: None,
        });
    }
    fork_tip
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
        let scene = render("erDiagram\n    CUSTOMER {\n        string name\n        int age\n    }");
        assert!(scene.len() >= 4, "entity needs rect + title + separator + attribute rows");
    }

    #[test]
    fn scene_with_relationship() {
        let scene = render("erDiagram\n    A ||--o{ B : has");
        assert!(scene.len() >= 5, "2 entities + edge + markers");
    }

    #[test]
    fn alternating_row_backgrounds() {
        let scene = render("erDiagram\n    T {\n        int a\n        int b\n        int c\n    }");
        // Should have striped backgrounds for odd rows
        let rects: Vec<_> = scene.elements().iter().filter(|e| {
            matches!(&e.primitive, Primitive::Rect { .. })
        }).collect();
        assert!(rects.len() >= 2, "main rect + at least one striped row");
    }

    #[test]
    fn crowsfoot_exactly_one() {
        let scene = render("erDiagram\n    A ||--|| B : is");
        // ExactlyOne markers produce perpendicular lines (Path primitives)
        let paths: Vec<_> = scene.elements().iter().filter(|e| {
            matches!(&e.primitive, Primitive::Path { .. })
        }).collect();
        assert!(paths.len() >= 5, "edge + 4 marker lines (2 per side)");
    }

    #[test]
    fn crowsfoot_zero_or_more() {
        let scene = render("erDiagram\n    A ||--o{ B : has");
        // ZeroOrMore marker has circles
        let circles: Vec<_> = scene.elements().iter().filter(|e| {
            matches!(&e.primitive, Primitive::Circle { .. })
        }).collect();
        assert!(circles.len() >= 1, "zero-or-more should have circle");
    }

    #[test]
    fn non_identifying_dashed() {
        let scene = render("erDiagram\n    A }|..|{ B : has");
        let has_dashed = scene.elements().iter().any(|e| {
            if let Primitive::Path { style, .. } = &e.primitive {
                style.stroke_dasharray.is_some()
            } else { false }
        });
        assert!(has_dashed, "non-identifying should have dashed line");
    }

    #[test]
    fn entity_title_bold() {
        let scene = render("erDiagram\n    CUSTOMER {\n        string name\n    }");
        let has_bold = scene.elements().iter().any(|e| {
            if let Primitive::Text { style, .. } = &e.primitive {
                style.font_weight == rusty_mermaid_core::FontWeight::Bold
            } else { false }
        });
        assert!(has_bold, "entity title should be bold");
    }
}
