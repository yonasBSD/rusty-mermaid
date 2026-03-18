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
///
/// For compound graphs, a constraint graph (CG) persists across ranks within
/// each sweep. After sorting each rank, `add_subgraph_constraints` records
/// which compound siblings appeared left-of-right, so subsequent ranks
/// respect the same relative order — preventing subgraph bounding boxes
/// from overlapping.
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

        // One constraint graph per sweep, persisted across all ranks.
        let mut cg = ConstraintGraph::new();

        if use_down {
            for rank in 1..=max {
                sweep_layer(g, rank, bias_right, true, &mut cg);
            }
        } else {
            for rank in (0..max).rev() {
                sweep_layer(g, rank, bias_right, false, &mut cg);
            }
        }

        let layering = util::build_layer_matrix(g);
        let cc = cross_count::cross_count(g, &layering);

        if cc < best_cc {
            last_best = 0;
            best_cc = cc;
            best_orders = g
                .node_ids()
                .filter_map(|nid| Some((nid, g.node(nid)?.order)))
                .collect();
        } else if (cc - best_cc).abs() < f64::EPSILON {
            // Equal cost: accept new ordering (matches JS dagre)
            best_orders = g
                .node_ids()
                .filter_map(|nid| Some((nid, g.node(nid)?.order)))
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
///
/// Mirrors JS dagre's `buildLayerGraph` + `sweepLayerGraphs`: always creates
/// a synthetic root node, parents all top-level entities at this rank to it,
/// then sorts via `sort_subgraph`. This ensures uniform constraint generation
/// across all ranks regardless of compound hierarchy shape.
fn sweep_layer(
    g: &mut Graph<NodeLabel, EdgeLabel>,
    rank: i32,
    bias_right: bool,
    use_in_edges: bool,
    cg: &mut ConstraintGraph,
) {
    // Collect nodes that participate in this rank:
    // - leaf nodes where rank == this rank
    // - compound nodes spanning this rank (minRank <= rank <= maxRank)
    let layer_nodes: Vec<NodeId> = g
        .node_ids()
        .filter(|&nid| {
            g.node(nid).is_some_and(|n| {
                n.rank == rank
                    || (n.min_rank.is_some_and(|min| min <= rank)
                        && n.max_rank.is_some_and(|max| max >= rank))
            })
        })
        .collect();

    if layer_nodes.is_empty() {
        return;
    }

    // Create synthetic root (matching JS dagre's buildLayerGraph)
    let synth_root = g.add_node(NodeLabel::new(0.0, 0.0));

    // Collect all unique top-level ancestors BEFORE reparenting any,
    // to avoid top_ancestor traversing into the synthetic root.
    let mut reparented: Vec<NodeId> = Vec::new();
    for &nid in &layer_nodes {
        let top = top_ancestor(g, nid);
        if !reparented.contains(&top) {
            reparented.push(top);
        }
    }
    for &top in &reparented {
        g.set_parent(top, synth_root);
    }

    let result =
        sort_subgraph::sort_subgraph(g, synth_root, cg, bias_right, use_in_edges, rank);
    for (i, &nid) in result.vs.iter().enumerate() {
        let Some(node) = g.node_mut(nid) else { continue };
        node.order = i;
    }
    add_subgraph_constraints(g, cg, &result.vs);

    // Clean up: unparent and remove synthetic root
    for &top in &reparented {
        g.remove_parent(top);
    }
    g.remove_node(synth_root);
}

/// Walk up the compound hierarchy to find the top-level ancestor.
fn top_ancestor(g: &Graph<NodeLabel, EdgeLabel>, mut nid: NodeId) -> NodeId {
    while let Some(parent) = g.parent(nid) {
        nid = parent;
    }
    nid
}

/// Add constraints between peer compound nodes to prevent subgraph interleaving.
///
/// Port of dagre's `addSubgraphConstraints`. For each node in the ordered
/// sequence, walks up the compound hierarchy. When it encounters a compound
/// sibling that differs from the previously seen sibling at the same level,
/// it adds a constraint edge (prev → current) to the CG. This forces
/// consistent left-to-right ordering of peer subgraphs across all ranks
/// within a sweep.
fn add_subgraph_constraints(
    g: &Graph<NodeLabel, EdgeLabel>,
    cg: &mut ConstraintGraph,
    vs: &[NodeId],
) {
    use std::collections::HashMap;

    let mut prev: HashMap<NodeId, NodeId> = HashMap::new(); // parent → last child seen
    let mut root_prev: Option<NodeId> = None; // last top-level child seen

    for &v in vs {
        let mut child = match g.parent(v) {
            Some(p) => p,
            None => continue, // unparented nodes don't generate constraints
        };

        loop {
            let parent = g.parent(child);

            let prev_child = if let Some(parent_id) = parent {
                let pc = prev.get(&parent_id).copied();
                prev.insert(parent_id, child);
                pc
            } else {
                let pc = root_prev;
                root_prev = Some(child);
                pc
            };

            if let Some(pc) = prev_child {
                if pc != child {
                    cg.add_edge(pc, child);
                    break; // stop after first constraint per node (matches JS `return`)
                }
            }

            match parent {
                Some(p) => child = p,
                None => break,
            }
        }
    }
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
