use rusty_mermaid_graph::{Graph, NodeId};

use crate::labels::{DummyKind, EdgeDummyData, EdgeLabel, NodeLabel};

/// Split long edges (spanning >1 rank) into chains of unit-length edges
/// through dummy nodes. Returns the head of each dummy chain (for undo).
///
/// Pre: DAG with ranks assigned. Post: every edge spans exactly 1 rank.
pub(crate) fn run(g: &mut Graph<NodeLabel, EdgeLabel>) -> Vec<NodeId> {
    let mut dummy_chains = Vec::new();

    let edges: Vec<_> = g.edge_ids().collect();
    for eid in edges {
        let Some((src, dst)) = g.edge_endpoints(eid) else {
            continue;
        };
        let src_rank = g.node(src).unwrap().rank;
        let dst_rank = g.node(dst).unwrap().rank;

        // Already unit length — nothing to do
        if dst_rank == src_rank + 1 {
            continue;
        }

        let edge_label = g.remove_edge(eid).unwrap();
        let label_rank = edge_label.label_rank;
        let weight = edge_label.weight;

        let mut prev = src;
        let mut rank = src_rank + 1;
        let mut first_dummy = None;

        while rank < dst_rank {
            let mut dummy_label = NodeLabel::new(0.0, 0.0);
            dummy_label.rank = rank;
            dummy_label.dummy = Some(DummyKind::Edge);

            // The first dummy in the chain stores the original edge data
            if first_dummy.is_none() {
                dummy_label.edge_data = Some(EdgeDummyData {
                    edge_label: edge_label.clone(),
                    edge_src: src,
                    edge_dst: dst,
                });
            }

            // If this rank is the label rank, promote to EdgeLabel dummy
            if label_rank == Some(rank) {
                dummy_label.width = edge_label.width;
                dummy_label.height = edge_label.height;
                dummy_label.dummy = Some(DummyKind::EdgeLabel);
                dummy_label.label_pos = Some(edge_label.labelpos);
            }

            let dummy = g.add_node(dummy_label);

            if first_dummy.is_none() {
                first_dummy = Some(dummy);
            }

            g.add_edge(prev, dummy, EdgeLabel::new().with_weight(weight));
            prev = dummy;
            rank += 1;
        }

        // Final edge to the original destination
        g.add_edge(prev, dst, EdgeLabel::new().with_weight(weight));

        if let Some(head) = first_dummy {
            dummy_chains.push(head);
        }
    }

    dummy_chains
}

