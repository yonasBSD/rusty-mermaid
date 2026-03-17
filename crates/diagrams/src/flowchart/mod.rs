pub mod bridge;
pub mod ir;
pub mod parser;

use rusty_mermaid_core::{
    BBox, Color, CurveType, MarkerType, PathSegment, Point, Primitive, Scene, Shape, Style,
    TextAnchor, TextStyle, interpolate,
};

use bridge::{LayoutResult, NodeLayout};
use ir::{ArrowEnd, StrokeType};

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

    for edge in &layout.edges {
        if edge.points.len() >= 2 {
            let points: Vec<Point> =
                edge.points.iter().map(|&(x, y)| Point::new(x, y)).collect();
            let segments = interpolate(&points, CurveType::Basis);

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

            scene.push(Primitive::Path {
                segments,
                style: edge_style(edge.stroke),
                marker_start,
                marker_end,
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

    for node in &layout.nodes {
        render_node(node, scene);
    }
}

// ---------------------------------------------------------------------------
// Shape rendering — formulas match mermaid.js
// ---------------------------------------------------------------------------

/// Merge a node's custom style (from classDef/style/:::class) onto the defaults.
fn merge_node_style(node: &NodeLayout) -> Style {
    let mut style = node_style();
    if let Some(custom) = &node.custom_style {
        if custom.fill.is_some() {
            style.fill = custom.fill;
        }
        if custom.stroke.is_some() {
            style.stroke = custom.stroke;
        }
        if custom.stroke_width.is_some() {
            style.stroke_width = custom.stroke_width;
        }
        if custom.stroke_dasharray.is_some() {
            style.stroke_dasharray = custom.stroke_dasharray.clone();
        }
        if custom.opacity.is_some() {
            style.opacity = custom.opacity;
        }
    }
    style
}

fn render_node(node: &NodeLayout, scene: &mut Scene) {
    let style = merge_node_style(node);
    let node_fill = style.fill;
    let cx = node.x;
    let cy = node.y;
    let w = node.width;
    let h = node.height;

    match node.shape {
        Shape::Rect => {
            scene.push(Primitive::Rect {
                bbox: BBox::new(cx, cy, w, h),
                rx: 0.0,
                ry: 0.0,
                style,
            });
        }
        Shape::RoundedRect => {
            scene.push(Primitive::Rect {
                bbox: BBox::new(cx, cy, w, h),
                rx: 5.0,
                ry: 5.0,
                style,
            });
        }
        Shape::Stadium => {
            // Pill shape: rx = half height so ends are semicircles
            let r = h / 2.0;
            scene.push(Primitive::Rect {
                bbox: BBox::new(cx, cy, w, h),
                rx: r,
                ry: r,
                style,
            });
        }
        Shape::Diamond => {
            render_diamond(cx, cy, w, h, style, scene);
        }
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
            // Dagre box already includes gap (sized in bridge), so
            // outer_r matches the dagre clip boundary.
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
        Shape::Hexagon => {
            render_hexagon(cx, cy, w, h, style, scene);
        }
        Shape::Parallelogram => {
            render_parallelogram(cx, cy, w, h, style, scene);
        }
        Shape::ParallelogramAlt => {
            render_parallelogram_alt(cx, cy, w, h, style, scene);
        }
        Shape::Trapezoid => {
            render_trapezoid(cx, cy, w, h, style, scene);
        }
        Shape::TrapezoidAlt => {
            render_trapezoid_alt(cx, cy, w, h, style, scene);
        }
        Shape::Cylinder => {
            render_cylinder(cx, cy, w, h, style, scene);
        }
        Shape::Subroutine => {
            render_subroutine(cx, cy, w, h, style, scene);
        }
        Shape::Asymmetric => {
            render_asymmetric(cx, cy, w, h, style, scene);
        }
        // Fallback: rounded rect for unhandled shapes
        _ => {
            scene.push(Primitive::Rect {
                bbox: BBox::new(cx, cy, w, h),
                rx: 3.0,
                ry: 3.0,
                style,
            });
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

    // Pick text color that contrasts with the node fill.
    let mut lstyle = label_style();
    if let Some(fill) = node_fill {
        let lum = fill.luminance();
        if lum < 0.4 {
            lstyle.fill = Some(Color::WHITE);
        } else if lum > 0.9 {
            lstyle.fill = Some(Color::BLACK);
        }
    }

    scene.push(Primitive::Text {
        position: Point::new(cx, label_y),
        content: node.label.clone(),
        anchor: TextAnchor::Middle,
        style: lstyle,
    });
}

/// Diamond: 4-point rhombus. Mermaid computes s = w + h, then draws a
/// diamond of that size. We use w and h directly to fit dagre's box.
fn render_diamond(cx: f64, cy: f64, w: f64, h: f64, style: Style, scene: &mut Scene) {
    let hw = w / 2.0;
    let hh = h / 2.0;
    scene.push(Primitive::Polygon {
        points: vec![
            Point::new(cx, cy - hh),     // top
            Point::new(cx + hw, cy),     // right
            Point::new(cx, cy + hh),     // bottom
            Point::new(cx - hw, cy),     // left
        ],
        style,
    });
}

/// Hexagon: 6 points. Cut amount m = h/4 on each side (matches mermaid f=4).
fn render_hexagon(cx: f64, cy: f64, w: f64, h: f64, style: Style, scene: &mut Scene) {
    let hw = w / 2.0;
    let hh = h / 2.0;
    let m = h / 4.0;
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
fn render_parallelogram(cx: f64, cy: f64, w: f64, h: f64, style: Style, scene: &mut Scene) {
    let hw = w / 2.0;
    let hh = h / 2.0;
    let skew = h / 2.0;
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
fn render_parallelogram_alt(cx: f64, cy: f64, w: f64, h: f64, style: Style, scene: &mut Scene) {
    let hw = w / 2.0;
    let hh = h / 2.0;
    let skew = h / 2.0;
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
fn render_trapezoid(cx: f64, cy: f64, w: f64, h: f64, style: Style, scene: &mut Scene) {
    let hw = w / 2.0;
    let hh = h / 2.0;
    let offset = h / 2.0;
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
fn render_trapezoid_alt(cx: f64, cy: f64, w: f64, h: f64, style: Style, scene: &mut Scene) {
    let hw = w / 2.0;
    let hh = h / 2.0;
    let offset = h / 2.0;
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
fn render_cylinder(cx: f64, cy: f64, w: f64, h: f64, style: Style, scene: &mut Scene) {
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
fn render_asymmetric(cx: f64, cy: f64, w: f64, h: f64, style: Style, scene: &mut Scene) {
    let hw = w / 2.0;
    let hh = h / 2.0;
    let notch = h / 4.0;
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
fn render_subroutine(cx: f64, cy: f64, w: f64, h: f64, style: Style, scene: &mut Scene) {
    let bar_offset = 8.0;
    // Outer rect (wider by bar_offset on each side)
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
