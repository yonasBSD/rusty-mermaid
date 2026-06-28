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

    /// Render mermaid, convert, and assert at least one arrow is LIVE-bound on
    /// both ends — i.e. it survives both backend gates (a marker made it an
    /// `Arrow`, and both endpoints are bindable shapes) and the binding is
    /// symmetric (each endpoint shape lists the arrow in `bound_elements`).
    fn assert_has_a_symmetric_bound_arrow(src: &str) {
        let theme = Theme::light();
        let scene = rusty_mermaid_diagrams::render_to_scene(src, &theme).unwrap();
        let elements = render_elements(&scene, &theme);
        let bound = elements.iter().find_map(|e| {
            let ElementKind::Arrow {
                start_binding: Some(s),
                end_binding: Some(d),
                ..
            } = &e.kind
            else {
                return None;
            };
            Some((e.id.clone(), s.element_id.clone(), d.element_id.clone()))
        });
        let (arrow_id, src_id, dst_id) =
            bound.unwrap_or_else(|| panic!("no fully-bound arrow in {src:?}"));
        for shape_id in [&src_id, &dst_id] {
            let shape = elements
                .iter()
                .find(|e| &e.id == shape_id)
                .unwrap_or_else(|| panic!("bound shape {shape_id} exists for {src:?}"));
            assert!(
                shape.bound_elements.iter().any(|be| be.id == arrow_id),
                "shape {shape_id} back-refs its arrow in {src:?}"
            );
        }
    }

    #[test]
    fn requirement_edges_bind_to_their_entities() {
        assert_has_a_symmetric_bound_arrow(
            "requirementDiagram\n    requirement A {\n        id: A\n    }\n    element B {\n        type: Module\n    }\n    B - satisfies -> A",
        );
    }

    #[test]
    fn c4_relationships_bind_to_their_elements() {
        assert_has_a_symmetric_bound_arrow(
            "C4Context\n    Person(admin, \"Admin\")\n    System(crm, \"CRM\")\n    Rel(admin, crm, \"Manages\")",
        );
    }

    #[test]
    fn class_typed_relations_bind_to_their_classes() {
        assert_has_a_symmetric_bound_arrow(
            "classDiagram\n    class Animal\n    class Dog\n    Animal <|-- Dog",
        );
    }

    #[test]
    fn block_edges_bind_to_their_blocks() {
        assert_has_a_symmetric_bound_arrow(
            "block-beta\n    a[\"Source\"]\n    b[\"Target\"]\n    a --> b",
        );
    }

    #[test]
    fn state_transitions_bind_to_their_states() {
        assert_has_a_symmetric_bound_arrow(
            "stateDiagram-v2\n    [*] --> Idle\n    Idle --> Running : go\n    Running --> Idle : stop",
        );
    }

    /// Count arrows that are LIVE-bound on both ends with symmetric back-refs.
    fn count_symmetric_bound_arrows(src: &str) -> usize {
        let theme = Theme::light();
        let scene = rusty_mermaid_diagrams::render_to_scene(src, &theme).unwrap();
        let elements = render_elements(&scene, &theme);
        elements
            .iter()
            .filter(|e| {
                let ElementKind::Arrow {
                    start_binding: Some(s),
                    end_binding: Some(d),
                    ..
                } = &e.kind
                else {
                    return false;
                };
                let backs = |id: &str| {
                    elements
                        .iter()
                        .any(|x| x.id == *id && x.bound_elements.iter().any(|be| be.id == e.id))
                };
                backs(&s.element_id) && backs(&d.element_id)
            })
            .count()
    }

    #[test]
    fn er_relationships_bind_to_their_entities() {
        assert_has_a_symmetric_bound_arrow("erDiagram\n    CUSTOMER ||--o{ ORDER : places");
    }

    #[test]
    fn mindmap_links_bind_parent_to_child() {
        assert_has_a_symmetric_bound_arrow("mindmap\n    Root\n        Alpha\n        Beta");
    }

    #[test]
    fn treeview_links_bind_parent_to_child() {
        assert_has_a_symmetric_bound_arrow("treeView-beta\n    root\n        a\n        b");
    }

    #[test]
    fn composite_state_boundary_transitions_bind() {
        // A transition into a composite state must bind to the composite's
        // container (tagged `Compound`) as well as to the leaf transition inside
        // it. The edge must reference the composite by its Compound id, not Node,
        // or apply_bindings can't resolve the endpoint and drops the WHOLE
        // binding — both the boundary arrow and the inner leaf arrow should bind.
        assert_eq!(
            count_symmetric_bound_arrows(
                "stateDiagram-v2\n    Outside --> Inner\n    state Inner {\n        A --> B\n    }"
            ),
            2,
            "the boundary transition and the inner leaf transition both bind"
        );
    }

    #[test]
    fn a_markerless_bound_edge_becomes_a_headless_bindable_arrow() {
        // A plain class association has no arrowhead, so it lowers to a `line`,
        // which can't carry a binding. A bound edge must be a (headless) arrow —
        // visually identical to the line, but now bindable and hand-editable.
        let theme = Theme::light();
        let scene = rusty_mermaid_diagrams::render_to_scene(
            "classDiagram\n    class A\n    class B\n    A -- B",
            &theme,
        )
        .unwrap();
        let elements = render_elements(&scene, &theme);
        let arrow = elements
            .iter()
            .find_map(|e| match &e.kind {
                ElementKind::Arrow {
                    start_arrowhead,
                    end_arrowhead,
                    start_binding,
                    end_binding,
                    ..
                } => Some((
                    start_arrowhead.is_none() && end_arrowhead.is_none(),
                    start_binding.is_some() && end_binding.is_some(),
                )),
                _ => None,
            })
            .expect("the plain association is a (headless) arrow, not a line");
        assert!(arrow.0, "a plain association carries no visible arrowhead");
        assert!(arrow.1, "both ends are bound");
    }

    #[test]
    fn self_loop_lists_its_arrow_once() {
        // A self-edge binds the same node on both ends; the node must list the
        // arrow exactly once (the di != si dedup in apply_bindings), not twice.
        let theme = Theme::light();
        let scene =
            rusty_mermaid_diagrams::render_to_scene("graph TD\n    A --> A", &theme).unwrap();
        let elements = render_elements(&scene, &theme);
        let arrow = elements
            .iter()
            .find(|e| matches!(e.kind, ElementKind::Arrow { .. }))
            .expect("a self-loop arrow");
        let node = elements
            .iter()
            .find(|e| matches!(e.kind, ElementKind::Rectangle) && !e.bound_elements.is_empty())
            .expect("the node the self-loop binds");
        let refs = node
            .bound_elements
            .iter()
            .filter(|be| be.id == arrow.id)
            .count();
        assert_eq!(refs, 1, "self-loop arrow listed once, not duplicated");
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
