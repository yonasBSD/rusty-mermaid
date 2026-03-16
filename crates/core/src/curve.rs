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

/// B-spline (basis) interpolation matching d3-shape's curveBasis.
///
/// Starts at points[0] exactly, smooths through interior via cubic B-spline,
/// ends at the last point exactly. This ensures edges connect to node boundaries.
fn interpolate_basis(points: &[Point]) -> Vec<PathSegment> {
    if points.len() < 3 {
        return interpolate_linear(points);
    }

    let n = points.len();
    let mut segs = Vec::new();

    // d3 curveBasis: MoveTo first point, then accumulate state.
    // point 0: moveTo
    // point 1: store (no output)
    // point 2: first basis segment output
    // point 3+: normal basis segments
    // lineEnd: final segment + lineTo last point

    segs.push(PathSegment::MoveTo(points[0]));

    if n == 3 {
        // With exactly 3 points: one basis curve from first→last via middle
        let (x0, y0) = (points[0].x, points[0].y);
        let (x1, y1) = (points[1].x, points[1].y);
        let (x2, y2) = (points[2].x, points[2].y);

        // d3 outputs: lineTo((5*x0+x1)/6, ...) then bezierCurveTo for point 2
        segs.push(PathSegment::LineTo(Point::new(
            (5.0 * x0 + x1) / 6.0,
            (5.0 * y0 + y1) / 6.0,
        )));
        basis_point(&mut segs, x0, y0, x1, y1, x2, y2);
        // lineEnd: final basis_point with x2,y2 repeated, then lineTo last
        basis_point(&mut segs, x1, y1, x2, y2, x2, y2);
        segs.push(PathSegment::LineTo(points[2]));
    } else {
        // 4+ points
        let (mut x0, mut y0) = (points[0].x, points[0].y);
        let (mut x1, mut y1) = (points[1].x, points[1].y);

        // point 2: first curve output
        let (x2, y2) = (points[2].x, points[2].y);
        segs.push(PathSegment::LineTo(Point::new(
            (5.0 * x0 + x1) / 6.0,
            (5.0 * y0 + y1) / 6.0,
        )));
        basis_point(&mut segs, x0, y0, x1, y1, x2, y2);
        x0 = x1;
        y0 = y1;
        x1 = x2;
        y1 = y2;

        // points 3..n-1
        for p in &points[3..] {
            basis_point(&mut segs, x0, y0, x1, y1, p.x, p.y);
            x0 = x1;
            y0 = y1;
            x1 = p.x;
            y1 = p.y;
        }

        // lineEnd: final segment + lineTo last
        basis_point(&mut segs, x0, y0, x1, y1, x1, y1);
        segs.push(PathSegment::LineTo(*points.last().unwrap()));
    }

    segs
}

/// Emit one cubic bezier segment for basis interpolation (matches d3's `point` helper).
fn basis_point(segs: &mut Vec<PathSegment>, x0: f64, y0: f64, x1: f64, y1: f64, x: f64, y: f64) {
    segs.push(PathSegment::CubicTo {
        cp1: Point::new((2.0 * x0 + x1) / 3.0, (2.0 * y0 + y1) / 3.0),
        cp2: Point::new((x0 + 2.0 * x1) / 3.0, (y0 + 2.0 * y1) / 3.0),
        to: Point::new(
            (x0 + 4.0 * x1 + x) / 6.0,
            (y0 + 4.0 * y1 + y) / 6.0,
        ),
    });
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
