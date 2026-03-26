use rusty_mermaid_core::{Direction, Shape};

use super::*;
use crate::flowchart::ir::*;

#[test]
fn layout_simple_chain() {
    let mut d = FlowDiagram::new(Direction::TB);
    d.vertices.push(FlowVertex::new("A", "Start", Shape::Rect));
    d.vertices.push(FlowVertex::new("B", "End", Shape::Rect));
    d.edges.push(FlowEdge::new("A", "B"));

    let result = layout(&d);
    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.edges.len(), 1);

    let a = result.nodes.iter().find(|n| n.id == "A").unwrap();
    let b = result.nodes.iter().find(|n| n.id == "B").unwrap();
    assert!(a.y < b.y, "A should be above B in TB layout");
}

#[test]
fn layout_lr_direction() {
    let mut d = FlowDiagram::new(Direction::LR);
    d.vertices.push(FlowVertex::new("A", "Left", Shape::Rect));
    d.vertices.push(FlowVertex::new("B", "Right", Shape::Rect));
    d.edges.push(FlowEdge::new("A", "B"));

    let result = layout(&d);
    let a = result.nodes.iter().find(|n| n.id == "A").unwrap();
    let b = result.nodes.iter().find(|n| n.id == "B").unwrap();
    assert!(a.x < b.x, "A should be left of B in LR layout");
}

#[test]
fn layout_from_parsed_mmd() {
    let d = crate::flowchart::parser::parse("graph TD\n    A[Start] --> B[End]").unwrap();
    let result = layout(&d);
    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.edges.len(), 1);
}

#[test]
fn subgraph_contains_children() {
    let mmd = "graph TD\n    subgraph outer[Outer]\n        subgraph inner[Inner]\n            A[Node A] --> B[Node B]\n        end\n        C[Node C]\n    end\n    C --> D[Node D]";
    let d = crate::flowchart::parser::parse(mmd).unwrap();
    let result = layout(&d);

    let inner_sg = result.subgraphs.iter().find(|sg| sg.id == "inner").unwrap();
    let a = result.nodes.iter().find(|n| n.id == "A").unwrap();
    let b = result.nodes.iter().find(|n| n.id == "B").unwrap();

    let sg_left = inner_sg.x - inner_sg.width / 2.0;
    let sg_right = inner_sg.x + inner_sg.width / 2.0;
    let a_left = a.x - a.width / 2.0;
    let a_right = a.x + a.width / 2.0;
    let b_left = b.x - b.width / 2.0;
    let b_right = b.x + b.width / 2.0;

    eprintln!(
        "inner sg: x={:.1} w={:.1} [{:.1}, {:.1}]",
        inner_sg.x, inner_sg.width, sg_left, sg_right
    );
    eprintln!(
        "A: x={:.1} w={:.1} [{:.1}, {:.1}]",
        a.x, a.width, a_left, a_right
    );
    eprintln!(
        "B: x={:.1} w={:.1} [{:.1}, {:.1}]",
        b.x, b.width, b_left, b_right
    );

    assert!(sg_left <= a_left, "inner should contain A horizontally");
    assert!(sg_right >= a_right, "inner should contain A horizontally");
    assert!(sg_left <= b_left, "inner should contain B horizontally");
    assert!(sg_right >= b_right, "inner should contain B horizontally");
}

#[test]
fn layout_edge_has_points() {
    let d = crate::flowchart::parser::parse("graph TD\n    A --> B --> C").unwrap();
    let result = layout(&d);
    for e in &result.edges {
        assert!(
            !e.points.is_empty(),
            "edge {}->{} should have points",
            e.src,
            e.dst
        );
    }
}

#[test]
fn subgraph_centered_on_children() {
    let mmd = "flowchart TD\n    subgraph outer[Level 1]\n        subgraph inner[Level 2]\n            A --> B\n        end\n        C --> A\n    end\n    D --> C";
    let d = crate::flowchart::parser::parse(mmd).unwrap();
    let result = layout(&d);

    for sg in &result.subgraphs {
        let sg_left = sg.x - sg.width / 2.0;
        let sg_right = sg.x + sg.width / 2.0;

        // Collect direct children bounds
        let children: Vec<_> = result
            .nodes
            .iter()
            .filter(|n| {
                let ir_sg = d.subgraphs.iter().find(|s| s.id == sg.id).unwrap();
                ir_sg.node_ids.contains(&n.id)
            })
            .collect();

        if children.is_empty() {
            continue;
        }

        let content_min = children
            .iter()
            .map(|n| n.x - n.width / 2.0)
            .fold(f64::INFINITY, f64::min);
        let content_max = children
            .iter()
            .map(|n| n.x + n.width / 2.0)
            .fold(f64::NEG_INFINITY, f64::max);

        let left_pad = content_min - sg_left;
        let right_pad = sg_right - content_max;

        eprintln!(
            "{}: center={:.1} left_pad={:.1} right_pad={:.1}",
            sg.id, sg.x, left_pad, right_pad
        );
        assert!(
            (left_pad - right_pad).abs() < 1.0,
            "{}: padding asymmetry {:.1} vs {:.1}",
            sg.id,
            left_pad,
            right_pad
        );
    }
}

