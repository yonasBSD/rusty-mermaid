use super::*;
use crate::common::test_helpers::test_helpers::*;
use rusty_mermaid_core::MarkerType;

#[test]
fn simple_state_diagram_to_scene() {
    let d = crate::state::parser::parse("stateDiagram-v2\n    [*] --> Still\n    Still --> Moving")
        .unwrap();
    let layout = crate::state::bridge::layout(&d);
    let scene = to_scene(&layout);

    assert_scene_valid(&scene);

    // At least: start circle + 2 state rects + 2 state labels + 2 edge paths
    assert!(
        scene.len() >= 7,
        "expected at least 7 primitives, got {}",
        scene.len()
    );
}

#[test]
fn start_end_circles() {
    let d = crate::state::parser::parse("stateDiagram-v2\n    [*] --> Active\n    Active --> [*]")
        .unwrap();
    let layout = crate::state::bridge::layout(&d);
    let scene = to_scene(&layout);

    // Start circle (filled) + end bullseye (outer + inner = 2 circles)
    assert!(
        count_circles(&scene) >= 3,
        "expected at least 3 Circle primitives (start + end bullseye), got {}",
        count_circles(&scene)
    );
}

#[test]
fn rounded_rect_states() {
    let d = crate::state::parser::parse("stateDiagram-v2\n    [*] --> Idle\n    Idle --> Active")
        .unwrap();
    let layout = crate::state::bridge::layout(&d);
    let scene = to_scene(&layout);

    // This filter is test-specific (checks rx/ry values), keep inline
    let rects: Vec<_> = scene
        .elements()
        .iter()
        .filter(|e| matches!(e.primitive, Primitive::Rect { rx, ry, .. } if rx == 5.0 && ry == 5.0))
        .collect();
    assert!(
        rects.len() >= 2,
        "expected at least 2 rounded Rect primitives for states, got {}",
        rects.len()
    );
}

#[test]
fn edges_produce_paths_with_arrow_markers() {
    let d = crate::state::parser::parse("stateDiagram-v2\n    [*] --> Still\n    Still --> Moving")
        .unwrap();
    let layout = crate::state::bridge::layout(&d);
    let scene = to_scene(&layout);

    let arrow_paths: Vec<_> = scene
        .elements()
        .iter()
        .filter(|e| {
            matches!(
                e.primitive,
                Primitive::Path {
                    marker_end: Some(MarkerType::ArrowPoint),
                    ..
                }
            )
        })
        .collect();
    assert_eq!(
        arrow_paths.len(),
        2,
        "two transitions should produce 2 Paths with ArrowPoint markers"
    );
}

#[test]
fn compound_state_produces_background_rect_and_separator() {
    let mmd = "stateDiagram-v2\n    state Outer {\n        Inner1\n        Inner2\n    }";
    let d = crate::state::parser::parse(mmd).unwrap();
    let layout = crate::state::bridge::layout(&d);
    let scene = to_scene(&layout);

    // Compound state produces: background Rect + label Text + separator Path
    assert!(
        has_rect(&scene),
        "compound state should produce background Rect"
    );

    // Separator line: a Path with exactly 2 segments (MoveTo + LineTo)
    // Test-specific filter — keep inline
    let separator_paths: Vec<_> = scene
        .elements()
        .iter()
        .filter(|e| {
            matches!(
                &e.primitive,
                Primitive::Path {
                    segments,
                    marker_start: None,
                    marker_end: None,
                    ..
                } if segments.len() == 2
                    && matches!(segments[0], PathSegment::MoveTo(_))
                    && matches!(segments[1], PathSegment::LineTo(_))
            )
        })
        .collect();
    assert!(
        !separator_paths.is_empty(),
        "compound state should produce a header separator line"
    );

    // Compound label text
    assert!(
        has_text(&scene, "Outer"),
        "compound state label should appear in scene"
    );
}

#[test]
fn edge_path_shortened_for_arrow_marker() {
    use crate::common::rendering::{marker_inset_px, prev_endpoint};
    let d = crate::state::parser::parse("stateDiagram-v2\n    Still --> Moving").unwrap();
    let layout = crate::state::bridge::layout(&d);
    let scene = to_scene(&layout);

    // Find target node boundary
    let target = layout.nodes.iter().find(|n| n.label == "Moving").unwrap();
    let node_top = target.y - target.height / 2.0;

    for e in scene.elements() {
        if let Primitive::Path {
            segments,
            marker_end: Some(MarkerType::ArrowPoint),
            style,
            ..
        } = &e.primitive
        {
            let endpoint = prev_endpoint(segments).unwrap();
            let sw = style.stroke_width.unwrap_or(1.5);
            let expected = marker_inset_px(MarkerType::ArrowPoint, sw);
            let gap = node_top - endpoint.y;
            assert!(
                gap > 0.0,
                "state edge endpoint ({:.1}) should be above node boundary ({:.1})",
                endpoint.y,
                node_top
            );
            assert!(
                (gap - expected).abs() < 1.5,
                "state edge gap ({gap:.1}) should be ~{expected:.1}px"
            );
        }
    }
}

