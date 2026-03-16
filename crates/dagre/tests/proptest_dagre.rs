use proptest::prelude::*;
use rusty_mermaid_dagre::{Acyclicer, EdgeLabel, NodeLabel, Ranker};
use rusty_mermaid_graph::Graph;

/// Generate a random graph with up to `max_nodes` nodes and `max_edges` edges.
fn arb_graph(
    max_nodes: usize,
    max_edges: usize,
) -> impl Strategy<Value = Graph<NodeLabel, EdgeLabel>> {
    (1..=max_nodes, 0..=max_edges).prop_flat_map(move |(n_nodes, n_edges)| {
        let edges = prop::collection::vec((0..n_nodes, 0..n_nodes, 1..4i32, 1..5u32), 0..=n_edges);
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
            g
        })
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    /// After acyclic removal, the graph must be a DAG (topo_sort succeeds).
    #[test]
    fn acyclic_produces_dag(mut g in arb_graph(15, 25)) {
        rusty_mermaid_dagre::acyclic::run(&mut g, Acyclicer::Dfs);
        prop_assert!(
            rusty_mermaid_graph::topo_sort(&g).is_some(),
            "graph should be acyclic after acyclic::run"
        );
    }

    /// After ranking, every edge must span >= its minlen.
    #[test]
    fn rank_respects_minlen_longest_path(mut g in arb_graph(15, 25)) {
        // Make DAG first
        rusty_mermaid_dagre::acyclic::run(&mut g, Acyclicer::Dfs);
        rusty_mermaid_dagre::rank::rank(&mut g, Ranker::LongestPath);

        for eid in g.edge_ids() {
            let (src, dst) = g.edge_endpoints(eid).unwrap();
            let span = g.node(dst).unwrap().rank - g.node(src).unwrap().rank;
            let minlen = g.edge(eid).unwrap().minlen;
            prop_assert!(
                span >= minlen,
                "edge {}->{}: span {} < minlen {}",
                src, dst, span, minlen
            );
        }
    }

    /// Network simplex ranking also respects minlen.
    #[test]
    fn rank_respects_minlen_network_simplex(mut g in arb_graph(12, 20)) {
        rusty_mermaid_dagre::acyclic::run(&mut g, Acyclicer::Dfs);
        rusty_mermaid_dagre::rank::rank(&mut g, Ranker::NetworkSimplex);

        for eid in g.edge_ids() {
            let (src, dst) = g.edge_endpoints(eid).unwrap();
            let span = g.node(dst).unwrap().rank - g.node(src).unwrap().rank;
            let minlen = g.edge(eid).unwrap().minlen;
            prop_assert!(
                span >= minlen,
                "edge {}->{}: span {} < minlen {}",
                src, dst, span, minlen
            );
        }
    }

    /// Ranks always start at 0 after normalization.
    #[test]
    fn ranks_start_at_zero(mut g in arb_graph(10, 15)) {
        rusty_mermaid_dagre::acyclic::run(&mut g, Acyclicer::Dfs);
        rusty_mermaid_dagre::rank::rank(&mut g, Ranker::NetworkSimplex);

        if g.node_count() > 0 {
            let min_rank = g.node_ids()
                .map(|nid| g.node(nid).unwrap().rank)
                .min()
                .unwrap();
            prop_assert_eq!(min_rank, 0);
        }
    }
}