/// Restore original long edges, collecting intermediate positions as points.
/// Removes all dummy nodes in the chains.
pub(crate) fn undo(g: &mut Graph<NodeLabel, EdgeLabel>, dummy_chains: &[NodeId]) {
    for &chain_head in dummy_chains {
        let Some(node) = g.node(chain_head) else {
            continue;
        };
        let Some(edge_data) = node.edge_data.clone() else {
            continue;
        };

        let mut orig_label = edge_data.edge_label;
        orig_label.points.clear();

        // Walk the dummy chain, collecting positions
        let mut v = chain_head;
        while let Some(node) = g.node(v) {
            if node.dummy.is_none() {
                break;
            }

            orig_label.points.push(rusty_mermaid_core::Point {
                x: node.x,
                y: node.y,
            });

            if node.dummy == Some(DummyKind::EdgeLabel) {
                orig_label.x = node.x;
                orig_label.y = node.y;
                orig_label.width = node.width;
                orig_label.height = node.height;
            }

            // Find the single successor (next in chain)
            let next: Vec<_> = g.successors(v).collect();
            g.remove_node(v);
            v = match next.first() {
                Some(&n) => n,
                None => break,
            };
        }

        // Re-add the original edge with collected points
        g.add_edge(edge_data.edge_src, edge_data.edge_dst, orig_label);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Acyclicer, Ranker};
    use crate::labels::DummyKind;

    fn ranked_graph() -> (Graph<NodeLabel, EdgeLabel>, NodeId, NodeId, NodeId) {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        let c = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, b, EdgeLabel::default());
        g.add_edge(a, c, EdgeLabel::default());
        g.node_mut(a).unwrap().rank = 0;
        g.node_mut(b).unwrap().rank = 1;
        g.node_mut(c).unwrap().rank = 3; // long edge a->c spans 3 ranks
        (g, a, b, c)
    }

    #[test]
    fn unit_length_edge_unchanged() {
        let (mut g, a, b, _) = ranked_graph();
        let chains = run(&mut g);
        // a->b was already unit length, no dummy chain for it
        // a->c was long, one chain
        assert_eq!(chains.len(), 1);
        // a->b still exists (though with new edge id after re-add)
        assert!(g.successors(a).any(|n| n == b));
    }

    #[test]
    fn long_edge_split_into_unit_edges() {
        let (mut g, _, _, _) = ranked_graph();
        let chains = run(&mut g);
        assert_eq!(chains.len(), 1);

        // Every edge should now span exactly 1 rank
        for eid in g.edge_ids() {
            let (src, dst) = g.edge_endpoints(eid).unwrap();
            let span = g.node(dst).unwrap().rank - g.node(src).unwrap().rank;
            assert_eq!(span, 1, "edge {src}->{dst} has span {span}");
        }

        // There should be dummy nodes between a and c
        // a(rank=0) -> dummy(rank=1) -> dummy(rank=2) -> c(rank=3)
        // But a->b also exists at rank 1, so we have a->b and a->dummy1
        let dummies: Vec<_> = g
            .node_ids()
            .filter(|&nid| g.node(nid).unwrap().dummy.is_some())
            .collect();
        assert_eq!(dummies.len(), 2); // ranks 1 and 2
    }

    #[test]
    fn dummy_nodes_have_correct_kind() {
        let (mut g, _, _, _) = ranked_graph();
        let chains = run(&mut g);

        let chain_head = chains[0];
        assert_eq!(g.node(chain_head).unwrap().dummy, Some(DummyKind::Edge));
        // First dummy stores edge data
        assert!(g.node(chain_head).unwrap().edge_data.is_some());
    }

    #[test]
    fn undo_restores_original_edge() {
        let (mut g, a, _, c) = ranked_graph();
        let orig_node_count = g.node_count();
        let chains = run(&mut g);
        assert!(g.node_count() > orig_node_count);

        undo(&mut g, &chains);
        // Dummy nodes removed, back to original count
        assert_eq!(g.node_count(), orig_node_count);
        // a->c edge restored
        assert!(g.successors(a).any(|n| n == c));
    }

    #[test]
    fn already_normalized_is_noop() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, b, EdgeLabel::default());
        g.node_mut(a).unwrap().rank = 0;
        g.node_mut(b).unwrap().rank = 1;

        let chains = run(&mut g);
        assert!(chains.is_empty());
        assert_eq!(g.node_count(), 2);
        assert_eq!(g.edge_count(), 1);
    }

    use proptest::prelude::*;

    fn arb_ranked_dag(
        max_nodes: usize,
        max_edges: usize,
    ) -> impl Strategy<Value = Graph<NodeLabel, EdgeLabel>> {
        (1..=max_nodes, 0..=max_edges).prop_flat_map(move |(n_nodes, n_edges)| {
            let edges =
                prop::collection::vec((0..n_nodes, 0..n_nodes, 1..4i32, 1..5u32), 0..=n_edges);
            edges.prop_map(move |edge_specs| {
                let mut g = Graph::new();
                let nodes: Vec<_> = (0..n_nodes)
                    .map(|_| g.add_node(NodeLabel::new(40.0, 20.0)))
                    .collect();
                for (src_idx, dst_idx, minlen, weight) in edge_specs {
                    g.add_edge(
                        nodes[src_idx],
                        nodes[dst_idx],
                        EdgeLabel::new().with_minlen(minlen).with_weight(weight as f64),
                    );
                }
                crate::acyclic::run(&mut g, Acyclicer::Dfs);
                crate::rank::rank(&mut g, Ranker::LongestPath);
                g
            })
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(200))]

        #[test]
        fn normalize_all_unit_length(mut g in arb_ranked_dag(12, 20)) {
            let _chains = run(&mut g);
            for eid in g.edge_ids() {
                let (src, dst) = g.edge_endpoints(eid).unwrap();
                let span = g.node(dst).unwrap().rank - g.node(src).unwrap().rank;
                prop_assert_eq!(
                    span, 1,
                    "edge {}->{}: span {} != 1",
                    src, dst, span
                );
            }
        }
    }
}
