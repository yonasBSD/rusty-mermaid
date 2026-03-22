use rusty_mermaid_graph::Graph;

use crate::labels::{DummyKind, EdgeLabel, NodeLabel, SelfEdge};
use crate::util;

/// Self-edge loop control point fractions (matches dagre.js).
const LOOP_INNER: f64 = 2.0 / 3.0;
const LOOP_OUTER: f64 = 5.0 / 6.0;

/// Remove self-edges from the graph, storing them on their source node.
pub(crate) fn remove_self_edges(g: &mut Graph<NodeLabel, EdgeLabel>) {
    let self_eids: Vec<_> = g
        .edge_ids()
        .filter(|&eid| {
            g.edge_endpoints(eid)
                .is_some_and(|(src, dst)| src == dst)
        })
        .collect();

    for eid in self_eids {
        let Some((src, dst)) = g.edge_endpoints(eid) else { continue };
        let Some(label) = g.edge(eid) else { continue };
        let label = label.clone();
        let Some(node) = g.node_mut(src) else { continue };
        node.self_edges.push(SelfEdge {
            src,
            dst,
            label,
        });
        g.remove_edge(eid);
    }
}

/// Insert dummy nodes for self-edges after ordering.
///
/// Each self-edge becomes a "selfedge" dummy node placed right after
/// its source node in the same layer.
pub(crate) fn insert_self_edges(g: &mut Graph<NodeLabel, EdgeLabel>) {
    let layers = util::build_layer_matrix(g);
    for layer in &layers {
        let mut order_shift = 0usize;
        for (i, &v) in layer.iter().enumerate() {
            let Some(node) = g.node_mut(v) else { continue };
            node.order = i + order_shift;

            let self_edges: Vec<SelfEdge> = std::mem::take(&mut node.self_edges);

            let rank = g.node(v).map_or(0, |n| n.rank);
            for se in self_edges {
                order_shift += 1;
                let mut dummy = NodeLabel::new(se.label.width, se.label.height);
                dummy.dummy = Some(DummyKind::SelfEdge);
                dummy.rank = rank;
                dummy.order = i + order_shift;
                dummy.self_edge_data = Some(SelfEdgeData {
                    src: se.src,
                    dst: se.dst,
                    label: se.label,
                });
                g.add_node(dummy);
            }
        }
    }
}

/// Position self-edges after coordinate assignment: create edge points
/// forming a loop to the right of the source node.
pub(crate) fn position_self_edges(g: &mut Graph<NodeLabel, EdgeLabel>) {
    let dummy_ids: Vec<_> = g
        .node_ids()
        .filter(|&nid| g.node(nid).is_some_and(|n| n.dummy == Some(DummyKind::SelfEdge)))
        .collect();

    for nid in dummy_ids {
        let Some(node) = g.node(nid) else { continue };
        let node = node.clone();
        let Some(sed) = node.self_edge_data.as_ref() else { continue };
        let Some(self_node) = g.node(sed.src) else { continue };
        let sx = self_node.x + self_node.width / 2.0;
        let sy = self_node.y;
        let dx = node.x - sx;
        let dy = self_node.height / 2.0;

        let mut label = sed.label.clone();
        label.points = vec![
            rusty_mermaid_core::Point { x: sx + LOOP_INNER * dx, y: sy - dy },
            rusty_mermaid_core::Point { x: sx + LOOP_OUTER * dx, y: sy - dy },
            rusty_mermaid_core::Point { x: sx + dx,              y: sy },
            rusty_mermaid_core::Point { x: sx + LOOP_OUTER * dx, y: sy + dy },
            rusty_mermaid_core::Point { x: sx + LOOP_INNER * dx, y: sy + dy },
        ];
        label.x = node.x;
        label.y = node.y;

        g.add_edge(sed.src, sed.dst, label);
        g.remove_node(nid);
    }
}

/// Data for a self-edge dummy node.
#[derive(Debug, Clone)]
pub(crate) struct SelfEdgeData {
    pub(crate) src: rusty_mermaid_graph::NodeId,
    pub(crate) dst: rusty_mermaid_graph::NodeId,
    pub(crate) label: EdgeLabel,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::labels::DummyKind;

    // ── remove_self_edges ──────────────────────────────────────────────

