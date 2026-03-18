use crate::{BBox, Color, Point, Style, TextStyle};

/// Backend-agnostic drawing output. The contract between layout and rendering.
#[derive(Debug, Clone)]
pub struct Scene {
    pub width: f64,
    pub height: f64,
    primitives: Vec<Primitive>,
    pub marker_color: Option<Color>,
}

impl Scene {
    pub fn new(width: f64, height: f64) -> Self {
        Self {
            width,
            height,
            primitives: Vec::new(),
            marker_color: None,
        }
    }

    pub fn push(&mut self, primitive: Primitive) {
        self.primitives.push(primitive);
    }

    pub fn primitives(&self) -> &[Primitive] {
        &self.primitives
    }
}

/// A single drawing element. SVG and gpui both consume these.
#[derive(Debug, Clone)]
pub enum Primitive {
    Rect {
        bbox: BBox,
        rx: f64,
        ry: f64,
        style: Style,
    },
    Circle {
        center: Point,
        radius: f64,
        style: Style,
    },
    Ellipse {
        center: Point,
        rx: f64,
        ry: f64,
        style: Style,
    },
    Path {
        segments: Vec<PathSegment>,
        style: Style,
        marker_start: Option<MarkerType>,
        marker_end: Option<MarkerType>,
    },
    Text {
        position: Point,
        content: String,
        anchor: TextAnchor,
        style: TextStyle,
    },
    Polygon {
        points: Vec<Point>,
        style: Style,
    },
    Group {
        transform: Transform,
        children: Vec<Primitive>,
    },
    Arc {
        center: Point,
        inner_r: f64,
        outer_r: f64,
        start_angle: f64,
        end_angle: f64,
        style: Style,
    },
}

/// Path drawing commands, mirroring SVG path data.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathSegment {
    MoveTo(Point),
    LineTo(Point),
    CubicTo {
        cp1: Point,
        cp2: Point,
        to: Point,
    },
    QuadTo {
        cp: Point,
        to: Point,
    },
    ArcTo {
        rx: f64,
        ry: f64,
        rotation: f64,
        large_arc: bool,
        sweep: bool,
        to: Point,
    },
    Close,
}

/// 2D affine transform.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Transform {
    #[default]
    Identity,
    Translate(f64, f64),
    Scale(f64, f64),
    Rotate {
        degrees: f64,
        cx: f64,
        cy: f64,
    },
}

/// Text horizontal alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextAnchor {
    Start,
    #[default]
    Middle,
    End,
}

/// Arrow/marker types for path endpoints.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarkerType {
    ArrowPoint,
    ArrowBarb,
    ArrowOpen,
    Circle,
    Cross,
    Aggregation,
    Composition,
    Dependency,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Color;

    #[test]
    fn scene_new_and_push() {
        let mut scene = Scene::new(800.0, 600.0);
        assert!(scene.primitives().is_empty());

        scene.push(Primitive::Circle {
            center: Point::new(100.0, 100.0),
            radius: 50.0,
            style: Style::default(),
        });
        assert_eq!(scene.primitives().len(), 1);
    }

    #[test]
    fn path_segment_move_and_line() {
        let segments = [
            PathSegment::MoveTo(Point::new(0.0, 0.0)),
            PathSegment::LineTo(Point::new(100.0, 0.0)),
            PathSegment::LineTo(Point::new(100.0, 100.0)),
            PathSegment::Close,
        ];
        assert_eq!(segments.len(), 4);
        assert_eq!(segments[0], PathSegment::MoveTo(Point::new(0.0, 0.0)));
    }

    #[test]
    fn path_segment_cubic() {
        let seg = PathSegment::CubicTo {
            cp1: Point::new(10.0, 20.0),
            cp2: Point::new(30.0, 40.0),
            to: Point::new(50.0, 50.0),
        };
        if let PathSegment::CubicTo { cp1, cp2, to } = seg {
            assert!((cp1.x - 10.0).abs() < f64::EPSILON);
            assert!((cp2.y - 40.0).abs() < f64::EPSILON);
            assert!((to.x - 50.0).abs() < f64::EPSILON);
        } else {
            panic!("expected CubicTo");
        }
    }

    #[test]
    fn transform_default_is_identity() {
        assert_eq!(Transform::default(), Transform::Identity);
    }

    #[test]
    fn text_anchor_default_is_middle() {
        assert_eq!(TextAnchor::default(), TextAnchor::Middle);
    }

    #[test]
    fn primitive_rect() {
        let rect = Primitive::Rect {
            bbox: BBox::new(50.0, 50.0, 100.0, 60.0),
            rx: 5.0,
            ry: 5.0,
            style: Style {
                fill: Some(Color::WHITE),
                stroke: Some(Color::BLACK),
                ..Default::default()
            },
        };
        if let Primitive::Rect { bbox, rx, .. } = &rect {
            assert!((bbox.width - 100.0).abs() < f64::EPSILON);
            assert!((*rx - 5.0).abs() < f64::EPSILON);
        } else {
            panic!("expected Rect");
        }
    }

    #[test]
    fn primitive_text() {
        let text = Primitive::Text {
            position: Point::new(10.0, 20.0),
            content: String::from("Hello"),
            anchor: TextAnchor::Start,
            style: TextStyle::default(),
        };
        if let Primitive::Text {
            content, anchor, ..
        } = &text
        {
            assert_eq!(content, "Hello");
            assert_eq!(*anchor, TextAnchor::Start);
        } else {
            panic!("expected Text");
        }
    }

    #[test]
    fn primitive_group_nesting() {
        let inner = Primitive::Circle {
            center: Point::new(0.0, 0.0),
            radius: 10.0,
            style: Style::default(),
        };
        let group = Primitive::Group {
            transform: Transform::Translate(50.0, 50.0),
            children: vec![inner],
        };
        if let Primitive::Group {
            transform,
            children,
        } = &group
        {
            assert_eq!(*transform, Transform::Translate(50.0, 50.0));
            assert_eq!(children.len(), 1);
        } else {
            panic!("expected Group");
        }
    }

    #[test]
    fn primitive_path_with_markers() {
        let path = Primitive::Path {
            segments: vec![
                PathSegment::MoveTo(Point::new(0.0, 0.0)),
                PathSegment::LineTo(Point::new(100.0, 100.0)),
            ],
            style: Style::default(),
            marker_start: None,
            marker_end: Some(MarkerType::ArrowPoint),
        };
        if let Primitive::Path {
            marker_end,
            segments,
            ..
        } = &path
        {
            assert_eq!(*marker_end, Some(MarkerType::ArrowPoint));
            assert_eq!(segments.len(), 2);
        } else {
            panic!("expected Path");
        }
    }

    #[test]
    fn primitive_arc() {
        let arc = Primitive::Arc {
            center: Point::new(100.0, 100.0),
            inner_r: 0.0,
            outer_r: 50.0,
            start_angle: 0.0,
            end_angle: std::f64::consts::PI,
            style: Style::default(),
        };
        if let Primitive::Arc {
            start_angle,
            end_angle,
            ..
        } = &arc
        {
            assert!((*start_angle - 0.0).abs() < f64::EPSILON);
            assert!((*end_angle - std::f64::consts::PI).abs() < f64::EPSILON);
        } else {
            panic!("expected Arc");
        }
    }
}
