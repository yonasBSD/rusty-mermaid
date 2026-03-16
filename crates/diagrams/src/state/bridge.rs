use std::collections::HashMap;

use rusty_mermaid_core::{SimpleTextMeasure, TextMeasure, TextStyle};
use rusty_mermaid_dagre::{DagreConfig, EdgeLabel, NodeLabel};
use rusty_mermaid_graph::{Graph, NodeId};

use super::ir::{StateDiagram, StateKind};

const PADDING_X: f64 = 16.0;
const PADDING_Y: f64 = 8.0;
const START_END_SIZE: f64 = 16.0;
const FORK_JOIN_WIDTH: f64 = 80.0;
const FORK_JOIN_HEIGHT: f64 = 6.0;
const CHOICE_SIZE: f64 = 28.0;

/// Layout result: node positions and edge points.
#[derive(Debug)]
pub struct LayoutResult {
    pub nodes: Vec<NodeLayout>,
    pub edges: Vec<EdgeLayout>,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug)]
pub struct NodeLayout {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug)]
pub struct EdgeLayout {
    pub src: String,
    pub dst: String,
    pub points: Vec<(f64, f64)>,
    pub label: Option<String>,
}

/// Layout with the default text measurer.
pub fn layout(diagram: &StateDiagram) -> LayoutResult {
    layout_with_measurer(diagram, &SimpleTextMeasure::default())
}

/// Layout with a custom text measurer.
pub fn layout_with_measurer(diagram: &StateDiagram, measurer: &impl TextMeasure) -> LayoutResult {
    let mut g = Graph::new();
    let style = TextStyle::default();
    let mut id_map: HashMap<&str, NodeId> = HashMap::new();

    // Add special start/end nodes for [*] pseudo-states.
    // [*] as source → start node, [*] as dest → end node.
    let has_start = diagram.transitions.iter().any(|t| t.src == "[*]");
    let has_end = diagram.transitions.iter().any(|t| t.dst == "[*]");

    if has_start {
        let nid = g.add_node(NodeLabel::new(START_END_SIZE, START_END_SIZE));
        id_map.insert("[*]_start", nid);
    }
    if has_end {
        let nid = g.add_node(NodeLabel::new(START_END_SIZE, START_END_SIZE));
        id_map.insert("[*]_end", nid);
    }

    // Add state nodes
    add_states(&diagram.states, &mut g, &mut id_map, measurer, &style);

    // Add edges
    for t in &diagram.transitions {
        let src_key = if t.src == "[*]" { "[*]_start" } else { t.src.as_str() };
        let dst_key = if t.dst == "[*]" { "[*]_end" } else { t.dst.as_str() };

        let Some(&src) = id_map.get(src_key) else { continue };
        let Some(&dst) = id_map.get(dst_key) else { continue };

        let mut label = EdgeLabel::default();
        if let Some(text) = &t.label {
            let (tw, th) = measurer.measure(text, &style);
            label.width = tw;
            label.height = th;
        }
        g.add_edge(src, dst, label);
    }

    // Configure and run layout
    let mut config = DagreConfig::default();
    config.rankdir = diagram.direction;
    rusty_mermaid_dagre::pipeline::layout(&mut g, &config);

    // Extract results
    let nid_to_id: HashMap<NodeId, &str> = id_map.iter().map(|(&id, &nid)| (nid, id)).collect();

    let mut nodes = Vec::new();
    let mut max_x: f64 = 0.0;
    let mut max_y: f64 = 0.0;

    for (&id_str, &nid) in &id_map {
        let n = g.node(nid).unwrap();
        nodes.push(NodeLayout {
            id: id_str.to_string(),
            x: n.x,
            y: n.y,
            width: n.width,
            height: n.height,
        });
        max_x = max_x.max(n.x + n.width / 2.0);
        max_y = max_y.max(n.y + n.height / 2.0);
    }

    let mut edges = Vec::new();
    for eid in g.edge_ids() {
        let (src, dst) = g.edge_endpoints(eid).unwrap();
        if let (Some(&src_id), Some(&dst_id)) = (nid_to_id.get(&src), nid_to_id.get(&dst)) {
            let e = g.edge(eid).unwrap();
            let points: Vec<(f64, f64)> = e.points.iter().map(|p| (p.x, p.y)).collect();
            let label = diagram
                .transitions
                .iter()
                .find(|t| {
                    let s = if t.src == "[*]" { "[*]_start" } else { t.src.as_str() };
                    let d = if t.dst == "[*]" { "[*]_end" } else { t.dst.as_str() };
                    s == src_id && d == dst_id
                })
                .and_then(|t| t.label.clone());
            edges.push(EdgeLayout {
                src: src_id.to_string(),
                dst: dst_id.to_string(),
                points,
                label,
            });
        }
    }

    LayoutResult {
        nodes,
        edges,
        width: max_x,
        height: max_y,
    }
}

