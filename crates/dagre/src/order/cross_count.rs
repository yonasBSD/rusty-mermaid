use std::collections::HashMap;

use rusty_mermaid_graph::{Graph, NodeId};

use crate::labels::{EdgeLabel, NodeLabel};

/// Count weighted edge crossings across all adjacent layer pairs.
pub fn cross_count(g: &Graph<NodeLabel, EdgeLabel>, layering: &[Vec<NodeId>]) -> f64 {
    let mut cc = 0.0;
    for i in 1..layering.len() {
        cc += two_layer_cross_count(g, &layering[i - 1], &layering[i]);
    }
    cc
}

/// Count weighted crossings between two adjacent layers using the
/// Barth accumulator tree algorithm.
///
/// Reference: Barth, Jünger, Mutzel, "Bilayer Cross Counting" (2004).
fn two_layer_cross_count(
    g: &Graph<NodeLabel, EdgeLabel>,
    north: &[NodeId],
    south: &[NodeId],
) -> f64 {
    if south.is_empty() {
        return 0.0;
    }

    // Map south layer nodes to their position
    let south_pos: HashMap<NodeId, usize> = south.iter().enumerate().map(|(i, &v)| (v, i)).collect();

    // Collect south-layer positions for edges from each north node, sorted
    let mut south_entries = Vec::new();
    for &v in north {
        let mut entries: Vec<(usize, f64)> = Vec::new();
        for eid in g.out_edges(v) {
            if let Some((_, dst)) = g.edge_endpoints(eid)
                && let Some(&pos) = south_pos.get(&dst)
            {
                let weight = g.edge(eid).map_or(1.0, |l| l.weight);
                entries.push((pos, weight));
            }
        }
        entries.sort_by_key(|&(pos, _)| pos);
        south_entries.extend(entries);
    }

    // Build accumulator tree
    let mut first_index = 1;
    while first_index < south.len() {
        first_index <<= 1;
    }
    let tree_size = 2 * first_index - 1;
    first_index -= 1;
    let mut tree = vec![0.0; tree_size];

    // Count crossings
    let mut cc = 0.0;
    for (pos, weight) in south_entries {
        let mut index = pos + first_index;
        tree[index] += weight;
        let mut weight_sum = 0.0;
        while index > 0 {
            if index % 2 != 0 {
                weight_sum += tree[index + 1];
            }
            index = (index - 1) / 2;
            tree[index] += weight;
        }
        cc += weight * weight_sum;
    }

    cc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_crossings() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        let c = g.add_node(NodeLabel::new(10.0, 10.0));
        let d = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, c, EdgeLabel::default());
        g.add_edge(b, d, EdgeLabel::default());

        let layering = vec![vec![a, b], vec![c, d]];
        assert!((cross_count(&g, &layering) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn one_crossing() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        let c = g.add_node(NodeLabel::new(10.0, 10.0));
        let d = g.add_node(NodeLabel::new(10.0, 10.0));
        // a->d and b->c cross
        g.add_edge(a, d, EdgeLabel::default());
        g.add_edge(b, c, EdgeLabel::default());

        let layering = vec![vec![a, b], vec![c, d]];
        assert!((cross_count(&g, &layering) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn weighted_crossings() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        let c = g.add_node(NodeLabel::new(10.0, 10.0));
        let d = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, d, EdgeLabel::new().with_weight(2.0));
        g.add_edge(b, c, EdgeLabel::new().with_weight(3.0));

        let layering = vec![vec![a, b], vec![c, d]];
        // crossing weight = 2.0 * 3.0 = 6.0
        assert!((cross_count(&g, &layering) - 6.0).abs() < f64::EPSILON);
    }

    #[test]
    fn empty_layer_no_panic() {
        let g: Graph<NodeLabel, EdgeLabel> = Graph::new();
        let layering: Vec<Vec<NodeId>> = vec![vec![], vec![]];
        assert!((cross_count(&g, &layering) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn three_layers() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        let c = g.add_node(NodeLabel::new(10.0, 10.0));
        let d = g.add_node(NodeLabel::new(10.0, 10.0));
        let e = g.add_node(NodeLabel::new(10.0, 10.0));
        // Layer 0: [a, b], Layer 1: [c, d], Layer 2: [e]
        // a->d, b->c (1 crossing in layers 0-1)
        // c->e, d->e (0 crossings in layers 1-2)
        g.add_edge(a, d, EdgeLabel::default());
        g.add_edge(b, c, EdgeLabel::default());
        g.add_edge(c, e, EdgeLabel::default());
        g.add_edge(d, e, EdgeLabel::default());

        let layering = vec![vec![a, b], vec![c, d], vec![e]];
        assert!((cross_count(&g, &layering) - 1.0).abs() < f64::EPSILON);
    }
}
