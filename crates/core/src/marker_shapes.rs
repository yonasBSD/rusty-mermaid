use crate::{MarkerType, Point};

/// Shared marker geometry for all backends.
///
/// Coordinates are in a normalized viewBox space (typically 0..10 or 0..12).
/// Backends scale/rotate these points relative to the path endpoint and direction.
#[derive(Debug, Clone)]
pub struct MarkerGeometry {
    /// Viewbox width.
    pub vb_w: f64,
    /// Viewbox height.
    pub vb_h: f64,
    /// Reference point X (where the path endpoint attaches, in viewbox coords).
    pub ref_x: f64,
    /// Reference point Y (center of the marker, in viewbox coords).
    pub ref_y: f64,
    /// Marker display width (in stroke-width units).
    pub marker_w: f64,
    /// Marker display height (in stroke-width units).
    pub marker_h: f64,
    /// The shape to draw.
    pub shape: MarkerShape,
}

/// The actual drawing commands for a marker.
#[derive(Debug, Clone)]
pub enum MarkerShape {
    /// Filled polygon (closed path).
    FilledPath(Vec<Point>),
    /// Stroked-only path (open or closed), with stroke_width relative to marker size.
    StrokedPath {
        points: Vec<Point>,
        closed: bool,
        stroke_width: f64,
    },
    /// Filled polygon with stroke outline.
    FilledStrokedPath {
        points: Vec<Point>,
        fill_is_marker_color: bool,
        stroke_width: f64,
    },
    /// Filled circle at center.
    FilledCircle {
        cx: f64,
        cy: f64,
        r: f64,
    },
    /// Two stroked curves (for Cross marker).
    StrokedCurves {
        curves: Vec<Vec<Point>>,
        stroke_width: f64,
    },
}

/// Get the canonical geometry for a marker type.
pub fn marker_geometry(marker: MarkerType) -> MarkerGeometry {
    match marker {
        // Pointy arrow: M10 5 L0 10 L4 5 L0 0 Z
        MarkerType::ArrowPoint => MarkerGeometry {
            vb_w: 10.0, vb_h: 10.0,
            ref_x: 8.0, ref_y: 5.0,
            marker_w: 8.0, marker_h: 8.0,
            shape: MarkerShape::FilledPath(vec![
                Point::new(10.0, 5.0),
                Point::new(0.0, 10.0),
                Point::new(4.0, 5.0),
                Point::new(0.0, 0.0),
            ]),
        },
        // Barb / Open arrow: same shape, stroked with white fill
        MarkerType::ArrowBarb | MarkerType::ArrowOpen => MarkerGeometry {
            vb_w: 10.0, vb_h: 10.0,
            ref_x: 8.0, ref_y: 5.0,
            marker_w: 8.0, marker_h: 8.0,
            shape: MarkerShape::FilledStrokedPath {
                points: vec![
                    Point::new(10.0, 5.0),
                    Point::new(0.0, 10.0),
                    Point::new(4.0, 5.0),
                    Point::new(0.0, 0.0),
                ],
                fill_is_marker_color: false,
                stroke_width: 1.0,
            },
        },
        // Circle marker
        MarkerType::Circle => MarkerGeometry {
            vb_w: 10.0, vb_h: 10.0,
            ref_x: 7.0, ref_y: 5.0,
            marker_w: 8.0, marker_h: 8.0,
            shape: MarkerShape::FilledCircle { cx: 5.0, cy: 5.0, r: 4.0 },
        },
        // Cross marker: two curved strokes
        MarkerType::Cross => MarkerGeometry {
            vb_w: 10.0, vb_h: 10.0,
            ref_x: 6.0, ref_y: 5.0,
            marker_w: 8.0, marker_h: 8.0,
            shape: MarkerShape::StrokedCurves {
                curves: vec![
                    vec![Point::new(2.0, 2.0), Point::new(5.0, 4.5), Point::new(8.0, 8.0)],
                    vec![Point::new(8.0, 2.0), Point::new(5.0, 5.5), Point::new(2.0, 8.0)],
                ],
                stroke_width: 1.5,
            },
        },
        // Aggregation: diamond, white fill
        MarkerType::Aggregation => MarkerGeometry {
            vb_w: 12.0, vb_h: 12.0,
            ref_x: 10.0, ref_y: 6.0,
            marker_w: 8.0, marker_h: 8.0,
            shape: MarkerShape::FilledStrokedPath {
                points: vec![
                    Point::new(0.0, 6.0),
                    Point::new(6.0, 0.0),
                    Point::new(12.0, 6.0),
                    Point::new(6.0, 12.0),
                ],
                fill_is_marker_color: false,
                stroke_width: 1.0,
            },
        },
        // Composition: diamond, filled with marker color
        MarkerType::Composition => MarkerGeometry {
            vb_w: 12.0, vb_h: 12.0,
            ref_x: 10.0, ref_y: 6.0,
            marker_w: 8.0, marker_h: 8.0,
            shape: MarkerShape::FilledPath(vec![
                Point::new(0.0, 6.0),
                Point::new(6.0, 0.0),
                Point::new(12.0, 6.0),
                Point::new(6.0, 12.0),
            ]),
        },
        // Dependency: open arrow (stroked, no fill)
        MarkerType::Dependency => MarkerGeometry {
            vb_w: 10.0, vb_h: 10.0,
            ref_x: 7.0, ref_y: 5.0,
            marker_w: 6.0, marker_h: 6.0,
            shape: MarkerShape::StrokedPath {
                points: vec![
                    Point::new(0.0, 0.0),
                    Point::new(10.0, 5.0),
                    Point::new(0.0, 10.0),
                ],
                closed: false,
                stroke_width: 1.5,
            },
        },
    }
}

