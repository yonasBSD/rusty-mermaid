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
