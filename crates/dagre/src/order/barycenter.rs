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
pub(crate) fn barycenter(
    graph: &Graph<NodeLabel, EdgeLabel>,
    movable: &[NodeId],
) -> Vec<BaryEntry> {
    compute_barycenters(graph, movable, true)
}

/// Compute barycenters based on out-edges (for upward sweeps).
pub(crate) fn barycenter_out(
    graph: &Graph<NodeLabel, EdgeLabel>,
    movable: &[NodeId],
) -> Vec<BaryEntry> {
    compute_barycenters(graph, movable, false)
}

fn compute_barycenters(
    graph: &Graph<NodeLabel, EdgeLabel>,
    movable: &[NodeId],
    use_in_edges: bool,
) -> Vec<BaryEntry> {
    movable
        .iter()
        .map(|&v| {
            let edges: Vec<_> = if use_in_edges {
                graph.in_edges(v).collect()
            } else {
                graph.out_edges(v).collect()
            };
            if edges.is_empty() {
                return BaryEntry {
                    v,
                    barycenter: None,
                    weight: 0.0,
                };
            }

            let mut sum = 0.0;
            let mut weight = 0.0;
            for eid in edges {
                let Some((src, dst)) = graph.edge_endpoints(eid) else {
                    continue;
                };
                let neighbor = if use_in_edges { src } else { dst };
                let edge_weight = graph.edge(eid).map_or(1.0, |l| l.weight);
                let order = graph.node(neighbor).map_or(0, |n| n.order) as f64;
                sum += edge_weight * order;
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