fn add_states<'a>(
    states: &'a [super::ir::StateNode],
    g: &mut Graph<NodeLabel, EdgeLabel>,
    id_map: &mut HashMap<&'a str, NodeId>,
    measurer: &impl TextMeasure,
    style: &TextStyle,
) {
    for s in states {
        let (width, height) = match &s.kind {
            StateKind::Fork | StateKind::Join => (FORK_JOIN_WIDTH, FORK_JOIN_HEIGHT),
            StateKind::Choice => (CHOICE_SIZE, CHOICE_SIZE),
            StateKind::Start | StateKind::End => (START_END_SIZE, START_END_SIZE),
            StateKind::Composite { children, .. } => {
                // Add children recursively, then create a compound parent
                add_states(children, g, id_map, measurer, style);
                (0.0, 0.0) // dagre sizes compound nodes automatically
            }
            StateKind::Normal => {
                let text = s.label.as_deref().unwrap_or(&s.id);
                let (tw, th) = measurer.measure(text, style);
                (tw + PADDING_X * 2.0, th + PADDING_Y * 2.0)
            }
        };

        let nid = g.add_node(NodeLabel::new(width, height));
        id_map.insert(&s.id, nid);

        // Set up compound hierarchy for composite states
        if let StateKind::Composite { children, .. } = &s.kind {
            for child in children {
                if let Some(&child_nid) = id_map.get(child.id.as_str()) {
                    g.set_parent(child_nid, nid);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rusty_mermaid_core::Direction;

    use super::*;
    use crate::state::ir::*;

    #[test]
    fn layout_simple_chain() {
        let mut d = StateDiagram::new(Direction::TB);
        d.states.push(StateNode::new("A", StateKind::Normal));
        d.states.push(StateNode::new("B", StateKind::Normal));
        d.transitions.push(StateTransition::new("A", "B"));

        let result = layout(&d);
        assert_eq!(result.nodes.len(), 2);
        assert_eq!(result.edges.len(), 1);

        let a = result.nodes.iter().find(|n| n.id == "A").unwrap();
        let b = result.nodes.iter().find(|n| n.id == "B").unwrap();
        assert!(a.y < b.y, "A should be above B in TB layout");
    }

    #[test]
    fn layout_with_start_end() {
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    [*] --> Still\n    Still --> [*]"
        ).unwrap();
        let result = layout(&d);
        // start + end + Still = 3 nodes
        assert_eq!(result.nodes.len(), 3);
        assert_eq!(result.edges.len(), 2);
    }

    #[test]
    fn layout_fork_join() {
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    state fork1 <<fork>>\n    state join1 <<join>>\n    [*] --> fork1\n    fork1 --> A\n    fork1 --> B\n    A --> join1\n    B --> join1\n    join1 --> [*]"
        ).unwrap();
        let result = layout(&d);
        let fork = result.nodes.iter().find(|n| n.id == "fork1").unwrap();
        assert!((fork.width - FORK_JOIN_WIDTH).abs() < 1.0);
    }

    #[test]
    fn layout_edge_has_points() {
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    A --> B\n    B --> C"
        ).unwrap();
        let result = layout(&d);
        for e in &result.edges {
            assert!(!e.points.is_empty(), "edge {}->{} should have points", e.src, e.dst);
        }
    }
}
