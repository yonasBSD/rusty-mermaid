pub mod bridge;
pub mod ir;
pub mod parser;

use rusty_mermaid_core::{
    BBox, CurveType, MarkerType, PathSegment, Point, Primitive, Scene, Shape, Style, TextAnchor,
    TextStyle, Theme, interpolate,
};

use bridge::LayoutResult;
use crate::common::layout::NodeLayout;
use ir::{ArrowEnd, StrokeType};

use crate::common::rendering::{
    contrasting_label_style, merge_custom_style, overlay_style, render_edge_label,
};

fn edge_style(stroke: StrokeType, theme: &Theme) -> Style {
    Style {
        stroke: Some(theme.edge_stroke),
        stroke_width: Some(match stroke {
            StrokeType::Thick => 3.5,
            _ => 1.5,
        }),
        stroke_dasharray: match stroke {
            StrokeType::Dotted => Some(vec![3.0, 3.0]),
            _ => None,
        },
        ..Default::default()
    }
}

/// Convert a flowchart layout result into a Scene of drawing primitives.
pub fn to_scene(layout: &LayoutResult) -> Scene {
    to_scene_themed(layout, &Theme::default())
}

/// Convert a flowchart layout result into a themed Scene.
pub fn to_scene_themed(layout: &LayoutResult, theme: &Theme) -> Scene {
    let mut scene = Scene::new(layout.width, layout.height);
    scene.marker_color = Some(theme.edge_stroke);
    layout_to_scene(layout, &mut scene, theme);
    scene
}

fn subgraph_style(theme: &Theme) -> Style {
    Style {
        fill: Some(theme.subgraph_fill),
        stroke: Some(theme.subgraph_stroke),
        stroke_width: Some(1.0),
        ..Default::default()
    }
}

fn subgraph_label_style(theme: &Theme) -> TextStyle {
    TextStyle {
        font_size: 13.0,
        fill: Some(theme.subgraph_label),
        font_weight: rusty_mermaid_core::FontWeight::Bold,
        ..Default::default()
    }
}

fn layout_to_scene(layout: &LayoutResult, scene: &mut Scene, theme: &Theme) {
    // Draw subgraph boundaries first (behind nodes)
    for sg in &layout.subgraphs {
        let bbox = BBox::new(sg.x, sg.y, sg.width, sg.height);
        scene.push(Primitive::Rect {
            bbox,
            rx: 5.0,
            ry: 5.0,
            style: subgraph_style(theme),
        });
        if let Some(label) = &sg.label {
            let top_y = sg.y - sg.height / 2.0;
            let left_x = sg.x - sg.width / 2.0;
            scene.push(Primitive::Text {
                position: Point::new(left_x + 8.0, top_y + 12.0),
                content: label.clone(),
                anchor: TextAnchor::Start,
                style: subgraph_label_style(theme),
            });
        }
    }

    for edge in &layout.edges {
        if edge.points.len() >= 2 {
            let segments = interpolate(&edge.points, CurveType::Basis);

            let marker_end = match edge.end_arrow {
                ArrowEnd::Arrow => Some(MarkerType::ArrowPoint),
                ArrowEnd::Circle => Some(MarkerType::Circle),
                ArrowEnd::Cross => Some(MarkerType::Cross),
                ArrowEnd::None => None,
            };
            let marker_start = match edge.start_arrow {
                ArrowEnd::Arrow => Some(MarkerType::ArrowPoint),
                ArrowEnd::Circle => Some(MarkerType::Circle),
                ArrowEnd::Cross => Some(MarkerType::Cross),
                ArrowEnd::None => None,
            };

            let mut estyle = edge_style(edge.stroke, theme);
            if let Some(custom) = &edge.custom_style {
                overlay_style(&mut estyle, custom);
            }

            scene.push(Primitive::Path {
                segments,
                style: estyle,
                marker_start,
                marker_end,
            });
            if let Some(label) = &edge.label {
                let mid = edge.points[edge.points.len() / 2];
                render_edge_label(scene, mid, label, edge.label_size, theme);
            }
        }
    }

    for node in &layout.nodes {
        render_node(node, scene, theme);
    }
}

