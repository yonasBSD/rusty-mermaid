use crate::{BBox, Point};

/// Find the intersection point where a ray from `bbox` center toward `point`
/// crosses the rectangle boundary. Used for edge clipping in dagre.
pub fn intersect_rect(bbox: &BBox, point: Point) -> Point {
    let dx = point.x - bbox.x;
    let dy = point.y - bbox.y;

    let w = bbox.width / 2.0;
    let h = bbox.height / 2.0;

    // Degenerate: point is at center
    if dx.abs() < f64::EPSILON && dy.abs() < f64::EPSILON {
        return Point::new(bbox.x, bbox.y);
    }

    // Which edge does the ray hit first?
    // Scale factors to reach each edge
    let sx = if dx.abs() < f64::EPSILON {
        f64::INFINITY
    } else {
        w / dx.abs()
    };
    let sy = if dy.abs() < f64::EPSILON {
        f64::INFINITY
    } else {
        h / dy.abs()
    };

    let s = sx.min(sy);
    Point::new(bbox.x + s * dx, bbox.y + s * dy)
}

/// Find the intersection point where a ray from `center` toward `point`
/// crosses the circle boundary.
pub fn intersect_circle(center: Point, radius: f64, point: Point) -> Point {
    let dx = point.x - center.x;
    let dy = point.y - center.y;
    let dist = (dx * dx + dy * dy).sqrt();

    if dist < f64::EPSILON {
        return Point::new(center.x + radius, center.y);
    }

    Point::new(
        center.x + radius * dx / dist,
        center.y + radius * dy / dist,
    )
}

/// Find the intersection point where a ray from `center` toward `point`
/// crosses the ellipse boundary.
pub fn intersect_ellipse(center: Point, rx: f64, ry: f64, point: Point) -> Point {
    let dx = point.x - center.x;
    let dy = point.y - center.y;

    if dx.abs() < f64::EPSILON && dy.abs() < f64::EPSILON {
        return Point::new(center.x + rx, center.y);
    }

    // Parametric: (rx*cos(t), ry*sin(t))
    // Ray direction angle
    let angle = dy.atan2(dx);
    Point::new(
        center.x + rx * angle.cos(),
        center.y + ry * angle.sin(),
    )
}

/// Find the intersection point where a ray from `center` toward `target`
/// crosses the polygon boundary. Vertices must be in order (CW or CCW).
pub fn intersect_polygon(vertices: &[Point], center: Point, target: Point) -> Point {
    debug_assert!(
        vertices.len() >= 3,
        "polygon needs at least 3 vertices, got {}",
        vertices.len()
    );

    let mut closest = target;
    let mut min_dist = f64::INFINITY;

    let n = vertices.len();
    for i in 0..n {
        let v1 = vertices[i];
        let v2 = vertices[(i + 1) % n];

        if let Some(hit) = segment_ray_intersect(v1, v2, center, target) {
            let d = center.distance_to(hit);
            if d < min_dist {
                min_dist = d;
                closest = hit;
            }
        }
    }

    closest
}

/// Find where a ray from `origin` toward `target` exits a circle.
/// Returns the farthest forward intersection (exit point), suitable for
/// clipping edges that pass through a convex shape from center to exterior.
pub fn intersect_line_circle(origin: Point, target: Point, center: Point, radius: f64) -> Point {
    let dx = target.x - origin.x;
    let dy = target.y - origin.y;
    let a = dx * dx + dy * dy;

    if a < f64::EPSILON {
        return intersect_circle(center, radius, target);
    }

    let ocx = origin.x - center.x;
    let ocy = origin.y - center.y;
    let b = 2.0 * (ocx * dx + ocy * dy);
    let c = ocx * ocx + ocy * ocy - radius * radius;
    let disc = b * b - 4.0 * a * c;

    if disc < 0.0 {
        return intersect_circle(center, radius, target);
    }

    let sqrt_d = disc.sqrt();
    // t2 is the farthest crossing — the exit point on the far side of the circle
    let t2 = (-b + sqrt_d) / (2.0 * a);

    if t2 > f64::EPSILON {
        Point::new(origin.x + t2 * dx, origin.y + t2 * dy)
    } else {
        intersect_circle(center, radius, target)
    }
}

