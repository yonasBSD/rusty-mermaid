use rusty_mermaid_core::PathSegment;

use crate::document::fmt_f64;

/// Convert a slice of PathSegments to an SVG `d` attribute string.
pub fn segments_to_d(segments: &[PathSegment]) -> String {
    let mut d = String::with_capacity(segments.len() * 16);
    for seg in segments {
        if !d.is_empty() {
            d.push(' ');
        }
        match seg {
            PathSegment::MoveTo(p) => {
                d.push_str(&format!("M{} {}", fmt_f64(p.x), fmt_f64(p.y)));
            }
            PathSegment::LineTo(p) => {
                d.push_str(&format!("L{} {}", fmt_f64(p.x), fmt_f64(p.y)));
            }
            PathSegment::CubicTo { cp1, cp2, to } => {
                d.push_str(&format!(
                    "C{} {} {} {} {} {}",
                    fmt_f64(cp1.x), fmt_f64(cp1.y),
                    fmt_f64(cp2.x), fmt_f64(cp2.y),
                    fmt_f64(to.x), fmt_f64(to.y),
                ));
            }
            PathSegment::QuadTo { cp, to } => {
                d.push_str(&format!(
                    "Q{} {} {} {}",
                    fmt_f64(cp.x), fmt_f64(cp.y),
                    fmt_f64(to.x), fmt_f64(to.y),
                ));
            }
            PathSegment::ArcTo {
                rx, ry, rotation, large_arc, sweep, to,
            } => {
                d.push_str(&format!(
                    "A{} {} {} {} {} {} {}",
                    fmt_f64(*rx), fmt_f64(*ry), fmt_f64(*rotation),
                    if *large_arc { 1 } else { 0 },
                    if *sweep { 1 } else { 0 },
                    fmt_f64(to.x), fmt_f64(to.y),
                ));
            }
            PathSegment::Close => {
                d.push('Z');
            }
        }
    }
    d
}

#[cfg(test)]
mod tests {
    use rusty_mermaid_core::Point;

    use super::*;

    #[test]
    fn move_and_line() {
        let segs = [
            PathSegment::MoveTo(Point::new(0.0, 0.0)),
            PathSegment::LineTo(Point::new(100.0, 50.0)),
        ];
        assert_eq!(segments_to_d(&segs), "M0 0 L100 50");
    }

    #[test]
    fn cubic_bezier() {
        let segs = [
            PathSegment::MoveTo(Point::new(0.0, 0.0)),
            PathSegment::CubicTo {
                cp1: Point::new(10.0, 20.0),
                cp2: Point::new(30.0, 40.0),
                to: Point::new(50.0, 50.0),
            },
        ];
        assert_eq!(segments_to_d(&segs), "M0 0 C10 20 30 40 50 50");
    }

    #[test]
    fn quad_bezier() {
        let segs = [
            PathSegment::MoveTo(Point::new(0.0, 0.0)),
            PathSegment::QuadTo {
                cp: Point::new(25.0, 50.0),
                to: Point::new(50.0, 0.0),
            },
        ];
        assert_eq!(segments_to_d(&segs), "M0 0 Q25 50 50 0");
    }

    #[test]
    fn arc_to() {
        let segs = [
            PathSegment::MoveTo(Point::new(10.0, 10.0)),
            PathSegment::ArcTo {
                rx: 25.0,
                ry: 25.0,
                rotation: 0.0,
                large_arc: true,
                sweep: false,
                to: Point::new(50.0, 50.0),
            },
        ];
        assert_eq!(segments_to_d(&segs), "M10 10 A25 25 0 1 0 50 50");
    }

    #[test]
    fn close_path() {
        let segs = [
            PathSegment::MoveTo(Point::new(0.0, 0.0)),
            PathSegment::LineTo(Point::new(100.0, 0.0)),
            PathSegment::LineTo(Point::new(50.0, 86.6)),
            PathSegment::Close,
        ];
        let d = segments_to_d(&segs);
        assert!(d.ends_with('Z'));
    }

    #[test]
    fn decimal_values() {
        let segs = [
            PathSegment::MoveTo(Point::new(10.5, 20.75)),
            PathSegment::LineTo(Point::new(30.1, 40.0)),
        ];
        assert_eq!(segments_to_d(&segs), "M10.5 20.75 L30.1 40");
    }
}
