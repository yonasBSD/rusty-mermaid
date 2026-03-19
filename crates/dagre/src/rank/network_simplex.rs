use rusty_mermaid_graph::{Graph, NodeId};

use crate::labels::{EdgeLabel, NodeLabel};
use crate::rank::feasible_tree::{feasible_tree_mut, NsTree};
use crate::rank::longest_path::longest_path;
use crate::util;

/// Optimal rank assignment using the network simplex algorithm.
/// Minimizes total weighted edge length subject to minlen constraints.
pub(crate) fn network_simplex(g: &mut Graph<NodeLabel, EdgeLabel>) {
    longest_path(g);
    util::normalize_ranks(g);

    if g.node_count() <= 1 {
        return;
    }

    let mut tree = feasible_tree_mut(g);
    init_cut_values(&mut tree, g);

    let max_iter = g.node_count() * g.node_count();
    let mut iter = 0;
    while let Some((leave_u, leave_v)) = find_leave_edge(&tree) {
        if let Some((enter_src, enter_dst)) = find_enter_edge(&tree, g, leave_u, leave_v) {
            exchange(&mut tree, g, leave_u, leave_v, enter_src, enter_dst);
        } else {
            break;
        }
        iter += 1;
        if iter >= max_iter {
            break;
        }
    }

    util::normalize_ranks(g);
}

/// Compute cut values for all tree edges (post-order traversal).
fn init_cut_values(tree: &mut NsTree, g: &Graph<NodeLabel, EdgeLabel>) {
    // Post-order: process children before parents
    let nodes: Vec<NodeId> = postorder_nodes(tree);
    for v in nodes {
        if tree.parent.get(&v).copied().flatten().is_some() {
            assign_cut_value(tree, g, v);
        }
    }
}

/// Post-order traversal of the tree.
fn postorder_nodes(tree: &NsTree) -> Vec<NodeId> {
    let mut result = Vec::new();
    let mut visited = std::collections::BTreeSet::new();
    postorder_dfs(tree, tree.root, &mut visited, &mut result);
    result
}

fn postorder_dfs(
    tree: &NsTree,
    v: NodeId,
    visited: &mut std::collections::BTreeSet<NodeId>,
    result: &mut Vec<NodeId>,
) {
    visited.insert(v);
    for &w in tree.neighbors(v) {
        if !visited.contains(&w) {
            postorder_dfs(tree, w, visited, result);
        }
    }
    result.push(v);
}

/// Compute the cut value for the tree edge connecting `child` to its parent.
fn assign_cut_value(tree: &mut NsTree, g: &Graph<NodeLabel, EdgeLabel>, child: NodeId) {
    let Some(parent) = tree.parent.get(&child).copied().flatten() else {
        return;
    };

    // Determine which direction the graph edge goes
    let child_is_tail = util::has_directed_edge(g, child, parent);
    let (graph_src, graph_dst) = if child_is_tail {
        (child, parent)
    } else {
        (parent, child)
    };

    let mut cut_value = util::combined_weight(g, graph_src, graph_dst);

    // For each edge incident to child (except the tree edge to parent)
    for (eid, src, dst) in util::node_edges(g, child) {
        let is_out_edge = src == child;
        let other = if is_out_edge { dst } else { src };

        if other == parent {
            continue;
        }

        let points_to_head = is_out_edge == child_is_tail;
        let edge_weight = g.edge(eid).map_or(1.0, |l| l.weight);

        cut_value += if points_to_head {
            edge_weight
        } else {
            -edge_weight
        };

        if tree.has_edge(child, other) {
            let other_cut = tree.get_cut_value(child, other);
            cut_value += if points_to_head {
                -other_cut
            } else {
                other_cut
            };
        }
    }

    tree.set_cut_value(child, parent, cut_value);
}

/// Find a tree edge with negative cut value (the leaving edge).
fn find_leave_edge(tree: &NsTree) -> Option<(NodeId, NodeId)> {
    for (&(u, v), &cv) in &tree.cut_values {
        if cv < 0.0 {
            return Some((u, v));
        }
    }
    None
}

