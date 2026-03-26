use std::collections::BTreeMap;

use rusty_mermaid_graph::{Graph, NodeId};

use crate::labels::{EdgeLabel, NodeLabel};

/// Result of nesting graph transformation, needed for cleanup.
pub(crate) struct NestingState {
    pub(crate) nesting_root: NodeId,
    pub(crate) node_rank_factor: i32,
}

/// Transform a compound graph for Sugiyama layout.
///
/// Creates a nesting root node, border top/bottom nodes for each compound node,
/// and nesting edges to ensure children are placed between their parent's borders.
/// Also multiplies all edge minlens by a factor to prevent nodes from landing on
/// border ranks.
///
/// Based on Sander, "Layout of Compound Directed Graphs."
pub(crate) fn run(graph: &mut Graph<NodeLabel, EdgeLabel>) -> NestingState {
    let root = graph.add_node(NodeLabel::new(0.0, 0.0));

    let depths = tree_depths(graph);
    let height = depths.values().copied().max().unwrap_or(1).max(1);
    let node_sep = 2 * height + 1;

    // Multiply all existing edge minlens by node_sep
    let eids: Vec<_> = graph.edge_ids().collect();
    for eid in eids {
        if let Some(label) = graph.edge_mut(eid) {
            label.minlen *= node_sep;
        }
    }

    // Weight sufficient to keep subgraphs compact
    let weight = sum_weights(graph) + 1.0;

    // Process top-level children
    let top_children: Vec<_> = graph.roots().collect();
    for child in top_children {
        dfs(graph, root, node_sep, weight, height, &depths, child);
    }

    NestingState {
        nesting_root: root,
        node_rank_factor: node_sep,
    }
}

fn dfs(
    graph: &mut Graph<NodeLabel, EdgeLabel>,
    root: NodeId,
    node_sep: i32,
    weight: f64,
    height: i32,
    depths: &BTreeMap<NodeId, i32>,
    v: NodeId,
) {
    let children: Vec<_> = graph.children(v).collect();

    if children.is_empty() {
        // Leaf node: connect to root
        if v != root {
            graph.add_edge(
                root,
                v,
                EdgeLabel::new()
                    .with_minlen(node_sep)
                    .with_weight(0.0)
                    .with_nesting(),
            );
        }
        return;
    }

    // Compound node: create border top/bottom
    // Must be marked as dummy=Border so BK sep() uses edgesep not nodesep
    let mut top_label = NodeLabel::new(0.0, 0.0);
    top_label.dummy = Some(crate::labels::DummyKind::Border);
    let top = graph.add_node(top_label);
    let mut bottom_label = NodeLabel::new(0.0, 0.0);
    bottom_label.dummy = Some(crate::labels::DummyKind::Border);
    let bottom = graph.add_node(bottom_label);

    graph.set_parent(top, v);
    if let Some(n) = graph.node_mut(v) {
        n.border_top = Some(top);
    }

    graph.set_parent(bottom, v);
    if let Some(n) = graph.node_mut(v) {
        n.border_bottom = Some(bottom);
    }

    for child in children {
        dfs(graph, root, node_sep, weight, height, depths, child);

        let Some(child_node) = graph.node(child) else {
            continue;
        };
        let child_top = child_node.border_top.unwrap_or(child);
        let child_bottom = child_node.border_bottom.unwrap_or(child);
        let this_weight = if child_node.border_top.is_some() {
            weight
        } else {
            2.0 * weight
        };
        let minlen = if child_top != child_bottom {
            1
        } else {
            let v_depth = depths.get(&v).copied().unwrap_or(1);
            height - v_depth + 1
        };

        // top -> child_top
        graph.add_edge(
            top,
            child_top,
            EdgeLabel::new()
                .with_minlen(minlen)
                .with_weight(this_weight)
                .with_nesting(),
        );

        // child_bottom -> bottom
        graph.add_edge(
            child_bottom,
            bottom,
            EdgeLabel::new()
                .with_minlen(minlen)
                .with_weight(this_weight)
                .with_nesting(),
        );
    }

    // Connect root to top if v has no parent
    if graph.parent(v).is_none() {
        let v_depth = depths.get(&v).copied().unwrap_or(1);
        graph.add_edge(
            root,
            top,
            EdgeLabel::new()
                .with_minlen(height + v_depth)
                .with_weight(0.0)
                .with_nesting(),
        );
    }
}