#[test]
fn shape_propagated_to_layout() {
    let d = crate::flowchart::parser::parse(
        "flowchart TD\n    A[Rect] --> B(Rounded) --> C{Diamond} --> D((Circle))",
    )
    .unwrap();
    let result = layout(&d);

    let a = result.nodes.iter().find(|n| n.id == "A").unwrap();
    let b = result.nodes.iter().find(|n| n.id == "B").unwrap();
    let c = result.nodes.iter().find(|n| n.id == "C").unwrap();
    let d = result.nodes.iter().find(|n| n.id == "D").unwrap();

    assert_eq!(a.shape, Shape::Rect);
    assert_eq!(b.shape, Shape::RoundedRect);
    assert_eq!(c.shape, Shape::Diamond);
    assert_eq!(d.shape, Shape::Circle);
}

#[test]
fn arrow_types_propagated() {
    let d = crate::flowchart::parser::parse("flowchart TD\n    A --o B\n    A --x C\n    A --- D")
        .unwrap();
    let result = layout(&d);

    let ab = result.edges.iter().find(|e| e.dst == "B").unwrap();
    let ac = result.edges.iter().find(|e| e.dst == "C").unwrap();
    let ad = result.edges.iter().find(|e| e.dst == "D").unwrap();

    assert_eq!(ab.end_arrow, ArrowEnd::Circle);
    assert_eq!(ac.end_arrow, ArrowEnd::Cross);
    assert_eq!(ad.end_arrow, ArrowEnd::None);
}

#[test]
fn bidirectional_arrows() {
    let d = crate::flowchart::parser::parse(
        "flowchart TD\n    A <--> B\n    C <-.-> D\n    E o--o F\n    G x--x H",
    )
    .unwrap();
    let result = layout(&d);

    let ab = result
        .edges
        .iter()
        .find(|e| e.src == "A" && e.dst == "B")
        .unwrap();
    assert_eq!(ab.start_arrow, ArrowEnd::Arrow);
    assert_eq!(ab.end_arrow, ArrowEnd::Arrow);

    let cd = result
        .edges
        .iter()
        .find(|e| e.src == "C" && e.dst == "D")
        .unwrap();
    assert_eq!(cd.start_arrow, ArrowEnd::Arrow);
    assert_eq!(cd.end_arrow, ArrowEnd::Arrow);

    let ef = result
        .edges
        .iter()
        .find(|e| e.src == "E" && e.dst == "F")
        .unwrap();
    assert_eq!(ef.start_arrow, ArrowEnd::Circle);
    assert_eq!(ef.end_arrow, ArrowEnd::Circle);

    let gh = result
        .edges
        .iter()
        .find(|e| e.src == "G" && e.dst == "H")
        .unwrap();
    assert_eq!(gh.start_arrow, ArrowEnd::Cross);
    assert_eq!(gh.end_arrow, ArrowEnd::Cross);
}

#[test]
fn subgraph_direction_lr_in_td() {
    let d = crate::flowchart::parser::parse(
            "flowchart TD\n    subgraph sub1[Process]\n        direction LR\n        A[Step 1] --> B[Step 2] --> C[Step 3]\n    end\n    D[Start] --> sub1\n    sub1 --> E[End]",
        ).unwrap();
    let result = layout(&d);

    let a = result.nodes.iter().find(|n| n.id == "A").unwrap();
    let b = result.nodes.iter().find(|n| n.id == "B").unwrap();
    let c = result.nodes.iter().find(|n| n.id == "C").unwrap();

    // With direction LR inside the subgraph, A/B/C should be
    // horizontally arranged (left to right), not vertically.
    assert!(a.x < b.x, "A should be left of B: {} < {}", a.x, b.x);
    assert!(b.x < c.x, "B should be left of C: {} < {}", b.x, c.x);
    // They should be at roughly the same y.
    assert!((a.y - b.y).abs() < 5.0, "A and B should be at same y");
    assert!((b.y - c.y).abs() < 5.0, "B and C should be at same y");
}

#[test]
fn subgraph_direction_skipped_with_external_edges() {
    // Edge from D directly to A (inside sub1) → external connection → direction ignored.
    let d = crate::flowchart::parser::parse(
            "flowchart TD\n    subgraph sub1[Process]\n        direction LR\n        A --> B --> C\n    end\n    D --> A",
        ).unwrap();
    let result = layout(&d);

    let a = result.nodes.iter().find(|n| n.id == "A").unwrap();
    let b = result.nodes.iter().find(|n| n.id == "B").unwrap();

    // With external connections, direction LR is ignored → defaults to TD.
    // A should be above B (or at least not horizontally arranged).
    assert!(a.y < b.y, "A should be above B (TD): {} < {}", a.y, b.y);
}

// ── shape_intersect tests ──────────────────────────────────────────

const TOL: f64 = 1e-6;

fn assert_near(actual: f64, expected: f64, msg: &str) {
    assert!(
        (actual - expected).abs() < TOL,
        "{msg}: expected {expected}, got {actual}",
    );
}

fn assert_point_near(actual: Point, expected: Point, msg: &str) {
    assert!(
        (actual.x - expected.x).abs() < TOL && (actual.y - expected.y).abs() < TOL,
        "{msg}: expected ({}, {}), got ({}, {})",
        expected.x,
        expected.y,
        actual.x,
        actual.y,
    );
}

