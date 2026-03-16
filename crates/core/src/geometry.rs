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
}
