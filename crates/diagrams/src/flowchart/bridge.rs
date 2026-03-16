use std::collections::HashMap;

use rusty_mermaid_core::{SimpleTextMeasure, TextMeasure, TextStyle};
use rusty_mermaid_dagre::{DagreConfig, EdgeLabel, NodeLabel};
use rusty_mermaid_graph::{Graph, NodeId};

use super::ir::FlowDiagram;
use crate::common::tokens::strip_html_tags;

const PADDING_X: f64 = 16.0;
const PADDING_Y: f64 = 8.0;

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

/// Build a dagre Graph from FlowDiagram IR, run layout, return positions.
pub fn layout(diagram: &FlowDiagram) -> LayoutResult {
    layout_with_measurer(diagram, &SimpleTextMeasure::default())
}

/// Layout with a custom text measurer.
pub fn layout_with_measurer(diagram: &FlowDiagram, measurer: &impl TextMeasure) -> LayoutResult {
    let mut g = Graph::new();
    let style = TextStyle::default();

    // Map vertex ID → NodeId
    let mut id_map: HashMap<&str, NodeId> = HashMap::new();

    for v in &diagram.vertices {
        let text = strip_html_tags(&v.label);
        let (tw, th) = measurer.measure(&text, &style);
        let width = tw + PADDING_X * 2.0;
        let height = th + PADDING_Y * 2.0;
        let nid = g.add_node(NodeLabel::new(width, height));
        id_map.insert(&v.id, nid);
    }

    // Set up compound hierarchy for subgraphs
    for sg in &diagram.subgraphs {
        // Create a compound parent node for the subgraph
        let sg_nid = g.add_node(NodeLabel::new(0.0, 0.0));
        id_map.insert(&sg.id, sg_nid);

        for child_id in &sg.node_ids {
            if let Some(&child_nid) = id_map.get(child_id.as_str()) {
                g.set_parent(child_nid, sg_nid);
            }
        }
        for child_sg_id in &sg.subgraph_ids {
            if let Some(&child_nid) = id_map.get(child_sg_id.as_str()) {
                g.set_parent(child_nid, sg_nid);
            }
        }
    }

    // Add edges
    for e in &diagram.edges {
        let Some(&src) = id_map.get(e.src.as_str()) else { continue };
        let Some(&dst) = id_map.get(e.dst.as_str()) else { continue };
        let mut label = EdgeLabel::default();
        if let Some(text) = &e.label {
            let (tw, th) = measurer.measure(text, &style);
            label.width = tw;
            label.height = th;
        }
        g.add_edge(src, dst, label);
    }

    // Configure
    let mut config = DagreConfig::default();
    config.rankdir = diagram.direction;

    // Run layout
    rusty_mermaid_dagre::pipeline::layout(&mut g, &config);

    // Extract results
    let nid_to_id: HashMap<NodeId, &str> = id_map.iter().map(|(&id, &nid)| (nid, id)).collect();

    let mut nodes = Vec::new();
    let mut max_x: f64 = 0.0;
    let mut max_y: f64 = 0.0;

    for v in &diagram.vertices {
        if let Some(&nid) = id_map.get(v.id.as_str()) {
            let n = g.node(nid).unwrap();
            nodes.push(NodeLayout {
                id: v.id.clone(),
                x: n.x,
                y: n.y,
                width: n.width,
                height: n.height,
            });
            max_x = max_x.max(n.x + n.width / 2.0);
            max_y = max_y.max(n.y + n.height / 2.0);
        }
    }

    let mut edges = Vec::new();
    for eid in g.edge_ids() {
        let (src, dst) = g.edge_endpoints(eid).unwrap();
        if let (Some(&src_id), Some(&dst_id)) = (nid_to_id.get(&src), nid_to_id.get(&dst)) {
            let e = g.edge(eid).unwrap();
            let points: Vec<(f64, f64)> = e.points.iter().map(|p| (p.x, p.y)).collect();
            let label = diagram
                .edges
                .iter()
                .find(|fe| fe.src == src_id && fe.dst == dst_id)
                .and_then(|fe| fe.label.clone());
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

#[cfg(test)]
mod tests {
    use rusty_mermaid_core::{Direction, Shape};

    use super::*;
    use crate::flowchart::ir::*;

    #[test]
    fn layout_simple_chain() {
        let mut d = FlowDiagram::new(Direction::TB);
        d.vertices.push(FlowVertex::new("A", "Start", Shape::Rect));
        d.vertices.push(FlowVertex::new("B", "End", Shape::Rect));
        d.edges.push(FlowEdge::new("A", "B"));

        let result = layout(&d);
        assert_eq!(result.nodes.len(), 2);
        assert_eq!(result.edges.len(), 1);

        let a = result.nodes.iter().find(|n| n.id == "A").unwrap();
        let b = result.nodes.iter().find(|n| n.id == "B").unwrap();
        assert!(a.y < b.y, "A should be above B in TB layout");
    }

    #[test]
    fn layout_lr_direction() {
        let mut d = FlowDiagram::new(Direction::LR);
        d.vertices.push(FlowVertex::new("A", "Left", Shape::Rect));
        d.vertices.push(FlowVertex::new("B", "Right", Shape::Rect));
        d.edges.push(FlowEdge::new("A", "B"));

        let result = layout(&d);
        let a = result.nodes.iter().find(|n| n.id == "A").unwrap();
        let b = result.nodes.iter().find(|n| n.id == "B").unwrap();
        assert!(a.x < b.x, "A should be left of B in LR layout");
    }

    #[test]
    fn layout_from_parsed_mmd() {
        let d = crate::flowchart::parser::parse("graph TD\n    A[Start] --> B[End]").unwrap();
        let result = layout(&d);
        assert_eq!(result.nodes.len(), 2);
        assert_eq!(result.edges.len(), 1);
    }

    #[test]
    fn layout_edge_has_points() {
        let d = crate::flowchart::parser::parse("graph TD\n    A --> B --> C").unwrap();
        let result = layout(&d);
        for e in &result.edges {
            assert!(!e.points.is_empty(), "edge {}->{} should have points", e.src, e.dst);
        }
    }
}