    #[test]
    fn remove_no_self_edges_is_noop() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, b, EdgeLabel::default());

        remove_self_edges(&mut g);

        assert_eq!(g.edge_count(), 1);
        assert!(g.node(a).unwrap().self_edges.is_empty());
        assert!(g.node(b).unwrap().self_edges.is_empty());
    }

    #[test]
    fn remove_single_self_edge() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, a, EdgeLabel::new().with_weight(7.0));

        remove_self_edges(&mut g);

        assert_eq!(g.edge_count(), 0);
        let stored = &g.node(a).unwrap().self_edges;
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].src, a);
        assert_eq!(stored[0].dst, a);
    }

    #[test]
    fn remove_multiple_self_edges_same_node() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, a, EdgeLabel::new().with_minlen(1));
        g.add_edge(a, a, EdgeLabel::new().with_minlen(2));
        g.add_edge(a, a, EdgeLabel::new().with_minlen(3));

        remove_self_edges(&mut g);

        assert_eq!(g.edge_count(), 0);
        assert_eq!(g.node(a).unwrap().self_edges.len(), 3);
    }

    #[test]
    fn remove_self_edges_preserves_normal_edges() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, b, EdgeLabel::default());
        g.add_edge(b, a, EdgeLabel::default());
        g.add_edge(a, a, EdgeLabel::new().with_weight(5.0));

        remove_self_edges(&mut g);

        // Only the self-edge removed; the two normal edges remain
        assert_eq!(g.edge_count(), 2);
        assert_eq!(g.node(a).unwrap().self_edges.len(), 1);
        assert!(g.node(b).unwrap().self_edges.is_empty());
    }

    #[test]
    fn remove_self_edge_preserves_label() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let label = EdgeLabel::new().with_minlen(4).with_weight(3.5);
        g.add_edge(a, a, label);

        remove_self_edges(&mut g);

        let stored = &g.node(a).unwrap().self_edges[0].label;
        assert_eq!(stored.minlen, 4);
        assert!((stored.weight - 3.5).abs() < f64::EPSILON);
    }

    // ── insert_self_edges ──────────────────────────────────────────────

    #[test]
    fn insert_no_self_edges_preserves_order() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        g.node_mut(a).unwrap().rank = 0;
        g.node_mut(a).unwrap().order = 0;
        g.node_mut(b).unwrap().rank = 0;
        g.node_mut(b).unwrap().order = 1;

        let count_before = g.node_count();
        insert_self_edges(&mut g);

        assert_eq!(g.node_count(), count_before);
        assert_eq!(g.node(a).unwrap().order, 0);
        assert_eq!(g.node(b).unwrap().order, 1);
    }

    #[test]
    fn insert_creates_dummy_with_correct_rank_and_order() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(40.0, 20.0));
        let b = g.add_node(NodeLabel::new(40.0, 20.0));
        g.node_mut(a).unwrap().rank = 0;
        g.node_mut(a).unwrap().order = 0;
        g.node_mut(b).unwrap().rank = 0;
        g.node_mut(b).unwrap().order = 1;

        // Simulate a previously removed self-edge on a
        g.node_mut(a).unwrap().self_edges.push(SelfEdge {
            src: a,
            dst: a,
            label: EdgeLabel::new().with_weight(2.0),
        });

        insert_self_edges(&mut g);

        // One dummy node should have been added
        assert_eq!(g.node_count(), 3);

        let dummies: Vec<_> = g
            .node_ids()
            .filter(|&nid| g.node(nid).unwrap().dummy == Some(DummyKind::SelfEdge))
            .collect();
        assert_eq!(dummies.len(), 1);

        let dummy = g.node(dummies[0]).unwrap();
        assert_eq!(dummy.rank, 0);
        assert_eq!(dummy.order, 1); // right after a (order 0)
        assert!(dummy.self_edge_data.is_some());
        let sed = dummy.self_edge_data.as_ref().unwrap();
        assert_eq!(sed.src, a);
        assert_eq!(sed.dst, a);
    }

    #[test]
    fn insert_shifts_order_of_subsequent_nodes() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        g.node_mut(a).unwrap().rank = 0;
        g.node_mut(a).unwrap().order = 0;
        g.node_mut(b).unwrap().rank = 0;
        g.node_mut(b).unwrap().order = 1;

        // Two self-edges on a
        g.node_mut(a).unwrap().self_edges.push(SelfEdge {
            src: a,
            dst: a,
            label: EdgeLabel::default(),
        });
        g.node_mut(a).unwrap().self_edges.push(SelfEdge {
            src: a,
            dst: a,
            label: EdgeLabel::default(),
        });

        insert_self_edges(&mut g);

        // a=0, dummy1=1, dummy2=2, b=3
        assert_eq!(g.node(a).unwrap().order, 0);
        assert_eq!(g.node(b).unwrap().order, 3);

        let mut dummy_orders: Vec<_> = g
            .node_ids()
            .filter(|&nid| g.node(nid).unwrap().dummy == Some(DummyKind::SelfEdge))
            .map(|nid| g.node(nid).unwrap().order)
            .collect();
        dummy_orders.sort();
        assert_eq!(dummy_orders, vec![1, 2]);
    }

    // ── position_self_edges ────────────────────────────────────────────

    fn setup_for_position() -> (Graph<NodeLabel, EdgeLabel>, rusty_mermaid_graph::NodeId) {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(100.0, 60.0));
        g.node_mut(a).unwrap().x = 50.0;
        g.node_mut(a).unwrap().y = 30.0;
        g.node_mut(a).unwrap().rank = 0;
        g.node_mut(a).unwrap().order = 0;

        // Simulate a removed self-edge, then inserted dummy
        let edge_label = EdgeLabel::new().with_weight(2.0);
        let mut dummy = NodeLabel::new(edge_label.width, edge_label.height);
        dummy.dummy = Some(DummyKind::SelfEdge);
        dummy.rank = 0;
        dummy.order = 1;
        dummy.x = 150.0;
        dummy.y = 30.0;
        dummy.self_edge_data = Some(SelfEdgeData {
            src: a,
            dst: a,
            label: edge_label,
        });
        g.add_node(dummy);

        (g, a)
    }

    #[test]
    fn position_removes_dummy_nodes() {
        let (mut g, _a) = setup_for_position();
        assert_eq!(g.node_count(), 2);

        position_self_edges(&mut g);

        // Dummy removed, only the original node remains
        assert_eq!(g.node_count(), 1);
        let dummies: Vec<_> = g
            .node_ids()
            .filter(|&nid| g.node(nid).unwrap().dummy == Some(DummyKind::SelfEdge))
            .collect();
        assert!(dummies.is_empty());
    }

    #[test]
    fn position_creates_five_point_loop() {
        let (mut g, a) = setup_for_position();

        position_self_edges(&mut g);

        // The self-edge should have been re-added to the graph
        assert_eq!(g.edge_count(), 1);

        let eid = g.edge_ids().next().unwrap();
        let (src, dst) = g.edge_endpoints(eid).unwrap();
        assert_eq!(src, a);
        assert_eq!(dst, a);

        let label = g.edge(eid).unwrap();
        assert_eq!(label.points.len(), 5);
    }

    #[test]
    fn position_sets_label_xy_from_dummy() {
        let (mut g, _a) = setup_for_position();

        position_self_edges(&mut g);

        let eid = g.edge_ids().next().unwrap();
        let label = g.edge(eid).unwrap();
        // Label position comes from the dummy node coordinates
        assert!((label.x - 150.0).abs() < f64::EPSILON);
        assert!((label.y - 30.0).abs() < f64::EPSILON);
    }

    #[test]
    fn position_loop_points_geometry() {
        let (mut g, _a) = setup_for_position();

        position_self_edges(&mut g);

        let eid = g.edge_ids().next().unwrap();
        let pts = &g.edge(eid).unwrap().points;
        let self_node_x = 50.0;
        let self_node_width = 100.0;
        let self_node_y = 30.0;
        let self_node_height = 60.0;

        let sx = self_node_x + self_node_width / 2.0; // 100.0
        let sy = self_node_y; // 30.0
        let dummy_x = 150.0;
        let dx = dummy_x - sx; // 50.0
        let dy = self_node_height / 2.0; // 30.0

        // Verify the 5-point loop path (use same constants as production code)
        let expected = vec![
            (sx + LOOP_INNER * dx, sy - dy),
            (sx + LOOP_OUTER * dx, sy - dy),
            (sx + dx, sy),
            (sx + LOOP_OUTER * dx, sy + dy),
            (sx + LOOP_INNER * dx, sy + dy),
        ];

        for (i, (ex, ey)) in expected.iter().enumerate() {
            assert!(
                (pts[i].x - ex).abs() < f64::EPSILON,
                "point {i} x: expected {ex}, got {}",
                pts[i].x
            );
            assert!(
                (pts[i].y - ey).abs() < f64::EPSILON,
                "point {i} y: expected {ey}, got {}",
                pts[i].y
            );
        }

        // Structural checks: top two points above center, bottom two below
        assert!(pts[0].y < sy);
        assert!(pts[1].y < sy);
        assert!((pts[2].y - sy).abs() < f64::EPSILON); // middle point at center y
        assert!(pts[3].y > sy);
        assert!(pts[4].y > sy);
    }

    #[test]
    fn position_preserves_edge_label_weight() {
        let (mut g, _a) = setup_for_position();

        position_self_edges(&mut g);

        let eid = g.edge_ids().next().unwrap();
        let label = g.edge(eid).unwrap();
        assert!((label.weight - 2.0).abs() < f64::EPSILON);
    }
}
