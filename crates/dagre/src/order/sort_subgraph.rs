use std::collections::HashMap;

use rusty_mermaid_graph::{Graph, NodeId};

use crate::labels::{EdgeLabel, NodeLabel};
use crate::order::barycenter::{self, BaryEntry};
use crate::order::resolve_conflicts::{self, ConstraintGraph, ResolvedEntry};

/// Result of sorting a subgraph.
#[derive(Debug)]
pub(crate) struct SortResult {
    pub(crate) vs: Vec<NodeId>,
    pub(crate) barycenter: Option<f64>,
    pub(crate) weight: f64,
}

/// Sort nodes within a subgraph (compound node or layer root) using barycenters.
///
/// For compound nodes, recursively sorts children, then merges results
/// respecting border constraints. The `rank` parameter specifies which layer
/// is being sorted — only children at this rank are included, and border nodes
/// are looked up for this specific rank.
pub(crate) fn sort_subgraph(
    g: &Graph<NodeLabel, EdgeLabel>,
    v: NodeId,
    cg: &ConstraintGraph,
    bias_right: bool,
    use_in_edges: bool,
    rank: i32,
) -> SortResult {
    // Filter children to only those at the current rank (or compound nodes spanning it)
    let children: Vec<_> = g
        .children(v)
        .filter(|&c| {
            let cn = g.node(c).unwrap();
            // Include if at this rank, or if it's a compound node spanning this rank
            cn.rank == rank
                || (cn.min_rank.is_some_and(|min| min <= rank)
                    && cn.max_rank.is_some_and(|max| max >= rank))
        })
        .collect();
    let node = g.node(v);
    let bl = node.and_then(|n| n.border_left.get(&rank).copied());
    let br = node.and_then(|n| n.border_right.get(&rank).copied());

    // Movable = children minus border nodes
    let movable: Vec<_> = if bl.is_some() {
        children
            .iter()
            .copied()
            .filter(|&w| Some(w) != bl && Some(w) != br)
            .collect()
    } else {
        children
    };

    // Compute barycenters
    let mut barycenters: Vec<BaryEntry> = if use_in_edges {
        barycenter::barycenter(g, &movable)
    } else {
        barycenter::barycenter_out(g, &movable)
    };

    // Recursively sort compound children and merge their barycenters.
    // Store sub-results to expand compound nodes in the final ordering.
    let mut sub_results: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for entry in &mut barycenters {
        if g.children(entry.v).next().is_some() {
            let sub = sort_subgraph(g, entry.v, cg, bias_right, use_in_edges, rank);
            if !sub.vs.is_empty() {
                sub_results.insert(entry.v, sub.vs);
            }
            if let Some(sub_bc) = sub.barycenter {
                merge_barycenters(entry, sub_bc, sub.weight);
            }
        }
    }

    // Resolve constraint conflicts
    let entries = resolve_conflicts::resolve_conflicts(&barycenters, cg);

    // Sort: partition into sortable (has barycenter) and unsortable
    let result = sort_entries(entries, bias_right);

    // Expand compound nodes: replace each compound node ID with its
    // recursively-sorted children so all real nodes appear in the ordering.
    let mut vs: Vec<NodeId> = Vec::new();
    for nid in result.vs {
        if let Some(expanded) = sub_results.get(&nid) {
            vs.extend(expanded);
        } else {
            vs.push(nid);
        }
    }
    if let (Some(left), Some(right)) = (bl, br) {
        let mut final_vs = vec![left];
        final_vs.extend(vs);
        final_vs.push(right);
        vs = final_vs;
    }

    SortResult {
        vs,
        barycenter: result.barycenter,
        weight: result.weight,
    }
}

fn merge_barycenters(entry: &mut BaryEntry, other_bc: f64, other_weight: f64) {
    if let Some(bc) = entry.barycenter {
        let total = entry.weight + other_weight;
        entry.barycenter = Some((bc * entry.weight + other_bc * other_weight) / total);
        entry.weight = total;
    } else {
        entry.barycenter = Some(other_bc);
        entry.weight = other_weight;
    }
}

struct SortedResult {
    vs: Vec<NodeId>,
    barycenter: Option<f64>,
    weight: f64,
}

/// Sort resolved entries: sortable (has barycenter) by barycenter,
/// unsortable interleaved by original index.
fn sort_entries(entries: Vec<ResolvedEntry>, bias_right: bool) -> SortedResult {
    let mut sortable: Vec<_> = entries.iter().filter(|e| e.barycenter.is_some()).collect();
    let mut unsortable: Vec<_> = entries.iter().filter(|e| e.barycenter.is_none()).collect();

    // Sort sortable by barycenter (tie-break by index, respecting bias)
    sortable.sort_by(|a, b| {
        let a_bc = a.barycenter.unwrap();
        let b_bc = b.barycenter.unwrap();
        a_bc.partial_cmp(&b_bc)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                if bias_right {
                    b.idx.cmp(&a.idx)
                } else {
                    a.idx.cmp(&b.idx)
                }
            })
    });

    // Sort unsortable by index descending (will be consumed from the end)
    unsortable.sort_by(|a, b| b.idx.cmp(&a.idx));

    let mut vs = Vec::new();
    let mut sum = 0.0;
    let mut weight = 0.0;
    let mut vs_index = 0;

    // Consume unsortable entries up to current index
    consume_unsortable(&mut vs, &mut unsortable, &mut vs_index);

    for entry in &sortable {
        vs_index += entry.vs.len();
        vs.extend(&entry.vs);
        sum += entry.barycenter.unwrap() * entry.weight;
        weight += entry.weight;
        consume_unsortable(&mut vs, &mut unsortable, &mut vs_index);
    }

    SortedResult {
        vs,
        barycenter: if weight > 0.0 {
            Some(sum / weight)
        } else {
            None
        },
        weight,
    }
}

fn consume_unsortable(
    vs: &mut Vec<NodeId>,
    unsortable: &mut Vec<&ResolvedEntry>,
    index: &mut usize,
) {
    while let Some(last) = unsortable.last() {
        if last.idx <= *index {
            vs.extend(&last.vs);
            *index += 1;
            unsortable.pop();
        } else {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_by_barycenter() {
        let mut g = Graph::new();
        let root = g.add_node(NodeLabel::new(0.0, 0.0));
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        let c = g.add_node(NodeLabel::new(10.0, 10.0));
        // Sources at rank 0 with known orders
        let s1 = g.add_node(NodeLabel::new(10.0, 10.0));
        let s2 = g.add_node(NodeLabel::new(10.0, 10.0));
        g.set_parent(a, root);
        g.set_parent(b, root);
        g.set_parent(c, root);
        g.node_mut(s1).unwrap().order = 0;
        g.node_mut(s2).unwrap().order = 2;
        // s1->c, s2->a means c should come before a
        g.add_edge(s1, c, EdgeLabel::default());
        g.add_edge(s2, a, EdgeLabel::default());
        // b has no in-edges

        let cg = ConstraintGraph::new();
        // All children are at rank 0 (implicitly), but for the test we match the sources' rank
        g.node_mut(a).unwrap().rank = 1;
        g.node_mut(b).unwrap().rank = 1;
        g.node_mut(c).unwrap().rank = 1;
        let result = sort_subgraph(&g, root, &cg, false, true, 1);
        // c (bc=0) should come before a (bc=2), b (no bc) fills gaps
        let c_pos = result.vs.iter().position(|&v| v == c).unwrap();
        let a_pos = result.vs.iter().position(|&v| v == a).unwrap();
        assert!(c_pos < a_pos);
    }
}