/// Standard test bbox: center (100, 100), size 80x60.
fn test_bbox() -> BBox {
    BBox::new(100.0, 100.0, 80.0, 60.0)
}

/// Square test bbox: center (100, 100), size 80x80.
fn square_bbox() -> BBox {
    BBox::new(100.0, 100.0, 80.0, 80.0)
}

// ── Shapes that return None ──

#[test]
fn shape_intersect_rect_returns_none() {
    let b = test_bbox();
    assert!(shape_intersect(Shape::Rect, b, Point::new(200.0, 100.0)).is_none());
    assert!(shape_intersect(Shape::Rect, b, Point::new(100.0, 0.0)).is_none());
    assert!(shape_intersect(Shape::Rect, b, Point::new(200.0, 200.0)).is_none());
}

#[test]
fn shape_intersect_rounded_rect_returns_none() {
    let b = test_bbox();
    assert!(shape_intersect(Shape::RoundedRect, b, Point::new(200.0, 100.0)).is_none());
}

#[test]
fn shape_intersect_stadium_right() {
    // test_bbox: center (100,100), 80x60. r=hh=30.
    // Right cap center: (100+40-30, 100) = (110, 100), r=30.
    // Horizontal ray rightward hits circle at (140, 100).
    let b = test_bbox();
    let p = shape_intersect(Shape::Stadium, b, Point::new(200.0, 100.0)).unwrap();
    assert_point_near(p, Point::new(140.0, 100.0), "stadium right");
}

#[test]
fn shape_intersect_stadium_left() {
    // Left cap center: (100-40+30, 100) = (90, 100), r=30.
    // Horizontal ray leftward hits circle at (60, 100).
    let b = test_bbox();
    let p = shape_intersect(Shape::Stadium, b, Point::new(0.0, 100.0)).unwrap();
    assert_point_near(p, Point::new(60.0, 100.0), "stadium left");
}

#[test]
fn shape_intersect_stadium_top_straight() {
    // Ray straight up exits through straight top edge — returns None (dagre handles it).
    let b = test_bbox();
    assert!(shape_intersect(Shape::Stadium, b, Point::new(100.0, 0.0)).is_none());
}

#[test]
fn shape_intersect_stadium_diagonal_into_cap() {
    // Ray at an angle into the right cap zone.
    // Right cap center: (110, 100), r=30.
    let b = test_bbox();
    let p = shape_intersect(Shape::Stadium, b, Point::new(200.0, 80.0)).unwrap();
    // Point should lie on the right cap circle
    let dist = Point::new(110.0, 100.0).distance_to(p);
    assert_near(dist, 30.0, "stadium diagonal should land on cap circle");
}

#[test]
fn shape_intersect_wildcard_shapes_return_none() {
    let b = test_bbox();
    // Subroutine, StateStart, etc. all hit the _ arm
    assert!(shape_intersect(Shape::Subroutine, b, Point::new(200.0, 100.0)).is_none());
    assert!(shape_intersect(Shape::StateStart, b, Point::new(200.0, 100.0)).is_none());
    assert!(shape_intersect(Shape::Note, b, Point::new(200.0, 100.0)).is_none());
    assert!(shape_intersect(Shape::ClassBox, b, Point::new(200.0, 100.0)).is_none());
}

// ── Diamond ──

#[test]
fn shape_intersect_diamond_right() {
    // Diamond vertices: top (cx, cy-hh), right (cx+hw, cy), bottom (cx, cy+hh), left (cx-hw, cy)
    // Ray rightward from center hits right vertex exactly.
    let b = square_bbox(); // 80x80, hw=hh=40
    let p = shape_intersect(Shape::Diamond, b, Point::new(200.0, 100.0)).unwrap();
    assert_point_near(p, Point::new(140.0, 100.0), "diamond right");
}

#[test]
fn shape_intersect_diamond_top() {
    let b = square_bbox();
    let p = shape_intersect(Shape::Diamond, b, Point::new(100.0, 0.0)).unwrap();
    assert_point_near(p, Point::new(100.0, 60.0), "diamond top");
}

#[test]
fn shape_intersect_diamond_bottom() {
    let b = square_bbox();
    let p = shape_intersect(Shape::Diamond, b, Point::new(100.0, 200.0)).unwrap();
    assert_point_near(p, Point::new(100.0, 140.0), "diamond bottom");
}

#[test]
fn shape_intersect_diamond_left() {
    let b = square_bbox();
    let p = shape_intersect(Shape::Diamond, b, Point::new(0.0, 100.0)).unwrap();
    assert_point_near(p, Point::new(60.0, 100.0), "diamond left");
}

#[test]
fn shape_intersect_diamond_diagonal() {
    // For a square diamond (hw=hh=40), a 45-degree ray hits the midpoint
    // of the top-right edge: from (100,60) to (140,100).
    // Midpoint = (120, 80). Ray from center (100,100) at 45° up-right:
    // parametric: (100+t, 100-t). Edge: from (100,60) to (140,100),
    // parametric: (100+40s, 60+40s). Solve: 100+t=100+40s → t=40s;
    // 100-t=60+40s → 100-40s=60+40s → 40=80s → s=0.5, t=20.
    // Hit: (120, 80).
    let b = square_bbox();
    let p = shape_intersect(Shape::Diamond, b, Point::new(200.0, 0.0)).unwrap();
    assert_point_near(p, Point::new(120.0, 80.0), "diamond diagonal up-right");
}