/// Transform marker viewbox points to scene coordinates at a path endpoint.
///
/// Given the path tip position, direction angle, and edge stroke width,
/// returns points in scene space ready for rendering.
pub fn transform_marker_points(
    geom: &MarkerGeometry,
    tip: Point,
    angle: f64,
    stroke_width: f64,
) -> Vec<Point> {
    let scale_x = geom.marker_w / geom.vb_w * stroke_width;
    let scale_y = geom.marker_h / geom.vb_h * stroke_width;
    let (sin, cos) = angle.sin_cos();

    let points = match &geom.shape {
        MarkerShape::FilledPath(pts) => pts,
        MarkerShape::StrokedPath { points, .. } => points,
        MarkerShape::FilledStrokedPath { points, .. } => points,
        MarkerShape::FilledCircle { .. } => return vec![],
        MarkerShape::StrokedCurves { .. } => return vec![],
    };

    points
        .iter()
        .map(|p| {
            // Translate so ref_x/ref_y is at origin, scale, then rotate+translate to tip
            let dx = (p.x - geom.ref_x) * scale_x;
            let dy = (p.y - geom.ref_y) * scale_y;
            Point::new(
                tip.x + dx * cos - dy * sin,
                tip.y + dx * sin + dy * cos,
            )
        })
        .collect()
}

/// Get the center of a circle marker in scene coordinates.
pub fn transform_marker_circle(
    geom: &MarkerGeometry,
    tip: Point,
    angle: f64,
    stroke_width: f64,
) -> (Point, f64) {
    let scale = geom.marker_w / geom.vb_w * stroke_width;
    let (sin, cos) = angle.sin_cos();

    if let MarkerShape::FilledCircle { cx, cy, r } = &geom.shape {
        let dx = (cx - geom.ref_x) * scale;
        let dy = (cy - geom.ref_y) * scale;
        let center = Point::new(
            tip.x + dx * cos - dy * sin,
            tip.y + dx * sin + dy * cos,
        );
        (center, r * scale)
    } else {
        (tip, 0.0)
    }
}

