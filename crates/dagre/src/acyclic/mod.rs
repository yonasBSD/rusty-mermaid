mod dfs_fas;
mod greedy_fas;

use rusty_mermaid_graph::Graph;

use crate::config::Acyclicer;
use crate::labels::{EdgeLabel, NodeLabel};

/// Remove cycles by reversing back-edges. Must call `undo` after layout
/// to restore original edge directions.
pub fn run(graph: &mut Graph<NodeLabel, EdgeLabel>, acyclicer: Acyclicer) {
    let fas = match acyclicer {
        Acyclicer::Dfs => dfs_fas::dfs_fas(graph),
        Acyclicer::Greedy => greedy_fas::greedy_fas(graph),
    };

    for eid in fas {
        if let Some((src, dst)) = graph.edge_endpoints(eid)
            && let Some(mut label) = graph.remove_edge(eid)
        {
            label.reversed = true;
            // Self-loops: just remove (reversing src==dst is a no-op)
            if src != dst {
                graph.add_edge(dst, src, label);
            }
        }
    }
}

/// Restore reversed edges to their original direction.
pub fn undo(graph: &mut Graph<NodeLabel, EdgeLabel>) {
    let reversed: Vec<_> = graph
        .edge_ids()
        .filter(|&eid| graph.edge(eid).is_some_and(|l| l.reversed))
        .collect();

    for eid in reversed {
        if let Some((src, dst)) = graph.edge_endpoints(eid)
            && let Some(mut label) = graph.remove_edge(eid)
        {
            label.reversed = false;
            graph.add_edge(dst, src, label);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusty_mermaid_graph::NodeId;

    fn make_cycle() -> (Graph<NodeLabel, EdgeLabel>, NodeId, NodeId, NodeId) {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        let c = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, b, EdgeLabel::default());
        g.add_edge(b, c, EdgeLabel::default());
        g.add_edge(c, a, EdgeLabel::default());
        (g, a, b, c)
    }

    fn is_acyclic(g: &Graph<NodeLabel, EdgeLabel>) -> bool {
        rusty_mermaid_graph::topo_sort(g).is_some()
    }

    #[test]
    fn dfs_run_makes_dag() {
        let (mut g, _, _, _) = make_cycle();
        assert!(!is_acyclic(&g));
        run(&mut g, Acyclicer::Dfs);
        assert!(is_acyclic(&g));
    }

    #[test]
    fn greedy_run_makes_dag() {
        let (mut g, _, _, _) = make_cycle();
        assert!(!is_acyclic(&g));
        run(&mut g, Acyclicer::Greedy);
        assert!(is_acyclic(&g));
    }

    #[test]
    fn undo_restores_edges() {
        let (mut g, _, _, _) = make_cycle();
        let orig_edge_count = g.edge_count();
        run(&mut g, Acyclicer::Dfs);
        undo(&mut g);
        assert_eq!(g.edge_count(), orig_edge_count);
        // No edges should be marked reversed
        for eid in g.edge_ids() {
            assert!(!g.edge(eid).unwrap().reversed);
        }
    }

    #[test]
    fn dag_unchanged() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, b, EdgeLabel::default());
        let orig_count = g.edge_count();
        run(&mut g, Acyclicer::Dfs);
        assert_eq!(g.edge_count(), orig_count);
        // No reversed edges
        for eid in g.edge_ids() {
            assert!(!g.edge(eid).unwrap().reversed);
        }
    }
}
