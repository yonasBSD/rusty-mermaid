use std::collections::HashSet;

use rusty_mermaid_graph::{Graph, NodeId};

use crate::labels::{EdgeLabel, NodeLabel};
use crate::util;

/// Assign initial order to nodes via DFS from sources.
///
/// Nodes are visited starting from rank-0 nodes (sorted by rank), following
/// successors. Each node is appended to its rank's layer in visit order.
/// Returns the resulting layer matrix.
pub(crate) fn init_order(graph: &mut Graph<NodeLabel, EdgeLabel>) -> Vec<Vec<NodeId>> {
    let max = util::max_rank(graph);
    let mut layers = vec![Vec::new(); (max + 1) as usize];
    let mut visited = HashSet::new();

    // Collect leaf nodes (no compound children), sorted by rank
    let mut simple_nodes: Vec<_> = graph
        .node_ids()
        .filter(|&nid| graph.children(nid).next().is_none())
        .collect();
    simple_nodes.sort_by_key(|&nid| graph.node(nid).map_or(0, |n| n.rank));

    for v in simple_nodes {
        dfs(graph, v, &mut visited, &mut layers);
    }

    // Assign order from layer positions
    for layer in &layers {
        for (i, &nid) in layer.iter().enumerate() {
            let Some(node) = graph.node_mut(nid) else {
                continue;
            };
            node.order = i;
        }
    }

    layers
}

fn dfs(
    graph: &Graph<NodeLabel, EdgeLabel>,
    v: NodeId,
    visited: &mut HashSet<NodeId>,
    layers: &mut [Vec<NodeId>],
) {
    if !visited.insert(v) {
        return;
    }
    let Some(node) = graph.node(v) else { return };
    let rank = node.rank;
    if rank >= 0 && (rank as usize) < layers.len() {
        layers[rank as usize].push(v);
    }
    let succs: Vec<_> = graph.successors(v).collect();
    for w in succs {
        dfs(graph, w, visited, layers);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_chain_preserves_order() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        let c = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, b, EdgeLabel::default());
        g.add_edge(b, c, EdgeLabel::default());
        g.node_mut(a).unwrap().rank = 0;
        g.node_mut(b).unwrap().rank = 1;
        g.node_mut(c).unwrap().rank = 2;

        let layers = init_order(&mut g);
        assert_eq!(layers.len(), 3);
        assert_eq!(layers[0], vec![a]);
        assert_eq!(layers[1], vec![b]);
        assert_eq!(layers[2], vec![c]);
    }

    #[test]
    fn diamond_assigns_orders() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        let c = g.add_node(NodeLabel::new(10.0, 10.0));
        let d = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, b, EdgeLabel::default());
        g.add_edge(a, c, EdgeLabel::default());
        g.add_edge(b, d, EdgeLabel::default());
        g.add_edge(c, d, EdgeLabel::default());
        g.node_mut(a).unwrap().rank = 0;
        g.node_mut(b).unwrap().rank = 1;
        g.node_mut(c).unwrap().rank = 1;
        g.node_mut(d).unwrap().rank = 2;

        let layers = init_order(&mut g);
        assert_eq!(layers[0].len(), 1);
        assert_eq!(layers[1].len(), 2);
        assert_eq!(layers[2].len(), 1);
        // Both b and c should have orders assigned
        assert_ne!(g.node(b).unwrap().order, g.node(c).unwrap().order);
    }

    #[test]
    fn disconnected_nodes_included() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        g.node_mut(a).unwrap().rank = 0;
        g.node_mut(b).unwrap().rank = 0;

        let layers = init_order(&mut g);
        assert_eq!(layers[0].len(), 2);
    }
}
