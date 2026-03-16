pub mod barycenter;
pub mod cross_count;
pub(crate) mod init_order;
pub(crate) mod resolve_conflicts;
pub(crate) mod sort_subgraph;

use rusty_mermaid_graph::{Graph, NodeId};

use crate::labels::{EdgeLabel, NodeLabel};
use crate::order::resolve_conflicts::ConstraintGraph;
use crate::util;

/// Assign node orders to minimize edge crossings.
///
/// Uses iterative barycenter heuristic: alternating up/down sweeps,
/// sorting each layer by the weighted average position of adjacent-layer
/// neighbors. Keeps the best ordering found.
pub fn order(g: &mut Graph<NodeLabel, EdgeLabel>) {
    let _layering = init_order::init_order(g);

    let max = util::max_rank(g);
    if max <= 0 {
        return;
    }

    let mut best_cc = f64::INFINITY;
    let mut best_orders: Vec<(NodeId, usize)> = Vec::new();

    // Match JS dagre loop: `for (let i=0, lastBest=0; lastBest<4; ++i, ++lastBest)`
    // lastBest is always incremented by the loop, only reset to 0 on strict improvement.
    let mut last_best = 0u32;
    let mut i = 0u32;
    while last_best < 4 {
        // Match JS dagre: i%2==1 → down sweep, i%2==0 → up sweep
        let use_down = i % 2 == 1;
        let bias_right = i % 4 >= 2;

        if use_down {
            for rank in 1..=max {
                sweep_layer(g, rank, bias_right, true);
            }
        } else {
            for rank in (0..max).rev() {
                sweep_layer(g, rank, bias_right, false);
            }
        }

        let layering = util::build_layer_matrix(g);
        let cc = cross_count::cross_count(g, &layering);

        if cc < best_cc {
            last_best = 0;
            best_cc = cc;
            best_orders = g
                .node_ids()
                .map(|nid| (nid, g.node(nid).unwrap().order))
                .collect();
        } else if (cc - best_cc).abs() < f64::EPSILON {
            // Equal cost: accept new ordering (matches JS dagre)
            best_orders = g
                .node_ids()
                .map(|nid| (nid, g.node(nid).unwrap().order))
                .collect();
        }

        i += 1;
        last_best += 1;
    }

    // Restore best ordering
    for (nid, order) in best_orders {
        if let Some(node) = g.node_mut(nid) {
            node.order = order;
        }
    }
}

/// Sort one layer using barycenters from the adjacent layer.
fn sweep_layer(
    g: &mut Graph<NodeLabel, EdgeLabel>,
    rank: i32,
    bias_right: bool,
    use_in_edges: bool,
) {
    // Collect nodes at this rank
    let layer_nodes: Vec<NodeId> = g
        .node_ids()
        .filter(|&nid| g.node(nid).unwrap().rank == rank)
        .collect();

    if layer_nodes.is_empty() {
        return;
    }

    // Try compound-aware sort if there's a layer root
    // For flat graphs, create a virtual root containing all layer nodes
    let root = find_or_create_layer_root(g, &layer_nodes);

    if let Some(root_id) = root {
        let cg = ConstraintGraph::new();
        let result = sort_subgraph::sort_subgraph(g, root_id, &cg, bias_right, use_in_edges);
        for (i, &nid) in result.vs.iter().enumerate() {
            g.node_mut(nid).unwrap().order = i;
        }
    } else {
        // Simple flat sort by barycenter
        let entries = if use_in_edges {
            barycenter::barycenter(g, &layer_nodes)
        } else {
            barycenter::barycenter_out(g, &layer_nodes)
        };

        // Sort: nodes with barycenter by barycenter, others keep position.
        // Use enumeration index (NodeId order, matching JS dagre's insertion order)
        // as a static tie-breaker that stays constant across sweeps.
        let mut indexed: Vec<_> = entries
            .iter()
            .enumerate()
            .map(|(i, e)| (e.v, e.barycenter, i))
            .collect();

        indexed.sort_by(|a, b| match (a.1, b.1) {
            (Some(a_bc), Some(b_bc)) => a_bc
                .partial_cmp(&b_bc)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    if bias_right {
                        b.2.cmp(&a.2)
                    } else {
                        a.2.cmp(&b.2)
                    }
                }),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.2.cmp(&b.2),
        });

        for (order, &(nid, _, _)) in indexed.iter().enumerate() {
            g.node_mut(nid).unwrap().order = order;
        }
    }
}

/// Find a common root node for a layer's nodes.
/// Returns Some(root) if all nodes share a common parent, None for flat graphs.
fn find_or_create_layer_root(
    g: &Graph<NodeLabel, EdgeLabel>,
    layer_nodes: &[NodeId],
) -> Option<NodeId> {
    if layer_nodes.is_empty() {
        return None;
    }

    // Check if there's a common parent
    let first_parent = g.parent(layer_nodes[0]);
    if first_parent.is_some() && layer_nodes.iter().all(|&n| g.parent(n) == first_parent) {
        return first_parent;
    }

    // No common parent — use flat sort (return None)
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_chain_no_crossings() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        let c = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, b, EdgeLabel::default());
        g.add_edge(b, c, EdgeLabel::default());
        g.node_mut(a).unwrap().rank = 0;
        g.node_mut(b).unwrap().rank = 1;
        g.node_mut(c).unwrap().rank = 2;

        order(&mut g);

        let layering = util::build_layer_matrix(&g);
        let cc = cross_count::cross_count(&g, &layering);
        assert_eq!(cc, 0.0);
    }

    #[test]
    fn reduces_crossings() {
        // Create a graph where initial ordering has crossings
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        let c = g.add_node(NodeLabel::new(10.0, 10.0));
        let d = g.add_node(NodeLabel::new(10.0, 10.0));
        // a->d, b->c creates a crossing if a,b are ordered before c,d
        g.add_edge(a, d, EdgeLabel::default());
        g.add_edge(b, c, EdgeLabel::default());
        g.node_mut(a).unwrap().rank = 0;
        g.node_mut(b).unwrap().rank = 0;
        g.node_mut(c).unwrap().rank = 1;
        g.node_mut(d).unwrap().rank = 1;
        // Force bad initial order
        g.node_mut(a).unwrap().order = 0;
        g.node_mut(b).unwrap().order = 1;
        g.node_mut(c).unwrap().order = 0;
        g.node_mut(d).unwrap().order = 1;

        // Verify initial crossing
        let initial_layering = util::build_layer_matrix(&g);
        let initial_cc = cross_count::cross_count(&g, &initial_layering);
        assert_eq!(initial_cc, 1.0);

        order(&mut g);

        let layering = util::build_layer_matrix(&g);
        let cc = cross_count::cross_count(&g, &layering);
        assert_eq!(cc, 0.0);
    }

    #[test]
    fn all_nodes_get_order() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        let c = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, b, EdgeLabel::default());
        g.add_edge(a, c, EdgeLabel::default());
        g.node_mut(a).unwrap().rank = 0;
        g.node_mut(b).unwrap().rank = 1;
        g.node_mut(c).unwrap().rank = 1;

        order(&mut g);

        // b and c should have different orders
        assert_ne!(g.node(b).unwrap().order, g.node(c).unwrap().order);
    }

    #[test]
    fn single_node_no_panic() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        g.node_mut(a).unwrap().rank = 0;

        order(&mut g);
        assert_eq!(g.node(a).unwrap().order, 0);
    }
}
