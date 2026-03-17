pub(crate) mod bk;

use rusty_mermaid_graph::Graph;

use crate::config::{DagreConfig, RankAlign};
use crate::labels::{EdgeLabel, NodeLabel};
use crate::util;

/// Assign x and y coordinates to all nodes.
pub(crate) fn position(g: &mut Graph<NodeLabel, EdgeLabel>, config: &DagreConfig) {
    position_y(g, config);
    let xs = bk::position_x(g, config);
    for (nid, x) in xs {
        g.node_mut(nid).unwrap().x = x;
    }
}

/// Assign y coordinates based on rank layers.
///
/// Each layer's y is determined by cumulative max-height + ranksep.
/// Within a layer, nodes are positioned according to rankalign (top/center/bottom).
fn position_y(g: &mut Graph<NodeLabel, EdgeLabel>, config: &DagreConfig) {
    let layering = util::build_layer_matrix_leaves(g);
    let mut prev_y = 0.0;

    for layer in &layering {
        if layer.is_empty() {
            prev_y += config.ranksep;
            continue;
        }

        let max_height = layer
            .iter()
            .map(|&v| g.node(v).unwrap().height)
            .fold(0.0f64, f64::max);

        for &v in layer {
            let node = g.node_mut(v).unwrap();
            node.y = match config.rankalign {
                RankAlign::Top => prev_y + node.height / 2.0,
                RankAlign::Bottom => prev_y + max_height - node.height / 2.0,
                RankAlign::Center => prev_y + max_height / 2.0,
            };
        }

        prev_y += max_height + config.ranksep;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ranked_graph() -> Graph<NodeLabel, EdgeLabel> {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(40.0, 20.0));
        let b = g.add_node(NodeLabel::new(40.0, 30.0));
        let c = g.add_node(NodeLabel::new(40.0, 10.0));
        g.add_edge(a, b, EdgeLabel::default());
        g.add_edge(a, c, EdgeLabel::default());
        g.node_mut(a).unwrap().rank = 0;
        g.node_mut(a).unwrap().order = 0;
        g.node_mut(b).unwrap().rank = 1;
        g.node_mut(b).unwrap().order = 0;
        g.node_mut(c).unwrap().rank = 1;
        g.node_mut(c).unwrap().order = 1;
        g
    }

    #[test]
    fn y_coords_center_aligned() {
        let mut g = ranked_graph();
        let config = DagreConfig::default();
        position_y(&mut g, &config);

        let a = g
            .node_ids()
            .find(|&nid| g.node(nid).unwrap().rank == 0)
            .unwrap();
        let a_y = g.node(a).unwrap().y;
        // Layer 0 has max_height=20, center → y = 0 + 20/2 = 10
        assert!((a_y - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn y_coords_top_aligned() {
        let mut g = ranked_graph();
        let mut config = DagreConfig::default();
        config.rankalign = RankAlign::Top;
        position_y(&mut g, &config);

        // Find node at rank 0
        let a = g
            .node_ids()
            .find(|&nid| g.node(nid).unwrap().rank == 0)
            .unwrap();
        let a_y = g.node(a).unwrap().y;
        // Layer 0: node height=20, top align → y = 0 + 20/2 = 10
        assert!((a_y - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn second_layer_offset_by_ranksep() {
        let mut g = ranked_graph();
        let config = DagreConfig::default(); // ranksep=50
        position_y(&mut g, &config);

        let a = g
            .node_ids()
            .find(|&nid| g.node(nid).unwrap().rank == 0)
            .unwrap();
        let b = g
            .node_ids()
            .find(|&nid| g.node(nid).unwrap().rank == 1)
            .unwrap();
        let a_y = g.node(a).unwrap().y;
        // Layer 0 max_height=20, so layer 1 starts at 20 + 50 = 70
        // Layer 1 max_height=30, center → y = 70 + 30/2 = 85
        let b_y = g.node(b).unwrap().y;
        assert!(b_y > a_y);
        assert!((b_y - 85.0).abs() < f64::EPSILON);
    }
}
