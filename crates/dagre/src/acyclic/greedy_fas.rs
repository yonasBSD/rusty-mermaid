use std::collections::{BTreeMap, VecDeque};

use rusty_mermaid_graph::{EdgeId, Graph, NodeId};

use crate::labels::{EdgeLabel, NodeLabel};

/// Find a feedback arc set using the greedy heuristic (Eades, Lin, Smyth).
/// Produces a small (not necessarily minimum) FAS by greedily building a
/// vertex ordering that minimizes backward edges.
pub(crate) fn greedy_fas(g: &Graph<NodeLabel, EdgeLabel>) -> Vec<EdgeId> {
    // Collect self-loops first (always in FAS)
    let mut fas: Vec<EdgeId> = Vec::new();
    for eid in g.edge_ids() {
        if let Some((src, dst)) = g.edge_endpoints(eid)
            && src == dst
        {
            fas.push(eid);
        }
    }

    if g.node_count() <= 1 {
        return fas;
    }

    // Build a simplified working graph: merge parallel edges by summing weights.
    let mut in_weight: BTreeMap<NodeId, f64> = BTreeMap::new();
    let mut out_weight: BTreeMap<NodeId, f64> = BTreeMap::new();
    let mut adj_out: BTreeMap<NodeId, BTreeMap<NodeId, f64>> = BTreeMap::new();
    let mut adj_in: BTreeMap<NodeId, BTreeMap<NodeId, f64>> = BTreeMap::new();
    let mut alive: BTreeMap<NodeId, bool> = BTreeMap::new();

    for nid in g.node_ids() {
        in_weight.insert(nid, 0.0);
        out_weight.insert(nid, 0.0);
        adj_out.entry(nid).or_default();
        adj_in.entry(nid).or_default();
        alive.insert(nid, true);
    }

    for eid in g.edge_ids() {
        if let Some((src, dst)) = g.edge_endpoints(eid) {
            if src == dst {
                continue; // skip self-loops
            }
            let w = g.edge(eid).map_or(1.0, |l| l.weight);
            *adj_out.entry(src).or_default().entry(dst).or_insert(0.0) += w;
            *adj_in.entry(dst).or_default().entry(src).or_insert(0.0) += w;
            *out_weight.entry(src).or_insert(0.0) += w;
            *in_weight.entry(dst).or_insert(0.0) += w;
        }
    }

    // Build the vertex ordering greedily.
    let mut seq_left: Vec<NodeId> = Vec::new();
    let mut seq_right: VecDeque<NodeId> = VecDeque::new();

    let mut remaining = g.node_count();

    while remaining > 0 {
        // Remove sinks (no outgoing edges)
        loop {
            let sink = alive
                .iter()
                .find(|(nid, is_alive)| {
                    **is_alive && out_weight.get(nid).copied().unwrap_or(0.0) <= 0.0
                })
                .map(|(&nid, _)| nid);
            let Some(v) = sink else { break };
            remove_node(
                v,
                &mut alive,
                &mut in_weight,
                &mut out_weight,
                &mut adj_out,
                &mut adj_in,
            );
            seq_right.push_front(v);
            remaining -= 1;
        }

        // Remove sources (no incoming edges)
        loop {
            let source = alive
                .iter()
                .find(|(nid, is_alive)| {
                    **is_alive && in_weight.get(nid).copied().unwrap_or(0.0) <= 0.0
                })
                .map(|(&nid, _)| nid);
            let Some(v) = source else { break };
            remove_node(
                v,
                &mut alive,
                &mut in_weight,
                &mut out_weight,
                &mut adj_out,
                &mut adj_in,
            );
            seq_left.push(v);
            remaining -= 1;
        }

        if remaining == 0 {
            break;
        }

        // Pick node with max (out_weight - in_weight)
        let best = alive
            .iter()
            .filter(|(_, is_alive)| **is_alive)
            .max_by(|(a, _), (b, _)| {
                let da = out_weight.get(a).unwrap_or(&0.0) - in_weight.get(a).unwrap_or(&0.0);
                let db = out_weight.get(b).unwrap_or(&0.0) - in_weight.get(b).unwrap_or(&0.0);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(&nid, _)| nid);

        if let Some(v) = best {
            remove_node(
                v,
                &mut alive,
                &mut in_weight,
                &mut out_weight,
                &mut adj_out,
                &mut adj_in,
            );
            seq_left.push(v);
            remaining -= 1;
        }
    }

    // Build final ordering: seq_left ++ seq_right
    seq_left.extend(seq_right);
    let position: BTreeMap<NodeId, usize> = seq_left
        .iter()
        .enumerate()
        .map(|(i, &nid)| (nid, i))
        .collect();

    // FAS += edges that go backward in the ordering (self-loops already collected)
    for eid in g.edge_ids() {
        if let Some((src, dst)) = g.edge_endpoints(eid) {
            if src == dst {
                continue; // already in fas
            }
            let src_pos = position.get(&src).copied().unwrap_or(0);
            let dst_pos = position.get(&dst).copied().unwrap_or(0);
            if src_pos >= dst_pos {
                fas.push(eid);
            }
        }
    }
    fas
}

fn remove_node(
    v: NodeId,
    alive: &mut BTreeMap<NodeId, bool>,
    in_weight: &mut BTreeMap<NodeId, f64>,
    out_weight: &mut BTreeMap<NodeId, f64>,
    adj_out: &mut BTreeMap<NodeId, BTreeMap<NodeId, f64>>,
    adj_in: &mut BTreeMap<NodeId, BTreeMap<NodeId, f64>>,
) {
    alive.insert(v, false);

    // Update neighbors' weights
    if let Some(outs) = adj_out.get(&v).cloned() {
        for (&dst, &w) in &outs {
            if alive.get(&dst).copied().unwrap_or(false) {
                *in_weight.entry(dst).or_insert(0.0) -= w;
                if let Some(ins) = adj_in.get_mut(&dst) {
                    ins.remove(&v);
                }
            }
        }
    }

    if let Some(ins) = adj_in.get(&v).cloned() {
        for (&src, &w) in &ins {
            if alive.get(&src).copied().unwrap_or(false) {
                *out_weight.entry(src).or_insert(0.0) -= w;
                if let Some(outs) = adj_out.get_mut(&src) {
                    outs.remove(&v);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dag_has_empty_fas() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, b, EdgeLabel::default());
        assert!(greedy_fas(&g).is_empty());
    }

    #[test]
    fn single_node() {
        let mut g: Graph<NodeLabel, EdgeLabel> = Graph::new();
        g.add_node(NodeLabel::new(10.0, 10.0));
        assert!(greedy_fas(&g).is_empty());
    }

    #[test]
    fn cycle_finds_fas() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        let c = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, b, EdgeLabel::default());
        g.add_edge(b, c, EdgeLabel::default());
        g.add_edge(c, a, EdgeLabel::default());
        let fas = greedy_fas(&g);
        // Should find at least one edge to break the cycle
        assert!(!fas.is_empty());
        assert!(fas.len() <= 3); // at most all edges
    }

    #[test]
    fn self_loop_in_fas() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, a, EdgeLabel::default());
        let fas = greedy_fas(&g);
        assert_eq!(fas.len(), 1);
    }
}
