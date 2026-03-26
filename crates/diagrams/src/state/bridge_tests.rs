    use rusty_mermaid_core::Direction;

    use super::*;
    use crate::state::ir::*;

    #[test]
    fn composite_children_aligned() {
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    [*] --> Active\n    state Active {\n        [*] --> Idle\n        Idle --> Running\n        Running --> Idle\n    }\n    Active --> [*]"
        ).unwrap();
        let result = layout(&d);

        let idle = result.nodes.iter().find(|n| n.id == "Idle").unwrap();
        let running = result.nodes.iter().find(|n| n.id == "Running").unwrap();
        assert!(
            (idle.x - running.x).abs() < 1.0,
            "Idle (x={:.1}) and Running (x={:.1}) should be x-aligned",
            idle.x, running.x
        );
    }

    #[test]
    fn layout_simple_chain() {
        let mut d = StateDiagram::new(Direction::TB);
        d.states.push(StateNode::new("A", StateKind::Normal));
        d.states.push(StateNode::new("B", StateKind::Normal));
        d.transitions.push(StateTransition::new("A", "B"));

        let result = layout(&d);
        assert_eq!(result.nodes.len(), 2);
        assert_eq!(result.edges.len(), 1);

        let a = result.nodes.iter().find(|n| n.id == "A").unwrap();
        let b = result.nodes.iter().find(|n| n.id == "B").unwrap();
        assert!(a.y < b.y, "A should be above B in TB layout");
    }

    #[test]
    fn layout_with_start_end() {
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    [*] --> Still\n    Still --> [*]"
        ).unwrap();
        let result = layout(&d);
        // start + end + Still = 3 nodes
        assert_eq!(result.nodes.len(), 3);
        assert_eq!(result.edges.len(), 2);
    }

    #[test]
    fn layout_fork_join() {
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    state fork1 <<fork>>\n    state join1 <<join>>\n    [*] --> fork1\n    fork1 --> A\n    fork1 --> B\n    A --> join1\n    B --> join1\n    join1 --> [*]"
        ).unwrap();
        let result = layout(&d);
        let fork = result.nodes.iter().find(|n| n.id == "fork1").unwrap();
        assert!((fork.width - FORK_JOIN_WIDTH).abs() < 1.0);
    }

    #[test]
    fn layout_edge_has_points() {
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    A --> B\n    B --> C"
        ).unwrap();
        let result = layout(&d);
        for e in &result.edges {
            assert!(!e.points.is_empty(), "edge {}->{} should have points", e.src, e.dst);
        }
    }

    #[test]
    fn layout_composite_has_inner_edges() {
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    [*] --> Active\n    state Active {\n        [*] --> Idle\n        Idle --> Running\n        Running --> Idle\n    }\n    Active --> [*]"
        ).unwrap();
        let result = layout(&d);

        // Nodes: [*]_start, [*]_end, Active, Active.[*]_start, Idle, Running
        assert!(result.nodes.iter().any(|n| n.id == "Idle"), "should have Idle");
        assert!(result.nodes.iter().any(|n| n.id == "Running"), "should have Running");
        assert!(result.nodes.iter().any(|n| n.id == "Active.[*]_start"), "should have inner start");

        // Should have inner edges
        assert!(result.edges.iter().any(|e| e.src == "Active.[*]_start" && e.dst == "Idle"),
            "should have inner [*] --> Idle edge");
        assert!(result.edges.iter().any(|e| e.src == "Idle" && e.dst == "Running"),
            "should have Idle --> Running edge");

        // Active should be marked as compound
        let active = result.nodes.iter().find(|n| n.id == "Active").unwrap();
        assert!(active.is_compound, "Active should be compound");
        let idle = result.nodes.iter().find(|n| n.id == "Idle").unwrap();
        let active_left = active.x - active.width / 2.0;
        let active_right = active.x + active.width / 2.0;
        assert!(active_left <= idle.x - idle.width / 2.0,
            "Active should contain Idle: active_left={active_left} idle_left={}",
            idle.x - idle.width / 2.0);
        assert!(active_right >= idle.x + idle.width / 2.0,
            "Active should contain Idle: active_right={active_right} idle_right={}",
            idle.x + idle.width / 2.0);

        // TB layout: [*]_start should be ABOVE Active, [*]_end BELOW
        let start = result.nodes.iter().find(|n| n.id == "[*]_start").unwrap();
        let end = result.nodes.iter().find(|n| n.id == "[*]_end").unwrap();
        let active_top = active.y - active.height / 2.0;
        let active_bottom = active.y + active.height / 2.0;
        assert!(start.y < active_top,
            "[*]_start (y={}) should be above Active top (y={active_top})",
            start.y);
        assert!(end.y > active_bottom,
            "[*]_end (y={}) should be below Active bottom (y={active_bottom})",
            end.y);
    }

    #[test]
    fn choice_diamond_edges_clip_to_polygon() {
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    state check <<choice>>\n    [*] --> check\n    check --> A : yes\n    check --> B : no\n    A --> [*]\n    B --> [*]"
        ).unwrap();
        let result = layout(&d);

        let check = result.nodes.iter().find(|n| n.id == "check").unwrap();
        let check_bottom_y = check.y + check.height / 2.0;
        let check_bottom_x = check.x; // diamond's bottom vertex

        // Edges from check should start ON the diamond polygon, not at the
        // bounding box corners. For edges going down-left/down-right, the
        // start point should be on the diamond edge, not at (corner_x, bottom_y).
        let check_to_a = result.edges.iter()
            .find(|e| e.src == "check" && e.dst == "A")
            .expect("check → A edge");
        let check_to_b = result.edges.iter()
            .find(|e| e.src == "check" && e.dst == "B")
            .expect("check → B edge");

        let ax = check_to_a.points[0].x;
        let ay = check_to_a.points[0].y;
        let bx = check_to_b.points[0].x;
        let by = check_to_b.points[0].y;

        // The start points should NOT be at the exact bottom-y of the bounding
        // box (that would be rect clipping). For diagonal exits, the polygon
        // intersection hits the diamond's slanted edge, producing y < bottom_y.
        // At minimum, for points not directly below center, x should differ
        // from center AND y should be < bottom_y (on the slanted edge).
        if (ax - check_bottom_x).abs() > 1.0 {
            assert!(ay < check_bottom_y - 0.5,
                "check→A start ({ax:.1},{ay:.1}) should be on diamond edge, not bbox bottom y={check_bottom_y:.1}");
        }
        if (bx - check_bottom_x).abs() > 1.0 {
            assert!(by < check_bottom_y - 0.5,
                "check→B start ({bx:.1},{by:.1}) should be on diamond edge, not bbox bottom y={check_bottom_y:.1}");
        }
    }

    #[test]
    fn node_shapes_propagated() {
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    state fork1 <<fork>>\n    state join1 <<join>>\n    state check <<choice>>\n    [*] --> fork1\n    fork1 --> A\n    fork1 --> B\n    A --> check\n    check --> join1 : yes\n    B --> join1\n    join1 --> [*]"
        ).unwrap();
        let result = layout(&d);

        let start = result.nodes.iter().find(|n| n.id == "[*]_start").unwrap();
        assert_eq!(start.shape, Shape::StateStart);

        let end = result.nodes.iter().find(|n| n.id == "[*]_end").unwrap();
        assert_eq!(end.shape, Shape::StateEnd);

        let fork = result.nodes.iter().find(|n| n.id == "fork1").unwrap();
        assert_eq!(fork.shape, Shape::ForkJoin);

        let join = result.nodes.iter().find(|n| n.id == "join1").unwrap();
        assert_eq!(join.shape, Shape::ForkJoin);

        let choice = result.nodes.iter().find(|n| n.id == "check").unwrap();
        assert_eq!(choice.shape, Shape::Choice);

        let a = result.nodes.iter().find(|n| n.id == "A").unwrap();
        assert_eq!(a.shape, Shape::RoundedRect);
    }

    #[test]
    fn history_state_shape_is_circle() {
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    state h1 <<history>>\n    [*] --> h1\n    h1 --> A"
        ).unwrap();
        let result = layout(&d);
        let h = result.nodes.iter().find(|n| n.id == "h1").unwrap();
        assert_eq!(h.shape, Shape::History);
        // Should be sized like start/end circles
        assert!((h.width - 16.0).abs() < 1.0);
    }

    #[test]
    fn layout_choice_branches() {
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    state check <<choice>>\n    [*] --> check\n    check --> A : yes\n    check --> B : no\n    A --> [*]\n    B --> [*]"
        ).unwrap();
        let result = layout(&d);

        let check = result.nodes.iter().find(|n| n.id == "check").unwrap();
        let a = result.nodes.iter().find(|n| n.id == "A").unwrap();
        let b = result.nodes.iter().find(|n| n.id == "B").unwrap();

        // check should be above A and B
        assert!(check.y < a.y, "check should be above A");
        assert!(check.y < b.y, "check should be above B");
    }

    #[test]
    fn layout_note_right() {
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    [*] --> Still\n    note right of Still : idle state\n    Still --> [*]"
        ).unwrap();
        let result = layout(&d);

        let still = result.nodes.iter().find(|n| n.id == "Still").unwrap();
        let note = result.nodes.iter().find(|n| n.id == "Still-note").unwrap();

        assert_eq!(note.shape, Shape::Note);
        assert_eq!(note.label, "idle state");
        // Note should be to the right of the state
        assert!(note.x > still.x,
            "note (x={:.1}) should be right of Still (x={:.1})", note.x, still.x);
    }

    #[test]
    fn layout_note_left() {
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    [*] --> Still\n    note left of Still : idle state\n    Still --> [*]"
        ).unwrap();
        let result = layout(&d);

        let still = result.nodes.iter().find(|n| n.id == "Still").unwrap();
        let note = result.nodes.iter().find(|n| n.id == "Still-note").unwrap();

        assert_eq!(note.shape, Shape::Note);
        // Note should be to the left of the state
        assert!(note.x < still.x,
            "note (x={:.1}) should be left of Still (x={:.1})", note.x, still.x);
    }

    #[test]
    fn layout_concurrent_regions() {
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    state Active {\n        A --> B\n        --\n        C --> D\n    }"
        ).unwrap();
        let result = layout(&d);

        // All four states should be present
        assert!(result.nodes.iter().any(|n| n.id == "A"));
        assert!(result.nodes.iter().any(|n| n.id == "B"));
        assert!(result.nodes.iter().any(|n| n.id == "C"));
        assert!(result.nodes.iter().any(|n| n.id == "D"));

        // Active should be compound with 2 regions
        let active = result.nodes.iter().find(|n| n.id == "Active").unwrap();
        assert!(active.is_compound);
        assert_eq!(active.region_count, 2);

        // Should have at least one divider
        assert!(!result.dividers.is_empty(),
            "concurrent regions should produce divider lines");
    }

    #[test]
    fn multi_source_end_bullseye_preserves_edges() {
        // When multiple transitions target [*], each edge should connect
        // from its own source — not all get overwritten to the last source's x.
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    [*] --> Still\n    Still --> [*]\n    Still --> Moving\n    Moving --> Still\n    Moving --> Crash\n    Crash --> [*]"
        ).unwrap();
        let result = layout(&d);

        let still = result.nodes.iter().find(|n| n.id == "Still").unwrap();
        let crash = result.nodes.iter().find(|n| n.id == "Crash").unwrap();

        // Both edges to [*]_end should exist
        let still_to_end = result.edges.iter()
            .find(|e| e.src == "Still" && e.dst == "[*]_end")
            .expect("Still → [*]_end edge should exist");
        let crash_to_end = result.edges.iter()
            .find(|e| e.src == "Crash" && e.dst == "[*]_end")
            .expect("Crash → [*]_end edge should exist");

        // The Still→[*] edge's last point should be closer to Still's x than Crash's x.
        // (Before the fix, both edges would have all points at Crash's x.)
        let still_edge_start_x = still_to_end.points[0].x;
        let crash_edge_start_x = crash_to_end.points[0].x;

        // The edges should start from different x positions (their respective sources)
        assert!(
            (still_edge_start_x - crash_edge_start_x).abs() > 1.0
                || (still.x - crash.x).abs() < 1.0, // unless they happen to be x-aligned
            "Still→[*] edge start (x={:.1}) and Crash→[*] edge start (x={:.1}) should differ \
             (Still.x={:.1}, Crash.x={:.1})",
            still_edge_start_x, crash_edge_start_x, still.x, crash.x
        );
    }

    #[test]
    fn concurrent_regions_centered_in_partitions() {
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    [*] --> Active\n    state Active {\n        [*] --> NumLockOff\n        NumLockOff --> NumLockOn : EvNumLockPressed\n        NumLockOn --> NumLockOff : EvNumLockPressed\n        --\n        [*] --> CapsLockOff\n        CapsLockOff --> CapsLockOn : EvCapsLockPressed\n        CapsLockOn --> CapsLockOff : EvCapsLockPressed\n    }\n    Active --> [*]"
        ).unwrap();
        let result = layout(&d);

        let active = result.nodes.iter().find(|n| n.id == "Active").unwrap();
        let numlock = result.nodes.iter().find(|n| n.id == "NumLockOff").unwrap();
        let capslock = result.nodes.iter().find(|n| n.id == "CapsLockOff").unwrap();

        let compound_left = active.x - active.width / 2.0;
        let partition_width = active.width / 2.0;
        let p0_cx = compound_left + partition_width * 0.5;
        let p1_cx = compound_left + partition_width * 1.5;

        assert!((numlock.x - p0_cx).abs() < 30.0,
            "NumLockOff (x={:.1}) should be near partition 0 center ({:.1})",
            numlock.x, p0_cx);
        assert!((capslock.x - p1_cx).abs() < 30.0,
            "CapsLockOff (x={:.1}) should be near partition 1 center ({:.1})",
            capslock.x, p1_cx);
    }

    #[test]
    fn edge_endpoint_touches_rect_boundary() {
        // Verify edges targeting RoundedRect nodes have endpoints on the node boundary.
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    [*] --> Active\n    state Active {\n        [*] --> Idle\n        Idle --> Running : start\n        Running --> Idle : stop\n        state hist1 <<history>>\n        Running --> hist1\n    }\n    Active --> Paused : pause\n    Paused --> Active : resume"
        ).unwrap();
        let result = layout(&d);

        let paused = result.nodes.iter().find(|n| n.id == "Paused").unwrap();
        let paused_top = paused.y - paused.height / 2.0;
        let paused_bottom = paused.y + paused.height / 2.0;
        let paused_left = paused.x - paused.width / 2.0;
        let paused_right = paused.x + paused.width / 2.0;

        // "pause" edge: Active → Paused, endpoint should touch Paused boundary
        let pause_edge = result.edges.iter()
            .find(|e| e.label.as_deref() == Some("pause"))
            .expect("pause edge should exist");
        let last_pt = *pause_edge.points.last().unwrap();
        assert!(
            (last_pt.x >= paused_left - 0.5 && last_pt.x <= paused_right + 0.5)
            && (last_pt.y >= paused_top - 0.5 && last_pt.y <= paused_bottom + 0.5),
            "pause edge endpoint ({:.2}, {:.2}) should be on Paused rect boundary \
             [x: {:.2}..{:.2}, y: {:.2}..{:.2}]",
            last_pt.x, last_pt.y,
            paused_left, paused_right, paused_top, paused_bottom
        );

        // Check that the endpoint is ON the boundary (not inside)
        let on_left = (last_pt.x - paused_left).abs() < 1.0;
        let on_right = (last_pt.x - paused_right).abs() < 1.0;
        let on_top = (last_pt.y - paused_top).abs() < 1.0;
        let on_bottom = (last_pt.y - paused_bottom).abs() < 1.0;
        assert!(on_left || on_right || on_top || on_bottom,
            "pause edge endpoint ({:.2}, {:.2}) should be ON boundary, not inside. \
             Distances: left={:.2} right={:.2} top={:.2} bottom={:.2}",
            last_pt.x, last_pt.y,
            (last_pt.x - paused_left).abs(),
            (last_pt.x - paused_right).abs(),
            (last_pt.y - paused_top).abs(),
            (last_pt.y - paused_bottom).abs()
        );

        // "resume" edge: Paused → Active, first point should touch Paused boundary
        let resume_edge = result.edges.iter()
            .find(|e| e.label.as_deref() == Some("resume"))
            .expect("resume edge should exist");
        let first_pt = resume_edge.points[0];
        assert!(
            (first_pt.x >= paused_left - 0.5 && first_pt.x <= paused_right + 0.5)
            && (first_pt.y >= paused_top - 0.5 && first_pt.y <= paused_bottom + 0.5),
            "resume edge start ({:.2}, {:.2}) should be on Paused rect boundary \
             [x: {:.2}..{:.2}, y: {:.2}..{:.2}]",
            first_pt.x, first_pt.y,
            paused_left, paused_right, paused_top, paused_bottom
        );
    }

    // ---------------------------------------------------------------
    // state_shape_intersect unit tests
    // ---------------------------------------------------------------

    fn bbox_at_origin(w: f64, h: f64) -> BBox {
        BBox::new(0.0, 0.0, w, h)
    }

    fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() < eps
    }

    fn assert_point_near(actual: Point, expected: Point, eps: f64, msg: &str) {
        assert!(
            approx_eq(actual.x, expected.x, eps) && approx_eq(actual.y, expected.y, eps),
            "{msg}: expected ({:.4}, {:.4}), got ({:.4}, {:.4})",
            expected.x, expected.y, actual.x, actual.y
        );
    }

    // --- RoundedRect, ForkJoinBar, NoteRect clip to rect boundary ---

    #[test]
    fn intersect_rounded_rect_from_above() {
        let bbox = bbox_at_origin(100.0, 60.0);
        let p = state_shape_intersect(Shape::RoundedRect, bbox, Point::new(0.0, -100.0)).unwrap();
        assert_point_near(p, Point::new(0.0, -30.0), 1e-6, "rounded rect from above");
    }

    #[test]
    fn intersect_rounded_rect_from_right() {
        let bbox = bbox_at_origin(100.0, 60.0);
        let p = state_shape_intersect(Shape::RoundedRect, bbox, Point::new(100.0, 0.0)).unwrap();
        assert_point_near(p, Point::new(50.0, 0.0), 1e-6, "rounded rect from right");
    }

    #[test]
    fn intersect_rounded_rect_diagonal() {
        let bbox = bbox_at_origin(100.0, 60.0);
        let p = state_shape_intersect(Shape::RoundedRect, bbox, Point::new(50.0, 50.0)).unwrap();
        // Ray at ~45 degrees from center: hits bottom edge at y=30
        assert_point_near(p, Point::new(30.0, 30.0), 1e-6, "rounded rect diagonal");
    }

    #[test]
    fn intersect_fork_join_bar_from_above() {
        let bbox = bbox_at_origin(70.0, 7.0);
        let p = state_shape_intersect(Shape::ForkJoin, bbox, Point::new(0.0, -50.0)).unwrap();
        assert_point_near(p, Point::new(0.0, -3.5), 1e-6, "fork-join from above");
    }

    #[test]
    fn intersect_fork_join_bar_diagonal() {
        let bbox = bbox_at_origin(70.0, 7.0);
        let p = state_shape_intersect(Shape::ForkJoin, bbox, Point::new(100.0, 100.0)).unwrap();
        // Wide rect: diagonal ray hits bottom edge (y=3.5) first
        assert_point_near(p, Point::new(3.5, 3.5), 1e-6, "fork-join diagonal");
    }

    #[test]
    fn intersect_note_rect_from_above() {
        let bbox = bbox_at_origin(80.0, 40.0);
        let p = state_shape_intersect(Shape::Note, bbox, Point::new(0.0, -50.0)).unwrap();
        assert_point_near(p, Point::new(0.0, -20.0), 1e-6, "note from above");
    }

    #[test]
    fn intersect_note_rect_from_left() {
        let bbox = bbox_at_origin(80.0, 40.0);
        let p = state_shape_intersect(Shape::Note, bbox, Point::new(-100.0, 0.0)).unwrap();
        assert_point_near(p, Point::new(-40.0, 0.0), 1e-6, "note from left");
    }

    // --- Circle shapes: StartCircle, EndBullseye, HistoryCircle ---
    //
    // intersect_circle projects the target point onto the circle boundary
    // along the ray from center toward target.

    #[test]
    fn intersect_start_circle_from_above() {
        let r = 8.0;
        let bbox = bbox_at_origin(r * 2.0, r * 2.0);
        let p = state_shape_intersect(Shape::StateStart, bbox, Point::new(0.0, -100.0)).unwrap();
        assert_point_near(p, Point::new(0.0, -r), 1e-6, "start circle from above");
    }

    #[test]
    fn intersect_start_circle_from_below() {
        let r = 8.0;
        let bbox = bbox_at_origin(r * 2.0, r * 2.0);
        let p = state_shape_intersect(Shape::StateStart, bbox, Point::new(0.0, 100.0)).unwrap();
        assert_point_near(p, Point::new(0.0, r), 1e-6, "start circle from below");
    }

    #[test]
    fn intersect_start_circle_from_left() {
        let r = 8.0;
        let bbox = bbox_at_origin(r * 2.0, r * 2.0);
        let p = state_shape_intersect(Shape::StateStart, bbox, Point::new(-100.0, 0.0)).unwrap();
        assert_point_near(p, Point::new(-r, 0.0), 1e-6, "start circle from left");
    }

    #[test]
    fn intersect_start_circle_from_right() {
        let r = 8.0;
        let bbox = bbox_at_origin(r * 2.0, r * 2.0);
        let p = state_shape_intersect(Shape::StateStart, bbox, Point::new(100.0, 0.0)).unwrap();
        assert_point_near(p, Point::new(r, 0.0), 1e-6, "start circle from right");
    }

    #[test]
    fn intersect_start_circle_diagonal() {
        let r = 8.0;
        let bbox = bbox_at_origin(r * 2.0, r * 2.0);
        let p = state_shape_intersect(Shape::StateStart, bbox, Point::new(100.0, 100.0)).unwrap();
        let s = r / 2.0_f64.sqrt();
        assert_point_near(p, Point::new(s, s), 1e-6, "start circle diagonal");
        // Point should be on the circle boundary
        let dist = (p.x * p.x + p.y * p.y).sqrt();
        assert!(approx_eq(dist, r, 1e-6), "point should be on circle boundary, dist={dist}");
    }

    #[test]
    fn intersect_end_bullseye_from_above() {
        let r = 8.0;
        let bbox = bbox_at_origin(r * 2.0, r * 2.0);
        let p = state_shape_intersect(Shape::StateEnd, bbox, Point::new(0.0, -50.0)).unwrap();
        assert_point_near(p, Point::new(0.0, -r), 1e-6, "end bullseye from above");
    }

    #[test]
    fn intersect_end_bullseye_diagonal() {
        let r = 10.0;
        let bbox = bbox_at_origin(r * 2.0, r * 2.0);
        let p = state_shape_intersect(Shape::StateEnd, bbox, Point::new(-80.0, 80.0)).unwrap();
        let s = r / 2.0_f64.sqrt();
        assert_point_near(p, Point::new(-s, s), 1e-6, "end bullseye diagonal bottom-left");
    }

    #[test]
    fn intersect_history_circle_from_right() {
        let r = 8.0;
        let bbox = bbox_at_origin(r * 2.0, r * 2.0);
        let p = state_shape_intersect(Shape::History, bbox, Point::new(200.0, 0.0)).unwrap();
        assert_point_near(p, Point::new(r, 0.0), 1e-6, "history circle from right");
    }

    #[test]
    fn intersect_history_circle_diagonal() {
        let r = 8.0;
        let bbox = bbox_at_origin(r * 2.0, r * 2.0);
        let p = state_shape_intersect(Shape::History, bbox, Point::new(-50.0, -50.0)).unwrap();
        let dist = (p.x * p.x + p.y * p.y).sqrt();
        assert!(approx_eq(dist, r, 1e-6), "history circle diagonal point on boundary");
    }

    #[test]
    fn circle_uses_max_of_width_height_for_radius() {
        // When width != height, radius = max(w, h) / 2
        let bbox = BBox::new(0.0, 0.0, 20.0, 10.0);
        let r = 10.0; // max(20, 10) / 2
        let p = state_shape_intersect(Shape::StateStart, bbox, Point::new(0.0, -100.0)).unwrap();
        assert_point_near(p, Point::new(0.0, -r), 1e-6, "radius uses max dimension");
    }

    // --- ChoiceDiamond: polygon intersection ---
    //
    // Diamond vertices at (cx, cy-hh), (cx+hw, cy), (cx, cy+hh), (cx-hw, cy)
    // For a square bbox (e.g., 28x28), the diamond has vertices at
    // top=(0,-14), right=(14,0), bottom=(0,14), left=(-14,0).

    #[test]
    fn intersect_diamond_from_above() {
        let s = 28.0;
        let bbox = bbox_at_origin(s, s);
        let p = state_shape_intersect(Shape::Choice, bbox, Point::new(0.0, -100.0)).unwrap();
        // Directly above center hits the top vertex
        assert_point_near(p, Point::new(0.0, -s / 2.0), 1e-6, "diamond from above");
    }

    #[test]
    fn intersect_diamond_from_below() {
        let s = 28.0;
        let bbox = bbox_at_origin(s, s);
        let p = state_shape_intersect(Shape::Choice, bbox, Point::new(0.0, 100.0)).unwrap();
        assert_point_near(p, Point::new(0.0, s / 2.0), 1e-6, "diamond from below");
    }

    #[test]
    fn intersect_diamond_from_left() {
        let s = 28.0;
        let bbox = bbox_at_origin(s, s);
        let p = state_shape_intersect(Shape::Choice, bbox, Point::new(-100.0, 0.0)).unwrap();
        assert_point_near(p, Point::new(-s / 2.0, 0.0), 1e-6, "diamond from left");
    }

    #[test]
    fn intersect_diamond_from_right() {
        let s = 28.0;
        let bbox = bbox_at_origin(s, s);
        let p = state_shape_intersect(Shape::Choice, bbox, Point::new(100.0, 0.0)).unwrap();
        assert_point_near(p, Point::new(s / 2.0, 0.0), 1e-6, "diamond from right");
    }

    #[test]
    fn intersect_diamond_diagonal_hits_edge_not_vertex() {
        // For a square diamond, a 45-degree ray from center should hit the
        // midpoint of an edge, not a vertex.
        let s = 28.0;
        let hw = s / 2.0;
        let bbox = bbox_at_origin(s, s);
        let p = state_shape_intersect(Shape::Choice, bbox, Point::new(100.0, 100.0)).unwrap();
        // The bottom-right edge goes from (hw, 0) to (0, hw).
        // Midpoint of that edge = (hw/2, hw/2).
        // A ray at 45 degrees hits exactly the midpoint for a square diamond.
        assert_point_near(p, Point::new(hw / 2.0, hw / 2.0), 1e-6, "diamond 45-deg hits edge midpoint");
    }

    #[test]
    fn intersect_diamond_diagonal_top_left() {
        let s = 28.0;
        let hw = s / 2.0;
        let bbox = bbox_at_origin(s, s);
        let p = state_shape_intersect(Shape::Choice, bbox, Point::new(-100.0, -100.0)).unwrap();
        // Top-left edge: from (0, -hw) to (-hw, 0). Midpoint = (-hw/2, -hw/2).
        assert_point_near(p, Point::new(-hw / 2.0, -hw / 2.0), 1e-6, "diamond 45-deg top-left");
    }

    #[test]
    fn intersect_diamond_non_square_bbox() {
        // Non-square diamond: w=40, h=20 => hw=20, hh=10
        // Vertices: top=(0,-10), right=(20,0), bottom=(0,10), left=(-20,0)
        let bbox = bbox_at_origin(40.0, 20.0);
        // Ray from center straight up
        let p = state_shape_intersect(Shape::Choice, bbox, Point::new(0.0, -100.0)).unwrap();
        assert_point_near(p, Point::new(0.0, -10.0), 1e-6, "non-square diamond from above");
        // Ray from center straight right
        let p = state_shape_intersect(Shape::Choice, bbox, Point::new(100.0, 0.0)).unwrap();
        assert_point_near(p, Point::new(20.0, 0.0), 1e-6, "non-square diamond from right");
    }

    #[test]
    fn intersect_diamond_point_lies_on_edge() {
        // Verify that the returned point is on one of the four diamond edges.
        let s = 28.0;
        let hw = s / 2.0;
        let bbox = bbox_at_origin(s, s);

        // Shoot a non-axis-aligned ray (e.g., 30 degrees from horizontal)
        let angle = std::f64::consts::FRAC_PI_6; // 30 degrees
        let target = Point::new(100.0 * angle.cos(), 100.0 * angle.sin());
        let p = state_shape_intersect(Shape::Choice, bbox, target).unwrap();

        // For a square diamond centered at origin with half-width hw,
        // a point is on the boundary iff |x|/hw + |y|/hw == 1
        let boundary_check = p.x.abs() / hw + p.y.abs() / hw;
        assert!(
            approx_eq(boundary_check, 1.0, 1e-6),
            "point ({:.4}, {:.4}) should be on diamond boundary: |x|/hw + |y|/hw = {:.6}, expected 1.0",
            p.x, p.y, boundary_check
        );
    }

    #[test]
    fn intersect_diamond_many_angles_all_on_boundary() {
        // Property test: for many ray angles, the intersection should lie on
        // the diamond boundary.
        let s = 28.0;
        let hw = s / 2.0;
        let bbox = bbox_at_origin(s, s);

        for deg in (0..360).step_by(15) {
            let angle = (deg as f64).to_radians();
            let target = Point::new(100.0 * angle.cos(), 100.0 * angle.sin());
            let p = state_shape_intersect(Shape::Choice, bbox, target).unwrap();
            let boundary_check = p.x.abs() / hw + p.y.abs() / hw;
            assert!(
                approx_eq(boundary_check, 1.0, 1e-4),
                "angle={deg}deg: point ({:.4}, {:.4}) off boundary: {:.6} != 1.0",
                p.x, p.y, boundary_check
            );
        }
    }

    #[test]
    fn intersect_circle_many_angles_all_on_boundary() {
        // Property test: for many ray angles, the intersection should lie on
        // the circle boundary.
        let r = 8.0;
        let bbox = bbox_at_origin(r * 2.0, r * 2.0);

        for deg in (0..360).step_by(15) {
            let angle = (deg as f64).to_radians();
            let target = Point::new(100.0 * angle.cos(), 100.0 * angle.sin());
            let p = state_shape_intersect(Shape::StateStart, bbox, target).unwrap();
            let dist = (p.x * p.x + p.y * p.y).sqrt();
            assert!(
                approx_eq(dist, r, 1e-6),
                "angle={deg}deg: point ({:.4}, {:.4}) off circle boundary: dist={:.6} != r={r}",
                p.x, p.y, dist
            );
        }
    }

    #[test]
    fn intersect_with_offset_center() {
        // BBox centered at (50, 30), not origin
        let bbox = BBox::new(50.0, 30.0, 16.0, 16.0);
        let r = 8.0;
        // Target far above the center
        let p = state_shape_intersect(Shape::StateStart, bbox, Point::new(50.0, -100.0)).unwrap();
        assert_point_near(p, Point::new(50.0, 30.0 - r), 1e-6, "offset center from above");
        // Target to the right
        let p = state_shape_intersect(Shape::StateEnd, bbox, Point::new(200.0, 30.0)).unwrap();
        assert_point_near(p, Point::new(50.0 + r, 30.0), 1e-6, "offset center from right");
    }

    #[test]
    fn intersect_diamond_offset_center() {
        let bbox = BBox::new(100.0, 50.0, 28.0, 28.0);
        let hw = 14.0;
        // Target directly above
        let p = state_shape_intersect(Shape::Choice, bbox, Point::new(100.0, -100.0)).unwrap();
        assert_point_near(p, Point::new(100.0, 50.0 - hw), 1e-6, "offset diamond from above");
        // Target directly right
        let p = state_shape_intersect(Shape::Choice, bbox, Point::new(300.0, 50.0)).unwrap();
        assert_point_near(p, Point::new(100.0 + hw, 50.0), 1e-6, "offset diamond from right");
    }
