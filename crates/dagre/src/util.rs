use rusty_mermaid_graph::{EdgeId, Graph, NodeId};

use crate::labels::{EdgeLabel, NodeLabel};

/// Slack of a directed edge: rank(dst) - rank(src) - minlen.
/// A tight edge has slack == 0.
pub(crate) fn slack(graph: &Graph<NodeLabel, EdgeLabel>, eid: EdgeId) -> i32 {
    let Some((src, dst)) = graph.edge_endpoints(eid) else {
        return 0;
    };
    let src_rank = graph.node(src).map_or(0, |n| n.rank);
    let dst_rank = graph.node(dst).map_or(0, |n| n.rank);
    let minlen = graph.edge(eid).map_or(1, |e| e.minlen);
    dst_rank - src_rank - minlen
}

/// Slack between a pair of nodes using effective (max) minlen.
pub(crate) fn slack_pair(graph: &Graph<NodeLabel, EdgeLabel>, src: NodeId, dst: NodeId) -> i32 {
    let src_rank = graph.node(src).map_or(0, |n| n.rank);
    let dst_rank = graph.node(dst).map_or(0, |n| n.rank);
    let minlen = effective_minlen(graph, src, dst);
    dst_rank - src_rank - minlen
}

/// Combined weight of all parallel edges from src to dst.
pub(crate) fn combined_weight(
    graph: &Graph<NodeLabel, EdgeLabel>,
    src: NodeId,
    dst: NodeId,
) -> f64 {
    let mut sum = 0.0;
    for eid in graph.out_edges(src) {
        if let Some((_, d)) = graph.edge_endpoints(eid)
            && d == dst
        {
            sum += graph.edge(eid).map_or(0.0, |l| l.weight);
        }
    }
    sum
}

/// Maximum minlen across all parallel edges from src to dst.
pub(crate) fn effective_minlen(
    graph: &Graph<NodeLabel, EdgeLabel>,
    src: NodeId,
    dst: NodeId,
) -> i32 {
    let mut max = 0;
    for eid in graph.out_edges(src) {
        if let Some((_, d)) = graph.edge_endpoints(eid)
            && d == dst
        {
            max = max.max(graph.edge(eid).map_or(1, |l| l.minlen));
        }
    }
    max
}

/// Shift all ranks so the minimum rank is 0.
pub(crate) fn normalize_ranks(graph: &mut Graph<NodeLabel, EdgeLabel>) {
    let min_rank = graph
        .node_ids()
        .filter_map(|id| graph.node(id).map(|n| n.rank))
        .min()
        .unwrap_or(0);

    for id in graph.node_ids().collect::<Vec<_>>() {
        if let Some(n) = graph.node_mut(id) {
            n.rank -= min_rank;
        }
    }
}

/// Check if a directed edge exists from src to dst in the graph.
pub(crate) fn has_directed_edge(
    graph: &Graph<NodeLabel, EdgeLabel>,
    src: NodeId,
    dst: NodeId,
) -> bool {
    graph
        .out_edges(src)
        .any(|eid| graph.edge_endpoints(eid).is_some_and(|(_, d)| d == dst))
}

/// All edges incident to a node (both in and out), with endpoints.
pub(crate) fn node_edges(
    graph: &Graph<NodeLabel, EdgeLabel>,
    v: NodeId,
) -> Vec<(EdgeId, NodeId, NodeId)> {
    let mut result = Vec::new();
    for eid in graph.in_edges(v) {
        if let Some((s, d)) = graph.edge_endpoints(eid) {
            result.push((eid, s, d));
        }
    }
    for eid in graph.out_edges(v) {
        if let Some((s, d)) = graph.edge_endpoints(eid) {
            result.push((eid, s, d));
        }
    }
    result
}

/// Maximum rank across all nodes in the graph.
pub fn max_rank(graph: &Graph<NodeLabel, EdgeLabel>) -> i32 {
    graph
        .node_ids()
        .filter_map(|id| graph.node(id).map(|n| n.rank))
        .max()
        .unwrap_or(0)
}

/// Build a layer matrix: layers[rank] = [node_ids sorted by order].
pub fn build_layer_matrix(graph: &Graph<NodeLabel, EdgeLabel>) -> Vec<Vec<NodeId>> {
    build_layer_matrix_filtered(graph, false)
}

/// Build a layer matrix excluding compound nodes (nodes with children).
///
/// Mirrors JS dagre's `asNonCompoundGraph` which strips compound nodes
/// before BK positioning so their zero-width/height doesn't corrupt alignment.
pub fn build_layer_matrix_leaves(graph: &Graph<NodeLabel, EdgeLabel>) -> Vec<Vec<NodeId>> {
    build_layer_matrix_filtered(graph, true)
}

fn build_layer_matrix_filtered(
    graph: &Graph<NodeLabel, EdgeLabel>,
    leaves_only: bool,
) -> Vec<Vec<NodeId>> {
    let max = max_rank(graph);
    let mut layers = vec![Vec::new(); (max + 1) as usize];
    for nid in graph.node_ids() {
        if leaves_only && graph.children(nid).next().is_some() {
            continue;
        }
        let Some(node) = graph.node(nid) else {
            continue;
        };
        let rank = node.rank;
        if rank >= 0 && (rank as usize) < layers.len() {
            layers[rank as usize].push(nid);
        }
    }
    // Sort each layer by current order
    for layer in &mut layers {
        layer.sort_by_key(|&nid| graph.node(nid).map_or(0, |n| n.order));
    }
    layers
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_chain() -> Graph<NodeLabel, EdgeLabel> {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        let c = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, b, EdgeLabel::default());
        g.add_edge(b, c, EdgeLabel::default());
        // Set ranks manually: a=0, b=1, c=2
        g.node_mut(a).unwrap().rank = 0;
        g.node_mut(b).unwrap().rank = 1;
        g.node_mut(c).unwrap().rank = 2;
        g
    }

    #[test]
    fn slack_tight_edge() {
        let g = make_chain();
        let eid = g.edge_ids().next().unwrap();
        assert_eq!(slack(&g, eid), 0);
    }

    #[test]
    fn normalize_ranks_shifts_to_zero() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, b, EdgeLabel::default());
        g.node_mut(a).unwrap().rank = -3;
        g.node_mut(b).unwrap().rank = -1;
        normalize_ranks(&mut g);
        assert_eq!(g.node(a).unwrap().rank, 0);
        assert_eq!(g.node(b).unwrap().rank, 2);
    }
}