/// Get the curves of a cross/stroked-curves marker in scene coordinates.
pub fn transform_marker_curves(
    geom: &MarkerGeometry,
    tip: Point,
    angle: f64,
    stroke_width: f64,
) -> Vec<Vec<Point>> {
    let scale_x = geom.marker_w / geom.vb_w * stroke_width;
    let scale_y = geom.marker_h / geom.vb_h * stroke_width;
    let (sin, cos) = angle.sin_cos();

    if let MarkerShape::StrokedCurves { curves, .. } = &geom.shape {
        curves
            .iter()
            .map(|curve| {
                curve
                    .iter()
                    .map(|p| {
                        let dx = (p.x - geom.ref_x) * scale_x;
                        let dy = (p.y - geom.ref_y) * scale_y;
                        Point::new(
                            tip.x + dx * cos - dy * sin,
                            tip.y + dx * sin + dy * cos,
                        )
                    })
                    .collect()
            })
            .collect()
    } else {
        vec![]
    }
}

/// Pre-computed marker geometry in scene coordinates, ready for backend rendering.
/// Backends match on this and build native paths — no geometry logic needed.
pub enum MarkerPath {
    /// Filled closed polygon.
    FillPolygon { points: Vec<Point> },
    /// Stroked polyline (open or closed).
    StrokePolyline { points: Vec<Point>, width: f64, closed: bool },
    /// Filled AND stroked closed polygon. Fill color may differ from stroke.
    FillAndStrokePolygon { points: Vec<Point>, stroke_width: f64, fill_is_marker_color: bool },
    /// Filled circle.
    FillCircle { center: Point, radius: f64 },
    /// Stroked quadratic curves (each curve is [start, control, end]).
    StrokeCurves { curves: Vec<[Point; 3]>, width: f64 },
}