// ── Circle ──

#[test]
fn shape_intersect_circle_right() {
    let b = BBox::new(100.0, 100.0, 60.0, 60.0); // r = 30
    let p = shape_intersect(Shape::Circle, b, Point::new(200.0, 100.0)).unwrap();
    assert_point_near(p, Point::new(130.0, 100.0), "circle right");
}

#[test]
fn shape_intersect_circle_top() {
    let b = BBox::new(100.0, 100.0, 60.0, 60.0);
    let p = shape_intersect(Shape::Circle, b, Point::new(100.0, 0.0)).unwrap();
    assert_point_near(p, Point::new(100.0, 70.0), "circle top");
}

#[test]
fn shape_intersect_circle_diagonal() {
    let b = BBox::new(100.0, 100.0, 60.0, 60.0); // r = 30
    let p = shape_intersect(Shape::Circle, b, Point::new(200.0, 200.0)).unwrap();
    let offset = 30.0 / 2.0_f64.sqrt();
    assert_point_near(
        p,
        Point::new(100.0 + offset, 100.0 + offset),
        "circle diagonal",
    );
}

// ── DoubleCircle ──

#[test]
fn shape_intersect_double_circle_right() {
    // DoubleCircle uses the same circle intersect, r = max(w,h)/2
    let b = BBox::new(100.0, 100.0, 70.0, 70.0); // r = 35
    let p = shape_intersect(Shape::DoubleCircle, b, Point::new(200.0, 100.0)).unwrap();
    assert_point_near(p, Point::new(135.0, 100.0), "double circle right");
}

#[test]
fn shape_intersect_double_circle_left() {
    let b = BBox::new(100.0, 100.0, 70.0, 70.0);
    let p = shape_intersect(Shape::DoubleCircle, b, Point::new(0.0, 100.0)).unwrap();
    assert_point_near(p, Point::new(65.0, 100.0), "double circle left");
}

// ── Hexagon ──

#[test]
fn shape_intersect_hexagon_right() {
    // Hexagon vertices with bbox 80x60: m = h/4 = 15
    // Right vertex: (cx+hw, cy) = (140, 100)
    let b = test_bbox();
    let p = shape_intersect(Shape::Hexagon, b, Point::new(200.0, 100.0)).unwrap();
    assert_point_near(p, Point::new(140.0, 100.0), "hexagon right");
}

#[test]
fn shape_intersect_hexagon_left() {
    let b = test_bbox();
    let p = shape_intersect(Shape::Hexagon, b, Point::new(0.0, 100.0)).unwrap();
    assert_point_near(p, Point::new(60.0, 100.0), "hexagon left");
}

#[test]
fn shape_intersect_hexagon_top() {
    // Top edge goes from (cx-hw+m, cy-hh) to (cx+hw-m, cy-hh) = (75, 70) to (125, 70).
    // Ray straight up from center hits this horizontal segment at (100, 70).
    let b = test_bbox();
    let p = shape_intersect(Shape::Hexagon, b, Point::new(100.0, 0.0)).unwrap();
    assert_point_near(p, Point::new(100.0, 70.0), "hexagon top");
}

// ── Parallelogram ──

#[test]
fn shape_intersect_parallelogram_right() {
    // skew = h/2 = 30. Vertices:
    //   top-left:  (100-40+30, 70)  = (90, 70)
    //   top-right: (100+40+30, 70)  = (170, 70)
    //   bot-right: (100+40-30, 130) = (110, 130)
    //   bot-left:  (100-40-30, 130) = (30, 130)
    // Ray rightward from (100,100): hits right edge between top-right (170,70) and bot-right (110,130).
    let b = test_bbox();
    let p = shape_intersect(Shape::Parallelogram, b, Point::new(300.0, 100.0)).unwrap();
    // Right edge: (170,70)→(110,130). Parametric: (170-60t, 70+60t).
    // Ray: (100+s, 100). Solve: 100=70+60t → t=0.5. x=170-30=140. Hit: (140, 100).
    assert_point_near(p, Point::new(140.0, 100.0), "parallelogram right");
}

#[test]
fn shape_intersect_parallelogram_top() {
    let b = test_bbox();
    let p = shape_intersect(Shape::Parallelogram, b, Point::new(100.0, 0.0)).unwrap();
    // Top edge: (90,70)→(170,70). Ray upward: (100, 100-s). Hits at y=70, x=100.
    assert_point_near(p, Point::new(100.0, 70.0), "parallelogram top");
}

// ── ParallelogramAlt ──

