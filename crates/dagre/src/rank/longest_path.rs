use std::collections::HashSet;

use rusty_mermaid_graph::{Graph, NodeId};

use crate::labels::{EdgeLabel, NodeLabel};

/// Assign ranks using longest-path algorithm.
/// Sources get the lowest ranks; each node's rank = min(succ.rank - minlen).
/// Results in a feasible ranking (all edges span >= minlen).
pub(crate) fn longest_path(graph: &mut Graph<NodeLabel, EdgeLabel>) {
    let mut visited = HashSet::new();
    let sources: Vec<NodeId> = graph.sources().collect();

    for src in sources {
        dfs(graph, src, &mut visited);
    }

    // Handle nodes unreachable from sources (shouldn't happen in a DAG
    // after acyclic, but be safe)
    for nid in graph.node_ids().collect::<Vec<_>>() {
        if !visited.contains(&nid) {
            dfs(graph, nid, &mut visited);
        }
    }
}

fn dfs(graph: &mut Graph<NodeLabel, EdgeLabel>, v: NodeId, visited: &mut HashSet<NodeId>) -> i32 {
    if visited.contains(&v) {
        return graph.node(v).map_or(0, |n| n.rank);
    }
    visited.insert(v);

    let out_edges: Vec<_> = graph.out_edges(v).collect();
    let mut rank = i32::MAX;

    for eid in out_edges {
        if let Some((_, dst)) = graph.edge_endpoints(eid) {
            let minlen = graph.edge(eid).map_or(1, |l| l.minlen);
            let succ_rank = dfs(graph, dst, visited);
            rank = rank.min(succ_rank - minlen);
        }
    }

    // Sink node (no out-edges): rank = 0
    if rank == i32::MAX {
        rank = 0;
    }

    if let Some(n) = graph.node_mut(v) {
        n.rank = rank;
    }
    rank
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::normalize_ranks;

    fn ranked(g: &Graph<NodeLabel, EdgeLabel>, nid: NodeId) -> i32 {
        g.node(nid).unwrap().rank
    }

    #[test]
    fn linear_chain() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        let c = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, b, EdgeLabel::default());
        g.add_edge(b, c, EdgeLabel::default());

        longest_path(&mut g);
        normalize_ranks(&mut g);

        assert_eq!(ranked(&g, a), 0);
        assert_eq!(ranked(&g, b), 1);
        assert_eq!(ranked(&g, c), 2);
    }

    #[test]
    fn diamond() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        let c = g.add_node(NodeLabel::new(10.0, 10.0));
        let d = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, b, EdgeLabel::default());
        g.add_edge(a, c, EdgeLabel::default());
        g.add_edge(b, d, EdgeLabel::default());
        g.add_edge(c, d, EdgeLabel::default());

        longest_path(&mut g);
        normalize_ranks(&mut g);

        assert_eq!(ranked(&g, a), 0);
        assert_eq!(ranked(&g, d), 2);
        // b and c should be at rank 1
        assert_eq!(ranked(&g, b), 1);
        assert_eq!(ranked(&g, c), 1);
    }

    #[test]
    fn respects_minlen() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, b, EdgeLabel::new().with_minlen(3));

        longest_path(&mut g);
        normalize_ranks(&mut g);

        assert_eq!(ranked(&g, a), 0);
        assert_eq!(ranked(&g, b), 3);
    }

    #[test]
    fn single_node() {
        let mut g: Graph<NodeLabel, EdgeLabel> = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));

        longest_path(&mut g);
        normalize_ranks(&mut g);

        assert_eq!(ranked(&g, a), 0);
    }

    #[test]
    fn disconnected() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        // No edges

        longest_path(&mut g);
        normalize_ranks(&mut g);

        // Both should be rank 0
        assert_eq!(ranked(&g, a), 0);
        assert_eq!(ranked(&g, b), 0);
    }

    #[test]
    fn all_edges_feasible() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        let c = g.add_node(NodeLabel::new(10.0, 10.0));
        let d = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, b, EdgeLabel::default());
        g.add_edge(a, c, EdgeLabel::new().with_minlen(2));
        g.add_edge(b, d, EdgeLabel::default());
        g.add_edge(c, d, EdgeLabel::default());

        longest_path(&mut g);
        normalize_ranks(&mut g);

        // Every edge must span >= its minlen
        for eid in g.edge_ids() {
            let (src, dst) = g.edge_endpoints(eid).unwrap();
            let span = ranked(&g, dst) - ranked(&g, src);
            let minlen = g.edge(eid).unwrap().minlen;
            assert!(
                span >= minlen,
                "edge {src}->{dst}: span={span} < minlen={minlen}"
            );
        }
    }
}
