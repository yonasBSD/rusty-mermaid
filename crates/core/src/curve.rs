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
        CurveType::Cardinal => interpolate_cardinal(points),
        CurveType::CatmullRom => interpolate_cardinal(points), // alpha=0 equivalent
        CurveType::MonotoneX => interpolate_monotone_x(points),
        CurveType::MonotoneY => interpolate_monotone_y(points),
        CurveType::Natural => interpolate_natural(points),
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

/// Cardinal spline interpolation matching d3-shape's curveCardinal (tension=0).
///
/// k = (1 - tension) / 6 = 1/6 produces uniform Catmull-Rom equivalent tangents.
/// Flat tangent at both endpoints (d3 default boundary behavior).
fn interpolate_cardinal(points: &[Point]) -> Vec<PathSegment> {
    if points.len() < 3 {
        return interpolate_linear(points);
    }

    let k = 1.0 / 6.0;
    let n = points.len();
    let mut segs = Vec::with_capacity(n);
    segs.push(PathSegment::MoveTo(points[0]));

    // d3 state after feeding points[0] and points[1]:
    // _x0 = p1.x, _x1 = p0.x, _x2 = p1.x → flat tangent at p0
    let (mut x0, mut y0) = (points[1].x, points[1].y);
    let (mut x1, mut y1) = (points[0].x, points[0].y);
    let (mut x2, mut y2) = (points[1].x, points[1].y);

    for i in 2..n {
        let (x, y) = (points[i].x, points[i].y);
        segs.push(PathSegment::CubicTo {
            cp1: Point::new(x1 + k * (x2 - x0), y1 + k * (y2 - y0)),
            cp2: Point::new(x2 + k * (x1 - x), y2 + k * (y1 - y)),
            to: Point::new(x2, y2),
        });
        x0 = x1;
        y0 = y1;
        x1 = x2;
        y1 = y2;
        x2 = x;
        y2 = y;
    }

    // lineEnd: final segment (flat tangent at endpoint since x1 cancels)
    segs.push(PathSegment::CubicTo {
        cp1: Point::new(x1 + k * (x2 - x0), y1 + k * (y2 - y0)),
        cp2: Point::new(x2, y2),
        to: Point::new(x2, y2),
    });

    segs
}

/// Monotone cubic Hermite interpolation in X (d3-shape's curveMonotoneX).
///
/// Uses Steffen's method to compute tangent slopes that preserve monotonicity,
/// then converts Hermite tangents to cubic Bezier control points.
fn interpolate_monotone_x(points: &[Point]) -> Vec<PathSegment> {
    if points.len() < 3 {
        return interpolate_linear(points);
    }

    let n = points.len();
    let mut tangents = vec![0.0_f64; n];

    // Compute interior tangents using Steffen's method (3-point window)
    for i in 1..n - 1 {
        let h0 = points[i].x - points[i - 1].x;
        let h1 = points[i + 1].x - points[i].x;
        let s0 = (points[i].y - points[i - 1].y) / h0;
        let s1 = (points[i + 1].y - points[i].y) / h1;
        let p = (s0 * h1 + s1 * h0) / (h0 + h1);
        tangents[i] = if s0.signum() != s1.signum() {
            0.0
        } else {
            let abs_s0 = s0.abs();
            let abs_s1 = s1.abs();
            let half_p = 0.5 * p.abs();
            s0.signum() * abs_s0.min(abs_s1).min(half_p)
        };
    }

    // Boundary tangents (one-sided: slope2 from d3)
    let h_first = points[1].x - points[0].x;
    let s_first = (points[1].y - points[0].y) / h_first;
    tangents[0] = (3.0 * s_first - tangents[1]) / 2.0;

    let h_last = points[n - 1].x - points[n - 2].x;
    let s_last = (points[n - 1].y - points[n - 2].y) / h_last;
    tangents[n - 1] = (3.0 * s_last - tangents[n - 2]) / 2.0;

    // Convert Hermite tangents → cubic Bezier segments
    let mut segs = Vec::with_capacity(n);
    segs.push(PathSegment::MoveTo(points[0]));

    for i in 0..n - 1 {
        let dx = (points[i + 1].x - points[i].x) / 3.0;
        segs.push(PathSegment::CubicTo {
            cp1: Point::new(points[i].x + dx, points[i].y + dx * tangents[i]),
            cp2: Point::new(points[i + 1].x - dx, points[i + 1].y - dx * tangents[i + 1]),
            to: points[i + 1],
        });
    }

    segs
}