#[test]
fn shape_intersect_parallelogram_alt_left() {
    // skew = h/2 = 30. Vertices:
    //   top-left:  (100-40-30, 70) = (30, 70)
    //   top-right: (100+40-30, 70) = (110, 70)
    //   bot-right: (100+40+30, 130) = (170, 130)
    //   bot-left:  (100-40+30, 130) = (90, 130)
    // Ray leftward: hits left edge (30,70)→(90,130).
    let b = test_bbox();
    let p = shape_intersect(Shape::ParallelogramAlt, b, Point::new(0.0, 100.0)).unwrap();
    // Left edge: (30,70)→(90,130). Parametric: (30+60t, 70+60t).
    // Ray: (100-s, 100). y=100: 70+60t=100 → t=0.5. x=30+30=60. Hit: (60, 100).
    assert_point_near(p, Point::new(60.0, 100.0), "parallelogram alt left");
}

#[test]
fn shape_intersect_parallelogram_alt_top() {
    let b = test_bbox();
    let p = shape_intersect(Shape::ParallelogramAlt, b, Point::new(100.0, 0.0)).unwrap();
    // Top edge: (30,70)→(110,70). Straight up hits at (100, 70).
    assert_point_near(p, Point::new(100.0, 70.0), "parallelogram alt top");
}

// ── Trapezoid ──

#[test]
fn shape_intersect_trapezoid_top() {
    // offset = h/2 = 30. Vertices:
    //   top-left:  (60, 70)
    //   top-right: (140, 70)
    //   bot-right: (170, 130)
    //   bot-left:  (30, 130)
    // Ray straight up hits top edge at (100, 70).
    let b = test_bbox();
    let p = shape_intersect(Shape::Trapezoid, b, Point::new(100.0, 0.0)).unwrap();
    assert_point_near(p, Point::new(100.0, 70.0), "trapezoid top");
}

#[test]
fn shape_intersect_trapezoid_bottom() {
    let b = test_bbox();
    let p = shape_intersect(Shape::Trapezoid, b, Point::new(100.0, 200.0)).unwrap();
    // Bottom edge: (170,130)→(30,130). Straight down hits at (100, 130).
    assert_point_near(p, Point::new(100.0, 130.0), "trapezoid bottom");
}

#[test]
fn shape_intersect_trapezoid_right() {
    let b = test_bbox();
    let p = shape_intersect(Shape::Trapezoid, b, Point::new(300.0, 100.0)).unwrap();
    // Right edge: (140,70)→(170,130). Parametric: (140+30t, 70+60t).
    // Ray: (100+s, 100). y=100: 70+60t=100 → t=0.5. x=140+15=155. Hit: (155, 100).
    assert_point_near(p, Point::new(155.0, 100.0), "trapezoid right");
}

// ── TrapezoidAlt ──

#[test]
fn shape_intersect_trapezoid_alt_top() {
    // offset = h/2 = 30. Vertices:
    //   top-left:  (30, 70)
    //   top-right: (170, 70)
    //   bot-right: (140, 130)
    //   bot-left:  (60, 130)
    // Ray straight up hits top edge at (100, 70).
    let b = test_bbox();
    let p = shape_intersect(Shape::TrapezoidAlt, b, Point::new(100.0, 0.0)).unwrap();
    assert_point_near(p, Point::new(100.0, 70.0), "trapezoid alt top");
}

#[test]
fn shape_intersect_trapezoid_alt_bottom() {
    let b = test_bbox();
    let p = shape_intersect(Shape::TrapezoidAlt, b, Point::new(100.0, 200.0)).unwrap();
    // Bottom edge: (140,130)→(60,130). Straight down hits at (100, 130).
    assert_point_near(p, Point::new(100.0, 130.0), "trapezoid alt bottom");
}

#[test]
fn shape_intersect_trapezoid_alt_left() {
    let b = test_bbox();
    let p = shape_intersect(Shape::TrapezoidAlt, b, Point::new(0.0, 100.0)).unwrap();
    // Left edge: (60,130)→(30,70). Parametric: (60-30t, 130-60t).
    // Ray: (100-s, 100). y=100: 130-60t=100 → t=0.5. x=60-15=45. Hit: (45, 100).
    assert_point_near(p, Point::new(45.0, 100.0), "trapezoid alt left");
}

// ── Cylinder ──
// bbox 80×60, cx=100, cy=100.
// rx = 40, ry = 40 / (2.5 + 80/50) = 40/4.1 ≈ 9.756
// body_h = 60 - ry ≈ 50.244
// top_cap_cy ≈ 100 - 25.122 = 74.878
// bot_cap_cy ≈ 100 + 25.122 = 125.122

#[test]
fn shape_intersect_cylinder_right() {
    let b = test_bbox();
    let p = shape_intersect(Shape::Cylinder, b, Point::new(200.0, 100.0)).unwrap();
    assert_point_near(p, Point::new(140.0, 100.0), "cylinder right");
}

#[test]
fn shape_intersect_cylinder_top() {
    // Axis-aligned upward ray hits top cap ellipse at (cx, top_cap_cy - ry).
    let b = test_bbox();
    let p = shape_intersect(Shape::Cylinder, b, Point::new(100.0, 0.0)).unwrap();
    let rx = 40.0;
    let ry = rx / (2.5 + 80.0 / 50.0);
    let body_h = 60.0 - ry;
    let top_cap_cy = 100.0 - body_h / 2.0;
    assert_near(p.x, 100.0, "cylinder top x");
    assert_near(p.y, top_cap_cy - ry, "cylinder top y");
}

