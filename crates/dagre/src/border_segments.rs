use rusty_mermaid_graph::{Graph, NodeId};

use crate::labels::{BorderType, DummyKind, EdgeLabel, NodeLabel};

/// Add left and right border dummy nodes to compound nodes at each rank
/// where the compound has children.
///
/// Pre: ranks assigned, compound nodes have minRank/maxRank set.
/// Post: compound nodes have borderLeft/borderRight chains linking
/// consecutive ranks.
pub(crate) fn add_border_segments(g: &mut Graph<NodeLabel, EdgeLabel>) {
    let roots: Vec<_> = g.roots().collect();
    for root in roots {
        dfs(g, root);
    }
}

fn dfs(g: &mut Graph<NodeLabel, EdgeLabel>, v: NodeId) {
    let children: Vec<_> = g.children(v).collect();
    for child in children {
        dfs(g, child);
    }

    let (min_rank, max_rank) = {
        let Some(node) = g.node(v) else { return };
        match (node.min_rank, node.max_rank) {
            (Some(min), Some(max)) => (min, max),
            _ => return, // not a compound node with rank bounds
        }
    };

    for rank in min_rank..=max_rank {
        add_border_node(g, BorderType::Left, v, rank);
        add_border_node(g, BorderType::Right, v, rank);
    }
}

fn add_border_node(
    g: &mut Graph<NodeLabel, EdgeLabel>,
    border_type: BorderType,
    sg: NodeId,
    rank: i32,
) {
    let mut label = NodeLabel::new(0.0, 0.0);
    label.rank = rank;
    label.dummy = Some(DummyKind::Border);
    label.border_type = Some(border_type);

    let curr = g.add_node(label);
    g.set_parent(curr, sg);

    // Get the previous border node at rank-1 and link them
    let prev = {
        let Some(sg_node) = g.node(sg) else { return };
        let borders = match border_type {
            BorderType::Left => &sg_node.border_left,
            BorderType::Right => &sg_node.border_right,
        };
        borders.get(&(rank - 1)).copied()
    };

    // Store current node in the compound's border map
    let Some(sg_node) = g.node_mut(sg) else { return };
    match border_type {
        BorderType::Left => sg_node.border_left.insert(rank, curr),
        BorderType::Right => sg_node.border_right.insert(rank, curr),
    };

    // Chain to previous border node
    if let Some(prev_id) = prev {
        g.add_edge(prev_id, curr, EdgeLabel::new().with_weight(1.0));
    }
}

/// Assign min_rank and max_rank to compound nodes based on their
/// border_top/border_bottom nodes' ranks.
pub(crate) fn assign_rank_min_max(g: &mut Graph<NodeLabel, EdgeLabel>) -> i32 {
    let mut max_rank = 0;

    let nids: Vec<_> = g.node_ids().collect();
    for nid in nids {
        let Some(node) = g.node(nid) else { continue };
        if let (Some(top), Some(bottom)) = (node.border_top, node.border_bottom) {
            let min = g.node(top).map_or(0, |n| n.rank);
            let max = g.node(bottom).map_or(0, |n| n.rank);

            let Some(node) = g.node_mut(nid) else { continue };
            node.min_rank = Some(min);
            node.max_rank = Some(max);

            if max > max_rank {
                max_rank = max;
            }
        }
    }

    max_rank
}

/// Extend min_rank/max_rank on compound nodes to cover children added after
/// the initial assign (e.g. dummy nodes from parent_dummy_chains).
///
/// This must run after parent_dummy_chains + add_border_segments so that
/// sort_subgraph's rank filter can find compound nodes at ranks where they
/// have dummy children.
pub(crate) fn extend_rank_min_max(g: &mut Graph<NodeLabel, EdgeLabel>) {
    let compounds: Vec<NodeId> = g
        .node_ids()
        .filter(|&nid| g.children(nid).next().is_some())
        .collect();

    for sg in compounds {
        let Some(node) = g.node(sg) else { continue };
        let Some(mut min) = node.min_rank else {
            continue;
        };
        let Some(mut max) = node.max_rank else {
            continue;
        };

        for child in g.children(sg) {
            let Some(cn) = g.node(child) else { continue };
            min = min.min(cn.rank);
            max = max.max(cn.rank);
        }

        let Some(node) = g.node_mut(sg) else { continue };
        node.min_rank = Some(min);
        node.max_rank = Some(max);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_compound_is_noop() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, b, EdgeLabel::default());
        g.node_mut(a).unwrap().rank = 0;
        g.node_mut(b).unwrap().rank = 1;

        let orig_count = g.node_count();
        add_border_segments(&mut g);
        assert_eq!(g.node_count(), orig_count);
    }

    #[test]
    fn compound_gets_border_nodes() {
        let mut g = Graph::new();
        let sg = g.add_node(NodeLabel::new(100.0, 100.0));
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        g.set_parent(a, sg);
        g.set_parent(b, sg);
        g.add_edge(a, b, EdgeLabel::default());
        g.node_mut(a).unwrap().rank = 1;
        g.node_mut(b).unwrap().rank = 2;

        // Simulate assignRankMinMax
        g.node_mut(sg).unwrap().min_rank = Some(1);
        g.node_mut(sg).unwrap().max_rank = Some(2);

        add_border_segments(&mut g);

        let sg_node = g.node(sg).unwrap();
        // Should have border_left and border_right at ranks 1 and 2
        assert_eq!(sg_node.border_left.len(), 2);
        assert_eq!(sg_node.border_right.len(), 2);
        assert!(sg_node.border_left.contains_key(&1));
        assert!(sg_node.border_left.contains_key(&2));
    }

    #[test]
    fn border_nodes_chained() {
        let mut g = Graph::new();
        let sg = g.add_node(NodeLabel::new(100.0, 100.0));
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        let c = g.add_node(NodeLabel::new(10.0, 10.0));
        g.set_parent(a, sg);
        g.set_parent(b, sg);
        g.set_parent(c, sg);
        g.node_mut(a).unwrap().rank = 0;
        g.node_mut(b).unwrap().rank = 1;
        g.node_mut(c).unwrap().rank = 2;
        g.node_mut(sg).unwrap().min_rank = Some(0);
        g.node_mut(sg).unwrap().max_rank = Some(2);

        add_border_segments(&mut g);

        let sg_node = g.node(sg).unwrap();
        // Left border at rank 0 -> rank 1 -> rank 2 should be chained
        let bl0 = sg_node.border_left[&0];
        let bl1 = sg_node.border_left[&1];
        let bl2 = sg_node.border_left[&2];
        assert!(g.successors(bl0).any(|n| n == bl1));
        assert!(g.successors(bl1).any(|n| n == bl2));
    }

    #[test]
    fn assign_rank_min_max_sets_bounds() {
        let mut g = Graph::new();
        let sg = g.add_node(NodeLabel::new(100.0, 100.0));
        let top = g.add_node(NodeLabel::new(0.0, 0.0));
        let bottom = g.add_node(NodeLabel::new(0.0, 0.0));
        g.node_mut(top).unwrap().rank = 1;
        g.node_mut(bottom).unwrap().rank = 4;
        g.node_mut(sg).unwrap().border_top = Some(top);
        g.node_mut(sg).unwrap().border_bottom = Some(bottom);

        let max = assign_rank_min_max(&mut g);

        let sg_node = g.node(sg).unwrap();
        assert_eq!(sg_node.min_rank, Some(1));
        assert_eq!(sg_node.max_rank, Some(4));
        assert_eq!(max, 4);
    }
}
