pub mod bridge;
pub mod ir;
pub mod parser;

use rusty_mermaid_core::{
    BBox, Color, CurveType, PathSegment, Point, Primitive, Scene, Style, TextAnchor, TextStyle,
    interpolate,
};

use bridge::{LayoutResult, NodeLayout, NodeShape};

fn node_style() -> Style {
    Style {
        fill: Some(Color::WHITE),
        stroke: Some(Color::rgb(51, 51, 51)),
        stroke_width: Some(1.5),
        ..Default::default()
    }
}

fn merge_node_style(node: &NodeLayout) -> Style {
    let mut style = node_style();
    if let Some(custom) = &node.custom_style {
        if custom.fill.is_some() { style.fill = custom.fill; }
        if custom.stroke.is_some() { style.stroke = custom.stroke; }
        if custom.stroke_width.is_some() { style.stroke_width = custom.stroke_width; }
        if custom.stroke_dasharray.is_some() { style.stroke_dasharray = custom.stroke_dasharray.clone(); }
        if custom.opacity.is_some() { style.opacity = custom.opacity; }
    }
    style
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
    let compounds: Vec<&NodeLayout> = layout.nodes.iter().filter(|n| n.is_compound).collect();

    // Render compound (container) nodes first so children draw on top
    for node in &compounds {
        let bbox = BBox::new(node.x, node.y, node.width, node.height);
        let left = node.x - node.width / 2.0;
        let right = node.x + node.width / 2.0;
        let top = node.y - node.height / 2.0;

        scene.push(Primitive::Rect {
            bbox,
            rx: 5.0,
            ry: 5.0,
            style: merge_node_style(node),
        });

        // Compound label at the top of the box
        let label_y = top + 14.0;
        scene.push(Primitive::Text {
            position: Point::new(node.x, label_y),
            content: node.label.clone(),
            anchor: TextAnchor::Middle,
            style: label_style(),
        });

        // Header separator line below the label
        let sep_y = top + 28.0;
        scene.push(Primitive::Path {
            segments: vec![
                PathSegment::MoveTo(Point::new(left, sep_y)),
                PathSegment::LineTo(Point::new(right, sep_y)),
            ],
            style: Style {
                stroke: Some(Color::rgb(51, 51, 51)),
                stroke_width: Some(1.0),
                ..Default::default()
            },
            marker_start: None,
            marker_end: None,
        });
    }
    // Edges behind nodes
    for edge in &layout.edges {
        if edge.points.len() >= 2 {
            let mut points: Vec<Point> =
                edge.points.iter().map(|&(x, y)| Point::new(x, y)).collect();

            // Clip edges at compound boundaries
            points = clip_at_compounds(&points, &compounds);

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

    // Then render leaf nodes on top
    for node in layout.nodes.iter().filter(|n| !n.is_compound) {
        match node.shape {
            NodeShape::StartCircle => {
                scene.push(Primitive::Circle {
                    center: Point::new(node.x, node.y),
                    radius: node.width / 2.0,
                    style: Style {
                        fill: Some(Color::rgb(51, 51, 51)),
                        stroke: Some(Color::rgb(51, 51, 51)),
                        ..Default::default()
                    },
                });
            }
            NodeShape::EndBullseye => {
                let r = node.width / 2.0;
                scene.push(Primitive::Circle {
                    center: Point::new(node.x, node.y),
                    radius: r,
                    style: Style {
                        fill: None,
                        stroke: Some(Color::rgb(51, 51, 51)),
                        stroke_width: Some(1.5),
                        ..Default::default()
                    },
                });
                scene.push(Primitive::Circle {
                    center: Point::new(node.x, node.y),
                    radius: r - 4.0,
                    style: Style {
                        fill: Some(Color::rgb(51, 51, 51)),
                        ..Default::default()
                    },
                });
            }
            NodeShape::ForkJoinBar => {
                scene.push(Primitive::Rect {
                    bbox: BBox::new(node.x, node.y, node.width, node.height),
                    rx: 0.0,
                    ry: 0.0,
                    style: Style {
                        fill: Some(Color::rgb(51, 51, 51)),
                        stroke: Some(Color::rgb(51, 51, 51)),
                        ..Default::default()
                    },
                });
            }
            NodeShape::ChoiceDiamond => {
                let hw = node.width / 2.0;
                let hh = node.height / 2.0;
                scene.push(Primitive::Polygon {
                    points: vec![
                        Point::new(node.x, node.y - hh),
                        Point::new(node.x + hw, node.y),
                        Point::new(node.x, node.y + hh),
                        Point::new(node.x - hw, node.y),
                    ],
                    style: merge_node_style(node),
                });
            }
            NodeShape::NoteRect => {
                scene.push(Primitive::Rect {
                    bbox: BBox::new(node.x, node.y, node.width, node.height),
                    rx: 0.0,
                    ry: 0.0,
                    style: Style {
                        fill: Some(Color::rgb(255, 255, 204)),
                        stroke: Some(Color::rgb(170, 170, 51)),
                        stroke_width: Some(1.0),
                        ..Default::default()
                    },
                });
                scene.push(Primitive::Text {
                    position: Point::new(node.x, node.y),
                    content: node.label.clone(),
                    anchor: TextAnchor::Middle,
                    style: TextStyle {
                        font_size: 12.0,
                        fill: Some(Color::rgb(51, 51, 51)),
                        ..Default::default()
                    },
                });
            }
            NodeShape::HistoryCircle => {
                let r = node.width / 2.0;
                scene.push(Primitive::Circle {
                    center: Point::new(node.x, node.y),
                    radius: r,
                    style: Style {
                        fill: Some(Color::WHITE),
                        stroke: Some(Color::rgb(51, 51, 51)),
                        stroke_width: Some(1.5),
                        ..Default::default()
                    },
                });
                scene.push(Primitive::Text {
                    position: Point::new(node.x, node.y),
                    content: "H".to_string(),
                    anchor: TextAnchor::Middle,
                    style: TextStyle {
                        font_size: 12.0,
                        fill: Some(Color::rgb(51, 51, 51)),
                        ..Default::default()
                    },
                });
            }
            NodeShape::RoundedRect => {
                let style = merge_node_style(node);
                let node_fill = style.fill;
                scene.push(Primitive::Rect {
                    bbox: BBox::new(node.x, node.y, node.width, node.height),
                    rx: 5.0,
                    ry: 5.0,
                    style,
                });
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
                    position: Point::new(node.x, node.y),
                    content: node.label.clone(),
                    anchor: TextAnchor::Middle,
                    style: lstyle,
                });
            }
        }
    }

    // Render concurrent region dividers
    for div in &layout.dividers {
        scene.push(Primitive::Path {
            segments: vec![
                PathSegment::MoveTo(Point::new(div.x1, div.y)),
                PathSegment::LineTo(Point::new(div.x2, div.y)),
            ],
            style: Style {
                stroke: Some(Color::rgb(128, 128, 128)),
                stroke_width: Some(1.0),
                stroke_dasharray: Some(vec![10.0, 10.0]),
                ..Default::default()
            },
            marker_start: None,
            marker_end: None,
        });
    }
}

