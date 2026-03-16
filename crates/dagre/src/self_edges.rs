use rusty_mermaid_graph::Graph;

use crate::labels::{DummyKind, EdgeLabel, NodeLabel, SelfEdge};
use crate::util;

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
        let (src, dst) = g.edge_endpoints(eid).unwrap();
        let label = g.edge(eid).unwrap().clone();
        g.node_mut(src).unwrap().self_edges.push(SelfEdge {
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
            g.node_mut(v).unwrap().order = i + order_shift;

            let self_edges: Vec<SelfEdge> =
                std::mem::take(&mut g.node_mut(v).unwrap().self_edges);

            for se in self_edges {
                order_shift += 1;
                let mut dummy = NodeLabel::new(se.label.width, se.label.height);
                dummy.dummy = Some(DummyKind::SelfEdge);
                dummy.rank = g.node(v).unwrap().rank;
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
        .filter(|&nid| g.node(nid).unwrap().dummy == Some(DummyKind::SelfEdge))
        .collect();

    for nid in dummy_ids {
        let node = g.node(nid).unwrap().clone();
        let sed = node.self_edge_data.as_ref().unwrap();
        let self_node = g.node(sed.src).unwrap();
        let sx = self_node.x + self_node.width / 2.0;
        let sy = self_node.y;
        let dx = node.x - sx;
        let dy = self_node.height / 2.0;

        let mut label = sed.label.clone();
        label.points = vec![
            rusty_mermaid_core::Point {
                x: sx + 2.0 * dx / 3.0,
                y: sy - dy,
            },
            rusty_mermaid_core::Point {
                x: sx + 5.0 * dx / 6.0,
                y: sy - dy,
            },
            rusty_mermaid_core::Point {
                x: sx + dx,
                y: sy,
            },
            rusty_mermaid_core::Point {
                x: sx + 5.0 * dx / 6.0,
                y: sy + dy,
            },
            rusty_mermaid_core::Point {
                x: sx + 2.0 * dx / 3.0,
                y: sy + dy,
            },
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
