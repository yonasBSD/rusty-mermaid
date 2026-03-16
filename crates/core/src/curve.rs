use crate::{PathSegment, Point};

/// Curve interpolation types for edge paths.
/// Maps to d3-shape curve factories used by mermaid/dagre.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CurveType {
    #[default]
    Linear,
    Basis,
    Cardinal,
    MonotoneX,
    MonotoneY,
    CatmullRom,
    Natural,
    Step,
    StepBefore,
    StepAfter,
    BumpX,
    BumpY,
}

/// Convert a sequence of points + curve type into path segments.
/// Pure math — no rendering. Both SVG and gpui consume the result.
pub fn interpolate(points: &[Point], curve: CurveType) -> Vec<PathSegment> {
    if points.is_empty() {
        return Vec::new();
    }
    if points.len() == 1 {
        return vec![PathSegment::MoveTo(points[0])];
    }

    match curve {
        CurveType::Linear => interpolate_linear(points),
        CurveType::Basis => interpolate_basis(points),
        CurveType::Step => interpolate_step(points, 0.5),
        CurveType::StepBefore => interpolate_step(points, 0.0),
        CurveType::StepAfter => interpolate_step(points, 1.0),
        CurveType::BumpX => interpolate_bump_x(points),
        CurveType::BumpY => interpolate_bump_y(points),
        // For curves not yet implemented, fall back to linear
        _ => interpolate_linear(points),
    }
}

fn interpolate_linear(points: &[Point]) -> Vec<PathSegment> {
    let mut segs = Vec::with_capacity(points.len());
    segs.push(PathSegment::MoveTo(points[0]));
    for &p in &points[1..] {
        segs.push(PathSegment::LineTo(p));
    }
    segs
}

/// B-spline (basis) interpolation. Produces smooth curves through averaged control points.
fn interpolate_basis(points: &[Point]) -> Vec<PathSegment> {
    if points.len() < 3 {
        return interpolate_linear(points);
    }

    let mut segs = Vec::new();

    // First point: move to the averaged start
    let p0 = points[0];
    let p1 = points[1];
    segs.push(PathSegment::MoveTo(Point::new(
        (2.0 * p0.x + p1.x) / 3.0,
        (2.0 * p0.y + p1.y) / 3.0,
    )));

    for i in 1..points.len() - 1 {
        let prev = points[i - 1];
        let curr = points[i];
        let next = points[i + 1];

        let cp1 = Point::new(
            (2.0 * curr.x + prev.x) / 3.0,
            (2.0 * curr.y + prev.y) / 3.0,
        );
        let cp2 = Point::new(
            (2.0 * curr.x + next.x) / 3.0,
            (2.0 * curr.y + next.y) / 3.0,
        );
        let to = Point::new(
            (prev.x + 4.0 * curr.x + next.x) / 6.0,
            (prev.y + 4.0 * curr.y + next.y) / 6.0,
        );

        segs.push(PathSegment::CubicTo { cp1, cp2, to });
    }

    // End: line to last point
    segs.push(PathSegment::LineTo(*points.last().unwrap()));
    segs
}

/// Step interpolation with configurable step position (0.0 = before, 0.5 = middle, 1.0 = after).
fn interpolate_step(points: &[Point], t: f64) -> Vec<PathSegment> {
    let mut segs = Vec::with_capacity(points.len() * 2);
    segs.push(PathSegment::MoveTo(points[0]));

    for i in 1..points.len() {
        let prev = points[i - 1];
        let curr = points[i];
        let mid_x = prev.x + (curr.x - prev.x) * t;
        let mid_y = prev.y + (curr.y - prev.y) * t;

        segs.push(PathSegment::LineTo(Point::new(mid_x, mid_y)));
        segs.push(PathSegment::LineTo(Point::new(
            mid_x + (curr.x - prev.x) * (1.0 - t),
            mid_y + (curr.y - prev.y) * (1.0 - t),
        )));
    }
    segs
}

/// BumpX: horizontal midpoint cubic bezier (S-curves with horizontal tangents).
fn interpolate_bump_x(points: &[Point]) -> Vec<PathSegment> {
    let mut segs = Vec::with_capacity(points.len());
    segs.push(PathSegment::MoveTo(points[0]));

    for i in 1..points.len() {
        let prev = points[i - 1];
        let curr = points[i];
        let mid_x = (prev.x + curr.x) / 2.0;

        segs.push(PathSegment::CubicTo {
            cp1: Point::new(mid_x, prev.y),
            cp2: Point::new(mid_x, curr.y),
            to: curr,
        });
    }
    segs
}

