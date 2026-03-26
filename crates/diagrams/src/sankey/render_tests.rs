use super::*;

fn render(input: &str) -> Scene {
    let d = parser::parse(input).unwrap();
    to_scene(&d)
}

#[test]
fn basic_renders() {
    let scene = render("sankey-beta\nA,B,10\nA,C,5\nB,D,8\nC,D,4");
    assert!(!scene.is_empty());
    assert!(scene.width > 0.0);
}

#[test]
fn has_nodes_and_links() {
    let scene = render("sankey-beta\nA,B,10\nA,C,5");
    // 2 links + 3 node rects + 3 labels = 8
    assert!(
        scene.len() >= 8,
        "expected >= 8 elements, got {}",
        scene.len()
    );
}

#[test]
fn node_rects_have_fixed_width() {
    let scene = render("sankey-beta\nA,B,100");
    let rects: Vec<_> = scene
        .elements()
        .iter()
        .filter(|e| matches!(&e.primitive, Primitive::Rect { .. }))
        .collect();
    assert_eq!(rects.len(), 2, "should have 2 node rects");
    for r in &rects {
        if let Primitive::Rect { bbox, .. } = &r.primitive {
            assert!((bbox.width - NODE_WIDTH).abs() < 0.01);
        }
    }
}

#[test]
fn links_are_curves() {
    let scene = render("sankey-beta\nA,B,10\nA,C,5");
    let curves = scene
        .elements()
        .iter()
        .filter(|e| {
            if let Primitive::Path { segments, .. } = &e.primitive {
                segments
                    .iter()
                    .any(|s| matches!(s, PathSegment::CubicTo { .. }))
            } else {
                false
            }
        })
        .count();
    assert_eq!(curves, 2, "should have 2 curved links");
}

#[test]
fn multi_column_layout() {
    let scene = render("sankey-beta\nA,B,10\nB,C,10\nC,D,10");
    // 4 columns: A→B→C→D
    let rects: Vec<_> = scene
        .elements()
        .iter()
        .filter_map(|e| {
            if let Primitive::Rect { bbox, .. } = &e.primitive {
                Some(bbox.x)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(rects.len(), 4);
    // Each subsequent node should be to the right
    for w in rects.windows(2) {
        assert!(w[1] > w[0], "nodes should progress left to right");
    }
}

#[test]
fn node_heights_proportional() {
    let scene = render("sankey-beta\nA,B,100\nA,C,50");
    let heights: Vec<_> = scene
        .elements()
        .iter()
        .filter_map(|e| {
            if let Primitive::Rect { bbox, .. } = &e.primitive {
                Some(bbox.height)
            } else {
                None
            }
        })
        .collect();
    // A has value 150, B has 100, C has 50
    // A should be tallest
    assert!(heights.len() == 3);
    assert!(heights[0] > heights[1], "A should be taller than B");
    assert!(heights[1] > heights[2], "B should be taller than C");
}

#[test]
fn all_positions_finite() {
    let scene = render("sankey-beta\nA,B,10\nB,C,5\nA,C,3\nC,D,8");
    for elem in scene.elements() {
        match &elem.primitive {
            Primitive::Rect { bbox, .. } => {
                assert!(bbox.x.is_finite() && bbox.y.is_finite());
                assert!(bbox.width.is_finite() && bbox.height.is_finite());
            }
            Primitive::Text { position, .. } => {
                assert!(position.x.is_finite() && position.y.is_finite());
            }
            Primitive::Path { segments, .. } => {
                for seg in segments {
                    match seg {
                        PathSegment::MoveTo(p) => {
                            assert!(p.x.is_finite() && p.y.is_finite());
                        }
                        PathSegment::CubicTo { cp1, cp2, to } => {
                            assert!(cp1.x.is_finite() && cp1.y.is_finite());
                            assert!(cp2.x.is_finite() && cp2.y.is_finite());
                            assert!(to.x.is_finite() && to.y.is_finite());
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}
