use std::collections::HashMap;

use rusty_mermaid_graph::NodeId;

use crate::order::barycenter::BaryEntry;

/// A resolved entry: possibly merged from multiple conflicting nodes.
#[derive(Debug, Clone)]
pub(crate) struct ResolvedEntry {
    pub(crate) vs: Vec<NodeId>,
    pub(crate) idx: usize,
    pub(crate) barycenter: Option<f64>,
    pub(crate) weight: f64,
}

/// Constraint graph: directed edges mean "left must come before right".
#[derive(Debug, Default)]
pub(crate) struct ConstraintGraph {
    edges: Vec<(NodeId, NodeId)>,
}

impl ConstraintGraph {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn add_edge(&mut self, from: NodeId, to: NodeId) {
        self.edges.push((from, to));
    }

    fn out_edges(&self, v: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        self.edges.iter().filter(move |(f, _)| *f == v).map(|(_, t)| *t)
    }

    fn in_edges(&self, v: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        self.edges.iter().filter(move |(_, t)| *t == v).map(|(f, _)| *f)
    }
}

/// Resolve conflicts between barycenter entries and constraint graph.
///
/// If a constraint requires A before B, but A's barycenter > B's barycenter,
/// merge them into a single entry with combined barycenter/weight.
///
/// Based on Forster, "A Fast and Simple Heuristic for Constrained Two-Level
/// Crossing Reduction."
pub(crate) fn resolve_conflicts(
    entries: &[BaryEntry],
    cg: &ConstraintGraph,
) -> Vec<ResolvedEntry> {
    // Build per-node entries
    let mut mapped: HashMap<NodeId, MappedEntry> = HashMap::new();
    for (i, entry) in entries.iter().enumerate() {
        mapped.insert(
            entry.v,
            MappedEntry {
                indegree: 0,
                incoming: Vec::new(),
                outgoing: Vec::new(),
                vs: vec![entry.v],
                idx: i,
                barycenter: entry.barycenter,
                weight: entry.weight,
                merged: false,
            },
        );
    }

    // Wire up constraint edges
    for &(from, to) in &cg.edges {
        if mapped.contains_key(&from) && mapped.contains_key(&to) {
            mapped.get_mut(&to).unwrap().indegree += 1;
            // Store constraint connections by node id
            mapped.get_mut(&from).unwrap().outgoing.push(to);
            mapped.get_mut(&to).unwrap().incoming.push(from);
        }
    }

    // Topological processing of constraint sources
    let mut source_set: Vec<NodeId> = mapped
        .iter()
        .filter(|(_, e)| e.indegree == 0)
        .map(|(&v, _)| v)
        .collect();

    let mut result_ids = Vec::new();

    while let Some(v_id) = source_set.pop() {
        result_ids.push(v_id);

        // Handle incoming: merge if constraint conflicts with barycenter
        let incoming: Vec<NodeId> = mapped[&v_id].incoming.clone();
        for u_id in incoming.into_iter().rev() {
            if mapped[&u_id].merged {
                continue;
            }
            let u_bc = mapped[&u_id].barycenter;
            let v_bc = mapped[&v_id].barycenter;
            if u_bc.is_none() || v_bc.is_none() || u_bc >= v_bc {
                merge_entries(&mut mapped, v_id, u_id);
            }
        }

        // Handle outgoing: decrement indegree, add to source set if zero
        let outgoing: Vec<NodeId> = mapped[&v_id].outgoing.clone();
        for w_id in outgoing {
            let w = mapped.get_mut(&w_id).unwrap();
            w.indegree -= 1;
            if w.indegree == 0 {
                source_set.push(w_id);
            }
        }
    }

    result_ids
        .into_iter()
        .filter(|v| !mapped[v].merged)
        .map(|v| {
            let e = &mapped[&v];
            ResolvedEntry {
                vs: e.vs.clone(),
                idx: e.idx,
                barycenter: e.barycenter,
                weight: e.weight,
            }
        })
        .collect()
}

fn merge_entries(mapped: &mut HashMap<NodeId, MappedEntry>, target_id: NodeId, source_id: NodeId) {
    let source = mapped[&source_id].clone();

    let mut sum = 0.0;
    let mut weight = 0.0;

    let target = mapped.get_mut(&target_id).unwrap();
    if target.weight > 0.0
        && let Some(bc) = target.barycenter
    {
        sum += bc * target.weight;
        weight += target.weight;
    }
    if source.weight > 0.0
        && let Some(bc) = source.barycenter
    {
        sum += bc * source.weight;
        weight += source.weight;
    }

    // Prepend source's vs to target's
    let mut new_vs = source.vs;
    new_vs.extend(target.vs.iter());
    target.vs = new_vs;

    if weight > 0.0 {
        target.barycenter = Some(sum / weight);
    }
    target.weight = weight;
    target.idx = target.idx.min(source.idx);

    mapped.get_mut(&source_id).unwrap().merged = true;
}

#[derive(Debug, Clone)]
struct MappedEntry {
    indegree: usize,
    incoming: Vec<NodeId>,
    outgoing: Vec<NodeId>,
    vs: Vec<NodeId>,
    idx: usize,
    barycenter: Option<f64>,
    weight: f64,
    merged: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(v: NodeId, bc: Option<f64>, w: f64) -> BaryEntry {
        BaryEntry {
            v,
            barycenter: bc,
            weight: w,
        }
    }

    #[test]
    fn no_conflicts() {
        let a = NodeId::from(0);
        let b = NodeId::from(1);
        let entries = vec![entry(a, Some(1.0), 1.0), entry(b, Some(2.0), 1.0)];
        let cg = ConstraintGraph::new();

        let result = resolve_conflicts(&entries, &cg);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn merge_on_conflict() {
        let a = NodeId::from(0);
        let b = NodeId::from(1);
        // Constraint: a before b, but a has higher barycenter
        let entries = vec![entry(a, Some(3.0), 1.0), entry(b, Some(1.0), 1.0)];
        let mut cg = ConstraintGraph::new();
        cg.add_edge(a, b);

        let result = resolve_conflicts(&entries, &cg);
        // Should be merged into one entry
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].vs.len(), 2);
    }

    #[test]
    fn no_merge_when_compatible() {
        let a = NodeId::from(0);
        let b = NodeId::from(1);
        // Constraint: a before b, and a.bc < b.bc — no conflict
        let entries = vec![entry(a, Some(1.0), 1.0), entry(b, Some(3.0), 1.0)];
        let mut cg = ConstraintGraph::new();
        cg.add_edge(a, b);

        let result = resolve_conflicts(&entries, &cg);
        assert_eq!(result.len(), 2);
    }
}