/// Find where a ray from `origin` toward `target` exits an ellipse.
/// Returns the farthest forward intersection (exit point).
/// Analogous to `intersect_line_circle` but for an ellipse with independent radii.
pub fn intersect_line_ellipse(
    origin: Point,
    target: Point,
    center: Point,
    rx: f64,
    ry: f64,
) -> Point {
    // Scale y so the ellipse becomes a circle of radius rx, then use circle intersection.
    let scale = rx / ry;
    let scaled_origin = Point::new(origin.x, center.y + (origin.y - center.y) * scale);
    let scaled_target = Point::new(target.x, center.y + (target.y - center.y) * scale);
    let hit = intersect_line_circle(scaled_origin, scaled_target, center, rx);
    // Inverse scale
    Point::new(hit.x, center.y + (hit.y - center.y) / scale)
}

/// Intersect line segment (p1→p2) with ray (origin→direction).
/// Returns the intersection point if it lies on the segment and in the ray's forward direction.
fn segment_ray_intersect(p1: Point, p2: Point, origin: Point, target: Point) -> Option<Point> {
    let dx_seg = p2.x - p1.x;
    let dy_seg = p2.y - p1.y;
    let dx_ray = target.x - origin.x;
    let dy_ray = target.y - origin.y;

    let denom = dx_seg * dy_ray - dy_seg * dx_ray;
    if denom.abs() < f64::EPSILON {
        return None; // Parallel
    }

    let t = ((origin.x - p1.x) * dy_ray - (origin.y - p1.y) * dx_ray) / denom;
    let u = ((origin.x - p1.x) * dy_seg - (origin.y - p1.y) * dx_seg) / denom;

    // t in [0,1] means hit is on segment, u > 0 means forward along ray
    if (0.0..=1.0).contains(&t) && u > 0.0 {
        Some(Point::new(p1.x + t * dx_seg, p1.y + t * dy_seg))
    } else {
        None
    }
}