/// Monotone cubic Hermite interpolation in Y (d3-shape's curveMonotoneY).
///
/// Identical to MonotoneX but with x↔y transposed.
fn interpolate_monotone_y(points: &[Point]) -> Vec<PathSegment> {
    // Transpose x↔y, run monotone_x, transpose back
    let transposed: Vec<Point> = points.iter().map(|p| Point::new(p.y, p.x)).collect();
    let segs = interpolate_monotone_x(&transposed);
    segs.into_iter()
        .map(|seg| match seg {
            PathSegment::MoveTo(p) => PathSegment::MoveTo(Point::new(p.y, p.x)),
            PathSegment::LineTo(p) => PathSegment::LineTo(Point::new(p.y, p.x)),
            PathSegment::CubicTo { cp1, cp2, to } => PathSegment::CubicTo {
                cp1: Point::new(cp1.y, cp1.x),
                cp2: Point::new(cp2.y, cp2.x),
                to: Point::new(to.y, to.x),
            },
            other => other,
        })
        .collect()
}

/// Natural cubic spline interpolation (d3-shape's curveNatural).
///
/// Solves a tridiagonal system for Bezier control points with
/// natural boundary conditions (zero second derivative at endpoints).
fn interpolate_natural(points: &[Point]) -> Vec<PathSegment> {
    if points.len() < 3 {
        return interpolate_linear(points);
    }

    let n = points.len() - 1; // number of segments
    let px = natural_control_points(&points.iter().map(|p| p.x).collect::<Vec<_>>());
    let py = natural_control_points(&points.iter().map(|p| p.y).collect::<Vec<_>>());

    let mut segs = Vec::with_capacity(n + 1);
    segs.push(PathSegment::MoveTo(points[0]));

    for i in 0..n {
        segs.push(PathSegment::CubicTo {
            cp1: Point::new(px.0[i], py.0[i]),
            cp2: Point::new(px.1[i], py.1[i]),
            to: points[i + 1],
        });
    }

    segs
}