// ---------------------------------------------------------------------------
// Shape rendering — formulas match mermaid.js
// ---------------------------------------------------------------------------

fn render_node(node: &NodeLayout, scene: &mut Scene, theme: &Theme) {
    let style = merge_custom_style(node.custom_style.as_ref(), theme);
    let node_fill = style.fill;
    let cx = node.x;
    let cy = node.y;
    let w = node.width;
    let h = node.height;

    let bbox = BBox::new(cx, cy, w, h);

    match node.shape {
        Shape::Rect => {
            scene.push(Primitive::Rect { bbox, rx: 0.0, ry: 0.0, style });
        }
        Shape::RoundedRect => {
            scene.push(Primitive::Rect { bbox, rx: 5.0, ry: 5.0, style });
        }
        Shape::Stadium => {
            let r = h / 2.0;
            scene.push(Primitive::Rect { bbox, rx: r, ry: r, style });
        }
        Shape::Diamond => render_diamond(bbox, style, scene),
        Shape::Circle => {
            let r = w.max(h) / 2.0;
            scene.push(Primitive::Circle {
                center: Point::new(cx, cy),
                radius: r,
                style,
            });
        }
        Shape::DoubleCircle => {
            let gap = 5.0;
            let outer_r = w.max(h) / 2.0;
            let inner_r = outer_r - gap;
            scene.push(Primitive::Circle {
                center: Point::new(cx, cy),
                radius: outer_r,
                style: style.clone(),
            });
            scene.push(Primitive::Circle {
                center: Point::new(cx, cy),
                radius: inner_r,
                style,
            });
        }
        Shape::Hexagon => render_hexagon(bbox, style, scene),
        Shape::Parallelogram => render_parallelogram(bbox, style, scene),
        Shape::ParallelogramAlt => render_parallelogram_alt(bbox, style, scene),
        Shape::Trapezoid => render_trapezoid(bbox, style, scene),
        Shape::TrapezoidAlt => render_trapezoid_alt(bbox, style, scene),
        Shape::Cylinder => render_cylinder(bbox, style, scene),
        Shape::Subroutine => render_subroutine(bbox, style, scene),
        Shape::Asymmetric => render_asymmetric(bbox, style, scene),
        _ => {
            scene.push(Primitive::Rect { bbox, rx: 3.0, ry: 3.0, style });
        }
    }

    // Cylinder: center text on the wall below the top elliptical cap.
    // Wall runs from (top + ry) to bottom; its center is cy + ry/2.
    let label_y = if node.shape == Shape::Cylinder {
        let rx = w / 2.0;
        let ry = rx / (2.5 + w / 50.0);
        cy + ry / 2.0
    } else {
        cy
    };

    scene.push(Primitive::Text {
        position: Point::new(cx, label_y),
        content: node.label.clone(),
        anchor: TextAnchor::Middle,
        style: contrasting_label_style(node_fill, theme),
    });
}

/// Diamond: 4-point rhombus. Mermaid computes s = w + h, then draws a
/// diamond of that size. We use w and h directly to fit dagre's box.
fn render_diamond(bbox: BBox, style: Style, scene: &mut Scene) {
    let (cx, cy) = (bbox.x, bbox.y);
    let (hw, hh) = (bbox.width / 2.0, bbox.height / 2.0);
    scene.push(Primitive::Polygon {
        points: vec![
            Point::new(cx, cy - hh),
            Point::new(cx + hw, cy),
            Point::new(cx, cy + hh),
            Point::new(cx - hw, cy),
        ],
        style,
    });
}

/// Hexagon: 6 points. Cut amount m = h/4 on each side (matches mermaid f=4).
fn render_hexagon(bbox: BBox, style: Style, scene: &mut Scene) {
    let (cx, cy) = (bbox.x, bbox.y);
    let (hw, hh) = (bbox.width / 2.0, bbox.height / 2.0);
    let m = bbox.height / 4.0;
    scene.push(Primitive::Polygon {
        points: vec![
            Point::new(cx - hw + m, cy - hh), // top-left
            Point::new(cx + hw - m, cy - hh), // top-right
            Point::new(cx + hw, cy),           // right
            Point::new(cx + hw - m, cy + hh), // bottom-right
            Point::new(cx - hw + m, cy + hh), // bottom-left
            Point::new(cx - hw, cy),           // left
        ],
        style,
    });
}

