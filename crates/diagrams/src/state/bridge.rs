use std::collections::{HashMap, HashSet};

use rusty_mermaid_core::{SimpleTextMeasure, TextMeasure, TextStyle};
use rusty_mermaid_dagre::{DagreConfig, EdgeLabel, NodeLabel};
use rusty_mermaid_graph::{Graph, NodeId};

use super::ir::{StateDiagram, StateKind, StateTransition};

const PADDING_X: f64 = 16.0;
const PADDING_Y: f64 = 8.0;
const START_END_SIZE: f64 = 16.0;
const FORK_JOIN_WIDTH: f64 = 70.0;
const FORK_JOIN_HEIGHT: f64 = 7.0;
const CHOICE_SIZE: f64 = 28.0;
/// Extra height added above compound nodes for the title + separator header.
/// Dagre doesn't know about the header, so without this the first inner
/// child overlaps the separator line.
const COMPOUND_HEADER_HEIGHT: f64 = 3.0;

/// Layout result: node positions and edge points.
#[derive(Debug)]
pub struct LayoutResult {
    pub nodes: Vec<NodeLayout>,
    pub edges: Vec<EdgeLayout>,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeShape {
    RoundedRect,
    StartCircle,
    EndBullseye,
    ForkJoinBar,
    ChoiceDiamond,
}

#[derive(Debug)]
pub struct NodeLayout {
    pub id: String,
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub is_compound: bool,
    pub shape: NodeShape,
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
    let mut id_map: HashMap<String, NodeId> = HashMap::new();
    let mut all_transitions: Vec<&StateTransition> = Vec::new();
    let mut synthetic_ids: HashSet<String> = HashSet::new();

    // Build graph: nodes, edges, compound hierarchy — all in one recursive walk
    add_scope(
        &diagram.states,
        &diagram.transitions,
        None,
        &mut g,
        &mut id_map,
        &mut all_transitions,
        &mut synthetic_ids,
        measurer,
        &style,
    );

    // Configure and run layout
    let mut config = DagreConfig::default();
    config.rankdir = diagram.direction;
    rusty_mermaid_dagre::pipeline::layout(&mut g, &config);

    // Extract results
    let nid_to_id: HashMap<NodeId, &str> = id_map.iter().map(|(id, &nid)| (nid, id.as_str())).collect();

    let mut nodes = Vec::new();
    let mut max_x: f64 = 0.0;
    let mut max_y: f64 = 0.0;

    for (id_str, &nid) in &id_map {
        if synthetic_ids.contains(id_str) {
            continue;
        }
        let n = g.node(nid).unwrap();
        let label = find_state_label(&diagram.states, id_str)
            .unwrap_or_else(|| id_str.to_string());
        nodes.push(NodeLayout {
            id: id_str.clone(),
            label,
            x: n.x,
            y: n.y,
            width: n.width,
            height: n.height,
            is_compound: is_compound_state(&diagram.states, id_str),
            shape: node_shape(&diagram.states, id_str),
        });
        max_x = max_x.max(n.x + n.width / 2.0);
        max_y = max_y.max(n.y + n.height / 2.0);
    }

    // Extend compound rects upward to make room for the title + separator
    // header.  Dagre doesn't know about the header, so without this the
    // first inner child circle overlaps the separator line.
    for node in &mut nodes {
        if node.is_compound {
            node.height += COMPOUND_HEADER_HEIGHT;
            node.y -= COMPOUND_HEADER_HEIGHT / 2.0;
        }
    }

    // Recompute max extents after compound adjustments
    max_x = 0.0;
    max_y = 0.0;
    for node in &nodes {
        max_x = max_x.max(node.x + node.width / 2.0);
        max_y = max_y.max(node.y + node.height / 2.0);
    }

