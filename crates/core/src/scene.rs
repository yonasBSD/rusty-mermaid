use std::fmt;

use crate::{BBox, Point, Style, TextStyle};

/// Semantic identity of a drawing element, linking a Primitive back to its
/// source in the diagram IR (node, edge, compound container, or label).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ElementId {
    pub kind: ElementKind,
    pub id: String,
}

impl ElementId {
    pub fn new(kind: ElementKind, id: impl Into<String>) -> Self {
        Self {
            kind,
            id: id.into(),
        }
    }

    pub fn node(id: impl Into<String>) -> Self {
        Self::new(ElementKind::Node, id)
    }

    pub fn edge(id: impl Into<String>) -> Self {
        Self::new(ElementKind::Edge, id)
    }

    pub fn compound(id: impl Into<String>) -> Self {
        Self::new(ElementKind::Compound, id)
    }

    pub fn label(id: impl Into<String>) -> Self {
        Self::new(ElementKind::Label, id)
    }
}

impl fmt::Display for ElementId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.kind, self.id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ElementKind {
    Node,
    Edge,
    Compound,
    Label,
}

impl fmt::Display for ElementKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Node => write!(f, "node"),
            Self::Edge => write!(f, "edge"),
            Self::Compound => write!(f, "compound"),
            Self::Label => write!(f, "label"),
        }
    }
}

/// A primitive paired with an optional semantic identity.
#[derive(Debug, Clone)]
pub struct Element {
    pub primitive: Primitive,
    pub id: Option<ElementId>,
}

/// Backend-agnostic drawing output. The contract between layout and rendering.
#[derive(Debug, Clone)]
pub struct Scene {
    pub width: f64,
    pub height: f64,
    elements: Vec<Element>,
}

impl Scene {
    pub fn new(width: f64, height: f64) -> Self {
        Self {
            width,
            height,
            elements: Vec::new(),
        }
    }

    pub fn push(&mut self, primitive: Primitive) {
        self.elements.push(Element {
            primitive,
            id: None,
        });
    }

    pub fn push_identified(&mut self, primitive: Primitive, id: ElementId) {
        self.elements.push(Element {
            primitive,
            id: Some(id),
        });
    }

    pub fn elements(&self) -> &[Element] {
        &self.elements
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Returns the ElementId at the given index, if any.
    pub fn element_id(&self, index: usize) -> Option<&ElementId> {
        self.elements.get(index).and_then(|e| e.id.as_ref())
    }

    /// Finds the first element with the given ElementId.
    pub fn find_by_id(&self, target: &ElementId) -> Option<(usize, &Element)> {
        self.elements
            .iter()
            .enumerate()
            .find(|(_, e)| e.id.as_ref() == Some(target))
    }

    /// Returns all elements matching a given ElementKind.
    pub fn find_by_kind(&self, kind: ElementKind) -> Vec<(usize, &Element)> {
        self.elements
            .iter()
            .enumerate()
            .filter(|(_, e)| e.id.as_ref().is_some_and(|id| id.kind == kind))
            .collect()
    }
}

/// A single drawing element. All backends (SVG, raster, gpui, wgpu, PDF) consume these.
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

/// Path drawing commands (standard 2D path model: MoveTo, LineTo, CubicTo, etc.).
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
        assert!(scene.is_empty());