#[test]
fn cross_compound_edge_arrow_touches_rect_boundary() {
    // Edges crossing compound boundaries (Active → Paused) must have arrow
    // tips touching the target rect boundary, not penetrating inside.
    use crate::common::rendering::prev_endpoint;

    let d = crate::state::parser::parse(
            "stateDiagram-v2\n    [*] --> Active\n    state Active {\n        [*] --> Idle\n        Idle --> Running : start\n        Running --> Idle : stop\n        state hist1 <<history>>\n        Running --> hist1\n    }\n    Active --> Paused : pause\n    Paused --> Active : resume"
        ).unwrap();
    let layout = crate::state::bridge::layout(&d);
    let scene = to_scene(&layout);

    let paused = layout.nodes.iter().find(|n| n.id == "Paused").unwrap();
    let paused_top = paused.y - paused.height / 2.0;

    // Check that every arrow-marked path ending near the Paused node
    // has its endpoint properly shortened (not inside the rect)
    let mut found_arrow_into_paused = false;
    for e in scene.elements() {
        if let Primitive::Path {
            segments,
            marker_end: Some(MarkerType::ArrowPoint),
            ..
        } = &e.primitive
        {
            let Some(endpoint) = prev_endpoint(segments) else {
                continue;
            };
            // Check edges pointing at Paused (endpoint near Paused's top)
            if (endpoint.y - paused_top).abs() < 10.0
                && endpoint.x >= paused.x - paused.width / 2.0 - 5.0
                && endpoint.x <= paused.x + paused.width / 2.0 + 5.0
            {
                found_arrow_into_paused = true;
                assert!(
                    endpoint.y < paused_top,
                    "arrow endpoint y={:.2} should be above Paused top boundary y={:.2} \
                         (arrow tip extends past endpoint to touch boundary)",
                    endpoint.y,
                    paused_top
                );
            }
        }
    }
    assert!(
        found_arrow_into_paused,
        "should find at least one arrow targeting Paused"
    );
}

#[test]
fn nested_compounds_render_outermost_first() {
    // Outer > Middle > Inner: compound rects must appear in the scene
    // in decreasing area order so inner composites paint on top.
    let d = crate::state::parser::parse(
            "stateDiagram-v2\n    [*] --> Outer\n    state Outer {\n        [*] --> Middle\n        state Middle {\n            [*] --> Inner\n            state Inner {\n                [*] --> Core\n                Core --> Processing\n                Processing --> Core\n                Processing --> [*]\n            }\n            Inner --> [*]\n        }\n        Middle --> [*]\n    }\n    Outer --> [*]"
        ).unwrap();
    let layout = crate::state::bridge::layout(&d);
    let scene = to_scene(&layout);

    // Collect compound rect areas in scene order
    let compound_areas: Vec<f64> = scene
        .elements()
        .iter()
        .filter_map(|e| {
            if let Primitive::Rect { bbox, rx, .. } = &e.primitive {
                // Compound rects use rx=5, filter by area > typical leaf node
                if *rx == 5.0 && bbox.width * bbox.height > 5000.0 {
                    return Some(bbox.width * bbox.height);
                }
            }
            None
        })
        .collect();

    assert!(
        compound_areas.len() >= 3,
        "expected at least 3 compound rects (Outer, Middle, Inner), got {}",
        compound_areas.len()
    );
    // Each successive compound rect must have equal or smaller area
    for w in compound_areas.windows(2) {
        assert!(
            w[0] >= w[1],
            "compound rects must be ordered largest-first for correct z-ordering: {:.0} < {:.0}",
            w[0],
            w[1]
        );
    }
}

#[test]
fn empty_state_diagram() {
    let d = crate::state::parser::parse("stateDiagram-v2").unwrap();
    let layout = crate::state::bridge::layout(&d);
    let scene = to_scene(&layout);

    assert!(scene.is_empty(), "empty diagram should produce empty scene");
}