#[test]
fn shape_intersect_cylinder_bottom() {
    let b = test_bbox();
    let p = shape_intersect(Shape::Cylinder, b, Point::new(100.0, 200.0)).unwrap();
    let rx = 40.0;
    let ry = rx / (2.5 + 80.0 / 50.0);
    let body_h = 60.0 - ry;
    let bot_cap_cy = 100.0 + body_h / 2.0;
    assert_near(p.x, 100.0, "cylinder bottom x");
    assert_near(p.y, bot_cap_cy + ry, "cylinder bottom y");
}

#[test]
fn shape_intersect_cylinder_diagonal_top_right() {
    // Diagonal ray into top-right cap region must land on ellipse, not rect.
    let b = test_bbox();
    let p = shape_intersect(Shape::Cylinder, b, Point::new(200.0, 0.0)).unwrap();
    let rx = 40.0;
    let ry = rx / (2.5 + 80.0 / 50.0);
    let body_h = 60.0 - ry;
    let top_cap_cy = 100.0 - body_h / 2.0;
    // Point must satisfy the ellipse equation for the top cap.
    let eq = ((p.x - 100.0) / rx).powi(2) + ((p.y - top_cap_cy) / ry).powi(2);
    assert!(
        (eq - 1.0).abs() < 0.01,
        "diagonal top-right should land on cap ellipse, eq = {eq}"
    );
}

#[test]
fn shape_intersect_cylinder_diagonal_bottom_left() {
    let b = test_bbox();
    let p = shape_intersect(Shape::Cylinder, b, Point::new(0.0, 200.0)).unwrap();
    let rx = 40.0;
    let ry = rx / (2.5 + 80.0 / 50.0);
    let body_h = 60.0 - ry;
    let bot_cap_cy = 100.0 + body_h / 2.0;
    let eq = ((p.x - 100.0) / rx).powi(2) + ((p.y - bot_cap_cy) / ry).powi(2);
    assert!(
        (eq - 1.0).abs() < 0.01,
        "diagonal bottom-left should land on cap ellipse, eq = {eq}"
    );
}

// ── Asymmetric ──

#[test]
fn shape_intersect_asymmetric_right() {
    // notch = h/4 = 15. Vertices:
    //   (60, 70), (140, 70), (140, 130), (60, 130), (75, 100)
    // Ray rightward from center (100,100) hits right edge (140,70)→(140,130) at (140,100).
    let b = test_bbox();
    let p = shape_intersect(Shape::Asymmetric, b, Point::new(200.0, 100.0)).unwrap();
    assert_point_near(p, Point::new(140.0, 100.0), "asymmetric right");
}

#[test]
fn shape_intersect_asymmetric_left() {
    // Ray leftward: hits the notch edges. The left side has two edges:
    //   bottom-left (60,130) → notch (75,100) and notch (75,100) → top-left (60,70).
    // Ray from center (100,100) going left: (100-s, 100). Hits notch vertex at (75, 100).
    let b = test_bbox();
    let p = shape_intersect(Shape::Asymmetric, b, Point::new(0.0, 100.0)).unwrap();
    assert_point_near(p, Point::new(75.0, 100.0), "asymmetric left notch");
}

#[test]
fn shape_intersect_asymmetric_top() {
    let b = test_bbox();
    let p = shape_intersect(Shape::Asymmetric, b, Point::new(100.0, 0.0)).unwrap();
    // Top edge: (60,70)→(140,70). Straight up hits at (100, 70).
    assert_point_near(p, Point::new(100.0, 70.0), "asymmetric top");
}

// ── Edge case: point on boundary returns Some for non-rect shapes ──

#[test]
fn shape_intersect_all_non_rect_return_some() {
    let b = test_bbox();
    let adj = Point::new(200.0, 100.0); // ray to the right
    let non_rect_shapes = [
        Shape::Diamond,
        Shape::Circle,
        Shape::DoubleCircle,
        Shape::Hexagon,
        Shape::Parallelogram,
        Shape::ParallelogramAlt,
        Shape::Trapezoid,
        Shape::TrapezoidAlt,
        Shape::Cylinder,
        Shape::Asymmetric,
        Shape::Stadium,
    ];
    for shape in non_rect_shapes {
        assert!(
            shape_intersect(shape, b, adj).is_some(),
            "{shape:?} should return Some",
        );
    }
}

// ── Intersection points lie on or near the shape boundary ──

#[test]
fn shape_intersect_diamond_boundary_distance() {
    // For a square diamond (hw=hh=40), the boundary is the set of points
    // where |x - cx|/hw + |y - cy|/hh = 1.
    let b = square_bbox();
    let (cx, cy, hw, hh) = (100.0, 100.0, 40.0, 40.0);
    let targets = [
        Point::new(200.0, 100.0),
        Point::new(100.0, 0.0),
        Point::new(0.0, 100.0),
        Point::new(100.0, 200.0),
        Point::new(200.0, 0.0),
        Point::new(0.0, 200.0),
        Point::new(200.0, 200.0),
        Point::new(0.0, 0.0),
    ];
    for t in targets {
        let p = shape_intersect(Shape::Diamond, b, t).unwrap();
        let diamond_eq = (p.x - cx).abs() / hw + (p.y - cy).abs() / hh;
        assert!(
            (diamond_eq - 1.0).abs() < TOL,
            "diamond boundary: target ({}, {}), hit ({}, {}), eq={diamond_eq}",
            t.x,
            t.y,
            p.x,
            p.y,
        );
    }
}