        scene.push(Primitive::Circle {
            center: Point::new(100.0, 100.0),
            radius: 50.0,
            style: Style::default(),
        });
        assert_eq!(scene.len(), 1);
        assert!(scene.element_id(0).is_none());
        assert!(matches!(
            scene.elements()[0].primitive,
            Primitive::Circle { .. }
        ));
    }

    #[test]
    fn push_identified_stores_id() {
        let mut scene = Scene::new(100.0, 100.0);
        scene.push_identified(
            Primitive::Rect {
                bbox: BBox::new(0.0, 0.0, 50.0, 30.0),
                rx: 0.0,
                ry: 0.0,
                style: Style::default(),
            },
            ElementId::node("A"),
        );
        assert_eq!(scene.len(), 1);
        assert_eq!(scene.element_id(0), Some(&ElementId::node("A")));
    }

    #[test]
    fn mixed_push_preserves_parallel_alignment() {
        let mut scene = Scene::new(200.0, 200.0);
        scene.push(Primitive::Circle {
            center: Point::new(0.0, 0.0),
            radius: 5.0,
            style: Style::default(),
        });
        scene.push_identified(
            Primitive::Rect {
                bbox: BBox::new(10.0, 10.0, 40.0, 20.0),
                rx: 0.0,
                ry: 0.0,
                style: Style::default(),
            },
            ElementId::node("B"),
        );
        scene.push(Primitive::Circle {
            center: Point::new(50.0, 50.0),
            radius: 5.0,
            style: Style::default(),
        });
        scene.push_identified(
            Primitive::Path {
                segments: vec![],
                style: Style::default(),
                marker_start: None,
                marker_end: None,
            },
            ElementId::edge("B->C"),
        );

        assert_eq!(scene.len(), 4);
        assert!(scene.element_id(0).is_none());
        assert_eq!(scene.element_id(1), Some(&ElementId::node("B")));
        assert!(scene.element_id(2).is_none());
        assert_eq!(scene.element_id(3), Some(&ElementId::edge("B->C")));
    }

    #[test]
    fn find_by_id_returns_first_match() {
        let mut scene = Scene::new(100.0, 100.0);
        scene.push_identified(
            Primitive::Rect {
                bbox: BBox::new(0.0, 0.0, 10.0, 10.0),
                rx: 0.0,
                ry: 0.0,
                style: Style::default(),
            },
            ElementId::node("X"),
        );
        scene.push_identified(
            Primitive::Circle {
                center: Point::new(50.0, 50.0),
                radius: 5.0,
                style: Style::default(),
            },
            ElementId::node("Y"),
        );

        let (idx, elem) = scene.find_by_id(&ElementId::node("Y")).unwrap();
        assert_eq!(idx, 1);
        assert!(matches!(elem.primitive, Primitive::Circle { .. }));
        assert!(scene.find_by_id(&ElementId::node("Z")).is_none());
    }

    #[test]
    fn find_by_kind_filters_correctly() {
        let mut scene = Scene::new(100.0, 100.0);
        scene.push_identified(
            Primitive::Rect {
                bbox: BBox::new(0.0, 0.0, 10.0, 10.0),
                rx: 0.0,
                ry: 0.0,
                style: Style::default(),
            },
            ElementId::node("A"),
        );
        scene.push_identified(
            Primitive::Path {
                segments: vec![],
                style: Style::default(),
                marker_start: None,
                marker_end: None,
            },
            ElementId::edge("A->B"),
        );
        scene.push_identified(
            Primitive::Rect {
                bbox: BBox::new(20.0, 20.0, 10.0, 10.0),
                rx: 0.0,
                ry: 0.0,
                style: Style::default(),
            },
            ElementId::node("B"),
        );
        scene.push(Primitive::Circle {
            center: Point::new(0.0, 0.0),
            radius: 3.0,
            style: Style::default(),
        });

        let nodes = scene.find_by_kind(ElementKind::Node);
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].1.id.as_ref().unwrap().id, "A");
        assert_eq!(nodes[1].1.id.as_ref().unwrap().id, "B");

        let edges = scene.find_by_kind(ElementKind::Edge);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].1.id.as_ref().unwrap().id, "A->B");

        let compounds = scene.find_by_kind(ElementKind::Compound);
        assert!(compounds.is_empty());
    }

    #[test]
    fn elements_slice_pairs_primitives_with_ids() {
        let mut scene = Scene::new(100.0, 100.0);
        scene.push(Primitive::Circle {
            center: Point::new(0.0, 0.0),
            radius: 5.0,
            style: Style::default(),
        });
        scene.push_identified(
            Primitive::Rect {
                bbox: BBox::new(0.0, 0.0, 10.0, 10.0),
                rx: 0.0,
                ry: 0.0,
                style: Style::default(),
            },
            ElementId::compound("sub1"),
        );

        let elems = scene.elements();
        assert_eq!(elems.len(), 2);
        assert!(elems[0].id.is_none());
        assert_eq!(elems[1].id.as_ref(), Some(&ElementId::compound("sub1")));
    }

    #[test]
    fn element_id_display() {
        assert_eq!(ElementId::node("A").to_string(), "node:A");
        assert_eq!(ElementId::edge("A->B").to_string(), "edge:A->B");
        assert_eq!(ElementId::compound("sub").to_string(), "compound:sub");
        assert_eq!(ElementId::label("lbl").to_string(), "label:lbl");
    }

    #[test]
    fn element_id_equality() {
        assert_eq!(ElementId::node("A"), ElementId::node("A"));
        assert_ne!(ElementId::node("A"), ElementId::node("B"));
        assert_ne!(ElementId::node("A"), ElementId::edge("A"));
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