    // Only emit edges that correspond to a real transition (filters out
    // synthetic scaffold edges used for compound ranking).
    let mut edges = Vec::new();
    for eid in g.edge_ids() {
        let (src, dst) = g.edge_endpoints(eid).unwrap();
        if let (Some(&src_id), Some(&dst_id)) = (nid_to_id.get(&src), nid_to_id.get(&dst)) {
            let matched = all_transitions.iter().find(|t| {
                let s = resolve_pseudo(&t.src, src_id);
                let d = resolve_pseudo(&t.dst, dst_id);
                s && d
            });
            let Some(transition) = matched else { continue };
            let e = g.edge(eid).unwrap();
            let points: Vec<(f64, f64)> = e.points.iter().map(|p| (p.x, p.y)).collect();
            edges.push(EdgeLayout {
                src: src_id.to_string(),
                dst: dst_id.to_string(),
                points,
                label: transition.label.clone(),
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

/// Check if a transition endpoint matches a resolved node ID.
/// Handles [*] → scoped pseudo-state name mapping, and composite
/// states redirected to inner pseudo-states.
fn resolve_pseudo(transition_id: &str, node_id: &str) -> bool {
    if transition_id == "[*]" {
        node_id.ends_with("[*]_start") || node_id.ends_with("[*]_end")
    } else if transition_id == node_id {
        true
    } else {
        // Composite redirect: transition says "Active", node is "Active.[*]_start"
        node_id == format!("{transition_id}.[*]_start")
            || node_id == format!("{transition_id}.[*]_end")
    }
}

/// Check if a state ID refers to a composite (compound) state.
fn is_compound_state(states: &[super::ir::StateNode], id: &str) -> bool {
    for s in states {
        if s.id == id {
            return s.is_composite();
        }
        if let StateKind::Composite { children, .. } = &s.kind {
            if is_compound_state(children, id) {
                return true;
            }
        }
    }
    false
}

/// Recursively find a state's label by ID across all nesting levels.
fn find_state_label(states: &[super::ir::StateNode], id: &str) -> Option<String> {
    for s in states {
        if s.id == id {
            return s.label.clone().or_else(|| Some(s.id.clone()));
        }
        if let StateKind::Composite { children, .. } = &s.kind {
            if let Some(label) = find_state_label(children, id) {
                return Some(label);
            }
        }
    }
    None
}

/// Determine the rendering shape for a node based on its ID and IR kind.
fn node_shape(states: &[super::ir::StateNode], id: &str) -> NodeShape {
    if id.ends_with("[*]_start") {
        return NodeShape::StartCircle;
    }
    if id.ends_with("[*]_end") {
        return NodeShape::EndBullseye;
    }
    match find_state_kind(states, id) {
        Some(StateKind::Fork | StateKind::Join) => NodeShape::ForkJoinBar,
        Some(StateKind::Choice) => NodeShape::ChoiceDiamond,
        Some(StateKind::Start) => NodeShape::StartCircle,
        Some(StateKind::End) => NodeShape::EndBullseye,
        _ => NodeShape::RoundedRect,
    }
}

/// Recursively find a state's kind by ID across all nesting levels.
fn find_state_kind<'a>(states: &'a [super::ir::StateNode], id: &str) -> Option<&'a StateKind> {
    for s in states {
        if s.id == id {
            return Some(&s.kind);
        }
        if let StateKind::Composite { children, .. } = &s.kind {
            if let Some(kind) = find_state_kind(children, id) {
                return Some(kind);
            }
        }
    }
    None
}

/// Process one scope (top-level or inside a composite): create nodes, pseudo-states,
/// edges, and compound parent relationships.
fn add_scope<'a>(
    states: &'a [super::ir::StateNode],
    transitions: &'a [StateTransition],
    parent: Option<(NodeId, &str)>, // (parent_nid, parent_id) for scoping [*] names
    g: &mut Graph<NodeLabel, EdgeLabel>,
    id_map: &mut HashMap<String, NodeId>,
    all_transitions: &mut Vec<&'a StateTransition>,
    synthetic_ids: &mut HashSet<String>,
    measurer: &impl TextMeasure,
    style: &TextStyle,
) {
    // Create [*] pseudo-states for this scope
    let scope_prefix = parent.map(|(_, id)| format!("{id}.")).unwrap_or_default();

    let has_start = transitions.iter().any(|t| t.src == "[*]");
    let has_end = transitions.iter().any(|t| t.dst == "[*]");

    if has_start {
        let key = format!("{scope_prefix}[*]_start");
        let nid = g.add_node(NodeLabel::new(START_END_SIZE, START_END_SIZE));
        if let Some((parent_nid, _)) = parent {
            g.set_parent(nid, parent_nid);
        }
        id_map.insert(key, nid);
    }
    if has_end {
        let key = format!("{scope_prefix}[*]_end");
        let nid = g.add_node(NodeLabel::new(START_END_SIZE, START_END_SIZE));
        if let Some((parent_nid, _)) = parent {
            g.set_parent(nid, parent_nid);
        }
        id_map.insert(key, nid);
    }

    // Add state nodes
    for s in states {
        let (width, height) = match &s.kind {
            StateKind::Fork | StateKind::Join => (FORK_JOIN_WIDTH, FORK_JOIN_HEIGHT),
            StateKind::Choice => (CHOICE_SIZE, CHOICE_SIZE),
            StateKind::Start | StateKind::End => (START_END_SIZE, START_END_SIZE),
            StateKind::Composite { children, transitions: inner_trans, .. } => {
                let nid = g.add_node(NodeLabel::new(0.0, 0.0));
                id_map.insert(s.id.clone(), nid);
                if let Some((parent_nid, _)) = parent {
                    g.set_parent(nid, parent_nid);
                }

                // Recurse into composite: add children, inner pseudo-states, inner edges
                add_scope(
                    children,
                    inner_trans,
                    Some((nid, &s.id)),
                    g,
                    id_map,
                    all_transitions,
                    synthetic_ids,
                    measurer,
                    style,
                );

                // Parent children to this composite
                for child in children {
                    if let Some(&child_nid) = id_map.get(child.id.as_str()) {
                        if g.parent(child_nid).is_none() {
                            g.set_parent(child_nid, nid);
                        }
                    }
                }

                // If this composite is an edge source in the outer scope but has
                // no inner [*]_end, create a synthetic exit node so that dagre
                // ranks the outgoing target below the compound's children.
                let inner_end_key = format!("{}.[*]_end", s.id);
                if !id_map.contains_key(&inner_end_key) {
                    let is_source = transitions.iter().any(|t| t.src == s.id);
                    if is_source {
                        let end_nid = g.add_node(NodeLabel::new(START_END_SIZE, START_END_SIZE));
                        g.set_parent(end_nid, nid);
                        synthetic_ids.insert(inner_end_key.clone());
                        id_map.insert(inner_end_key, end_nid);

                        // Single edge from one child to exit so it ranks at
                        // the bottom.  Using all children creates competing
                        // alignment forces in the BK position phase (their
                        // normalized dummy nodes distort x-coordinates).
                        // Mermaid's findNonClusterChild picks one child — we
                        // pick the last to maximise the chance it is already
                        // the lowest-ranked node in the compound.
                        if let Some(last) = children.last() {
                            if let Some(&child_nid) = id_map.get(last.id.as_str()) {
                                g.add_edge(child_nid, end_nid, EdgeLabel::default());
                            }
                        }
                    }
                }

                continue; // already added the node
            }
            StateKind::Normal => {
                let text = s.label.as_deref().unwrap_or(&s.id);
                let (tw, th) = measurer.measure(text, style);
                (tw + PADDING_X * 2.0, th + PADDING_Y * 2.0)
            }
        };

        let nid = g.add_node(NodeLabel::new(width, height));
        id_map.insert(s.id.clone(), nid);

        if let Some((parent_nid, _)) = parent {
            g.set_parent(nid, parent_nid);
        }
    }

    // Add edges for this scope's transitions.
    // Edges to/from composite states are redirected to inner pseudo-states
    // so dagre assigns correct ranks (above/below the compound).
    for t in transitions {
        let mut src_key = if t.src == "[*]" {
            format!("{scope_prefix}[*]_start")
        } else {
            t.src.clone()
        };
        let mut dst_key = if t.dst == "[*]" {
            format!("{scope_prefix}[*]_end")
        } else {
            t.dst.clone()
        };

        // Redirect: edge FROM composite → use inner [*]_end
        if t.src != "[*]" {
            let inner_end = format!("{}.[*]_end", t.src);
            if id_map.contains_key(&inner_end) {
                src_key = inner_end;
            }
        }
        // Redirect: edge TO composite → use inner [*]_start
        if t.dst != "[*]" {
            let inner_start = format!("{}.[*]_start", t.dst);
            if id_map.contains_key(&inner_start) {
                dst_key = inner_start;
            }
        }

        let Some(&src) = id_map.get(&src_key) else { continue };
        let Some(&dst) = id_map.get(&dst_key) else { continue };

        let mut label = EdgeLabel::default();
        if let Some(text) = &t.label {
            let (tw, th) = measurer.measure(text, style);
            label.width = tw;
            label.height = th;
        }
        g.add_edge(src, dst, label);
        all_transitions.push(t);
    }
}

#[cfg(test)]
mod tests {
    use rusty_mermaid_core::Direction;

    use super::*;
    use crate::state::ir::*;

    #[test]
    fn composite_children_aligned() {
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    [*] --> Active\n    state Active {\n        [*] --> Idle\n        Idle --> Running\n        Running --> Idle\n    }\n    Active --> [*]"
        ).unwrap();
        let result = layout(&d);

        let idle = result.nodes.iter().find(|n| n.id == "Idle").unwrap();
        let running = result.nodes.iter().find(|n| n.id == "Running").unwrap();
        assert!(
            (idle.x - running.x).abs() < 1.0,
            "Idle (x={:.1}) and Running (x={:.1}) should be x-aligned",
            idle.x, running.x
        );
    }

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

    #[test]
    fn layout_composite_has_inner_edges() {
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    [*] --> Active\n    state Active {\n        [*] --> Idle\n        Idle --> Running\n        Running --> Idle\n    }\n    Active --> [*]"
        ).unwrap();
        let result = layout(&d);

        // Nodes: [*]_start, [*]_end, Active, Active.[*]_start, Idle, Running
        assert!(result.nodes.iter().any(|n| n.id == "Idle"), "should have Idle");
        assert!(result.nodes.iter().any(|n| n.id == "Running"), "should have Running");
        assert!(result.nodes.iter().any(|n| n.id == "Active.[*]_start"), "should have inner start");

        // Should have inner edges
        assert!(result.edges.iter().any(|e| e.src == "Active.[*]_start" && e.dst == "Idle"),
            "should have inner [*] --> Idle edge");
        assert!(result.edges.iter().any(|e| e.src == "Idle" && e.dst == "Running"),
            "should have Idle --> Running edge");

        // Active should be marked as compound
        let active = result.nodes.iter().find(|n| n.id == "Active").unwrap();
        assert!(active.is_compound, "Active should be compound");
        let idle = result.nodes.iter().find(|n| n.id == "Idle").unwrap();
        let active_left = active.x - active.width / 2.0;
        let active_right = active.x + active.width / 2.0;
        assert!(active_left <= idle.x - idle.width / 2.0,
            "Active should contain Idle: active_left={active_left} idle_left={}",
            idle.x - idle.width / 2.0);
        assert!(active_right >= idle.x + idle.width / 2.0,
            "Active should contain Idle: active_right={active_right} idle_right={}",
            idle.x + idle.width / 2.0);

        // TB layout: [*]_start should be ABOVE Active, [*]_end BELOW
        let start = result.nodes.iter().find(|n| n.id == "[*]_start").unwrap();
        let end = result.nodes.iter().find(|n| n.id == "[*]_end").unwrap();
        let active_top = active.y - active.height / 2.0;
        let active_bottom = active.y + active.height / 2.0;
        assert!(start.y < active_top,
            "[*]_start (y={}) should be above Active top (y={active_top})",
            start.y);
        assert!(end.y > active_bottom,
            "[*]_end (y={}) should be below Active bottom (y={active_bottom})",
            end.y);
    }