/// Remove nesting artifacts: the nesting root and all nesting edges.
pub(crate) fn cleanup(graph: &mut Graph<NodeLabel, EdgeLabel>, state: &NestingState) {
    graph.remove_node(state.nesting_root);

    let nesting_edges: Vec<_> = graph
        .edge_ids()
        .filter(|&eid| graph.edge(eid).is_some_and(|l| l.nesting_edge))
        .collect();
    for eid in nesting_edges {
        graph.remove_edge(eid);
    }
}

/// Compute depth of each node in the compound hierarchy.
/// Root-level nodes get depth 1, their children get 2, etc.
fn tree_depths(graph: &Graph<NodeLabel, EdgeLabel>) -> BTreeMap<NodeId, i32> {
    let mut depths = BTreeMap::new();

    fn dfs_depth(
        graph: &Graph<NodeLabel, EdgeLabel>,
        v: NodeId,
        depth: i32,
        depths: &mut BTreeMap<NodeId, i32>,
    ) {
        let children: Vec<_> = graph.children(v).collect();
        for child in children {
            dfs_depth(graph, child, depth + 1, depths);
        }
        depths.insert(v, depth);
    }

    let roots: Vec<_> = graph.roots().collect();
    for root in roots {
        dfs_depth(graph, root, 1, &mut depths);
    }
    depths
}

fn sum_weights(graph: &Graph<NodeLabel, EdgeLabel>) -> f64 {
    graph.edge_ids()
        .filter_map(|eid| graph.edge(eid).map(|l| l.weight))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_graph_connects_to_root() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, b, EdgeLabel::default());

        let state = run(&mut g);

        // Root node exists and has edges to both a and b
        assert!(g.has_node(state.nesting_root));
        let root_succs: Vec<_> = g.successors(state.nesting_root).collect();
        assert_eq!(root_succs.len(), 2);
    }

    #[test]
    fn compound_gets_border_nodes() {
        let mut g = Graph::new();
        let parent = g.add_node(NodeLabel::new(100.0, 100.0));
        let child = g.add_node(NodeLabel::new(10.0, 10.0));
        g.set_parent(child, parent);

        let state = run(&mut g);

        let parent_label = g.node(parent).unwrap();
        assert!(parent_label.border_top.is_some());
        assert!(parent_label.border_bottom.is_some());

        let top = parent_label.border_top.unwrap();
        let bottom = parent_label.border_bottom.unwrap();
        // Border nodes are children of the compound
        assert_eq!(g.parent(top), Some(parent));
        assert_eq!(g.parent(bottom), Some(parent));

        cleanup(&mut g, &state);
        // Root and nesting edges removed
        assert!(!g.has_node(state.nesting_root));
    }

    #[test]
    fn cleanup_removes_nesting_edges() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, b, EdgeLabel::default());

        let state = run(&mut g);
        let edge_count_before = g.edge_count();
        assert!(edge_count_before > 1); // nesting edges added

        cleanup(&mut g, &state);
        // Only original edge remains
        assert_eq!(g.edge_count(), 1);
    }

    #[test]
    fn minlens_scaled() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        let eid = g.add_edge(a, b, EdgeLabel::default());

        let _state = run(&mut g);

        // Original edge minlen should be scaled by node_sep (2*1+1 = 3 for flat graph)
        let label = g.edge(eid).unwrap();
        assert!(label.minlen > 1);
    }
}
