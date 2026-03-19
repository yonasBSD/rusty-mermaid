pub mod bridge;
pub mod ir;
pub mod parser;

use rusty_mermaid_core::{
    BBox, Color, CurveType, PathSegment, Point, Primitive, Scene, Shape, Style, TextAnchor,
    TextStyle, Theme, interpolate,
};

use bridge::LayoutResult;
use crate::common::layout::NodeLayout;

use crate::common::rendering::{
    contrasting_label_style, merge_custom_style, overlay_style, render_edge_label,
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
    let compounds: Vec<&NodeLayout> = layout.nodes.iter().filter(|n| n.is_compound).collect();

    // Render compound (container) nodes first so children draw on top
    for node in &compounds {
        let bbox = BBox::new(node.x, node.y, node.width, node.height);
        let left = node.x - node.width / 2.0;
        let right = node.x + node.width / 2.0;
        let top = node.y - node.height / 2.0;

        let mut cstyle = Style {
            fill: Some(theme.composite_fill),
            stroke: Some(theme.composite_stroke),
            stroke_width: Some(1.5),
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

        // Compound label at the top of the box
        let label_y = top + 12.0;
        scene.push(Primitive::Text {
            position: Point::new(node.x, label_y),
            content: node.label.clone(),
            anchor: TextAnchor::Middle,
            style: TextStyle {
                fill: Some(theme.composite_label),
                ..Default::default()
            },
        });

        // Header separator line below the label
        let sep_y = top + 24.0;
        scene.push(Primitive::Path {
            segments: vec![
                PathSegment::MoveTo(Point::new(left, sep_y)),
                PathSegment::LineTo(Point::new(right, sep_y)),
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
    // Render concurrent region dashed rectangles
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

    // Edges behind nodes
    for edge in &layout.edges {
        if edge.points.len() >= 2 {
            let segments = interpolate(&edge.points, CurveType::Basis);

            // Clip interpolated path at compound boundaries
            let segments = clip_segments_at_compounds(&segments, &compounds);
            let label_pos = path_midpoint(&segments);
            scene.push(Primitive::Path {
                segments,
                style: Style {
                    stroke: Some(theme.edge_stroke),
                    stroke_width: Some(1.5),
                    ..Default::default()
                },
                marker_start: None,
                marker_end: Some(rusty_mermaid_core::MarkerType::ArrowPoint),
            });
            if let Some(label) = &edge.label {
                let mid = label_pos.unwrap_or(edge.points[edge.points.len() / 2]);
                render_edge_label(scene, mid, label, edge.label_size, theme);
            }
        }
    }

    // Then render leaf nodes on top
    for node in layout.nodes.iter().filter(|n| !n.is_compound) {
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
                        stroke_width: Some(1.5),
                        ..Default::default()
                    },
                });
                scene.push(Primitive::Circle {
                    center: Point::new(node.x, node.y),
                    radius: r - 4.0,
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
                        font_size: 12.0,
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

    // Render concurrent region dividers (mermaid uses stroke-dasharray: 3)
    for div in &layout.dividers {
        scene.push(Primitive::Path {
            segments: vec![
                PathSegment::MoveTo(div.start),
                PathSegment::LineTo(div.end),
            ],
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
/// Uses De Casteljau subdivision for precise cubic Bezier splitting.
fn clip_segments_at_compounds(
    segments: &[PathSegment],
    compounds: &[&NodeLayout],
) -> Vec<PathSegment> {
    let mut result = segments.to_vec();
    for compound in compounds {
        let left = compound.x - compound.width / 2.0;
        let right = compound.x + compound.width / 2.0;
        let top = compound.y - compound.height / 2.0;
        let bottom = compound.y + compound.height / 2.0;
        result = clip_path_at_rect(&result, left, right, top, bottom);
    }
    result
}

fn clip_path_at_rect(
    segments: &[PathSegment],
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
) -> Vec<PathSegment> {
    if segments.is_empty() {
        return vec![];
    }
    let inside = |p: &Point| p.x >= left && p.x <= right && p.y >= top && p.y <= bottom;

    let first = path_first_point(segments);
    let last = path_last_point(segments);

    match (first, last) {
        (Some(f), Some(l)) if !inside(&f) && inside(&l) => {
            clip_path_entering(segments, left, right, top, bottom)
        }
        (Some(f), Some(l)) if inside(&f) && !inside(&l) => {
            clip_path_exiting(segments, left, right, top, bottom)
        }
        (Some(f), Some(l)) if inside(&f) && inside(&l) => {
            // U-turn: path exits the compound and re-enters.
            // Keep the start (bullseye connection), clip at re-entry so the
            // arrow tip touches the compound boundary instead of penetrating.
            clip_path_entering(segments, left, right, top, bottom)
        }
        _ => segments.to_vec(),
    }
}

fn path_first_point(segments: &[PathSegment]) -> Option<Point> {
    match segments.first()? {
        PathSegment::MoveTo(p) => Some(*p),
        _ => None,
    }
}

fn path_last_point(segments: &[PathSegment]) -> Option<Point> {
    for seg in segments.iter().rev() {
        match seg {
            PathSegment::MoveTo(p) | PathSegment::LineTo(p) => return Some(*p),
            PathSegment::CubicTo { to, .. }
            | PathSegment::QuadTo { to, .. }
            | PathSegment::ArcTo { to, .. } => return Some(*to),
            PathSegment::Close => continue,
        }
    }
    None
}

fn clip_path_entering(
    segments: &[PathSegment],
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
) -> Vec<PathSegment> {
    let inside = |p: &Point| p.x >= left && p.x <= right && p.y >= top && p.y <= bottom;
    let mut result = Vec::new();
    let mut cursor = Point::new(0.0, 0.0);

    for seg in segments {
        match seg {
            PathSegment::MoveTo(p) => {
                result.push(*seg);
                cursor = *p;
            }
            PathSegment::LineTo(p) => {
                if !inside(&cursor) && inside(p) {
                    if let Some(hit) = line_rect_intersect(&cursor, p, left, right, top, bottom) {
                        result.push(PathSegment::LineTo(hit));
                    }
                    return result;
                }
                result.push(*seg);
                cursor = *p;
            }
            PathSegment::CubicTo { cp1, cp2, to } => {
                if !inside(&cursor) && inside(to) {
                    if let Some(t) =
                        find_cubic_rect_crossing(&cursor, cp1, cp2, to, left, right, top, bottom)
                    {
                        let (a, d, f, _, _) = de_casteljau_split(&cursor, cp1, cp2, to, t);
                        result.push(PathSegment::CubicTo { cp1: a, cp2: d, to: f });
                    }
                    return result;
                }
                result.push(*seg);
                cursor = *to;
            }
            _ => {
                result.push(*seg);
            }
        }
    }
    result
}

fn clip_path_exiting(
    segments: &[PathSegment],
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
) -> Vec<PathSegment> {
    let inside = |p: &Point| p.x >= left && p.x <= right && p.y >= top && p.y <= bottom;
    let mut cursor = Point::new(0.0, 0.0);

    for (i, seg) in segments.iter().enumerate() {
        match seg {
            PathSegment::MoveTo(p) => {
                cursor = *p;
            }
            PathSegment::LineTo(p) => {
                if inside(&cursor)
                    && !inside(p)
                    && let Some(hit) = line_rect_intersect(&cursor, p, left, right, top, bottom)
                {
                    let mut result = vec![PathSegment::MoveTo(hit), PathSegment::LineTo(*p)];
                    result.extend_from_slice(&segments[i + 1..]);
                    return result;
                }
                cursor = *p;
            }
            PathSegment::CubicTo { cp1, cp2, to } => {
                if inside(&cursor)
                    && !inside(to)
                    && let Some(t) =
                        find_cubic_rect_crossing(&cursor, cp1, cp2, to, left, right, top, bottom)
                {
                    let (_, _, f, e, c) = de_casteljau_split(&cursor, cp1, cp2, to, t);
                    let mut result = vec![
                        PathSegment::MoveTo(f),
                        PathSegment::CubicTo { cp1: e, cp2: c, to: *to },
                    ];
                    result.extend_from_slice(&segments[i + 1..]);
                    return result;
                }
                cursor = *to;
            }
            _ => {}
        }
    }
    segments.to_vec()
}

fn cubic_eval(p0: &Point, p1: &Point, p2: &Point, p3: &Point, t: f64) -> Point {
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let t2 = t * t;
    Point::new(
        mt2 * mt * p0.x + 3.0 * mt2 * t * p1.x + 3.0 * mt * t2 * p2.x + t2 * t * p3.x,
        mt2 * mt * p0.y + 3.0 * mt2 * t * p1.y + 3.0 * mt * t2 * p2.y + t2 * t * p3.y,
    )
}

/// De Casteljau subdivision at parameter t.
/// Returns (a, d, f, e, c) where first half = (p0, a, d, f), second half = (f, e, c, p3).
fn de_casteljau_split(
    p0: &Point,
    p1: &Point,
    p2: &Point,
    p3: &Point,
    t: f64,
) -> (Point, Point, Point, Point, Point) {
    let lerp =
        |a: &Point, b: &Point, t: f64| Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t);
    let a = lerp(p0, p1, t);
    let b = lerp(p1, p2, t);
    let c = lerp(p2, p3, t);
    let d = lerp(&a, &b, t);
    let e = lerp(&b, &c, t);
    let f = lerp(&d, &e, t);
    (a, d, f, e, c)
}

/// Find parameter t where a cubic Bezier crosses a rectangle boundary.
/// Sampling + binary search for robust intersection.
#[allow(clippy::too_many_arguments)]
fn find_cubic_rect_crossing(
    p0: &Point,
    p1: &Point,
    p2: &Point,
    p3: &Point,
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
) -> Option<f64> {
    let inside = |p: &Point| p.x >= left && p.x <= right && p.y >= top && p.y <= bottom;

    const N: usize = 64;
    let mut prev_in = inside(p0);

    for i in 1..=N {
        let t = i as f64 / N as f64;
        let pt = cubic_eval(p0, p1, p2, p3, t);
        let pt_in = inside(&pt);

        if prev_in != pt_in {
            let mut lo = (i - 1) as f64 / N as f64;
            let mut hi = t;
            for _ in 0..20 {
                let mid = (lo + hi) / 2.0;
                let mid_pt = cubic_eval(p0, p1, p2, p3, mid);
                if inside(&mid_pt) == prev_in {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            return Some((lo + hi) / 2.0);
        }

        prev_in = pt_in;
    }
    None
}

/// Find the arc-length midpoint of a path by flattening cubics to polylines.
fn path_midpoint(segments: &[PathSegment]) -> Option<Point> {
    let polyline = flatten_path(segments);
    if polyline.len() < 2 {
        return polyline.first().copied();
    }

    let total: f64 = polyline.windows(2).map(|w| point_dist(&w[0], &w[1])).sum();
    if total < 1e-10 {
        return Some(polyline[0]);
    }

    let target = total / 2.0;
    let mut acc = 0.0;
    for w in polyline.windows(2) {
        let d = point_dist(&w[0], &w[1]);
        if acc + d >= target {
            let t = (target - acc) / d;
            return Some(Point::new(
                w[0].x + (w[1].x - w[0].x) * t,
                w[0].y + (w[1].y - w[0].y) * t,
            ));
        }
        acc += d;
    }
    polyline.last().copied()
}

fn flatten_path(segments: &[PathSegment]) -> Vec<Point> {
    let mut pts = Vec::new();
    let mut cursor = Point::new(0.0, 0.0);
    for seg in segments {
        match seg {
            PathSegment::MoveTo(p) => {
                pts.push(*p);
                cursor = *p;
            }
            PathSegment::LineTo(p) => {
                pts.push(*p);
                cursor = *p;
            }
            PathSegment::CubicTo { cp1, cp2, to } => {
                const N: usize = 16;
                for i in 1..=N {
                    let t = i as f64 / N as f64;
                    pts.push(cubic_eval(&cursor, cp1, cp2, to, t));
                }
                cursor = *to;
            }
            _ => {}
        }
    }
    pts
}

fn point_dist(a: &Point, b: &Point) -> f64 {
    ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt()
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
        if let Some((t, pt)) = segment_intersect(a, b, c, d)
            && best.is_none_or(|(best_t, _)| t < best_t)
        {
            best = Some((t, pt));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::test_helpers::test_helpers::*;
    use rusty_mermaid_core::MarkerType;

    #[test]
    fn simple_state_diagram_to_scene() {
        let d =
            crate::state::parser::parse("stateDiagram-v2\n    [*] --> Still\n    Still --> Moving")
                .unwrap();
        let layout = crate::state::bridge::layout(&d);
        let scene = to_scene(&layout);

        assert_scene_valid(&scene);

        let prims = scene.primitives();
        // At least: start circle + 2 state rects + 2 state labels + 2 edge paths
        assert!(
            prims.len() >= 7,
            "expected at least 7 primitives, got {}",
            prims.len()
        );
    }

    #[test]
    fn start_end_circles() {
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    [*] --> Active\n    Active --> [*]",
        )
        .unwrap();
        let layout = crate::state::bridge::layout(&d);
        let scene = to_scene(&layout);

        // Start circle (filled) + end bullseye (outer + inner = 2 circles)
        assert!(
            count_circles(&scene) >= 3,
            "expected at least 3 Circle primitives (start + end bullseye), got {}",
            count_circles(&scene)
        );
    }

    #[test]
    fn rounded_rect_states() {
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    [*] --> Idle\n    Idle --> Active",
        )
        .unwrap();
        let layout = crate::state::bridge::layout(&d);
        let scene = to_scene(&layout);

        // This filter is test-specific (checks rx/ry values), keep inline
        let rects: Vec<_> = scene
            .primitives()
            .iter()
            .filter(|p| matches!(p, Primitive::Rect { rx, ry, .. } if *rx == 5.0 && *ry == 5.0))
            .collect();
        assert!(
            rects.len() >= 2,
            "expected at least 2 rounded Rect primitives for states, got {}",
            rects.len()
        );
    }

    #[test]
    fn edges_produce_paths_with_arrow_markers() {
        let d =
            crate::state::parser::parse("stateDiagram-v2\n    [*] --> Still\n    Still --> Moving")
                .unwrap();
        let layout = crate::state::bridge::layout(&d);
        let scene = to_scene(&layout);

        let arrow_paths: Vec<_> = scene
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
            arrow_paths.len(),
            2,
            "two transitions should produce 2 Paths with ArrowPoint markers"
        );
    }

    #[test]
    fn compound_state_produces_background_rect_and_separator() {
        let mmd = "stateDiagram-v2\n    state Outer {\n        Inner1\n        Inner2\n    }";
        let d = crate::state::parser::parse(mmd).unwrap();
        let layout = crate::state::bridge::layout(&d);
        let scene = to_scene(&layout);

        // Compound state produces: background Rect + label Text + separator Path
        assert!(
            has_rect(&scene),
            "compound state should produce background Rect"
        );

        // Separator line: a Path with exactly 2 segments (MoveTo + LineTo)
        // Test-specific filter — keep inline
        let separator_paths: Vec<_> = scene
            .primitives()
            .iter()
            .filter(|p| {
                matches!(
                    p,
                    Primitive::Path {
                        segments,
                        marker_start: None,
                        marker_end: None,
                        ..
                    } if segments.len() == 2
                        && matches!(segments[0], PathSegment::MoveTo(_))
                        && matches!(segments[1], PathSegment::LineTo(_))
                )
            })
            .collect();
        assert!(
            !separator_paths.is_empty(),
            "compound state should produce a header separator line"
        );

        // Compound label text
        assert!(has_text(&scene, "Outer"), "compound state label should appear in scene");
    }

    #[test]
    fn empty_state_diagram() {
        let d = crate::state::parser::parse("stateDiagram-v2").unwrap();
        let layout = crate::state::bridge::layout(&d);
        let scene = to_scene(&layout);

        assert!(
            scene.primitives().is_empty(),
            "empty diagram should produce empty scene"
        );
    }
}
