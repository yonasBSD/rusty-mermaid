use std::collections::HashMap;

use rusty_mermaid_core::{SimpleTextMeasure, TextMeasure, TextStyle};
use rusty_mermaid_dagre::{DagreConfig, EdgeLabel, NodeLabel};
use rusty_mermaid_graph::{Graph, NodeId};

use super::ir::{FlowDiagram, StrokeType};
use crate::common::tokens::strip_html_tags;

const PADDING_X: f64 = 16.0;
const PADDING_Y: f64 = 8.0;

/// Layout result: node positions and edge points.
#[derive(Debug)]
pub struct LayoutResult {
    pub nodes: Vec<NodeLayout>,
    pub edges: Vec<EdgeLayout>,
    pub subgraphs: Vec<SubgraphLayout>,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug)]
pub struct NodeLayout {
    pub id: String,
    pub label: String,
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
    pub stroke: StrokeType,
}

#[derive(Debug)]
pub struct SubgraphLayout {
    pub id: String,
    pub label: Option<String>,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
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

    // Set up compound hierarchy for subgraphs (two passes: create all nodes
    // first so that parent→child references resolve regardless of order)
    for sg in &diagram.subgraphs {
        let sg_nid = g.add_node(NodeLabel::new(0.0, 0.0));
        id_map.insert(&sg.id, sg_nid);
    }
    for sg in &diagram.subgraphs {
        let &sg_nid = id_map.get(sg.id.as_str()).unwrap();
        for child_id in &sg.node_ids {
            if let Some(&child_nid) = id_map.get(child_id.as_str()) {
                // First-wins: don't reparent nodes already assigned to a subgraph.
                // Nodes referenced in edges across subgraph boundaries appear in
                // multiple subgraphs' node_ids; they belong to the first one.
                if g.parent(child_nid).is_none() {
                    g.set_parent(child_nid, sg_nid);
                }
            }
        }
        for child_sg_id in &sg.subgraph_ids {
            if let Some(&child_nid) = id_map.get(child_sg_id.as_str()) {
                if g.parent(child_nid).is_none() {
                    g.set_parent(child_nid, sg_nid);
                }
            }
        }
    }

