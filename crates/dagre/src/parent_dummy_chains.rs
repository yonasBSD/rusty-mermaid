use std::collections::BTreeMap;

use rusty_mermaid_graph::{Graph, NodeId};

use crate::labels::{EdgeLabel, NodeLabel};

/// Assign proper parents to dummy nodes in long-edge chains so they
/// respect the compound hierarchy.
///
/// Each dummy in a chain gets parented to the appropriate compound node
/// along the path from the edge's source to its destination through
/// their lowest common ancestor. When source and destination are in
/// different top-level compounds (LCA = graph root), dummies in the gap
/// between compounds are left unparented.
pub(crate) fn parent_dummy_chains(
    graph: &mut Graph<NodeLabel, EdgeLabel>,
    dummy_chains: &[NodeId],
) {
    let postorder_nums = postorder(graph);

    for &chain_head in dummy_chains {
        let Some(node) = graph.node(chain_head) else {
            continue;
        };
        let Some(ref edge_data) = node.edge_data else {
            continue;
        };
        let edge_src = edge_data.edge_src;
        let edge_dst = edge_data.edge_dst;

        let (path, lca) = find_path(graph, &postorder_nums, edge_src, edge_dst);
        let mut path_idx = 0;
        let mut ascending = true;

        let mut v = chain_head;
        while let Some(node) = graph.node(v) {
            if node.dummy.is_none() {
                break;
            }
            let node_rank = node.rank;

            if ascending {
                // Walk up through source's ancestors until we reach the LCA
                // or find a compound whose maxRank covers the current rank
                while path_idx < path.len() && path[path_idx] != lca {
                    if let Some(path_node) = path[path_idx] {
                        let max_rank =
                            graph.node(path_node).and_then(|n| n.max_rank).unwrap_or(0);
                        if max_rank >= node_rank {
                            break;
                        }
                    }
                    path_idx += 1;
                }

                if path_idx < path.len() && path[path_idx] == lca {
                    ascending = false;
                }
            }

            if !ascending {
                // Walk down through destination's ancestors
                while path_idx + 1 < path.len() {
                    if let Some(next) = path[path_idx + 1] {
                        let min_rank =
                            graph.node(next).and_then(|n| n.min_rank).unwrap_or(i32::MAX);
                        if min_rank > node_rank {
                            break;
                        }
                    } else {
                        break;
                    }
                    path_idx += 1;
                }
            }

            // Parent the dummy: Some(id) → parent to that compound,
            // None → leave unparented (at graph root level)
            if path_idx < path.len()
                && let Some(parent_id) = path[path_idx] {
                    graph.set_parent(v, parent_id);
                }
                // None means graph root — dummy stays unparented

            // Move to next dummy in chain
            let next: Vec<_> = graph.successors(v).collect();
            match next.first() {
                Some(&n) => v = n,
                None => break,
            }
        }
    }
}

/// Find a path from v to w through their lowest common ancestor (LCA)
/// in the compound hierarchy. Returns (path, lca).
///
/// The path contains `Option<NodeId>` — `None` represents the graph root.
/// When v and w are in different top-level compounds (no common compound
/// ancestor), lca = None and the path includes None as a sentinel between
/// the source and destination compound hierarchies.
fn find_path(
    graph: &Graph<NodeLabel, EdgeLabel>,
    postorder_nums: &BTreeMap<NodeId, PostorderNum>,
    v: NodeId,
    w: NodeId,
) -> (Vec<Option<NodeId>>, Option<NodeId>) {
    let v_num = postorder_nums.get(&v).copied().unwrap_or_default();
    let w_num = postorder_nums.get(&w).copied().unwrap_or_default();
    let low = v_num.low.min(w_num.low);
    let lim = v_num.lim.max(w_num.lim);

    // Traverse up from v to find the LCA
    let mut v_path: Vec<Option<NodeId>> = Vec::new();
    let mut parent = graph.parent(v);
    let lca: Option<NodeId>;

    loop {
        v_path.push(parent);
        match parent {
            Some(p) => {
                let p_num = postorder_nums.get(&p).copied().unwrap_or_default();
                if p_num.low <= low && lim <= p_num.lim {
                    lca = Some(p);
                    break;
                }
                parent = graph.parent(p);
            }
            None => {
                // Reached graph root without finding an LCA compound
                lca = None;
                break;
            }
        }
    }

    // Traverse from w up to LCA
    let mut w_path: Vec<Option<NodeId>> = Vec::new();
    parent = graph.parent(w);
    loop {
        if parent == lca {
            break;
        }
        w_path.push(parent);
        match parent {
            Some(p) => parent = graph.parent(p),
            None => break,
        }
    }

    w_path.reverse();
    v_path.extend(w_path);
    (v_path, lca)
}

#[derive(Debug, Clone, Copy, Default)]
struct PostorderNum {
    low: usize,
    lim: usize,
}

