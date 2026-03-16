use std::collections::HashSet;

use rusty_mermaid_graph::{EdgeId, Graph, NodeId};

use crate::labels::{EdgeLabel, NodeLabel};

/// Find a feedback arc set using DFS. Back-edges (edges from a node
/// to an ancestor on the DFS stack) form the FAS.
pub(crate) fn dfs_fas(g: &Graph<NodeLabel, EdgeLabel>) -> Vec<EdgeId> {
    let mut fas = Vec::new();
    let mut visited = HashSet::new();
    let mut on_stack = HashSet::new();

    for node in g.node_ids().collect::<Vec<_>>() {
        if !visited.contains(&node) {
            dfs(g, node, &mut visited, &mut on_stack, &mut fas);
        }
    }
    fas
}

fn dfs(
    g: &Graph<NodeLabel, EdgeLabel>,
    v: NodeId,
    visited: &mut HashSet<NodeId>,
    on_stack: &mut HashSet<NodeId>,
    fas: &mut Vec<EdgeId>,
) {
    if visited.contains(&v) {
        return;
    }
    visited.insert(v);
    on_stack.insert(v);

    for eid in g.out_edges(v).collect::<Vec<_>>() {
        if let Some((_, dst)) = g.edge_endpoints(eid) {
            if on_stack.contains(&dst) {
                fas.push(eid);
            } else {
                dfs(g, dst, visited, on_stack, fas);
            }
        }
    }

    on_stack.remove(&v);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dag_has_empty_fas() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        let c = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, b, EdgeLabel::default());
        g.add_edge(b, c, EdgeLabel::default());
        assert!(dfs_fas(&g).is_empty());
    }

    #[test]
    fn cycle_detected() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        let c = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, b, EdgeLabel::default());
        g.add_edge(b, c, EdgeLabel::default());
        g.add_edge(c, a, EdgeLabel::default());
        let fas = dfs_fas(&g);
        assert_eq!(fas.len(), 1);
    }

    #[test]
    fn self_loop_detected() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, a, EdgeLabel::default());
        let fas = dfs_fas(&g);
        assert_eq!(fas.len(), 1);
    }

    #[test]
    fn disconnected_with_cycle() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        let c = g.add_node(NodeLabel::new(10.0, 10.0));
        let d = g.add_node(NodeLabel::new(10.0, 10.0));
        // Component 1: a -> b (acyclic)
        g.add_edge(a, b, EdgeLabel::default());
        // Component 2: c -> d -> c (cycle)
        g.add_edge(c, d, EdgeLabel::default());
        g.add_edge(d, c, EdgeLabel::default());
        let fas = dfs_fas(&g);
        assert_eq!(fas.len(), 1);
    }
}
