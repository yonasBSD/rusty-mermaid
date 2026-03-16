use std::collections::HashMap;

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
pub(crate) fn run(g: &mut Graph<NodeLabel, EdgeLabel>) -> NestingState {
    let root = g.add_node(NodeLabel::new(0.0, 0.0));

    let depths = tree_depths(g);
    let height = depths.values().copied().max().unwrap_or(1).max(1);
    let node_sep = 2 * height + 1;

    // Multiply all existing edge minlens by node_sep
    let eids: Vec<_> = g.edge_ids().collect();
    for eid in eids {
        if let Some(label) = g.edge_mut(eid) {
            label.minlen *= node_sep;
        }
    }

    // Weight sufficient to keep subgraphs compact
    let weight = sum_weights(g) + 1.0;

    // Process top-level children
    let top_children: Vec<_> = g.roots().collect();
    for child in top_children {
        dfs(g, root, node_sep, weight, height, &depths, child);
    }

    NestingState {
        nesting_root: root,
        node_rank_factor: node_sep,
    }
}

fn dfs(
    g: &mut Graph<NodeLabel, EdgeLabel>,
    root: NodeId,
    node_sep: i32,
    weight: f64,
    height: i32,
    depths: &HashMap<NodeId, i32>,
    v: NodeId,
) {
    let children: Vec<_> = g.children(v).collect();

    if children.is_empty() {
        // Leaf node: connect to root
        if v != root {
            g.add_edge(
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
    let top = g.add_node(NodeLabel::new(0.0, 0.0));
    let bottom = g.add_node(NodeLabel::new(0.0, 0.0));

    g.set_parent(top, v);
    g.node_mut(v).unwrap().border_top = Some(top);

    g.set_parent(bottom, v);
    g.node_mut(v).unwrap().border_bottom = Some(bottom);

    for child in children {
        dfs(g, root, node_sep, weight, height, depths, child);

        let child_node = g.node(child).unwrap();
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
        g.add_edge(
            top,
            child_top,
            EdgeLabel::new()
                .with_minlen(minlen)
                .with_weight(this_weight)
                .with_nesting(),
        );

        // child_bottom -> bottom
        g.add_edge(
            child_bottom,
            bottom,
            EdgeLabel::new()
                .with_minlen(minlen)
                .with_weight(this_weight)
                .with_nesting(),
        );
    }

    // Connect root to top if v has no parent
    if g.parent(v).is_none() {
        let v_depth = depths.get(&v).copied().unwrap_or(1);
        g.add_edge(
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
pub(crate) fn cleanup(g: &mut Graph<NodeLabel, EdgeLabel>, state: &NestingState) {
    g.remove_node(state.nesting_root);

    let nesting_edges: Vec<_> = g
        .edge_ids()
        .filter(|&eid| g.edge(eid).is_some_and(|l| l.nesting_edge))
        .collect();
    for eid in nesting_edges {
        g.remove_edge(eid);
    }
}

/// Compute depth of each node in the compound hierarchy.
/// Root-level nodes get depth 1, their children get 2, etc.
fn tree_depths(g: &Graph<NodeLabel, EdgeLabel>) -> HashMap<NodeId, i32> {
    let mut depths = HashMap::new();

    fn dfs_depth(
        g: &Graph<NodeLabel, EdgeLabel>,
        v: NodeId,
        depth: i32,
        depths: &mut HashMap<NodeId, i32>,
    ) {
        let children: Vec<_> = g.children(v).collect();
        for child in children {
            dfs_depth(g, child, depth + 1, depths);
        }
        depths.insert(v, depth);
    }

    let roots: Vec<_> = g.roots().collect();
    for root in roots {
        dfs_depth(g, root, 1, &mut depths);
    }
    depths
}

fn sum_weights(g: &Graph<NodeLabel, EdgeLabel>) -> f64 {
    g.edge_ids()
        .filter_map(|eid| g.edge(eid).map(|l| l.weight))
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