/// Compute postorder numbering over the compound hierarchy.
fn postorder(graph: &Graph<NodeLabel, EdgeLabel>) -> BTreeMap<NodeId, PostorderNum> {
    let mut result = BTreeMap::new();
    let mut counter = 0;

    fn dfs(
        graph: &Graph<NodeLabel, EdgeLabel>,
        v: NodeId,
        counter: &mut usize,
        result: &mut BTreeMap<NodeId, PostorderNum>,
    ) {
        let low = *counter;
        let children: Vec<_> = graph.children(v).collect();
        for child in children {
            dfs(graph, child, counter, result);
        }
        result.insert(v, PostorderNum { low, lim: *counter });
        *counter += 1;
    }

    let roots: Vec<_> = graph.roots().collect();
    for root in roots {
        dfs(graph, root, &mut counter, &mut result);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::labels::DummyKind;
    use crate::normalize;

    #[test]
    fn dummy_chain_gets_parented() {
        // Build: sg contains a and b. Edge a->c spans multiple ranks.
        // After normalize, dummy nodes between a and c should get parented to sg.
        let mut g = Graph::new();
        let sg = g.add_node(NodeLabel::new(100.0, 100.0));
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        let c = g.add_node(NodeLabel::new(10.0, 10.0));
        g.set_parent(a, sg);
        g.set_parent(b, sg);
        g.add_edge(a, b, EdgeLabel::default());
        g.add_edge(a, c, EdgeLabel::default());
        g.node_mut(a).unwrap().rank = 0;
        g.node_mut(b).unwrap().rank = 1;
        g.node_mut(c).unwrap().rank = 3;

        // Set compound node rank bounds
        g.node_mut(sg).unwrap().min_rank = Some(0);
        g.node_mut(sg).unwrap().max_rank = Some(1);

        let chains = normalize::run(&mut g);
        parent_dummy_chains(&mut g, &chains);

        // Dummy at rank 1 should be parented to sg (within sg's rank range)
        // Dummy at rank 2 should not be parented to sg (outside range)
        for &chain_head in &chains {
            let mut v = chain_head;
            loop {
                let node = g.node(v).unwrap();
                if node.dummy.is_none() {
                    break;
                }
                if node.rank <= 1 {
                    assert_eq!(
                        g.parent(v),
                        Some(sg),
                        "dummy at rank {} should be in sg",
                        node.rank
                    );
                }
                let next: Vec<_> = g.successors(v).collect();
                match next.first() {
                    Some(&n) => v = n,
                    None => break,
                }
            }
        }
    }

    #[test]
    fn cross_compound_dummies_unparented() {
        // sg1 contains a, sg2 contains b. Edge a->b crosses compounds.
        // Dummy nodes between sg1 and sg2 should be unparented (at graph root).
        let mut g = Graph::new();
        let sg1 = g.add_node(NodeLabel::new(100.0, 100.0));
        let sg2 = g.add_node(NodeLabel::new(100.0, 100.0));
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        g.set_parent(a, sg1);
        g.set_parent(b, sg2);
        g.add_edge(a, b, EdgeLabel::default());
        g.node_mut(a).unwrap().rank = 0;
        g.node_mut(b).unwrap().rank = 4;
        g.node_mut(sg1).unwrap().min_rank = Some(0);
        g.node_mut(sg1).unwrap().max_rank = Some(0);
        g.node_mut(sg2).unwrap().min_rank = Some(4);
        g.node_mut(sg2).unwrap().max_rank = Some(4);

        let chains = normalize::run(&mut g);
        parent_dummy_chains(&mut g, &chains);

        // Dummies at ranks 1-3 should be unparented (between the two compounds)
        for &chain_head in &chains {
            let mut v = chain_head;
            loop {
                let node = g.node(v).unwrap();
                if node.dummy.is_none() {
                    break;
                }
                let r = node.rank;
                if r >= 1 && r <= 3 {
                    assert!(
                        g.parent(v).is_none(),
                        "dummy at rank {r} should be unparented (between compounds), but has parent {:?}",
                        g.parent(v)
                    );
                }
                let next: Vec<_> = g.successors(v).collect();
                match next.first() {
                    Some(&n) => v = n,
                    None => break,
                }
            }
        }
    }

    #[test]
    fn no_compound_is_noop() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, b, EdgeLabel::default());
        g.node_mut(a).unwrap().rank = 0;
        g.node_mut(b).unwrap().rank = 3;

        let chains = normalize::run(&mut g);
        parent_dummy_chains(&mut g, &chains);

        // No compound nodes, so no parenting happens
        let dummies: Vec<_> = g
            .node_ids()
            .filter(|&n| g.node(n).unwrap().dummy == Some(DummyKind::Edge))
            .collect();
        for d in dummies {
            assert!(g.parent(d).is_none());
        }
    }

    #[test]
    fn postorder_numbering() {
        let mut g: Graph<NodeLabel, EdgeLabel> = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        g.set_parent(b, a);

        let nums = postorder(&g);
        // a contains b, so a.lim > b.lim
        assert!(nums[&a].lim > nums[&b].lim);
        assert!(nums[&a].low <= nums[&b].low);
    }
}