#[test]
fn shape_intersect_circle_boundary_distance() {
    let b = BBox::new(100.0, 100.0, 60.0, 60.0); // r = 30
    let r = 30.0;
    let targets = [
        Point::new(200.0, 100.0),
        Point::new(100.0, 0.0),
        Point::new(0.0, 100.0),
        Point::new(100.0, 200.0),
        Point::new(200.0, 200.0),
        Point::new(0.0, 0.0),
    ];
    for t in targets {
        let p = shape_intersect(Shape::Circle, b, t).unwrap();
        let dist = Point::new(100.0, 100.0).distance_to(p);
        assert!(
            (dist - r).abs() < TOL,
            "circle boundary: target ({}, {}), hit ({}, {}), dist={dist}",
            t.x,
            t.y,
            p.x,
            p.y,
        );
    }
}

#[test]
fn shape_intersect_hexagon_boundary_on_polygon() {
    // Verify hit points lie on the hexagon polygon edges.
    let b = test_bbox();
    let (cx, cy) = (100.0, 100.0);
    let (hw, hh) = (40.0, 30.0);
    let m = 60.0 / 4.0; // 15
    let verts = [
        Point::new(cx - hw + m, cy - hh),
        Point::new(cx + hw - m, cy - hh),
        Point::new(cx + hw, cy),
        Point::new(cx + hw - m, cy + hh),
        Point::new(cx - hw + m, cy + hh),
        Point::new(cx - hw, cy),
    ];

    let targets = [
        Point::new(200.0, 100.0),
        Point::new(0.0, 100.0),
        Point::new(100.0, 0.0),
        Point::new(100.0, 200.0),
    ];

    for t in targets {
        let p = shape_intersect(Shape::Hexagon, b, t).unwrap();
        // Point should be on one of the polygon edges.
        let on_edge = is_on_polygon_edge(&verts, p);
        assert!(
            on_edge,
            "hexagon: target ({}, {}), hit ({}, {}) not on edge",
            t.x, t.y, p.x, p.y,
        );
    }
}

/// Check whether a point lies on any edge of a polygon (within tolerance).
fn is_on_polygon_edge(verts: &[Point], p: Point) -> bool {
    let n = verts.len();
    for i in 0..n {
        let a = verts[i];
        let b = verts[(i + 1) % n];
        if point_on_segment(a, b, p) {
            return true;
        }
    }
    false
}

/// Check whether point p lies on segment a→b (within tolerance).
fn point_on_segment(a: Point, b: Point, p: Point) -> bool {
    let ab = ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt();
    if ab < f64::EPSILON {
        return a.distance_to(p) < TOL;
    }
    let ap = a.distance_to(p);
    let pb = p.distance_to(b);
    (ap + pb - ab).abs() < TOL
}

/// Distance from point p to line segment a→b.
fn point_to_segment_dist(a: Point, b: Point, p: Point) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-12 {
        return a.distance_to(p);
    }
    let t = ((p.x - a.x) * dx + (p.y - a.y) * dy) / len_sq;
    let t = t.clamp(0.0, 1.0);
    let proj = Point::new(a.x + t * dx, a.y + t * dy);
    p.distance_to(proj)
}

/// Check if a point lies on any edge of a polygon (generous tolerance for proptest).
fn near_polygon_edge(verts: &[Point], p: Point, tol: f64) -> bool {
    let n = verts.len();
    (0..n).any(|i| point_to_segment_dist(verts[i], verts[(i + 1) % n], p) < tol)
}