/// Find a non-tree edge to enter the tree, replacing the leaving edge.
/// The entering edge must cross the same cut with minimum slack.
fn find_enter_edge(
    tree: &NsTree,
    g: &Graph<NodeLabel, EdgeLabel>,
    leave_u: NodeId,
    leave_v: NodeId,
) -> Option<(NodeId, NodeId)> {
    // Determine which end is the "tail" component
    // The tail component is the side of the tree edge where the
    // leaving edge points "from" in the graph direction
    let (v, w) = if util::has_directed_edge(g, leave_u, leave_v) {
        (leave_u, leave_v)
    } else {
        (leave_v, leave_u)
    };

    let v_lim = tree.lim.get(&v).copied().unwrap_or(0);
    let w_lim = tree.lim.get(&w).copied().unwrap_or(0);
    let flip = v_lim > w_lim;

    let tail_node = if flip { w } else { v };

    let mut best: Option<(NodeId, NodeId, i32)> = None;

    for eid in g.edge_ids() {
        if let Some((src, dst)) = g.edge_endpoints(eid) {
            let src_desc = tree.is_descendant(src, tail_node);
            let dst_desc = tree.is_descendant(dst, tail_node);

            // Edge must cross the cut: one end in tail, one end not
            let crosses = if flip {
                src_desc && !dst_desc
            } else {
                !src_desc && dst_desc
            };

            if crosses {
                let s = util::slack(g, eid);
                if best.is_none_or(|(_, _, bs)| s < bs) {
                    best = Some((src, dst, s));
                }
            }
        }
    }

    best.map(|(src, dst, _)| (src, dst))
}

/// Swap the leaving tree edge for the entering edge, then update the tree.
fn exchange(
    tree: &mut NsTree,
    g: &mut Graph<NodeLabel, EdgeLabel>,
    leave_u: NodeId,
    leave_v: NodeId,
    enter_src: NodeId,
    enter_dst: NodeId,
) {
    // Remove leaving edge — tree splits into two components
    tree.remove_edge(leave_u, leave_v);

    // Shift ranks BEFORE adding entering edge, so BFS only finds one component
    let enter_slack = util::slack_pair(g, enter_src, enter_dst);
    if enter_slack != 0 {
        let component = bfs_component(tree, enter_dst);
        for nid in component {
            if let Some(n) = g.node_mut(nid) {
                n.rank -= enter_slack;
            }
        }
    }

    // Add entering edge to reconnect the tree
    tree.add_edge(enter_src, enter_dst);

    // Recompute DFS numbering and cut values
    tree.init_low_lim();
    init_cut_values(tree, g);
}

/// BFS in the tree from a start node.
fn bfs_component(tree: &NsTree, start: NodeId) -> Vec<NodeId> {
    let mut visited = std::collections::BTreeSet::new();
    let mut queue = std::collections::VecDeque::new();
    visited.insert(start);
    queue.push_back(start);

    while let Some(v) = queue.pop_front() {
        for &w in tree.neighbors(v) {
            if visited.insert(w) {
                queue.push_back(w);
            }
        }
    }

    visited.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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

        network_simplex(&mut g);

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

        network_simplex(&mut g);

        assert_eq!(ranked(&g, a), 0);
        assert_eq!(ranked(&g, d), 2);
    }

    #[test]
    fn respects_minlen() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, b, EdgeLabel::new().with_minlen(3));

        network_simplex(&mut g);

        assert!(ranked(&g, b) - ranked(&g, a) >= 3);
    }

    #[test]
    fn single_node() {
        let mut g: Graph<NodeLabel, EdgeLabel> = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        network_simplex(&mut g);
        assert_eq!(ranked(&g, a), 0);
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

        network_simplex(&mut g);

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

    #[test]
    fn weighted_edges_prefer_short_heavy() {
        // Heavy edge should be kept tight; light edge can be longer
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        let c = g.add_node(NodeLabel::new(10.0, 10.0));
        // a->b: weight 10 (heavy, want tight)
        g.add_edge(a, b, EdgeLabel::new().with_weight(10.0));
        // a->c: weight 1 (light, can be longer)
        g.add_edge(a, c, EdgeLabel::new().with_weight(1.0));
        // b->c: weight 1
        g.add_edge(b, c, EdgeLabel::default());

        network_simplex(&mut g);

        // a->b should be tight (span == 1)
        assert_eq!(ranked(&g, b) - ranked(&g, a), 1);
    }
}
