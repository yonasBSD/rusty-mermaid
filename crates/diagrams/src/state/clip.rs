use rusty_mermaid_core::{PathSegment, Point};

use crate::common::layout::NodeLayout;

/// Clip interpolated path segments at compound node boundaries.
/// Uses De Casteljau subdivision for precise cubic Bezier splitting.
pub(super) fn clip_segments_at_compounds(
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
                        let s = de_casteljau_split(&cursor, cp1, cp2, to, t);
                        result.push(PathSegment::CubicTo {
                            cp1: s.left_cp1,
                            cp2: s.left_cp2,
                            to: s.mid,
                        });
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
                    let s = de_casteljau_split(&cursor, cp1, cp2, to, t);
                    let mut result = vec![
                        PathSegment::MoveTo(s.mid),
                        PathSegment::CubicTo {
                            cp1: s.right_cp1,
                            cp2: s.right_cp2,
                            to: *to,
                        },
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

struct CubicSplit {
    left_cp1: Point,
    left_cp2: Point,
    mid: Point,
    right_cp1: Point,
    right_cp2: Point,
}

fn de_casteljau_split(p0: &Point, p1: &Point, p2: &Point, p3: &Point, t: f64) -> CubicSplit {
    let lerp =
        |a: &Point, b: &Point, t: f64| Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t);
    let a = lerp(p0, p1, t);
    let b = lerp(p1, p2, t);
    let c = lerp(p2, p3, t);
    let d = lerp(&a, &b, t);
    let e = lerp(&b, &c, t);
    let f = lerp(&d, &e, t);
    CubicSplit {
        left_cp1: a,
        left_cp2: d,
        mid: f,
        right_cp1: e,
        right_cp2: c,
    }
}

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

pub(super) fn path_midpoint(segments: &[PathSegment]) -> Option<Point> {
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

fn line_rect_intersect(
    a: &Point,
    b: &Point,
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
) -> Option<Point> {
    let edges: [(Point, Point); 4] = [
        (Point::new(left, top), Point::new(right, top)),
        (Point::new(left, bottom), Point::new(right, bottom)),
        (Point::new(left, top), Point::new(left, bottom)),
        (Point::new(right, top), Point::new(right, bottom)),
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

fn segment_intersect(a: &Point, b: &Point, c: &Point, d: &Point) -> Option<(f64, Point)> {
    let dx1 = b.x - a.x;
    let dy1 = b.y - a.y;
    let dx2 = d.x - c.x;
    let dy2 = d.y - c.y;

    let denom = dx1 * dy2 - dy1 * dx2;
    if denom.abs() < 1e-10 {
        return None;
    }

    let t = ((c.x - a.x) * dy2 - (c.y - a.y) * dx2) / denom;
    let u = ((c.x - a.x) * dy1 - (c.y - a.y) * dx1) / denom;

    if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
        Some((t, Point::new(a.x + t * dx1, a.y + t * dy1)))
    } else {
        None
    }
}