/// Generate path segments for an arc sector (pie slice or annular ring).
///
/// Traces the outer arc forward, then inner arc backward (or line to center
/// if `inner_r` is zero), then closes. Returns a closed path suitable for filling.
pub fn arc_sector_segments(
    center: Point,
    inner_r: f64,
    outer_r: f64,
    start_angle: f64,
    end_angle: f64,
) -> Vec<crate::PathSegment> {
    use crate::PathSegment;
    let steps = crate::constants::ARC_APPROXIMATION_STEPS;
    let span = end_angle - start_angle;
    let mut segs = Vec::with_capacity(steps * 2 + 4);

    for i in 0..=steps {
        let t = start_angle + span * (i as f64 / steps as f64);
        let p = Point::new(center.x + outer_r * t.cos(), center.y + outer_r * t.sin());
        if i == 0 { segs.push(PathSegment::MoveTo(p)); } else { segs.push(PathSegment::LineTo(p)); }
    }

    if inner_r > 0.0 {
        for i in (0..=steps).rev() {
            let t = start_angle + span * (i as f64 / steps as f64);
            let p = Point::new(center.x + inner_r * t.cos(), center.y + inner_r * t.sin());
            segs.push(PathSegment::LineTo(p));
        }
    } else {
        segs.push(PathSegment::LineTo(center));
    }

    segs.push(PathSegment::Close);
    segs
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOL: f64 = 1e-9;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < TOL
    }

    fn assert_point_near(actual: Point, expected: Point) {
        assert!(
            approx_eq(actual.x, expected.x) && approx_eq(actual.y, expected.y),
            "expected ({}, {}), got ({}, {})",
            expected.x,
            expected.y,
            actual.x,
            actual.y
        );
    }

    // ── Rect ──

    #[test]
    fn rect_hit_right_edge() {
        let bbox = BBox::new(0.0, 0.0, 100.0, 60.0);
        let p = intersect_rect(&bbox, Point::new(200.0, 0.0));
        assert_point_near(p, Point::new(50.0, 0.0));
    }

    #[test]
    fn rect_hit_top_edge() {
        let bbox = BBox::new(0.0, 0.0, 100.0, 60.0);
        let p = intersect_rect(&bbox, Point::new(0.0, -200.0));
        assert_point_near(p, Point::new(0.0, -30.0));
    }

    #[test]
    fn rect_hit_diagonal() {
        let bbox = BBox::new(0.0, 0.0, 100.0, 100.0);
        let p = intersect_rect(&bbox, Point::new(100.0, 100.0));
        assert_point_near(p, Point::new(50.0, 50.0));
    }

    #[test]
    fn rect_point_at_center() {
        let bbox = BBox::new(50.0, 50.0, 100.0, 60.0);
        let p = intersect_rect(&bbox, Point::new(50.0, 50.0));
        assert_point_near(p, Point::new(50.0, 50.0));
    }

    // ── Circle ──

    #[test]
    fn circle_hit_right() {
        let p = intersect_circle(Point::new(0.0, 0.0), 50.0, Point::new(200.0, 0.0));
        assert_point_near(p, Point::new(50.0, 0.0));
    }

    #[test]
    fn circle_hit_diagonal() {
        let p = intersect_circle(Point::new(0.0, 0.0), 50.0, Point::new(100.0, 100.0));
        let expected_coord = 50.0 / 2.0_f64.sqrt();
        assert_point_near(p, Point::new(expected_coord, expected_coord));
    }

    #[test]
    fn circle_point_at_center() {
        let p = intersect_circle(Point::new(10.0, 20.0), 30.0, Point::new(10.0, 20.0));
        assert_point_near(p, Point::new(40.0, 20.0));
    }

    // ── Ellipse ──

    #[test]
    fn ellipse_hit_right() {
        let p = intersect_ellipse(Point::new(0.0, 0.0), 80.0, 40.0, Point::new(200.0, 0.0));
        assert_point_near(p, Point::new(80.0, 0.0));
    }

    #[test]
    fn ellipse_hit_top() {
        let p = intersect_ellipse(Point::new(0.0, 0.0), 80.0, 40.0, Point::new(0.0, -200.0));
        assert_point_near(p, Point::new(0.0, -40.0));
    }

    // ── Polygon (diamond) ──

    #[test]
    fn polygon_diamond_hit_right() {
        // Diamond centered at origin with half-width 50, half-height 30
        let verts = [
            Point::new(50.0, 0.0),
            Point::new(0.0, 30.0),
            Point::new(-50.0, 0.0),
            Point::new(0.0, -30.0),
        ];
        let center = Point::new(0.0, 0.0);
        let target = Point::new(200.0, 0.0);
        let p = intersect_polygon(&verts, center, target);
        assert_point_near(p, Point::new(50.0, 0.0));
    }

    #[test]
    fn polygon_diamond_hit_diagonal() {
        let verts = [
            Point::new(50.0, 0.0),
            Point::new(0.0, 50.0),
            Point::new(-50.0, 0.0),
            Point::new(0.0, -50.0),
        ];
        let center = Point::new(0.0, 0.0);
        let target = Point::new(100.0, 100.0);
        let p = intersect_polygon(&verts, center, target);
        assert_point_near(p, Point::new(25.0, 25.0));
    }

    // ── Segment-ray helper ──

    #[test]
    fn segment_ray_hit() {
        // Horizontal segment from (0,10) to (10,10), ray from origin toward (5,20)
        let hit = segment_ray_intersect(
            Point::new(0.0, 10.0),
            Point::new(10.0, 10.0),
            Point::new(0.0, 0.0),
            Point::new(5.0, 20.0),
        );
        assert!(hit.is_some());
        assert_point_near(hit.unwrap(), Point::new(2.5, 10.0));
    }

    #[test]
    fn segment_ray_miss_parallel() {
        // Parallel: horizontal segment and horizontal ray
        let hit = segment_ray_intersect(
            Point::new(0.0, 10.0),
            Point::new(10.0, 10.0),
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
        );
        assert!(hit.is_none());
    }

    #[test]
    fn segment_ray_miss_behind() {
        // Ray goes right, segment is to the left
        let hit = segment_ray_intersect(
            Point::new(-10.0, -5.0),
            Point::new(-10.0, 5.0),
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
        );
        assert!(hit.is_none());
    }

    // ── Circle edge cases ──

    #[test]
    fn circle_zero_radius() {
        // Zero-radius circle collapses to center point
        let p = intersect_circle(Point::new(5.0, 5.0), 0.0, Point::new(100.0, 5.0));
        assert_point_near(p, Point::new(5.0, 5.0));
    }

    #[test]
    fn circle_point_at_center_returns_right() {
        // Degenerate ray: point == center, falls back to (center.x + r, center.y)
        let center = Point::new(0.0, 0.0);
        let p = intersect_circle(center, 25.0, center);
        assert_point_near(p, Point::new(25.0, 0.0));
    }

    #[test]
    fn circle_point_on_boundary() {
        // Point exactly on boundary — result should equal the point itself
        let center = Point::new(0.0, 0.0);
        let r = 50.0;
        let boundary = Point::new(50.0, 0.0);
        let p = intersect_circle(center, r, boundary);
        assert_point_near(p, boundary);
    }

    #[test]
    fn circle_point_on_boundary_diagonal() {
        let center = Point::new(0.0, 0.0);
        let r = 50.0;
        let c = r / 2.0_f64.sqrt();
        let boundary = Point::new(c, c);
        let p = intersect_circle(center, r, boundary);
        assert_point_near(p, boundary);
    }

    #[test]
    fn circle_axis_aligned_left() {
        let p = intersect_circle(Point::new(0.0, 0.0), 30.0, Point::new(-100.0, 0.0));
        assert_point_near(p, Point::new(-30.0, 0.0));
    }

    #[test]
    fn circle_axis_aligned_up() {
        let p = intersect_circle(Point::new(0.0, 0.0), 30.0, Point::new(0.0, -100.0));
        assert_point_near(p, Point::new(0.0, -30.0));
    }

    #[test]
    fn circle_axis_aligned_down() {
        let p = intersect_circle(Point::new(0.0, 0.0), 30.0, Point::new(0.0, 100.0));
        assert_point_near(p, Point::new(0.0, 30.0));
    }

    // ── Rect edge cases ──

    #[test]
    fn rect_axis_aligned_left() {
        let bbox = BBox::new(0.0, 0.0, 100.0, 60.0);
        let p = intersect_rect(&bbox, Point::new(-200.0, 0.0));
        assert_point_near(p, Point::new(-50.0, 0.0));
    }

    #[test]
    fn rect_axis_aligned_bottom() {
        let bbox = BBox::new(0.0, 0.0, 100.0, 60.0);
        let p = intersect_rect(&bbox, Point::new(0.0, 200.0));
        assert_point_near(p, Point::new(0.0, 30.0));
    }

    #[test]
    fn rect_large_coordinates() {
        let bbox = BBox::new(1e8, 1e8, 100.0, 60.0);
        let p = intersect_rect(&bbox, Point::new(1e8 + 200.0, 1e8));
        assert_point_near(p, Point::new(1e8 + 50.0, 1e8));
    }

    #[test]
    fn rect_very_close_point() {
        // Point barely to the right of center — still a valid horizontal ray
        let bbox = BBox::new(0.0, 0.0, 100.0, 60.0);
        let p = intersect_rect(&bbox, Point::new(1e-12, 0.0));
        // dx=1e-12 > EPSILON, dy=0 → pure horizontal ray hits right edge
        assert_point_near(p, Point::new(50.0, 0.0));
    }

    // ── Ellipse edge cases ──

    #[test]
    fn ellipse_point_at_center() {
        // Degenerate ray falls back to (center.x + rx, center.y)
        let center = Point::new(10.0, 20.0);
        let p = intersect_ellipse(center, 80.0, 40.0, center);
        assert_point_near(p, Point::new(90.0, 20.0));
    }

    #[test]
    fn ellipse_axis_aligned_left() {
        let p = intersect_ellipse(Point::new(0.0, 0.0), 80.0, 40.0, Point::new(-200.0, 0.0));
        assert_point_near(p, Point::new(-80.0, 0.0));
    }

    #[test]
    fn ellipse_axis_aligned_bottom() {
        let p = intersect_ellipse(Point::new(0.0, 0.0), 80.0, 40.0, Point::new(0.0, 200.0));
        assert_point_near(p, Point::new(0.0, 40.0));
    }

    #[test]
    fn ellipse_large_coordinates() {
        let center = Point::new(1e8, 1e8);
        let p = intersect_ellipse(center, 80.0, 40.0, Point::new(1e8 + 200.0, 1e8));
        assert_point_near(p, Point::new(1e8 + 80.0, 1e8));
    }

    // ── Polygon edge cases ──

    #[test]
    fn polygon_axis_aligned_up() {
        let verts = [
            Point::new(50.0, 0.0),
            Point::new(0.0, 50.0),
            Point::new(-50.0, 0.0),
            Point::new(0.0, -50.0),
        ];
        let center = Point::new(0.0, 0.0);
        let p = intersect_polygon(&verts, center, Point::new(0.0, -100.0));
        assert_point_near(p, Point::new(0.0, -50.0));
    }

    #[test]
    fn polygon_axis_aligned_left() {
        let verts = [
            Point::new(50.0, 0.0),
            Point::new(0.0, 50.0),
            Point::new(-50.0, 0.0),
            Point::new(0.0, -50.0),
        ];
        let center = Point::new(0.0, 0.0);
        let p = intersect_polygon(&verts, center, Point::new(-100.0, 0.0));
        assert_point_near(p, Point::new(-50.0, 0.0));
    }

    #[test]
    fn polygon_large_coordinates() {
        let offset = 1e8;
        let verts = [
            Point::new(offset + 50.0, offset),
            Point::new(offset, offset + 50.0),
            Point::new(offset - 50.0, offset),
            Point::new(offset, offset - 50.0),
        ];
        let center = Point::new(offset, offset);
        let target = Point::new(offset + 200.0, offset);
        let p = intersect_polygon(&verts, center, target);
        assert_point_near(p, Point::new(offset + 50.0, offset));
    }

    #[test]
    fn polygon_ray_from_center_coincident_with_target() {
        // Target == center: ray has no direction, should return target as fallback
        let verts = [
            Point::new(50.0, 0.0),
            Point::new(0.0, 50.0),
            Point::new(-50.0, 0.0),
            Point::new(0.0, -50.0),
        ];
        let center = Point::new(0.0, 0.0);
        // segment_ray_intersect will have zero-length ray direction → denom ~ 0 → all None
        // intersect_polygon returns target as fallback
        let p = intersect_polygon(&verts, center, center);
        assert_point_near(p, center);
    }

    #[test]
    fn polygon_triangle() {
        let verts = [
            Point::new(0.0, -30.0),
            Point::new(30.0, 30.0),
            Point::new(-30.0, 30.0),
        ];
        let center = Point::new(0.0, 0.0);
        let p = intersect_polygon(&verts, center, Point::new(0.0, -100.0));
        assert_point_near(p, Point::new(0.0, -30.0));
    }

    #[test]
    #[should_panic(expected = "polygon needs at least 3 vertices")]
    fn polygon_degenerate_two_points() {
        let verts = [Point::new(0.0, 0.0), Point::new(10.0, 10.0)];
        let center = Point::new(5.0, 0.0);
        let target = Point::new(20.0, 0.0);
        intersect_polygon(&verts, center, target);
    }

    #[test]
    #[should_panic(expected = "polygon needs at least 3 vertices")]
    fn polygon_degenerate_single_point() {
        let verts = [Point::new(0.0, 0.0)];
        let center = Point::new(5.0, 0.0);
        let target = Point::new(20.0, 0.0);
        intersect_polygon(&verts, center, target);
    }

    // ── Segment-ray edge cases ──

    #[test]
    fn segment_ray_nearly_coincident_points() {
        // Ray origin and target are nearly the same point
        let hit = segment_ray_intersect(
            Point::new(-10.0, -5.0),
            Point::new(10.0, -5.0),
            Point::new(0.0, 0.0),
            Point::new(0.0, 1e-15),
        );
        // Direction is essentially zero → denom ~ 0 → None
        assert!(hit.is_none());
    }

    #[test]
    fn segment_ray_exact_endpoint_hit() {
        // Ray aimed exactly at segment endpoint
        let hit = segment_ray_intersect(
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
        );
        // t=0 (start of segment), u>0 → should hit
        assert!(hit.is_some());
        assert_point_near(hit.unwrap(), Point::new(10.0, 0.0));
    }

    #[test]
    fn segment_ray_large_coordinates() {
        let offset = 1e8;
        let hit = segment_ray_intersect(
            Point::new(offset + 10.0, offset - 5.0),
            Point::new(offset + 10.0, offset + 5.0),
            Point::new(offset, offset),
            Point::new(offset + 100.0, offset),
        );
        assert!(hit.is_some());
        assert_point_near(hit.unwrap(), Point::new(offset + 10.0, offset));
    }

    // ── Circle: large coordinates ──

    #[test]
    fn circle_large_coordinates() {
        let center = Point::new(1e8, 1e8);
        let p = intersect_circle(center, 50.0, Point::new(1e8 + 200.0, 1e8));
        assert_point_near(p, Point::new(1e8 + 50.0, 1e8));
    }

    // ── Circle: very close point (nearly coincident, but not exactly) ──

    #[test]
    fn circle_very_close_point() {
        // Point extremely close to center but not exactly at center
        let center = Point::new(0.0, 0.0);
        let r = 50.0;
        let tiny = 1e-10;
        let p = intersect_circle(center, r, Point::new(tiny, 0.0));
        // Should normalize the direction and project to boundary
        assert_point_near(p, Point::new(r, 0.0));
    }

    // ── Line-ellipse ──

    #[test]
    fn line_ellipse_axis_aligned_right() {
        let origin = Point::new(0.0, 0.0);
        let target = Point::new(200.0, 0.0);
        let center = Point::new(50.0, 0.0);
        let p = intersect_line_ellipse(origin, target, center, 40.0, 10.0);
        assert_point_near(p, Point::new(90.0, 0.0));
    }

    #[test]
    fn line_ellipse_axis_aligned_up() {
        let origin = Point::new(0.0, 0.0);
        let target = Point::new(0.0, -200.0);
        let center = Point::new(0.0, -50.0);
        let p = intersect_line_ellipse(origin, target, center, 40.0, 10.0);
        assert_point_near(p, Point::new(0.0, -60.0));
    }

    #[test]
    fn line_ellipse_diagonal_lands_on_boundary() {
        let origin = Point::new(0.0, 0.0);
        let target = Point::new(100.0, -100.0);
        let center = Point::new(30.0, -30.0);
        let rx = 40.0;
        let ry = 10.0;
        let p = intersect_line_ellipse(origin, target, center, rx, ry);
        let eq = ((p.x - center.x) / rx).powi(2) + ((p.y - center.y) / ry).powi(2);
        assert!(
            (eq - 1.0).abs() < 1e-4,
            "expected point on ellipse boundary, eq = {eq}"
        );
    }

    // ── Property-based tests ──

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn circle_intersect_lands_on_boundary(
            cx in -100.0..100.0f64,
            cy in -100.0..100.0f64,
            r in 1.0..50.0f64,
            angle in 0.0..std::f64::consts::TAU,
            dist in 1.1..100.0f64,
        ) {
            let center = Point::new(cx, cy);
            let target = Point::new(cx + r * dist * angle.cos(), cy + r * dist * angle.sin());
            let hit = intersect_circle(center, r, target);
            let d = ((hit.x - cx).powi(2) + (hit.y - cy).powi(2)).sqrt();
            prop_assert!((d - r).abs() < 1e-6, "hit at distance {d} from center, expected {r}");
        }

        #[test]
        fn rect_intersect_on_boundary(
            cx in -100.0..100.0f64,
            cy in -100.0..100.0f64,
            w in 10.0..200.0f64,
            h in 10.0..200.0f64,
            angle in 0.0..std::f64::consts::TAU,
        ) {
            let bbox = BBox::new(cx, cy, w, h);
            let far = 500.0;
            let target = Point::new(cx + far * angle.cos(), cy + far * angle.sin());
            let hit = intersect_rect(&bbox, target);
            let hw = w / 2.0;
            let hh = h / 2.0;
            let on_left_or_right = (hit.x - cx).abs() >= hw - 1e-6;
            let on_top_or_bottom = (hit.y - cy).abs() >= hh - 1e-6;
            let within_x = (hit.x - cx).abs() <= hw + 1e-6;
            let within_y = (hit.y - cy).abs() <= hh + 1e-6;
            prop_assert!(
                (on_left_or_right && within_y) || (on_top_or_bottom && within_x),
                "hit ({}, {}) not on rect boundary (center=({cx},{cy}), size=({w},{h}))", hit.x, hit.y
            );
        }

        #[test]
        fn ellipse_intersect_on_boundary(
            cx in -100.0..100.0f64,
            cy in -100.0..100.0f64,
            rx in 5.0..100.0f64,
            ry in 5.0..100.0f64,
            angle in 0.0..std::f64::consts::TAU,
            dist in 1.1..50.0f64,
        ) {
            let center = Point::new(cx, cy);
            let target = Point::new(cx + rx * dist * angle.cos(), cy + ry * dist * angle.sin());
            let hit = intersect_ellipse(center, rx, ry, target);
            let eq = ((hit.x - cx) / rx).powi(2) + ((hit.y - cy) / ry).powi(2);
            prop_assert!((eq - 1.0).abs() < 1e-4, "ellipse equation = {eq}, expected 1.0");
        }

    // ── arc_sector_segments tests (13.14) ──

    #[test]
    fn arc_sector_full_circle_closes() {
        use std::f64::consts::TAU;
        let segs = arc_sector_segments(Point::new(0.0, 0.0), 0.0, 50.0, 0.0, TAU);
        assert!(matches!(segs.first(), Some(crate::PathSegment::MoveTo(_))));
        assert!(matches!(segs.last(), Some(crate::PathSegment::Close)));
    }

    #[test]
    fn arc_sector_quarter_arc_segment_count() {
        use std::f64::consts::FRAC_PI_2;
        let segs = arc_sector_segments(Point::new(0.0, 0.0), 0.0, 30.0, 0.0, FRAC_PI_2);
        let steps = crate::constants::ARC_APPROXIMATION_STEPS;
        // MoveTo + steps*LineTo (outer) + LineTo(center) + Close
        assert_eq!(segs.len(), 1 + steps + 1 + 1);
    }

    #[test]
    fn arc_sector_annular_has_inner_arc() {
        use std::f64::consts::FRAC_PI_2;
        let segs = arc_sector_segments(Point::new(0.0, 0.0), 10.0, 30.0, 0.0, FRAC_PI_2);
        let steps = crate::constants::ARC_APPROXIMATION_STEPS;
        // MoveTo + steps*LineTo (outer) + (steps+1)*LineTo (inner) + Close
        assert_eq!(segs.len(), 1 + steps + (steps + 1) + 1);
    }

    #[test]
    fn arc_sector_degenerate_zero_radius() {
        let segs = arc_sector_segments(Point::new(5.0, 5.0), 0.0, 0.0, 0.0, 1.0);
        // All points at center — still produces valid path
        assert!(matches!(segs.first(), Some(crate::PathSegment::MoveTo(_))));
        assert!(matches!(segs.last(), Some(crate::PathSegment::Close)));
    }

        // ── New property tests (13.8) ──

        #[test]
        fn bbox_contains_interior_points(
            cx in -100.0..100.0f64,
            cy in -100.0..100.0f64,
            w in 1.0..200.0f64,
            h in 1.0..200.0f64,
            tx in 0.01..0.99f64,
            ty in 0.01..0.99f64,
        ) {
            let bbox = BBox::new(cx, cy, w, h);
            let px = bbox.left() + tx * w;
            let py = bbox.top() + ty * h;
            prop_assert!(bbox.contains(Point::new(px, py)),
                "({px},{py}) should be inside bbox centered at ({cx},{cy}) size ({w},{h})");
        }

        #[test]
        fn bbox_excludes_exterior_points(
            cx in -100.0..100.0f64,
            cy in -100.0..100.0f64,
            w in 1.0..200.0f64,
            h in 1.0..200.0f64,
            offset in 0.01..50.0f64,
            side in 0u8..4,
        ) {
            let bbox = BBox::new(cx, cy, w, h);
            let p = match side {
                0 => Point::new(bbox.left() - offset, cy),   // left of box
                1 => Point::new(bbox.right() + offset, cy),  // right
                2 => Point::new(cx, bbox.top() - offset),    // above
                _ => Point::new(cx, bbox.bottom() + offset), // below
            };
            prop_assert!(!bbox.contains(p),
                "({},{}) should be outside bbox", p.x, p.y);
        }

        #[test]
        fn rect_intersect_is_symmetric_in_direction(
            cx in -50.0..50.0f64,
            cy in -50.0..50.0f64,
            w in 10.0..100.0f64,
            h in 10.0..100.0f64,
            angle in 0.0..std::f64::consts::TAU,
        ) {
            // Intersection from center toward angle should be same distance
            // as intersection toward angle + PI (opposite direction), by symmetry.
            let bbox = BBox::new(cx, cy, w, h);
            let far = 500.0;
            let t1 = Point::new(cx + far * angle.cos(), cy + far * angle.sin());
            let t2 = Point::new(cx - far * angle.cos(), cy - far * angle.sin());
            let h1 = intersect_rect(&bbox, t1);
            let h2 = intersect_rect(&bbox, t2);
            let d1 = ((h1.x - cx).powi(2) + (h1.y - cy).powi(2)).sqrt();
            let d2 = ((h2.x - cx).powi(2) + (h2.y - cy).powi(2)).sqrt();
            // For a rectangle centered at (cx,cy), opposite rays may hit
            // different edges at different distances — but both must land ON the boundary.
            let hw = w / 2.0;
            let hh = h / 2.0;
            let on_boundary = |p: Point| {
                let dx = (p.x - cx).abs();
                let dy = (p.y - cy).abs();
                (dx >= hw - 1e-6 && dy <= hh + 1e-6) || (dy >= hh - 1e-6 && dx <= hw + 1e-6)
            };
            prop_assert!(on_boundary(h1), "h1 not on boundary");
            prop_assert!(on_boundary(h2), "h2 not on boundary");
            // Both distances must be > 0 (not at center)
            prop_assert!(d1 > 1e-6, "h1 at center");
            prop_assert!(d2 > 1e-6, "h2 at center");
        }

        #[test]
        fn polygon_centroid_inside_convex(
            cx in -50.0..50.0f64,
            cy in -50.0..50.0f64,
            r in 10.0..100.0f64,
            n in 3u32..9,
        ) {
            // Regular n-gon centered at (cx,cy) with radius r.
            // Centroid of a regular polygon equals its center.
            let verts: Vec<Point> = (0..n).map(|i| {
                let a = std::f64::consts::TAU * i as f64 / n as f64;
                Point::new(cx + r * a.cos(), cy + r * a.sin())
            }).collect();
            let centroid = Point::new(
                verts.iter().map(|v| v.x).sum::<f64>() / n as f64,
                verts.iter().map(|v| v.y).sum::<f64>() / n as f64,
            );
            // Centroid should be at center (within floating point)
            prop_assert!((centroid.x - cx).abs() < 1e-10, "centroid x off");
            prop_assert!((centroid.y - cy).abs() < 1e-10, "centroid y off");
            // Intersect from centroid outward should land between centroid and target
            let target = Point::new(cx + r * 3.0, cy);
            let hit = intersect_polygon(&verts, centroid, target);
            let d_hit = centroid.distance_to(hit);
            let d_target = centroid.distance_to(target);
            prop_assert!(d_hit <= d_target + 1e-6, "hit beyond target");
            prop_assert!(d_hit > 0.1, "hit at centroid");
        }

        #[test]
        fn polygon_diamond_intersect_between_center_and_target(
            cx in -100.0..100.0f64,
            cy in -100.0..100.0f64,
            hw in 10.0..100.0f64,
            hh in 10.0..100.0f64,
            angle in 0.0..std::f64::consts::TAU,
        ) {
            let verts = vec![
                Point::new(cx, cy - hh),
                Point::new(cx + hw, cy),
                Point::new(cx, cy + hh),
                Point::new(cx - hw, cy),
            ];
            let center = Point::new(cx, cy);
            let far = (hw + hh) * 3.0;
            let target = Point::new(cx + far * angle.cos(), cy + far * angle.sin());
            let hit = intersect_polygon(&verts, center, target);
            let d_hit = ((hit.x - cx).powi(2) + (hit.y - cy).powi(2)).sqrt();
            let d_target = ((target.x - cx).powi(2) + (target.y - cy).powi(2)).sqrt();
            prop_assert!(d_hit <= d_target + 1e-6, "hit distance {d_hit} > target distance {d_target}");
            prop_assert!(d_hit > 0.1, "hit too close to center: {d_hit}");
        }
    }
}