/// Parallelogram (lean right): skew = h/2 (mermaid uses 3*h/6 = h/2).
fn render_parallelogram(bbox: BBox, style: Style, scene: &mut Scene) {
    let (cx, cy) = (bbox.x, bbox.y);
    let (hw, hh) = (bbox.width / 2.0, bbox.height / 2.0);
    let skew = bbox.height / 2.0;
    scene.push(Primitive::Polygon {
        points: vec![
            Point::new(cx - hw + skew, cy - hh), // top-left
            Point::new(cx + hw + skew, cy - hh), // top-right
            Point::new(cx + hw - skew, cy + hh), // bottom-right
            Point::new(cx - hw - skew, cy + hh), // bottom-left
        ],
        style,
    });
}

/// Parallelogram alt (lean left): opposite skew direction.
fn render_parallelogram_alt(bbox: BBox, style: Style, scene: &mut Scene) {
    let (cx, cy) = (bbox.x, bbox.y);
    let (hw, hh) = (bbox.width / 2.0, bbox.height / 2.0);
    let skew = bbox.height / 2.0;
    scene.push(Primitive::Polygon {
        points: vec![
            Point::new(cx - hw - skew, cy - hh), // top-left
            Point::new(cx + hw - skew, cy - hh), // top-right
            Point::new(cx + hw + skew, cy + hh), // bottom-right
            Point::new(cx - hw + skew, cy + hh), // bottom-left
        ],
        style,
    });
}

/// Trapezoid: top narrower than bottom. Offset = h/2 on each side.
fn render_trapezoid(bbox: BBox, style: Style, scene: &mut Scene) {
    let (cx, cy) = (bbox.x, bbox.y);
    let (hw, hh) = (bbox.width / 2.0, bbox.height / 2.0);
    let offset = bbox.height / 2.0;
    scene.push(Primitive::Polygon {
        points: vec![
            Point::new(cx - hw, cy - hh),            // top-left
            Point::new(cx + hw, cy - hh),            // top-right
            Point::new(cx + hw + offset, cy + hh),   // bottom-right (wider)
            Point::new(cx - hw - offset, cy + hh),   // bottom-left (wider)
        ],
        style,
    });
}

/// Trapezoid alt (inverted): bottom narrower than top.
fn render_trapezoid_alt(bbox: BBox, style: Style, scene: &mut Scene) {
    let (cx, cy) = (bbox.x, bbox.y);
    let (hw, hh) = (bbox.width / 2.0, bbox.height / 2.0);
    let offset = bbox.height / 2.0;
    scene.push(Primitive::Polygon {
        points: vec![
            Point::new(cx - hw - offset, cy - hh),   // top-left (wider)
            Point::new(cx + hw + offset, cy - hh),   // top-right (wider)
            Point::new(cx + hw, cy + hh),            // bottom-right
            Point::new(cx - hw, cy + hh),            // bottom-left
        ],
        style,
    });
}

/// Cylinder: rect body + elliptical top/bottom caps.
/// Mermaid: ry = rx / (2.5 + w/50).
///
/// Drawn as two paths: (1) body with bottom arc, (2) top ellipse on top.
/// A single combined path causes fill artifacts where the top cap's
/// interior remains unfilled due to winding/even-odd fill rules.
fn render_cylinder(bbox: BBox, style: Style, scene: &mut Scene) {
    let (cx, cy, w, h) = (bbox.x, bbox.y, bbox.width, bbox.height);
    let hw = w / 2.0;
    let rx = hw;
    let ry = rx / (2.5 + w / 50.0);
    let body_h = h - ry;
    let top = cy - body_h / 2.0;
    let bottom = cy + body_h / 2.0;

    // Body: left side down → bottom arc → right side up → top back-arc to close
    let body = vec![
        PathSegment::MoveTo(Point::new(cx - hw, top)),
        PathSegment::LineTo(Point::new(cx - hw, bottom)),
        PathSegment::ArcTo {
            rx,
            ry,
            rotation: 0.0,
            large_arc: false,
            sweep: false,
            to: Point::new(cx + hw, bottom),
        },
        PathSegment::LineTo(Point::new(cx + hw, top)),
        // Close with back-arc so the top edge is smooth
        PathSegment::ArcTo {
            rx,
            ry,
            rotation: 0.0,
            large_arc: false,
            sweep: true,
            to: Point::new(cx - hw, top),
        },
    ];
    scene.push(Primitive::Path {
        segments: body,
        style: style.clone(),
        marker_start: None,
        marker_end: None,
    });

    // Top ellipse (front arc only — drawn on top of body)
    let top_cap = vec![
        PathSegment::MoveTo(Point::new(cx - hw, top)),
        PathSegment::ArcTo {
            rx,
            ry,
            rotation: 0.0,
            large_arc: false,
            sweep: true,
            to: Point::new(cx + hw, top),
        },
        PathSegment::ArcTo {
            rx,
            ry,
            rotation: 0.0,
            large_arc: false,
            sweep: true,
            to: Point::new(cx - hw, top),
        },
    ];
    scene.push(Primitive::Path {
        segments: top_cap,
        style,
        marker_start: None,
        marker_end: None,
    });
}