/// Check if a point lies on the visual boundary of a shape.
fn on_shape_boundary(shape: Shape, bbox: BBox, p: Point) -> bool {
    let tol = 0.5;
    let (cx, cy, w, h) = (bbox.x, bbox.y, bbox.width, bbox.height);
    let hw = w / 2.0;
    let hh = h / 2.0;

    match shape {
        Shape::Diamond => {
            let verts = [
                Point::new(cx, cy - hh),
                Point::new(cx + hw, cy),
                Point::new(cx, cy + hh),
                Point::new(cx - hw, cy),
            ];
            near_polygon_edge(&verts, p, tol)
        }
        Shape::Circle | Shape::DoubleCircle => {
            let r = w.max(h) / 2.0;
            (Point::new(cx, cy).distance_to(p) - r).abs() < tol
        }
        Shape::Hexagon => {
            let m = h / 4.0;
            let verts = [
                Point::new(cx - hw + m, cy - hh),
                Point::new(cx + hw - m, cy - hh),
                Point::new(cx + hw, cy),
                Point::new(cx + hw - m, cy + hh),
                Point::new(cx - hw + m, cy + hh),
                Point::new(cx - hw, cy),
            ];
            near_polygon_edge(&verts, p, tol)
        }
        Shape::Parallelogram => {
            let skew = h / 2.0;
            let verts = [
                Point::new(cx - hw + skew, cy - hh),
                Point::new(cx + hw + skew, cy - hh),
                Point::new(cx + hw - skew, cy + hh),
                Point::new(cx - hw - skew, cy + hh),
            ];
            near_polygon_edge(&verts, p, tol)
        }
        Shape::ParallelogramAlt => {
            let skew = h / 2.0;
            let verts = [
                Point::new(cx - hw - skew, cy - hh),
                Point::new(cx + hw - skew, cy - hh),
                Point::new(cx + hw + skew, cy + hh),
                Point::new(cx - hw + skew, cy + hh),
            ];
            near_polygon_edge(&verts, p, tol)
        }
        Shape::Trapezoid => {
            let offset = h / 2.0;
            let verts = [
                Point::new(cx - hw, cy - hh),
                Point::new(cx + hw, cy - hh),
                Point::new(cx + hw + offset, cy + hh),
                Point::new(cx - hw - offset, cy + hh),
            ];
            near_polygon_edge(&verts, p, tol)
        }
        Shape::TrapezoidAlt => {
            let offset = h / 2.0;
            let verts = [
                Point::new(cx - hw - offset, cy - hh),
                Point::new(cx + hw + offset, cy - hh),
                Point::new(cx + hw, cy + hh),
                Point::new(cx - hw, cy + hh),
            ];
            near_polygon_edge(&verts, p, tol)
        }
        Shape::Cylinder => {
            let rx = hw;
            let ry = rx / (2.5 + w / 50.0);
            let body_h = h - ry;
            let top_cy = cy - body_h / 2.0;
            let bot_cy = cy + body_h / 2.0;
            // Straight sides
            if p.y >= top_cy - tol && p.y <= bot_cy + tol {
                if (p.x - (cx - hw)).abs() < tol || (p.x - (cx + hw)).abs() < tol {
                    return true;
                }
            }
            // Top cap ellipse
            let eq_top = ((p.x - cx) / rx).powi(2) + ((p.y - top_cy) / ry).powi(2);
            if (eq_top - 1.0).abs() < 0.1 && p.y <= top_cy + tol {
                return true;
            }
            // Bottom cap ellipse
            let eq_bot = ((p.x - cx) / rx).powi(2) + ((p.y - bot_cy) / ry).powi(2);
            if (eq_bot - 1.0).abs() < 0.1 && p.y >= bot_cy - tol {
                return true;
            }
            false
        }
        Shape::Asymmetric => {
            let notch = (h / 4.0).min(hw * 0.8);
            let verts = [
                Point::new(cx - hw, cy - hh),
                Point::new(cx + hw, cy - hh),
                Point::new(cx + hw, cy + hh),
                Point::new(cx - hw, cy + hh),
                Point::new(cx - hw + notch, cy),
            ];
            near_polygon_edge(&verts, p, tol)
        }
        Shape::Stadium => {
            let r = hh;
            let left_cx = cx - hw + r;
            let right_cx = cx + hw - r;
            if left_cx >= right_cx {
                return (Point::new(cx, cy).distance_to(p) - r).abs() < tol;
            }
            // Straight top/bottom
            if p.x >= left_cx - tol && p.x <= right_cx + tol {
                if (p.y - (cy - hh)).abs() < tol || (p.y - (cy + hh)).abs() < tol {
                    return true;
                }
            }
            // Left cap
            if (Point::new(left_cx, cy).distance_to(p) - r).abs() < tol && p.x <= left_cx + tol {
                return true;
            }
            // Right cap
            if (Point::new(right_cx, cy).distance_to(p) - r).abs() < tol && p.x >= right_cx - tol {
                return true;
            }
            false
        }
        _ => false,
    }
}

// ── Property-based tests ──

use proptest::prelude::*;

fn non_rect_shapes() -> impl Strategy<Value = Shape> {
    prop_oneof![
        Just(Shape::Diamond),
        Just(Shape::Circle),
        Just(Shape::DoubleCircle),
        Just(Shape::Hexagon),
        Just(Shape::Parallelogram),
        Just(Shape::ParallelogramAlt),
        Just(Shape::Trapezoid),
        Just(Shape::TrapezoidAlt),
        Just(Shape::Cylinder),
        Just(Shape::Asymmetric),
        Just(Shape::Stadium),
    ]
}

proptest! {
    #[test]
    fn shape_intersect_on_boundary(
        shape in non_rect_shapes(),
        cx in 0.0..200.0f64,
        cy in 0.0..200.0f64,
        w in 40.0..200.0f64,
        h in 40.0..200.0f64,
        angle in 0.0..std::f64::consts::TAU,
    ) {
        let bbox = BBox::new(cx, cy, w, h);
        let far = (w + h) * 3.0;
        let target = Point::new(cx + far * angle.cos(), cy + far * angle.sin());

        if let Some(p) = shape_intersect(shape, bbox, target) {
            prop_assert!(
                on_shape_boundary(shape, bbox, p),
                "{shape:?}: hit ({}, {}) not on boundary \
                 (center=({cx},{cy}), size=({w},{h}), angle={angle:.4})",
                p.x, p.y,
            );
        }
    }
}
