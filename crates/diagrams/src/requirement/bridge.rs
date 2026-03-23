use std::collections::BTreeMap;

use rusty_mermaid_core::{
    intersect_rect, Direction, Point, SimpleTextMeasure, Style, TextMeasure, TextStyle,
};
use rusty_mermaid_dagre::{DagreConfig, EdgeLabel, NodeLabel};
use rusty_mermaid_graph::{Graph, NodeId};

use crate::common::layout::EdgeLayout;

use super::ir::*;

const PADDING_X: f64 = 16.0;
const PADDING_Y: f64 = 8.0;
const LINE_GAP: f64 = 4.0;
const MIN_NODE_WIDTH: f64 = 120.0;

/// Layout result for requirement diagrams.
#[derive(Debug)]
pub struct LayoutResult {
    pub nodes: Vec<ReqNodeLayout>,
    pub edges: Vec<ReqEdgeLayout>,
    pub width: f64,
    pub height: f64,
}

/// A positioned requirement or element box.
#[derive(Debug)]
pub struct ReqNodeLayout {
    pub name: String,
    pub type_label: String,
    pub lines: Vec<String>,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub custom_style: Option<Style>,
}

/// A positioned relationship edge.
#[derive(Debug)]
pub struct ReqEdgeLayout {
    pub edge: EdgeLayout,
    pub rel_type: RelationshipType,
}

pub fn layout(diagram: &RequirementDiagram) -> LayoutResult {
    layout_with_measurer(diagram, &SimpleTextMeasure::default())
}

pub fn layout_with_measurer(diagram: &RequirementDiagram, measurer: &impl TextMeasure) -> LayoutResult {
    let style = TextStyle::default();
    let line_height = measurer.measure("X", &style).1;
    let mut g: Graph<NodeLabel, EdgeLabel> = Graph::new();
    let mut id_map: BTreeMap<String, NodeId> = BTreeMap::new();
    let mut node_infos: BTreeMap<String, (String, Vec<String>)> = BTreeMap::new();

    // Add requirement nodes
    for req in &diagram.requirements {
        let mut lines = Vec::new();
        lines.push(format!("<<{}>>", req.display_type()));
        if let Some(id) = &req.id { lines.push(format!("Id: {id}")); }
        if let Some(text) = &req.text { lines.push(format!("Text: {text}")); }
        if let Some(risk) = &req.risk { lines.push(format!("Risk: {}", risk_label(*risk))); }
        if let Some(vm) = &req.verify_method { lines.push(format!("Verify: {}", verify_label(*vm))); }

        let max_line_w = lines.iter()
            .map(|l| measurer.measure(l, &style).0)
            .fold(0.0f64, f64::max);
        let name_w = measurer.measure(&req.name, &style).0;
        let content_w = max_line_w.max(name_w);
        let width = (content_w + PADDING_X * 2.0).max(MIN_NODE_WIDTH);
        let height = (lines.len() as f64 + 1.0) * (line_height + LINE_GAP) + PADDING_Y * 2.0;

        let nid = g.add_node(NodeLabel::new(width, height));
        id_map.insert(req.name.clone(), nid);
        node_infos.insert(req.name.clone(), (req.name.clone(), lines));
    }

    // Add element nodes
    for elem in &diagram.elements {
        let mut lines = Vec::new();
        lines.push("<<Element>>".to_string());
        if let Some(t) = &elem.elem_type { lines.push(format!("Type: {t}")); }
        if let Some(d) = &elem.docref { lines.push(format!("Doc: {d}")); }

        let max_line_w = lines.iter()
            .map(|l| measurer.measure(l, &style).0)
            .fold(0.0f64, f64::max);
        let name_w = measurer.measure(&elem.name, &style).0;
        let content_w = max_line_w.max(name_w);
        let width = (content_w + PADDING_X * 2.0).max(MIN_NODE_WIDTH);
        let height = (lines.len() as f64 + 1.0) * (line_height + LINE_GAP) + PADDING_Y * 2.0;

        let nid = g.add_node(NodeLabel::new(width, height));
        id_map.insert(elem.name.clone(), nid);
        node_infos.insert(elem.name.clone(), (elem.name.clone(), lines));
    }

    // Add edges
    for rel in &diagram.relationships {
        let Some(&src) = id_map.get(&rel.src) else { continue };
        let Some(&dst) = id_map.get(&rel.dst) else { continue };
        let mut label = EdgeLabel::default();
        let label_text = format!("<<{}>>", rel.rel_type.label());
        let (tw, th) = measurer.measure(&label_text, &style);
        label.width = tw;
        label.height = th;
        g.add_edge(src, dst, label);
    }

    let config = DagreConfig {
        rankdir: diagram.direction,
        ..Default::default()
    };
    rusty_mermaid_dagre::pipeline::layout(&mut g, &config);

    // Extract nodes
    let mut nodes = Vec::new();
    let mut max_x: f64 = 0.0;
    let mut max_y: f64 = 0.0;

    for (name, (type_label, lines)) in &node_infos {
        let Some(&nid) = id_map.get(name) else { continue };
        let Some(n) = g.node(nid) else { continue };
        nodes.push(ReqNodeLayout {
            name: name.clone(),
            type_label: type_label.clone(),
            lines: lines.clone(),
            x: n.x,
            y: n.y,
            width: n.width,
            height: n.height,
            custom_style: None,
        });
        max_x = max_x.max(n.x + n.width / 2.0);
        max_y = max_y.max(n.y + n.height / 2.0);
    }

    // Extract edges
    let mut edges = Vec::new();
    for rel in &diagram.relationships {
        let Some(&src_nid) = id_map.get(&rel.src) else { continue };
        let Some(&dst_nid) = id_map.get(&rel.dst) else { continue };

        let edge_id = g.edge_ids()
            .find(|&eid| g.edge_endpoints(eid).is_some_and(|(s, d)| s == src_nid && d == dst_nid));
        let Some(eid) = edge_id else { continue };
        let Some(edge_label) = g.edge(eid) else { continue };

        let mut points = edge_label.points.clone();
        if points.is_empty() {
            let src_n = g.node(src_nid).unwrap();
            let dst_n = g.node(dst_nid).unwrap();
            points = vec![Point::new(src_n.x, src_n.y), Point::new(dst_n.x, dst_n.y)];
        }

        if let Some(src_n) = g.node(src_nid) {
            let bbox = rusty_mermaid_core::BBox::new(src_n.x, src_n.y, src_n.width, src_n.height);
            if points.len() >= 2 { points[0] = intersect_rect(&bbox, points[1]); }
        }
        if let Some(dst_n) = g.node(dst_nid) {
            let bbox = rusty_mermaid_core::BBox::new(dst_n.x, dst_n.y, dst_n.width, dst_n.height);
            let n = points.len();
            if n >= 2 { points[n - 1] = intersect_rect(&bbox, points[n - 2]); }
        }

        let label_text = format!("<<{}>>", rel.rel_type.label());
        let label_size = Some(measurer.measure(&label_text, &style));

        edges.push(ReqEdgeLayout {
            edge: EdgeLayout {
                src: rel.src.clone(),
                dst: rel.dst.clone(),
                points,
                label: Some(label_text),
                label_size,
                stroke: if rel.rel_type.is_dashed() {
                    crate::common::layout::StrokeType::Dotted
                } else {
                    crate::common::layout::StrokeType::Normal
                },
                start_arrow: crate::common::layout::ArrowEnd::None,
                end_arrow: crate::common::layout::ArrowEnd::Arrow,
                custom_style: None,
            },
            rel_type: rel.rel_type,
        });
    }

    LayoutResult { nodes, edges, width: max_x, height: max_y }
}

