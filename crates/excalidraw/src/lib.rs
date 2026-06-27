//! Excalidraw rendering backend for rusty-mermaid: converts a laid-out [`Scene`]
//! into editable `.excalidraw` JSON. Shapes become native Excalidraw elements,
//! and the scene's recorded `edge_bindings` become real arrow start/end bindings
//! (plus the shapes' `boundElements`), so the output is hand-editable on a
//! canvas, not a flat image.

mod convert;
mod element;

pub use element::{AppState, Binding, BoundElement, ElementKind, ExElement, ExScene, Roundness};

use rusty_mermaid_core::{Color, Scene, Theme};

/// Convert a Scene into Excalidraw elements (themed). O(n + e) in the primitive
/// and edge-binding counts: one pass mints ids + indexes source ids, one pass
/// emits elements, and each binding resolves in O(1) through the index.
pub fn render_elements(scene: &Scene, theme: &Theme) -> Vec<ExElement> {
    convert::scene_to_elements(scene, theme)
}

/// Render a Scene to a full `.excalidraw` JSON document. O(n).
pub fn to_json(scene: &Scene, theme: &Theme) -> String {
    let elements = render_elements(scene, theme);
    let doc = ExScene::new(elements, color_hex(theme.background));
    // ExScene is plain serde-derived data with no maps keyed by non-strings and
    // no custom Serialize, so this cannot fail. Panicking beats unwrap_or_default:
    // an empty string would be a silently-corrupt document a caller treats as ok.
    serde_json::to_string(&doc).expect("ExScene serialization is infallible")
}