    #[test]
    fn node_shapes_propagated() {
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    state fork1 <<fork>>\n    state join1 <<join>>\n    state check <<choice>>\n    [*] --> fork1\n    fork1 --> A\n    fork1 --> B\n    A --> check\n    check --> join1 : yes\n    B --> join1\n    join1 --> [*]"
        ).unwrap();
        let result = layout(&d);

        let start = result.nodes.iter().find(|n| n.id == "[*]_start").unwrap();
        assert_eq!(start.shape, NodeShape::StartCircle);

        let end = result.nodes.iter().find(|n| n.id == "[*]_end").unwrap();
        assert_eq!(end.shape, NodeShape::EndBullseye);

        let fork = result.nodes.iter().find(|n| n.id == "fork1").unwrap();
        assert_eq!(fork.shape, NodeShape::ForkJoinBar);

        let join = result.nodes.iter().find(|n| n.id == "join1").unwrap();
        assert_eq!(join.shape, NodeShape::ForkJoinBar);

        let choice = result.nodes.iter().find(|n| n.id == "check").unwrap();
        assert_eq!(choice.shape, NodeShape::ChoiceDiamond);

        let a = result.nodes.iter().find(|n| n.id == "A").unwrap();
        assert_eq!(a.shape, NodeShape::RoundedRect);
    }

    #[test]
    fn layout_choice_branches() {
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    state check <<choice>>\n    [*] --> check\n    check --> A : yes\n    check --> B : no\n    A --> [*]\n    B --> [*]"
        ).unwrap();
        let result = layout(&d);

        let check = result.nodes.iter().find(|n| n.id == "check").unwrap();
        let a = result.nodes.iter().find(|n| n.id == "A").unwrap();
        let b = result.nodes.iter().find(|n| n.id == "B").unwrap();

        // check should be above A and B
        assert!(check.y < a.y, "check should be above A");
        assert!(check.y < b.y, "check should be above B");
    }
}