fn risk_label(r: RiskLevel) -> &'static str {
    match r {
        RiskLevel::Low => "Low",
        RiskLevel::Medium => "Medium",
        RiskLevel::High => "High",
    }
}

fn verify_label(v: VerifyMethod) -> &'static str {
    match v {
        VerifyMethod::Analysis => "Analysis",
        VerifyMethod::Demonstration => "Demonstration",
        VerifyMethod::Inspection => "Inspection",
        VerifyMethod::Test => "Test",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_and_layout(input: &str) -> LayoutResult {
        let diagram = super::super::parser::parse(input).unwrap();
        layout(&diagram)
    }

    #[test]
    fn single_requirement() {
        let r = parse_and_layout("requirementDiagram\n    requirement REQ {\n        id: R1\n    }");
        assert_eq!(r.nodes.len(), 1);
        assert!(r.nodes[0].width > 0.0);
    }

    #[test]
    fn requirement_and_element() {
        let r = parse_and_layout("requirementDiagram\n    requirement REQ {\n        id: R1\n    }\n    element COMP {\n        type: Module\n    }\n    REQ - traces -> COMP");
        assert_eq!(r.nodes.len(), 2);
        assert_eq!(r.edges.len(), 1);
    }

    #[test]
    fn contains_is_solid() {
        let r = parse_and_layout("requirementDiagram\n    requirement A {\n        id: A\n    }\n    requirement B {\n        id: B\n    }\n    A - contains -> B");
        assert_eq!(r.edges[0].edge.stroke, crate::common::layout::StrokeType::Normal);
    }

    #[test]
    fn satisfies_is_dashed() {
        let r = parse_and_layout("requirementDiagram\n    requirement A {\n        id: A\n    }\n    requirement B {\n        id: B\n    }\n    A - satisfies -> B");
        assert_eq!(r.edges[0].edge.stroke, crate::common::layout::StrokeType::Dotted);
    }
}