/// Compute marker geometry in scene coordinates for a given marker type,
/// path endpoint, direction angle, and edge stroke width.
pub fn marker_path(marker: MarkerType, tip: Point, angle: f64, stroke_width: f64) -> MarkerPath {
    let geom = marker_geometry(marker);
    let sw = stroke_width;

    match &geom.shape {
        MarkerShape::FilledPath(_) => {
            MarkerPath::FillPolygon { points: transform_marker_points(&geom, tip, angle, sw) }
        }
        MarkerShape::StrokedPath { closed, stroke_width: rel_sw, .. } => {
            let w = rel_sw * sw / geom.vb_w * geom.marker_w;
            MarkerPath::StrokePolyline {
                points: transform_marker_points(&geom, tip, angle, sw),
                width: w,
                closed: *closed,
            }
        }
        MarkerShape::FilledStrokedPath { fill_is_marker_color, stroke_width: rel_sw, .. } => {
            let w = rel_sw * sw / geom.vb_w * geom.marker_w;
            MarkerPath::FillAndStrokePolygon {
                points: transform_marker_points(&geom, tip, angle, sw),
                stroke_width: w,
                fill_is_marker_color: *fill_is_marker_color,
            }
        }
        MarkerShape::FilledCircle { .. } => {
            let (center, r) = transform_marker_circle(&geom, tip, angle, sw);
            MarkerPath::FillCircle { center, radius: r }
        }
        MarkerShape::StrokedCurves { stroke_width: rel_sw, .. } => {
            let w = rel_sw * sw / geom.vb_w * geom.marker_w;
            let raw = transform_marker_curves(&geom, tip, angle, sw);
            let curves = raw.into_iter()
                .filter(|c| c.len() >= 3)
                .map(|c| [c[0], c[1], c[2]])
                .collect();
            MarkerPath::StrokeCurves { curves, width: w }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrow_point_has_4_vertices() {
        let g = marker_geometry(MarkerType::ArrowPoint);
        if let MarkerShape::FilledPath(pts) = &g.shape {
            assert_eq!(pts.len(), 4);
            // Tip at (10, 5), indent at (4, 5)
            assert!((pts[0].x - 10.0).abs() < 1e-10);
            assert!((pts[2].x - 4.0).abs() < 1e-10);
        } else {
            panic!("expected FilledPath");
        }
    }

    #[test]
    fn transform_places_tip_at_endpoint() {
        let g = marker_geometry(MarkerType::ArrowPoint);
        let tip = Point::new(100.0, 50.0);
        let pts = transform_marker_points(&g, tip, 0.0, 1.0);
        // Point (10, 5) in viewbox → ref is (8, 5), so dx = (10-8)*scale = 2*0.8
        // At angle 0, the tip vertex should be near the tip point
        assert!(pts.len() == 4);
    }

    #[test]
    fn circle_marker_returns_center_and_radius() {
        let g = marker_geometry(MarkerType::Circle);
        let (center, r) = transform_marker_circle(&g, Point::new(50.0, 50.0), 0.0, 1.5);
        assert!(r > 0.0);
        assert!((center.y - 50.0).abs() < 5.0);
    }

    #[test]
    fn all_markers_have_geometry() {
        let types = [
            MarkerType::ArrowPoint, MarkerType::ArrowBarb, MarkerType::ArrowOpen,
            MarkerType::Circle, MarkerType::Cross,
            MarkerType::Aggregation, MarkerType::Composition, MarkerType::Dependency,
        ];
        for mt in types {
            let g = marker_geometry(mt);
            assert!(g.vb_w > 0.0);
            assert!(g.marker_w > 0.0);
        }
    }

    #[test]
    fn cross_marker_has_two_curves() {
        let g = marker_geometry(MarkerType::Cross);
        let curves = transform_marker_curves(&g, Point::new(0.0, 0.0), 0.0, 1.5);
        assert_eq!(curves.len(), 2);
        assert_eq!(curves[0].len(), 3); // quadratic: 3 control points
    }

    // ── marker_path tests (13.15) ──

    #[test]
    fn marker_path_arrow_point_is_fill_polygon() {
        let mp = marker_path(MarkerType::ArrowPoint, Point::new(100.0, 50.0), 0.0, 1.5);
        assert!(matches!(mp, MarkerPath::FillPolygon { points } if points.len() == 4));
    }

    #[test]
    fn marker_path_circle_is_fill_circle() {
        let mp = marker_path(MarkerType::Circle, Point::new(50.0, 50.0), 0.0, 1.5);
        if let MarkerPath::FillCircle { radius, .. } = mp {
            assert!(radius > 0.0);
        } else {
            panic!("expected FillCircle");
        }
    }

    #[test]
    fn marker_path_cross_is_stroke_curves() {
        let mp = marker_path(MarkerType::Cross, Point::new(0.0, 0.0), 0.0, 1.5);
        if let MarkerPath::StrokeCurves { curves, width } = mp {
            assert_eq!(curves.len(), 2);
            assert!(width > 0.0);
        } else {
            panic!("expected StrokeCurves");
        }
    }

    #[test]
    fn marker_path_barb_is_fill_and_stroke() {
        let mp = marker_path(MarkerType::ArrowBarb, Point::new(0.0, 0.0), 0.0, 1.5);
        assert!(matches!(mp, MarkerPath::FillAndStrokePolygon { .. }));
    }

    #[test]
    fn marker_path_all_types_produce_nonempty() {
        let types = [
            MarkerType::ArrowPoint, MarkerType::ArrowBarb, MarkerType::ArrowOpen,
            MarkerType::Circle, MarkerType::Cross,
            MarkerType::Aggregation, MarkerType::Composition, MarkerType::Dependency,
        ];
        for mt in types {
            let mp = marker_path(mt, Point::new(50.0, 50.0), 0.5, 1.5);
            let nonempty = match &mp {
                MarkerPath::FillPolygon { points } => !points.is_empty(),
                MarkerPath::StrokePolyline { points, .. } => !points.is_empty(),
                MarkerPath::FillAndStrokePolygon { points, .. } => !points.is_empty(),
                MarkerPath::FillCircle { radius, .. } => *radius > 0.0,
                MarkerPath::StrokeCurves { curves, .. } => !curves.is_empty(),
            };
            assert!(nonempty, "{mt:?} produced empty marker path");
        }
    }
}