/// Clip edge points at compound boundaries.
/// If an edge enters or exits a compound rect, truncate at the border.
fn clip_at_compounds(points: &[Point], compounds: &[&NodeLayout]) -> Vec<Point> {
    let mut pts = points.to_vec();
    for compound in compounds {
        let left = compound.x - compound.width / 2.0;
        let right = compound.x + compound.width / 2.0;
        let top = compound.y - compound.height / 2.0;
        let bottom = compound.y + compound.height / 2.0;

        let inside = |p: &Point| p.x >= left && p.x <= right && p.y >= top && p.y <= bottom;

        let first_in = inside(&pts[0]);
        let last_in = inside(pts.last().unwrap());

        if !first_in && last_in {
            // Edge enters compound: clip where it crosses the border
            pts = clip_entering(&pts, left, right, top, bottom);
        } else if first_in && !last_in {
            // Edge exits compound: reverse, clip, reverse back
            pts.reverse();
            pts = clip_entering(&pts, left, right, top, bottom);
            pts.reverse();
        }
    }
    pts
}

/// Given points going from outside to inside a rect, find the border crossing
/// and return only the outside portion ending at the border.
fn clip_entering(points: &[Point], left: f64, right: f64, top: f64, bottom: f64) -> Vec<Point> {
    let inside = |p: &Point| p.x >= left && p.x <= right && p.y >= top && p.y <= bottom;

    for i in 0..points.len() - 1 {
        let a = &points[i];
        let b = &points[i + 1];
        if !inside(a) && inside(b) {
            if let Some(hit) = line_rect_intersect(a, b, left, right, top, bottom) {
                let mut result: Vec<Point> = points[..=i].to_vec();
                result.push(hit);
                return result;
            }
        }
    }
    points.to_vec()
}

/// Find where a line segment (a→b) crosses a rectangle boundary.
fn line_rect_intersect(
    a: &Point,
    b: &Point,
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
) -> Option<Point> {
    let edges: [(Point, Point); 4] = [
        (Point::new(left, top), Point::new(right, top)),       // top
        (Point::new(left, bottom), Point::new(right, bottom)), // bottom
        (Point::new(left, top), Point::new(left, bottom)),     // left
        (Point::new(right, top), Point::new(right, bottom)),   // right
    ];

    let mut best: Option<(f64, Point)> = None;
    for (c, d) in &edges {
        if let Some((t, pt)) = segment_intersect(a, b, c, d) {
            if best.is_none() || t < best.unwrap().0 {
                best = Some((t, pt));
            }
        }
    }
    best.map(|(_, pt)| pt)
}

/// Intersect two line segments. Returns (t, point) where t is parameter along a→b.
fn segment_intersect(a: &Point, b: &Point, c: &Point, d: &Point) -> Option<(f64, Point)> {
    let dx1 = b.x - a.x;
    let dy1 = b.y - a.y;
    let dx2 = d.x - c.x;
    let dy2 = d.y - c.y;

    let denom = dx1 * dy2 - dy1 * dx2;
    if denom.abs() < 1e-10 {
        return None; // parallel
    }

    let t = ((c.x - a.x) * dy2 - (c.y - a.y) * dx2) / denom;
    let u = ((c.x - a.x) * dy1 - (c.y - a.y) * dx1) / denom;

    if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
        Some((t, Point::new(a.x + t * dx1, a.y + t * dy1)))
    } else {
        None
    }
}
