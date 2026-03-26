pub mod bridge;
mod center;
mod clip;
pub mod ir;
pub mod parser;
mod scope;

use rusty_mermaid_core::{
    BBox, Color, CurveType, PathSegment, Point, Primitive, Scene, Shape, Style, TextAnchor,
    TextStyle, Theme, interpolate,
};

use crate::common::layout::NodeLayout;
use bridge::LayoutResult;

use crate::common::rendering::{
    contrasting_label_style, merge_custom_style, overlay_style, render_edge_label,
    shorten_path_for_markers,
};

/// Convert a state diagram layout result into a Scene of drawing primitives.
pub fn to_scene(layout: &LayoutResult) -> Scene {
    to_scene_themed(layout, &Theme::default())
}

/// Convert a state diagram layout result into a themed Scene.
pub fn to_scene_themed(layout: &LayoutResult, theme: &Theme) -> Scene {
    let mut scene = Scene::new(layout.width, layout.height);
    layout_to_scene(layout, &mut scene, theme);
    scene
}

fn layout_to_scene(layout: &LayoutResult, scene: &mut Scene, theme: &Theme) {
    let mut compounds: Vec<&NodeLayout> = layout.nodes.iter().filter(|n| n.is_compound).collect();
    compounds.sort_by(|a, b| {
        let area_a = a.width * a.height;
        let area_b = b.width * b.height;
        area_b
            .partial_cmp(&area_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    render_compound_nodes(&compounds, scene, theme);
    render_region_rects(layout, scene, theme);
    render_edges(layout, &compounds, scene, theme);
    render_leaf_nodes(layout, scene, theme);
    render_dividers(layout, scene, theme);
}

const COMPOUND_LABEL_OFFSET_Y: f64 = 12.0;
const COMPOUND_SEP_OFFSET_Y: f64 = 24.0;
const STATE_END_INNER_INSET: f64 = 4.0;

fn render_compound_nodes(compounds: &[&NodeLayout], scene: &mut Scene, theme: &Theme) {
    for node in compounds {
        let bbox = BBox::new(node.x, node.y, node.width, node.height);
        let left = node.x - node.width / 2.0;
        let right = node.x + node.width / 2.0;
        let top = node.y - node.height / 2.0;

        let mut cstyle = Style {
            fill: Some(theme.composite_fill),
            stroke: Some(theme.composite_stroke),
            stroke_width: Some(theme.default_stroke_width),
            ..Default::default()
        };
        if let Some(custom) = &node.custom_style {
            overlay_style(&mut cstyle, custom);
        }
        scene.push(Primitive::Rect {
            bbox,
            rx: 5.0,
            ry: 5.0,
            style: cstyle,
        });
        scene.push(Primitive::Text {
            position: Point::new(node.x, top + COMPOUND_LABEL_OFFSET_Y),
            content: node.label.clone(),
            anchor: TextAnchor::Middle,
            style: TextStyle {
                fill: Some(theme.composite_label),
                ..Default::default()
            },
        });
        scene.push(Primitive::Path {
            segments: vec![
                PathSegment::MoveTo(Point::new(left, top + COMPOUND_SEP_OFFSET_Y)),
                PathSegment::LineTo(Point::new(right, top + COMPOUND_SEP_OFFSET_Y)),
            ],
            style: Style {
                stroke: Some(theme.composite_stroke),
                stroke_width: Some(1.0),
                ..Default::default()
            },
            marker_start: None,
            marker_end: None,
        });
    }
}

fn render_region_rects(layout: &LayoutResult, scene: &mut Scene, theme: &Theme) {
    for rr in &layout.region_rects {
        scene.push(Primitive::Rect {
            bbox: BBox::new(
                rr.x + rr.width / 2.0,
                rr.y + rr.height / 2.0,
                rr.width,
                rr.height,
            ),
            rx: 0.0,
            ry: 0.0,
            style: Style {
                fill: Some(Color::TRANSPARENT),
                stroke: Some(theme.region_stroke),
                stroke_width: Some(0.5),
                stroke_dasharray: Some(vec![10.0, 10.0]),
                ..Default::default()
            },
        });
    }
}

fn render_edges(
    layout: &LayoutResult,
    compounds: &[&NodeLayout],
    scene: &mut Scene,
    theme: &Theme,
) {
    for edge in &layout.edges {
        if edge.points.len() < 2 {
            continue;
        }
        let segments = interpolate(&edge.points, CurveType::Basis);
        let mut segments = clip::clip_segments_at_compounds(&segments, compounds);
        let label_pos = clip::path_midpoint(&segments);
        let marker_end = Some(rusty_mermaid_core::MarkerType::ArrowPoint);
        let sw = theme.default_stroke_width;
        shorten_path_for_markers(&mut segments, None, marker_end, sw);
        scene.push(Primitive::Path {
            segments,
            style: Style {
                stroke: Some(theme.edge_stroke),
                stroke_width: Some(sw),
                ..Default::default()
            },
            marker_start: None,
            marker_end,
        });
        if let Some(label) = &edge.label {
            let mid = label_pos.unwrap_or(edge.points[edge.points.len() / 2]);
            render_edge_label(scene, mid, label, edge.label_size, theme);
        }
    }
}

fn render_leaf_nodes(layout: &LayoutResult, scene: &mut Scene, theme: &Theme) {
    for node in layout.nodes.iter().filter(|n| !n.is_compound) {
        render_leaf_node(node, scene, theme);
    }
}

fn render_leaf_node(node: &NodeLayout, scene: &mut Scene, theme: &Theme) {
    match node.shape {
        Shape::StateStart => {
            scene.push(Primitive::Circle {
                center: Point::new(node.x, node.y),
                radius: node.width / 2.0,
                style: Style {
                    fill: Some(theme.start_fill),
                    stroke: Some(theme.start_fill),
                    ..Default::default()
                },
            });
        }
        Shape::StateEnd => {
            let r = node.width / 2.0;
            scene.push(Primitive::Circle {
                center: Point::new(node.x, node.y),
                radius: r,
                style: Style {
                    fill: Some(Color::TRANSPARENT),
                    stroke: Some(theme.node_stroke),
                    stroke_width: Some(theme.default_stroke_width),
                    ..Default::default()
                },
            });
            scene.push(Primitive::Circle {
                center: Point::new(node.x, node.y),
                radius: r - STATE_END_INNER_INSET,
                style: Style {
                    fill: Some(theme.end_inner_fill),
                    ..Default::default()
                },
            });
        }
        Shape::ForkJoin => {
            scene.push(Primitive::Rect {
                bbox: BBox::new(node.x, node.y, node.width, node.height),
                rx: 0.0,
                ry: 0.0,
                style: Style {
                    fill: Some(theme.start_fill),
                    stroke: Some(theme.start_fill),
                    ..Default::default()
                },
            });
        }
        Shape::Choice => {
            let hw = node.width / 2.0;
            let hh = node.height / 2.0;
            scene.push(Primitive::Polygon {
                points: vec![
                    Point::new(node.x, node.y - hh),
                    Point::new(node.x + hw, node.y),
                    Point::new(node.x, node.y + hh),
                    Point::new(node.x - hw, node.y),
                ],
                style: merge_custom_style(node.custom_style.as_ref(), theme),
            });
        }
        Shape::Note => {
            scene.push(Primitive::Rect {
                bbox: BBox::new(node.x, node.y, node.width, node.height),
                rx: 0.0,
                ry: 0.0,
                style: Style {
                    fill: Some(theme.note_fill),
                    stroke: Some(theme.note_stroke),
                    stroke_width: Some(1.0),
                    ..Default::default()
                },
            });
            scene.push(Primitive::Text {
                position: Point::new(node.x, node.y),
                content: node.label.clone(),
                anchor: TextAnchor::Middle,
                style: TextStyle {
                    font_size: theme.font_size_edge_label,
                    fill: Some(theme.note_text),
                    ..Default::default()
                },
            });
        }
        Shape::History => {
            let r = node.width / 2.0;
            scene.push(Primitive::Circle {
                center: Point::new(node.x, node.y),
                radius: r,
                style: Style {
                    fill: Some(theme.composite_fill),
                    stroke: Some(theme.node_stroke),
                    stroke_width: Some(theme.default_stroke_width),
                    ..Default::default()
                },
            });
            scene.push(Primitive::Text {
                position: Point::new(node.x, node.y),
                content: "H".to_string(),
                anchor: TextAnchor::Middle,
                style: TextStyle {
                    font_size: theme.font_size_edge_label,
                    fill: Some(theme.node_text),
                    ..Default::default()
                },
            });
        }
        Shape::RoundedRect | _ => {
            let style = merge_custom_style(node.custom_style.as_ref(), theme);
            let node_fill = style.fill;
            scene.push(Primitive::Rect {
                bbox: BBox::new(node.x, node.y, node.width, node.height),
                rx: 5.0,
                ry: 5.0,
                style,
            });
            scene.push(Primitive::Text {
                position: Point::new(node.x, node.y),
                content: node.label.clone(),
                anchor: TextAnchor::Middle,
                style: contrasting_label_style(node_fill, theme),
            });
        }
    }
}

fn render_dividers(layout: &LayoutResult, scene: &mut Scene, theme: &Theme) {
    for div in &layout.dividers {
        scene.push(Primitive::Path {
            segments: vec![PathSegment::MoveTo(div.start), PathSegment::LineTo(div.end)],
            style: Style {
                stroke: Some(theme.divider_stroke),
                stroke_width: Some(1.0),
                stroke_dasharray: Some(vec![3.0]),
                ..Default::default()
            },
            marker_start: None,
            marker_end: None,
        });
    }
}

/// Clip interpolated path segments at compound node boundaries.
#[cfg(test)]
mod render_tests;