/// BumpY: vertical midpoint cubic bezier (S-curves with vertical tangents).
fn interpolate_bump_y(points: &[Point]) -> Vec<PathSegment> {
    let mut segs = Vec::with_capacity(points.len());
    segs.push(PathSegment::MoveTo(points[0]));

    for i in 1..points.len() {
        let prev = points[i - 1];
        let curr = points[i];
        let mid_y = (prev.y + curr.y) / 2.0;

        segs.push(PathSegment::CubicTo {
            cp1: Point::new(prev.x, mid_y),
            cp2: Point::new(curr.x, mid_y),
            to: curr,
        });
    }
    segs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_linear() {
        assert_eq!(CurveType::default(), CurveType::Linear);
    }

    #[test]
    fn curve_type_is_copy() {
        let c = CurveType::Cardinal;
        let c2 = c;
        assert_eq!(c, c2);
    }

    #[test]
    fn interpolate_empty() {
        assert!(interpolate(&[], CurveType::Linear).is_empty());
    }

    #[test]
    fn interpolate_single_point() {
        let segs = interpolate(&[Point::new(5.0, 10.0)], CurveType::Linear);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0], PathSegment::MoveTo(Point::new(5.0, 10.0)));
    }

    #[test]
    fn interpolate_linear_two_points() {
        let pts = [Point::new(0.0, 0.0), Point::new(100.0, 100.0)];
        let segs = interpolate(&pts, CurveType::Linear);
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0], PathSegment::MoveTo(Point::new(0.0, 0.0)));
        assert_eq!(segs[1], PathSegment::LineTo(Point::new(100.0, 100.0)));
    }

    #[test]
    fn interpolate_linear_multi() {
        let pts = [
            Point::new(0.0, 0.0),
            Point::new(50.0, 100.0),
            Point::new(100.0, 0.0),
        ];
        let segs = interpolate(&pts, CurveType::Linear);
        assert_eq!(segs.len(), 3);
    }

    #[test]
    fn interpolate_step_produces_double_segments() {
        let pts = [Point::new(0.0, 0.0), Point::new(100.0, 100.0)];
        let segs = interpolate(&pts, CurveType::Step);
        // MoveTo + 2 LineTo per step
        assert_eq!(segs.len(), 3);
    }

    #[test]
    fn interpolate_step_before() {
        let pts = [Point::new(0.0, 0.0), Point::new(100.0, 100.0)];
        let segs = interpolate(&pts, CurveType::StepBefore);
        // t=0.0: step happens at start — first LineTo goes straight to (0,0), then to (100,100)
        assert_eq!(segs.len(), 3);
        assert_eq!(segs[0], PathSegment::MoveTo(Point::new(0.0, 0.0)));
    }

    #[test]
    fn interpolate_bump_x_produces_cubics() {
        let pts = [
            Point::new(0.0, 0.0),
            Point::new(100.0, 50.0),
            Point::new(200.0, 0.0),
        ];
        let segs = interpolate(&pts, CurveType::BumpX);
        assert_eq!(segs.len(), 3);
        assert!(matches!(segs[0], PathSegment::MoveTo(_)));
        assert!(matches!(segs[1], PathSegment::CubicTo { .. }));
        assert!(matches!(segs[2], PathSegment::CubicTo { .. }));
    }

    #[test]
    fn interpolate_bump_y_produces_cubics() {
        let pts = [Point::new(0.0, 0.0), Point::new(50.0, 100.0)];
        let segs = interpolate(&pts, CurveType::BumpY);
        assert_eq!(segs.len(), 2);
        if let PathSegment::CubicTo { cp1, cp2, to } = segs[1] {
            // Vertical midpoint tangents
            assert!((cp1.x - 0.0).abs() < f64::EPSILON);
            assert!((cp1.y - 50.0).abs() < f64::EPSILON);
            assert!((cp2.x - 50.0).abs() < f64::EPSILON);
            assert!((cp2.y - 50.0).abs() < f64::EPSILON);
            assert!((to.x - 50.0).abs() < f64::EPSILON);
            assert!((to.y - 100.0).abs() < f64::EPSILON);
        } else {
            panic!("expected CubicTo");
        }
    }

    #[test]
    fn interpolate_basis_smooth() {
        let pts = [
            Point::new(0.0, 0.0),
            Point::new(50.0, 100.0),
            Point::new(100.0, 0.0),
            Point::new(150.0, 100.0),
        ];
        let segs = interpolate(&pts, CurveType::Basis);
        // MoveTo + CubicTo per inner point + final LineTo
        assert!(segs.len() >= 3);
        assert!(matches!(segs[0], PathSegment::MoveTo(_)));
    }

    #[test]
    fn interpolate_basis_two_points_falls_back_to_linear() {
        let pts = [Point::new(0.0, 0.0), Point::new(100.0, 100.0)];
        let segs = interpolate(&pts, CurveType::Basis);
        assert_eq!(segs.len(), 2);
        assert!(matches!(segs[1], PathSegment::LineTo(_)));
    }

    #[test]
    fn unimplemented_curve_falls_back_to_linear() {
        let pts = [Point::new(0.0, 0.0), Point::new(100.0, 100.0)];
        let segs = interpolate(&pts, CurveType::Natural);
        assert_eq!(segs.len(), 2);
        assert!(matches!(segs[1], PathSegment::LineTo(_)));
    }
}
