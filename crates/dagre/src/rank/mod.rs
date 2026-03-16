pub(crate) mod feasible_tree;
pub(crate) mod longest_path;
pub(crate) mod network_simplex;

use rusty_mermaid_graph::Graph;

use crate::config::Ranker;
use crate::labels::{EdgeLabel, NodeLabel};
use crate::util;

/// Assign ranks to all nodes in the graph.
pub fn rank(g: &mut Graph<NodeLabel, EdgeLabel>, ranker: Ranker) {
    match ranker {
        Ranker::LongestPath => {
            longest_path::longest_path(g);
            util::normalize_ranks(g);
        }
        Ranker::TightTree => {
            // Tight tree: longest path + feasible tree (no NS optimization)
            longest_path::longest_path(g);
            util::normalize_ranks(g);
            let _tree = feasible_tree::feasible_tree_mut(g);
            util::normalize_ranks(g);
        }
        Ranker::NetworkSimplex => {
            network_simplex::network_simplex(g);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusty_mermaid_graph::NodeId;

    fn ranked(g: &Graph<NodeLabel, EdgeLabel>, nid: NodeId) -> i32 {
        g.node(nid).unwrap().rank
    }

    fn assert_feasible(g: &Graph<NodeLabel, EdgeLabel>) {
        for eid in g.edge_ids() {
            let (src, dst) = g.edge_endpoints(eid).unwrap();
            let span = ranked(g, dst) - ranked(g, src);
            let minlen = g.edge(eid).unwrap().minlen;
            assert!(
                span >= minlen,
                "infeasible: {src}->{dst} span={span} < minlen={minlen}"
            );
        }
    }

    fn make_diamond() -> Graph<NodeLabel, EdgeLabel> {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        let c = g.add_node(NodeLabel::new(10.0, 10.0));
        let d = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, b, EdgeLabel::default());
        g.add_edge(a, c, EdgeLabel::default());
        g.add_edge(b, d, EdgeLabel::default());
        g.add_edge(c, d, EdgeLabel::default());
        g
    }

    #[test]
    fn longest_path_ranker() {
        let mut g = make_diamond();
        rank(&mut g, Ranker::LongestPath);
        assert_feasible(&g);
    }

    #[test]
    fn tight_tree_ranker() {
        let mut g = make_diamond();
        rank(&mut g, Ranker::TightTree);
        assert_feasible(&g);
    }

    #[test]
    fn network_simplex_ranker() {
        let mut g = make_diamond();
        rank(&mut g, Ranker::NetworkSimplex);
        assert_feasible(&g);
    }

    #[test]
    fn ranks_start_at_zero() {
        let mut g = make_diamond();
        rank(&mut g, Ranker::NetworkSimplex);
        let min_rank = g
            .node_ids()
            .map(|nid| ranked(&g, nid))
            .min()
            .unwrap();
        assert_eq!(min_rank, 0);
    }
}