/// Solve the tridiagonal system for natural cubic spline control points.
/// Returns (first_control_points, second_control_points) for each segment.
fn natural_control_points(x: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let n = x.len() - 1;
    debug_assert!(n >= 2);

    let mut a = vec![0.0; n];
    let mut b = vec![0.0; n];
    let mut r = vec![0.0; n];

    // Setup: boundary + interior equations
    b[0] = 2.0;
    r[0] = x[0] + 2.0 * x[1];
    for i in 1..n - 1 {
        a[i] = 1.0;
        b[i] = 4.0;
        r[i] = 4.0 * x[i] + 2.0 * x[i + 1];
    }
    a[n - 1] = 2.0;
    b[n - 1] = 7.0;
    r[n - 1] = 8.0 * x[n - 1] + x[n];

    // Forward elimination
    for i in 1..n {
        let m = a[i] / b[i - 1];
        b[i] -= m;
        r[i] -= m * r[i - 1];
    }

    // Back substitution → first control points stored in `a`
    a[n - 1] = r[n - 1] / b[n - 1];
    for i in (0..n - 1).rev() {
        a[i] = (r[i] - a[i + 1]) / b[i];
    }

    // Second control points
    for i in 0..n - 1 {
        b[i] = 2.0 * x[i + 1] - a[i + 1];
    }
    b[n - 1] = (x[n] + a[n - 1]) / 2.0;

    (a, b)
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
    fn interpolate_cardinal_two_points_falls_back_to_linear() {
        let pts = [Point::new(0.0, 0.0), Point::new(100.0, 100.0)];
        let segs = interpolate(&pts, CurveType::Cardinal);
        assert_eq!(segs.len(), 2);
        assert!(matches!(segs[1], PathSegment::LineTo(_)));
    }

    #[test]
    fn interpolate_cardinal_three_points() {
        let pts = [
            Point::new(0.0, 0.0),
            Point::new(50.0, 100.0),
            Point::new(100.0, 0.0),
        ];
        let segs = interpolate(&pts, CurveType::Cardinal);
        // MoveTo + 2 CubicTo (one per segment)
        assert_eq!(segs.len(), 3);
        assert!(matches!(segs[0], PathSegment::MoveTo(_)));
        assert!(matches!(segs[1], PathSegment::CubicTo { .. }));
        assert!(matches!(segs[2], PathSegment::CubicTo { .. }));

        // First cubic: flat tangent at start → cp1 == start point
        if let PathSegment::CubicTo { cp1, to, .. } = segs[1] {
            assert!((cp1.x - 0.0).abs() < 1e-10, "cp1.x should be 0 (flat tangent)");
            assert!((cp1.y - 0.0).abs() < 1e-10, "cp1.y should be 0 (flat tangent)");
            assert!((to.x - 50.0).abs() < 1e-10);
            assert!((to.y - 100.0).abs() < 1e-10);
        } else {
            panic!("expected CubicTo");
        }

        // Last cubic: flat tangent at end → cp2 == end point
        if let PathSegment::CubicTo { cp2, to, .. } = segs[2] {
            assert!((cp2.x - 100.0).abs() < 1e-10, "cp2 should equal endpoint (flat tangent)");
            assert!((cp2.y - 0.0).abs() < 1e-10);
            assert!((to.x - 100.0).abs() < 1e-10);
            assert!((to.y - 0.0).abs() < 1e-10);
        } else {
            panic!("expected CubicTo");
        }
    }

    #[test]
    fn interpolate_cardinal_symmetric() {
        // Symmetric points → symmetric control points
        let pts = [
            Point::new(0.0, 0.0),
            Point::new(50.0, 100.0),
            Point::new(100.0, 0.0),
        ];
        let segs = interpolate(&pts, CurveType::Cardinal);
        if let (PathSegment::CubicTo { cp2: cp2_1, .. }, PathSegment::CubicTo { cp1: cp1_2, .. }) =
            (&segs[1], &segs[2])
        {
            // cp2 of first segment and cp1 of second should be symmetric about x=50
            assert!(
                (cp2_1.x + cp1_2.x - 100.0).abs() < 1e-10,
                "control points should be symmetric: {} + {} = {}",
                cp2_1.x,
                cp1_2.x,
                cp2_1.x + cp1_2.x
            );
        }
    }

    #[test]
    fn interpolate_cardinal_four_points_produces_three_cubics() {
        let pts = [
            Point::new(0.0, 0.0),
            Point::new(33.0, 100.0),
            Point::new(66.0, 0.0),
            Point::new(100.0, 100.0),
        ];
        let segs = interpolate(&pts, CurveType::Cardinal);
        assert_eq!(segs.len(), 4); // MoveTo + 3 CubicTo
        assert!(matches!(segs[0], PathSegment::MoveTo(_)));
        for s in &segs[1..] {
            assert!(matches!(s, PathSegment::CubicTo { .. }));
        }
    }

    #[test]
    fn interpolate_catmullrom_equals_cardinal() {
        // CatmullRom with default alpha=0 is Cardinal with tension=0
        let pts = [
            Point::new(0.0, 0.0),
            Point::new(50.0, 100.0),
            Point::new(100.0, 0.0),
        ];
        let cardinal = interpolate(&pts, CurveType::Cardinal);
        let catmull = interpolate(&pts, CurveType::CatmullRom);
        assert_eq!(cardinal.len(), catmull.len());
        for (c, m) in cardinal.iter().zip(catmull.iter()) {
            assert_eq!(c, m);
        }
    }

    #[test]
    fn interpolate_monotone_x_three_points() {
        let pts = [
            Point::new(0.0, 0.0),
            Point::new(50.0, 100.0),
            Point::new(100.0, 0.0),
        ];
        let segs = interpolate(&pts, CurveType::MonotoneX);
        assert_eq!(segs.len(), 3); // MoveTo + 2 CubicTo
        assert!(matches!(segs[0], PathSegment::MoveTo(_)));
        assert!(matches!(segs[1], PathSegment::CubicTo { .. }));
        assert!(matches!(segs[2], PathSegment::CubicTo { .. }));

        // Endpoints must be exact
        if let PathSegment::CubicTo { to, .. } = segs[1] {
            assert!((to.x - 50.0).abs() < 1e-10);
            assert!((to.y - 100.0).abs() < 1e-10);
        }
        if let PathSegment::CubicTo { to, .. } = segs[2] {
            assert!((to.x - 100.0).abs() < 1e-10);
            assert!((to.y - 0.0).abs() < 1e-10);
        }
    }

    #[test]
    fn interpolate_monotone_x_preserves_monotonicity() {
        // Monotonically increasing y → tangents should be non-negative
        let pts = [
            Point::new(0.0, 0.0),
            Point::new(50.0, 30.0),
            Point::new(100.0, 80.0),
            Point::new(150.0, 100.0),
        ];
        let segs = interpolate(&pts, CurveType::MonotoneX);
        assert_eq!(segs.len(), 4);
        // All control points should have y values between their segment endpoints
        for s in &segs[1..] {
            if let PathSegment::CubicTo { cp1, cp2, to } = s {
                assert!(cp1.y >= -1e-10, "cp1.y should be non-negative: {}", cp1.y);
                assert!(cp2.y <= to.y + 1e-10, "cp2.y should not exceed endpoint");
            }
        }
    }

    #[test]
    fn interpolate_monotone_x_two_points_linear() {
        let pts = [Point::new(0.0, 0.0), Point::new(100.0, 100.0)];
        let segs = interpolate(&pts, CurveType::MonotoneX);
        assert_eq!(segs.len(), 2);
        assert!(matches!(segs[1], PathSegment::LineTo(_)));
    }

    #[test]
    fn interpolate_monotone_y_transposes() {
        let pts = [
            Point::new(0.0, 0.0),
            Point::new(100.0, 50.0),
            Point::new(0.0, 100.0),
        ];
        let segs = interpolate(&pts, CurveType::MonotoneY);
        assert_eq!(segs.len(), 3);
        assert!(matches!(segs[0], PathSegment::MoveTo(_)));
        assert!(matches!(segs[1], PathSegment::CubicTo { .. }));

        // Endpoints preserved after transpose
        if let PathSegment::CubicTo { to, .. } = segs[2] {
            assert!((to.x - 0.0).abs() < 1e-10);
            assert!((to.y - 100.0).abs() < 1e-10);
        }
    }

    #[test]
    fn interpolate_natural_three_points() {
        let pts = [
            Point::new(0.0, 0.0),
            Point::new(50.0, 100.0),
            Point::new(100.0, 0.0),
        ];
        let segs = interpolate(&pts, CurveType::Natural);
        assert_eq!(segs.len(), 3); // MoveTo + 2 CubicTo
        assert!(matches!(segs[0], PathSegment::MoveTo(_)));
        assert!(matches!(segs[1], PathSegment::CubicTo { .. }));
        assert!(matches!(segs[2], PathSegment::CubicTo { .. }));

        // Endpoints must be exact
        if let PathSegment::CubicTo { to, .. } = segs[2] {
            assert!((to.x - 100.0).abs() < 1e-10);
            assert!((to.y - 0.0).abs() < 1e-10);
        }
    }

    #[test]
    fn interpolate_natural_symmetric() {
        // Symmetric input → symmetric control points
        let pts = [
            Point::new(0.0, 0.0),
            Point::new(50.0, 100.0),
            Point::new(100.0, 0.0),
        ];
        let segs = interpolate(&pts, CurveType::Natural);
        if let (PathSegment::CubicTo { cp1: a1, cp2: a2, .. }, PathSegment::CubicTo { cp1: b1, cp2: b2, .. }) =
            (&segs[1], &segs[2])
        {
            // cp1 of seg1 and cp2 of seg2 should mirror about x=50
            assert!((a1.x + b2.x - 100.0).abs() < 1e-10, "should mirror: {} + {}", a1.x, b2.x);
            assert!((a2.x + b1.x - 100.0).abs() < 1e-10, "should mirror: {} + {}", a2.x, b1.x);
        }
    }

    #[test]
    fn interpolate_natural_two_points_linear() {
        let pts = [Point::new(0.0, 0.0), Point::new(100.0, 100.0)];
        let segs = interpolate(&pts, CurveType::Natural);
        assert_eq!(segs.len(), 2);
        assert!(matches!(segs[1], PathSegment::LineTo(_)));
    }

    #[test]
    fn interpolate_natural_four_points() {
        let pts = [
            Point::new(0.0, 0.0),
            Point::new(33.0, 100.0),
            Point::new(66.0, 0.0),
            Point::new(100.0, 100.0),
        ];
        let segs = interpolate(&pts, CurveType::Natural);
        assert_eq!(segs.len(), 4); // MoveTo + 3 CubicTo
        for s in &segs[1..] {
            assert!(matches!(s, PathSegment::CubicTo { .. }));
        }
    }
}
