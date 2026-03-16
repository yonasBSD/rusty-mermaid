use rusty_mermaid_graph::{Graph, NodeId};

use crate::labels::{EdgeLabel, NodeLabel};

/// Barycenter result for a single node.
#[derive(Debug, Clone)]
pub(crate) struct BaryEntry {
    pub(crate) v: NodeId,
    pub(crate) barycenter: Option<f64>,
    pub(crate) weight: f64,
}

/// Compute barycenters for nodes in `movable` based on their in-edges.
///
/// The barycenter of a node is the weighted average order of its predecessors.
/// Nodes with no in-edges get `barycenter: None`.
pub(crate) fn barycenter(
    g: &Graph<NodeLabel, EdgeLabel>,
    movable: &[NodeId],
) -> Vec<BaryEntry> {
    movable
        .iter()
        .map(|&v| {
            let in_edges: Vec<_> = g.in_edges(v).collect();
            if in_edges.is_empty() {
                return BaryEntry {
                    v,
                    barycenter: None,
                    weight: 0.0,
                };
            }

            let mut sum = 0.0;
            let mut weight = 0.0;
            for eid in in_edges {
                let (src, _) = g.edge_endpoints(eid).unwrap();
                let edge_weight = g.edge(eid).map_or(1.0, |l| l.weight);
                let src_order = g.node(src).unwrap().order as f64;
                sum += edge_weight * src_order;
                weight += edge_weight;
            }

            BaryEntry {
                v,
                barycenter: Some(sum / weight),
                weight,
            }
        })
        .collect()
}

/// Compute barycenters based on out-edges (for upward sweeps).
pub(crate) fn barycenter_out(
    g: &Graph<NodeLabel, EdgeLabel>,
    movable: &[NodeId],
) -> Vec<BaryEntry> {
    movable
        .iter()
        .map(|&v| {
            let out_edges: Vec<_> = g.out_edges(v).collect();
            if out_edges.is_empty() {
                return BaryEntry {
                    v,
                    barycenter: None,
                    weight: 0.0,
                };
            }

            let mut sum = 0.0;
            let mut weight = 0.0;
            for eid in out_edges {
                let (_, dst) = g.edge_endpoints(eid).unwrap();
                let edge_weight = g.edge(eid).map_or(1.0, |l| l.weight);
                let dst_order = g.node(dst).unwrap().order as f64;
                sum += edge_weight * dst_order;
                weight += edge_weight;
            }

            BaryEntry {
                v,
                barycenter: Some(sum / weight),
                weight,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_in_edges_gives_none() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        g.node_mut(a).unwrap().order = 0;

        let result = barycenter(&g, &[a]);
        assert_eq!(result.len(), 1);
        assert!(result[0].barycenter.is_none());
    }

    #[test]
    fn single_predecessor() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, b, EdgeLabel::default());
        g.node_mut(a).unwrap().order = 3;

        let result = barycenter(&g, &[b]);
        assert_eq!(result.len(), 1);
        assert!((result[0].barycenter.unwrap() - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn weighted_average_of_predecessors() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        let c = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, c, EdgeLabel::new().with_weight(1.0));
        g.add_edge(b, c, EdgeLabel::new().with_weight(3.0));
        g.node_mut(a).unwrap().order = 0;
        g.node_mut(b).unwrap().order = 4;

        let result = barycenter(&g, &[c]);
        // (1*0 + 3*4) / (1+3) = 12/4 = 3.0
        assert!((result[0].barycenter.unwrap() - 3.0).abs() < f64::EPSILON);
        assert!((result[0].weight - 4.0).abs() < f64::EPSILON);
    }
}
