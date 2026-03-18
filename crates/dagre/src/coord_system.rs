use rusty_mermaid_core::Direction;
use rusty_mermaid_graph::Graph;

use crate::labels::{EdgeLabel, NodeLabel};

/// Pre-layout transform: for LR/RL layouts, swap width/height
/// so the algorithm lays out as if TB, then we undo after.
pub(crate) fn adjust(g: &mut Graph<NodeLabel, EdgeLabel>, rankdir: Direction) {
    if rankdir == Direction::LR || rankdir == Direction::RL {
        swap_width_height(g);
    }
}

/// Post-layout inverse transform: restore correct orientation.
pub(crate) fn undo(g: &mut Graph<NodeLabel, EdgeLabel>, rankdir: Direction) {
    if rankdir == Direction::BT || rankdir == Direction::RL {
        reverse_y(g);
    }
    if rankdir == Direction::LR || rankdir == Direction::RL {
        swap_xy(g);
        swap_width_height(g);
    }
}

fn swap_width_height(g: &mut Graph<NodeLabel, EdgeLabel>) {
    for nid in g.node_ids().collect::<Vec<_>>() {
        let Some(n) = g.node_mut(nid) else { continue };
        std::mem::swap(&mut n.width, &mut n.height);
    }
    for eid in g.edge_ids().collect::<Vec<_>>() {
        let Some(e) = g.edge_mut(eid) else { continue };
        std::mem::swap(&mut e.width, &mut e.height);
    }
}

fn reverse_y(g: &mut Graph<NodeLabel, EdgeLabel>) {
    for nid in g.node_ids().collect::<Vec<_>>() {
        if let Some(n) = g.node_mut(nid) {
            n.y = -n.y;
        }
    }
    for eid in g.edge_ids().collect::<Vec<_>>() {
        let Some(e) = g.edge_mut(eid) else { continue };
        e.y = -e.y;
        for p in &mut e.points {
            p.y = -p.y;
        }
    }
}

fn swap_xy(g: &mut Graph<NodeLabel, EdgeLabel>) {
    for nid in g.node_ids().collect::<Vec<_>>() {
        let Some(n) = g.node_mut(nid) else { continue };
        std::mem::swap(&mut n.x, &mut n.y);
    }
    for eid in g.edge_ids().collect::<Vec<_>>() {
        let Some(e) = g.edge_mut(eid) else { continue };
        std::mem::swap(&mut e.x, &mut e.y);
        for p in &mut e.points {
            std::mem::swap(&mut p.x, &mut p.y);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusty_mermaid_core::Point;

    fn make_graph() -> (Graph<NodeLabel, EdgeLabel>, rusty_mermaid_graph::NodeId) {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(100.0, 50.0));
        g.node_mut(a).unwrap().x = 10.0;
        g.node_mut(a).unwrap().y = 20.0;
        let b = g.add_node(NodeLabel::new(80.0, 40.0));
        let eid = g.add_edge(a, b, EdgeLabel::default());
        let e = g.edge_mut(eid).unwrap();
        e.x = 5.0;
        e.y = 15.0;
        e.points = vec![Point { x: 1.0, y: 2.0 }];
        (g, a)
    }

    #[test]
    fn tb_is_noop() {
        let (mut g, a) = make_graph();
        adjust(&mut g, Direction::TB);
        assert!((g.node(a).unwrap().width - 100.0).abs() < f64::EPSILON);
        undo(&mut g, Direction::TB);
        assert!((g.node(a).unwrap().x - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn lr_swaps_dimensions() {
        let (mut g, a) = make_graph();
        adjust(&mut g, Direction::LR);
        assert!((g.node(a).unwrap().width - 50.0).abs() < f64::EPSILON);
        assert!((g.node(a).unwrap().height - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn lr_roundtrip() {
        let (mut g, a) = make_graph();
        adjust(&mut g, Direction::LR);
        // Simulate layout setting x=10, y=20
        g.node_mut(a).unwrap().x = 10.0;
        g.node_mut(a).unwrap().y = 20.0;
        undo(&mut g, Direction::LR);
        // After undo: x,y swapped → x=20, y=10; dimensions restored
        assert!((g.node(a).unwrap().x - 20.0).abs() < f64::EPSILON);
        assert!((g.node(a).unwrap().y - 10.0).abs() < f64::EPSILON);
        assert!((g.node(a).unwrap().width - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn bt_reverses_y() {
        let (mut g, a) = make_graph();
        undo(&mut g, Direction::BT);
        assert!((g.node(a).unwrap().y - -20.0).abs() < f64::EPSILON);
    }
}