    // Add edges
    for e in &diagram.edges {
        let Some(&src) = id_map.get(e.src.as_str()) else { continue };
        let Some(&dst) = id_map.get(e.dst.as_str()) else { continue };
        let mut label = EdgeLabel::default();
        label.minlen = e.minlen;
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

    // Recenter compound nodes on their content.  The BK position
    // algorithm can place left/right border nodes asymmetrically,
    // causing unequal left/right padding.  Redistribute padding
    // evenly by centering each compound on its children's bounding
    // box.  Process inner-to-outer so parent compounds see the
    // updated child positions.
    for sg in diagram.subgraphs.iter().rev() {
        let Some(&nid) = id_map.get(sg.id.as_str()) else {
            continue;
        };
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        for child in g.children(nid).collect::<Vec<_>>() {
            let c = g.node(child).unwrap();
            if c.width > 0.0 || c.height > 0.0 {
                min_x = min_x.min(c.x - c.width / 2.0);
                max_x = max_x.max(c.x + c.width / 2.0);
            }
        }
        if min_x.is_finite() && max_x.is_finite() {
            g.node_mut(nid).unwrap().x = (min_x + max_x) / 2.0;
        }
    }

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
                label: strip_html_tags(&v.label),
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
            let flow_edge = diagram
                .edges
                .iter()
                .find(|fe| fe.src == src_id && fe.dst == dst_id);
            let label = flow_edge.and_then(|fe| fe.label.clone());
            let stroke = flow_edge.map_or(StrokeType::Normal, |fe| fe.stroke);
            edges.push(EdgeLayout {
                src: src_id.to_string(),
                dst: dst_id.to_string(),
                points,
                label,
                stroke,
            });
        }
    }

    // Extract subgraph positions from dagre's compound node bounds
    // (padding and label space are already included by remove_border_nodes).
    let mut subgraphs = Vec::new();
    for sg in &diagram.subgraphs {
        if let Some(&nid) = id_map.get(sg.id.as_str()) {
            let n = g.node(nid).unwrap();
            if n.width <= 0.0 || n.height <= 0.0 {
                continue;
            }
            subgraphs.push(SubgraphLayout {
                id: sg.id.clone(),
                label: sg.label.clone(),
                x: n.x,
                y: n.y,
                width: n.width,
                height: n.height,
            });
            max_x = max_x.max(n.x + n.width / 2.0);
            max_y = max_y.max(n.y + n.height / 2.0);
        }
    }

    LayoutResult {
        nodes,
        edges,
        subgraphs,
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
    fn subgraph_contains_children() {
        let mmd = "graph TD\n    subgraph outer[Outer]\n        subgraph inner[Inner]\n            A[Node A] --> B[Node B]\n        end\n        C[Node C]\n    end\n    C --> D[Node D]";
        let d = crate::flowchart::parser::parse(mmd).unwrap();
        let result = layout(&d);

        let inner_sg = result.subgraphs.iter().find(|sg| sg.id == "inner").unwrap();
        let a = result.nodes.iter().find(|n| n.id == "A").unwrap();
        let b = result.nodes.iter().find(|n| n.id == "B").unwrap();

        let sg_left = inner_sg.x - inner_sg.width / 2.0;
        let sg_right = inner_sg.x + inner_sg.width / 2.0;
        let a_left = a.x - a.width / 2.0;
        let a_right = a.x + a.width / 2.0;
        let b_left = b.x - b.width / 2.0;
        let b_right = b.x + b.width / 2.0;

        eprintln!("inner sg: x={:.1} w={:.1} [{:.1}, {:.1}]", inner_sg.x, inner_sg.width, sg_left, sg_right);
        eprintln!("A: x={:.1} w={:.1} [{:.1}, {:.1}]", a.x, a.width, a_left, a_right);
        eprintln!("B: x={:.1} w={:.1} [{:.1}, {:.1}]", b.x, b.width, b_left, b_right);

        assert!(sg_left <= a_left, "inner should contain A horizontally");
        assert!(sg_right >= a_right, "inner should contain A horizontally");
        assert!(sg_left <= b_left, "inner should contain B horizontally");
        assert!(sg_right >= b_right, "inner should contain B horizontally");
    }

    #[test]
    fn layout_edge_has_points() {
        let d = crate::flowchart::parser::parse("graph TD\n    A --> B --> C").unwrap();
        let result = layout(&d);
        for e in &result.edges {
            assert!(!e.points.is_empty(), "edge {}->{} should have points", e.src, e.dst);
        }
    }

    #[test]
    fn subgraph_centered_on_children() {
        let mmd = "flowchart TD\n    subgraph outer[Level 1]\n        subgraph inner[Level 2]\n            A --> B\n        end\n        C --> A\n    end\n    D --> C";
        let d = crate::flowchart::parser::parse(mmd).unwrap();
        let result = layout(&d);

        for sg in &result.subgraphs {
            let sg_left = sg.x - sg.width / 2.0;
            let sg_right = sg.x + sg.width / 2.0;

            // Collect direct children bounds
            let children: Vec<_> = result
                .nodes
                .iter()
                .filter(|n| {
                    let ir_sg = d.subgraphs.iter().find(|s| s.id == sg.id).unwrap();
                    ir_sg.node_ids.contains(&n.id)
                })
                .collect();

            if children.is_empty() {
                continue;
            }

            let content_min = children
                .iter()
                .map(|n| n.x - n.width / 2.0)
                .fold(f64::INFINITY, f64::min);
            let content_max = children
                .iter()
                .map(|n| n.x + n.width / 2.0)
                .fold(f64::NEG_INFINITY, f64::max);

            let left_pad = content_min - sg_left;
            let right_pad = sg_right - content_max;

            eprintln!(
                "{}: center={:.1} left_pad={:.1} right_pad={:.1}",
                sg.id, sg.x, left_pad, right_pad
            );
            assert!(
                (left_pad - right_pad).abs() < 1.0,
                "{}: padding asymmetry {:.1} vs {:.1}",
                sg.id,
                left_pad,
                right_pad
            );
        }
    }
}