/// Asymmetric (flag/pennant): rectangle with a V-notch on the left.
/// Mermaid `>text]` shape. Notch indentation = h/4.
fn render_asymmetric(bbox: BBox, style: Style, scene: &mut Scene) {
    let (cx, cy) = (bbox.x, bbox.y);
    let (hw, hh) = (bbox.width / 2.0, bbox.height / 2.0);
    let notch = (bbox.height / 4.0).min(hw);
    scene.push(Primitive::Polygon {
        points: vec![
            Point::new(cx - hw, cy - hh),         // top-left
            Point::new(cx + hw, cy - hh),         // top-right
            Point::new(cx + hw, cy + hh),         // bottom-right
            Point::new(cx - hw, cy + hh),         // bottom-left
            Point::new(cx - hw + notch, cy),      // left V-notch (pointing right)
        ],
        style,
    });
}

/// Subroutine: rect with double vertical bars (8px offset each side).
fn render_subroutine(bbox: BBox, style: Style, scene: &mut Scene) {
    let (cx, cy, w, h) = (bbox.x, bbox.y, bbox.width, bbox.height);
    let bar_offset = 8.0;
    scene.push(Primitive::Rect {
        bbox: BBox::new(cx, cy, w + bar_offset * 2.0, h),
        rx: 0.0,
        ry: 0.0,
        style: style.clone(),
    });
    // Left inner vertical bar
    let left = cx - w / 2.0;
    let top = cy - h / 2.0;
    let bottom = cy + h / 2.0;
    scene.push(Primitive::Path {
        segments: vec![
            PathSegment::MoveTo(Point::new(left, top)),
            PathSegment::LineTo(Point::new(left, bottom)),
        ],
        style: Style {
            fill: None,
            stroke: style.stroke,
            stroke_width: style.stroke_width,
            ..Default::default()
        },
        marker_start: None,
        marker_end: None,
    });
    // Right inner vertical bar
    let right = cx + w / 2.0;
    scene.push(Primitive::Path {
        segments: vec![
            PathSegment::MoveTo(Point::new(right, top)),
            PathSegment::LineTo(Point::new(right, bottom)),
        ],
        style: Style {
            fill: None,
            stroke: style.stroke,
            stroke_width: style.stroke_width,
            ..Default::default()
        },
        marker_start: None,
        marker_end: None,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::test_helpers::test_helpers::*;

    #[test]
    fn simple_flowchart_to_scene() {
        let d = crate::flowchart::parser::parse("graph TD\n    A --> B").unwrap();
        let layout = crate::flowchart::bridge::layout(&d);
        let scene = to_scene(&layout);

        assert_scene_valid(&scene);

        let prims = scene.primitives();
        // At minimum: 2 nodes (Rect + Text each) + 1 edge (Path)
        assert!(prims.len() >= 5, "expected at least 5 primitives, got {}", prims.len());

        assert!(has_rect(&scene), "scene should contain Rect primitives for nodes");
        assert!(has_path(&scene), "scene should contain Path primitives for edges");
        assert!(count_texts(&scene) > 0, "scene should contain Text primitives for labels");
    }

    #[test]
    fn diamond_node_produces_polygon() {
        let d = crate::flowchart::parser::parse("flowchart TD\n    A{Decision}").unwrap();
        let layout = crate::flowchart::bridge::layout(&d);
        let scene = to_scene(&layout);

        assert!(
            count_polygons(&scene) > 0,
            "diamond shape should produce at least one Polygon primitive"
        );
        // Diamond has exactly 4 points
        let polygons: Vec<_> = scene
            .primitives()
            .iter()
            .filter(|p| matches!(p, Primitive::Polygon { .. }))
            .collect();
        if let Primitive::Polygon { points, .. } = &polygons[0] {
            assert_eq!(points.len(), 4, "diamond polygon should have 4 vertices");
        }
    }

    #[test]
    fn circle_node_produces_circle() {
        let d = crate::flowchart::parser::parse("flowchart TD\n    A((Round))").unwrap();
        let layout = crate::flowchart::bridge::layout(&d);
        let scene = to_scene(&layout);

        assert!(
            has_circle(&scene),
            "circle shape should produce Circle primitive"
        );
        let circles: Vec<_> = scene
            .primitives()
            .iter()
            .filter(|p| matches!(p, Primitive::Circle { .. }))
            .collect();
        if let Primitive::Circle { radius, .. } = &circles[0] {
            assert!(*radius > 0.0);
        }
    }

    #[test]
    fn edges_produce_paths_with_markers() {
        let d = crate::flowchart::parser::parse("flowchart TD\n    A --> B").unwrap();
        let layout = crate::flowchart::bridge::layout(&d);
        let scene = to_scene(&layout);

        let edge_paths: Vec<_> = scene
            .primitives()
            .iter()
            .filter(|p| {
                matches!(
                    p,
                    Primitive::Path {
                        marker_end: Some(MarkerType::ArrowPoint),
                        ..
                    }
                )
            })
            .collect();
        assert_eq!(
            edge_paths.len(),
            1,
            "one edge should produce one Path with ArrowPoint marker"
        );
    }

    #[test]
    fn subgraph_produces_background_rect() {
        let mmd = "flowchart TD\n    subgraph sg[My Group]\n        A --> B\n    end";
        let d = crate::flowchart::parser::parse(mmd).unwrap();
        let layout = crate::flowchart::bridge::layout(&d);
        let scene = to_scene(&layout);

        // Subgraph rect is rendered first, before node rects.
        // Count all rects: should be at least 3 (1 subgraph + 2 nodes).
        assert!(
            count_rects(&scene) >= 3,
            "expected at least 3 Rects (1 subgraph bg + 2 nodes), got {}",
            count_rects(&scene)
        );

        // First rect should be the subgraph background (rendered before nodes)
        let rects: Vec<_> = scene
            .primitives()
            .iter()
            .filter(|p| matches!(p, Primitive::Rect { .. }))
            .collect();
        if let Primitive::Rect { rx, ry, .. } = &rects[0] {
            assert!((*rx - 5.0).abs() < f64::EPSILON, "subgraph rect should have rx=5");
            assert!((*ry - 5.0).abs() < f64::EPSILON, "subgraph rect should have ry=5");
        }

        // Subgraph label text should appear
        assert!(has_text(&scene, "My Group"), "subgraph label text should be in scene");
    }

    #[test]
    fn empty_layout_produces_empty_scene() {
        let layout = crate::flowchart::bridge::LayoutResult {
            nodes: vec![],
            edges: vec![],
            subgraphs: vec![],
            width: 0.0,
            height: 0.0,
        };
        let scene = to_scene(&layout);
        assert!(scene.primitives().is_empty());
        assert!((scene.width - 0.0).abs() < f64::EPSILON);
        assert!((scene.height - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn themed_scene_has_marker_color() {
        let d = crate::flowchart::parser::parse("graph TD\n    A --> B").unwrap();
        let layout = crate::flowchart::bridge::layout(&d);
        let theme = Theme::default();
        let scene = to_scene_themed(&layout, &theme);

        assert_eq!(
            scene.marker_color,
            Some(theme.edge_stroke),
            "themed scene marker_color should match theme.edge_stroke"
        );
    }
}