/// `#rrggbb` for an (opaque) [`Color`].
pub(crate) fn color_hex(c: Color) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::ElementKind;

    #[test]
    fn color_hex_formats_two_digit_hex() {
        assert_eq!(color_hex(Color::rgb(255, 0, 16)), "#ff0010");
        assert_eq!(color_hex(Color::rgb(0, 0, 0)), "#000000");
    }

    #[test]
    fn empty_scene_renders_a_valid_envelope() {
        let scene = Scene::empty();
        let json = to_json(&scene, &Theme::light());
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["type"], "excalidraw");
        assert_eq!(v["version"], 2);
        assert_eq!(v["source"], "rusty-mermaid");
        assert!(v["elements"].as_array().unwrap().is_empty());
        assert!(v["appState"]["viewBackgroundColor"].is_string());
    }

    #[test]
    fn flowchart_arrow_is_bound_to_its_endpoint_nodes() {
        let theme = Theme::light();
        let scene =
            rusty_mermaid_diagrams::render_to_scene("graph TD\n    A --> B", &theme).unwrap();
        let elements = render_elements(&scene, &theme);

        let arrows: Vec<&ExElement> = elements
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::Arrow { .. }))
            .collect();
        assert_eq!(arrows.len(), 1, "one arrow for A-->B");
        let arrow = arrows[0];
        let ElementKind::Arrow {
            start_binding,
            end_binding,
            ..
        } = &arrow.kind
        else {
            panic!("expected an arrow");
        };
        let start = start_binding.as_ref().expect("arrow start bound to a node");
        let end = end_binding.as_ref().expect("arrow end bound to a node");

        // Both endpoints resolve to real shapes, and each shape lists the arrow
        // in its boundElements (the binding is symmetric, ready to hand-edit).
        for shape_id in [&start.element_id, &end.element_id] {
            let shape = elements
                .iter()
                .find(|e| &e.id == shape_id)
                .expect("bound shape exists");
            assert!(
                shape.bound_elements.iter().any(|be| be.id == arrow.id),
                "shape {shape_id} lists the arrow as a back-ref"
            );
        }
    }

    #[test]
    fn arrow_into_a_diamond_node_leaves_the_unbindable_end_unbound() {
        // A decision node `{...}` lowers to a Polygon → an Excalidraw `line`, and
        // Excalidraw refuses bindings to a line. The arrow must bind its bindable
        // end (the rectangle) and leave the diamond end unbound, with no back-ref
        // on the line — so start/end and boundElements stay symmetric per endpoint.
        let theme = Theme::light();
        let scene = rusty_mermaid_diagrams::render_to_scene(
            "graph TD\n    A[Start] --> B{Decision}",
            &theme,
        )
        .unwrap();
        let elements = render_elements(&scene, &theme);

        let arrow = elements
            .iter()
            .find(|e| matches!(e.kind, ElementKind::Arrow { .. }))
            .expect("one arrow for A-->B");
        let ElementKind::Arrow {
            start_binding,
            end_binding,
            ..
        } = &arrow.kind
        else {
            panic!("expected an arrow");
        };
        let start = start_binding
            .as_ref()
            .expect("start binds to the rectangle (A)");
        assert!(
            end_binding.is_none(),
            "the diamond end (B) is a line — Excalidraw won't bind it, so we don't"
        );

        // The diamond did lower to a line, and no line carries an arrow back-ref.
        assert!(
            elements
                .iter()
                .any(|e| matches!(e.kind, ElementKind::Line { .. })),
            "the diamond lowered to a line"
        );
        let rect = elements
            .iter()
            .find(|e| e.id == start.element_id)
            .expect("the bound rectangle exists");
        assert!(
            rect.bound_elements.iter().any(|be| be.id == arrow.id),
            "the rectangle lists the arrow as a back-ref"
        );
        for e in &elements {
            if matches!(e.kind, ElementKind::Line { .. }) {
                assert!(
                    e.bound_elements.is_empty(),
                    "a line never hosts an arrow back-ref"
                );
            }
        }
    }

    #[test]
    fn node_shapes_map_to_excalidraw_kinds() {
        let theme = Theme::light();
        let scene = rusty_mermaid_diagrams::render_to_scene(
            "graph TD\n    A[Rect]\n    B((Circle))",
            &theme,
        )
        .unwrap();
        let elements = render_elements(&scene, &theme);
        assert!(
            elements
                .iter()
                .any(|e| matches!(e.kind, ElementKind::Rectangle)),
            "a rectangle node maps to an Excalidraw rectangle"
        );
        assert!(
            elements
                .iter()
                .any(|e| matches!(e.kind, ElementKind::Ellipse)),
            "a circle node maps to an Excalidraw ellipse"
        );
        assert!(
            elements
                .iter()
                .any(|e| matches!(e.kind, ElementKind::Text { .. })),
            "node labels map to text"
        );
    }

    #[test]
    fn to_json_emits_load_bearing_fields() {
        let theme = Theme::light();
        let scene =
            rusty_mermaid_diagrams::render_to_scene("graph TD\n    A --> B --> C", &theme).unwrap();
        let v: serde_json::Value = serde_json::from_str(&to_json(&scene, &theme)).unwrap();
        let elems = v["elements"].as_array().unwrap();
        assert!(!elems.is_empty());
        for e in elems {
            assert!(e["id"].is_string(), "every element has an id");
            assert!(e["type"].is_string(), "every element has a type");
            assert!(e["x"].is_number() && e["y"].is_number(), "geometry present");
            assert!(e["version"].is_number() && e["seed"].is_number());
        }
    }

    /// Regression sentinel (§4.7): conversion stays O(n). Binding reconstruction
    /// resolves endpoints through an index (O(1)/edge); a regression to a linear
    /// `find_by_id` per edge would be O(n²) and blow this budget at 10k elements.
    #[test]
    fn conversion_stays_linear_in_element_count() {
        use rusty_mermaid_core::{
            BBox, EdgeBinding, ElementId, MarkerType, PathSegment, Point, Primitive, Scene, Style,
        };
        let n = 10_000usize;
        let mut scene = Scene::new(1.0, 1.0);
        for i in 0..n {
            scene.push_identified(
                Primitive::Rect {
                    bbox: BBox::new(i as f64, 0.0, 1.0, 1.0),
                    rx: 0.0,
                    ry: 0.0,
                    style: Style::default(),
                },
                ElementId::node(format!("n{i}")),
            );
        }
        for i in 0..(n - 1) {
            let eid = ElementId::edge(format!("e{i}"));
            scene.push_identified(
                Primitive::Path {
                    segments: vec![
                        PathSegment::MoveTo(Point::new(i as f64, 0.0)),
                        PathSegment::LineTo(Point::new(i as f64 + 1.0, 0.0)),
                    ],
                    style: Style::default(),
                    marker_start: None,
                    marker_end: Some(MarkerType::ArrowPoint),
                },
                eid.clone(),
            );
            scene.push_edge_binding(EdgeBinding {
                edge: eid,
                src: ElementId::node(format!("n{i}")),
                dst: ElementId::node(format!("n{}", i + 1)),
            });
        }

        let start = std::time::Instant::now();
        let elements = render_elements(&scene, &Theme::light());
        let elapsed = start.elapsed();

        assert_eq!(elements.len(), 2 * n - 1, "every node + edge emitted");
        assert!(
            elapsed.as_millis() < 1000,
            "conversion of {n} nodes took {elapsed:?}; O(n²) binding suspected"
        );
    }

    #[test]
    fn element_kind_carries_the_excalidraw_type_tag() {
        assert_eq!(ElementKind::Rectangle.type_str(), "rectangle");
        assert_eq!(
            ElementKind::Arrow {
                points: vec![[0.0, 0.0], [10.0, 0.0]],
                last_committed_point: None,
                start_binding: None,
                end_binding: None,
                start_arrowhead: None,
                end_arrowhead: Some("arrow".into()),
            }
            .type_str(),
            "arrow"
        );
    }
}
