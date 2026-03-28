use std::collections::BTreeMap;

use rusty_mermaid_core::{Point, SimpleTextMeasure, Style, TextMeasure, TextStyle, intersect_rect};
use rusty_mermaid_dagre::{DagreConfig, EdgeLabel, NodeLabel};
use rusty_mermaid_graph::{Graph, NodeId};

use crate::common::layout::EdgeLayout;

use super::ir::*;

const PADDING_X: f64 = 16.0;
const PADDING_Y: f64 = 8.0;
const LINE_GAP: f64 = 4.0;
const MIN_NODE_WIDTH: f64 = 120.0;

type NodeInfoMap = BTreeMap<String, (String, Vec<String>)>;

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

pub fn layout_with_measurer(
    diagram: &RequirementDiagram,
    measurer: &impl TextMeasure,
) -> LayoutResult {
    let style = TextStyle::default();
    let line_height = measurer.measure("X", &style).height;
    let (mut graph, id_map, node_infos) = build_req_graph(diagram, measurer, &style, line_height);

    let config = DagreConfig {
        rankdir: diagram.direction,
        ..Default::default()
    };
    rusty_mermaid_dagre::pipeline::layout(&mut graph, &config);

    let (nodes, max_x, max_y) = extract_req_nodes(&graph, &id_map, &node_infos);
    let edges = extract_req_edges(diagram, &graph, &id_map, &style);

    LayoutResult {
        nodes,
        edges,
        width: max_x,
        height: max_y,
    }
}

fn measure_req_box(
    name: &str,
    lines: &[String],
    style: &TextStyle,
    line_height: f64,
) -> (f64, f64) {
    let max_line_w = lines
        .iter()
        .map(|l| SimpleTextMeasure::measure_raw(l, style).width)
        .fold(0.0f64, f64::max);
    let name_w = SimpleTextMeasure::measure_raw(name, style).width;
    let content_w = max_line_w.max(name_w);
    let width = (content_w + PADDING_X * 2.0).max(MIN_NODE_WIDTH);
    let height = (lines.len() as f64 + 1.0) * (line_height + LINE_GAP) + PADDING_Y * 2.0;
    (width, height)
}

fn build_req_graph(
    diagram: &RequirementDiagram,
    measurer: &impl TextMeasure,
    style: &TextStyle,
    line_height: f64,
) -> (
    Graph<NodeLabel, EdgeLabel>,
    BTreeMap<String, NodeId>,
    NodeInfoMap,
) {
    let mut graph: Graph<NodeLabel, EdgeLabel> = Graph::new();
    let mut id_map: BTreeMap<String, NodeId> = BTreeMap::new();
    let mut node_infos: NodeInfoMap = BTreeMap::new();

    for req in &diagram.requirements {
        let mut lines = Vec::new();
        lines.push(format!("<<{}>>", req.display_type()));
        if let Some(id) = &req.id {
            lines.push(format!("Id: {id}"));
        }
        if let Some(text) = &req.text {
            lines.push(format!("Text: {text}"));
        }
        if let Some(risk) = &req.risk {
            lines.push(format!("Risk: {}", risk_label(*risk)));
        }
        if let Some(vm) = &req.verify_method {
            lines.push(format!("Verify: {}", verify_label(*vm)));
        }

        let (width, height) = measure_req_box(&req.name, &lines, style, line_height);
        let nid = graph.add_node(NodeLabel::new(width, height));
        id_map.insert(req.name.clone(), nid);
        node_infos.insert(req.name.clone(), (req.name.clone(), lines));
    }

    for elem in &diagram.elements {
        let mut lines = Vec::new();
        lines.push("<<Element>>".to_string());
        if let Some(t) = &elem.elem_type {
            lines.push(format!("Type: {t}"));
        }
        if let Some(d) = &elem.docref {
            lines.push(format!("Doc: {d}"));
        }

        let (width, height) = measure_req_box(&elem.name, &lines, style, line_height);
        let nid = graph.add_node(NodeLabel::new(width, height));
        id_map.insert(elem.name.clone(), nid);
        node_infos.insert(elem.name.clone(), (elem.name.clone(), lines));
    }

    for rel in &diagram.relationships {
        let Some(&src) = id_map.get(&rel.src) else {
            continue;
        };
        let Some(&dst) = id_map.get(&rel.dst) else {
            continue;
        };
        let mut label = EdgeLabel::default();
        let label_text = format!("<<{}>>", rel.rel_type.label());
        let ts = measurer.measure(&label_text, style);
        label.width = ts.width;
        label.height = ts.height;
        graph.add_edge(src, dst, label);
    }

    (graph, id_map, node_infos)
}

fn extract_req_nodes(
    graph: &Graph<NodeLabel, EdgeLabel>,
    id_map: &BTreeMap<String, NodeId>,
    node_infos: &BTreeMap<String, (String, Vec<String>)>,
) -> (Vec<ReqNodeLayout>, f64, f64) {
    let mut nodes = Vec::new();
    let mut max_x: f64 = 0.0;
    let mut max_y: f64 = 0.0;

    for (name, (type_label, lines)) in node_infos {
        let Some(&nid) = id_map.get(name) else {
            continue;
        };
        let Some(n) = graph.node(nid) else { continue };
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

    (nodes, max_x, max_y)
}

fn extract_req_edges(
    diagram: &RequirementDiagram,
    graph: &Graph<NodeLabel, EdgeLabel>,
    id_map: &BTreeMap<String, NodeId>,
    style: &TextStyle,
) -> Vec<ReqEdgeLayout> {
    let mut edges = Vec::new();
    for rel in &diagram.relationships {
        let Some(&src_nid) = id_map.get(&rel.src) else {
            continue;
        };
        let Some(&dst_nid) = id_map.get(&rel.dst) else {
            continue;
        };

        let edge_id = graph.edge_ids().find(|&eid| {
            graph
                .edge_endpoints(eid)
                .is_some_and(|(s, d)| s == src_nid && d == dst_nid)
        });
        let Some(eid) = edge_id else { continue };
        let Some(edge_label) = graph.edge(eid) else {
            continue;
        };

        let mut points = edge_label.points.clone();
        if points.is_empty() {
            let (Some(src_n), Some(dst_n)) = (graph.node(src_nid), graph.node(dst_nid)) else {
                continue;
            };
            points = vec![Point::new(src_n.x, src_n.y), Point::new(dst_n.x, dst_n.y)];
        }

        clip_req_edge_endpoints(graph, src_nid, dst_nid, &mut points);

        let label_text = format!("<<{}>>", rel.rel_type.label());
        let label_style = TextStyle {
            font_size: rusty_mermaid_core::Theme::default().font_size_edge_label,
            ..style.clone()
        };
        let ts = SimpleTextMeasure::measure_raw(&label_text, &label_style);
        let label_size = Some((ts.width, ts.height));

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
    edges
}

fn clip_req_edge_endpoints(
    graph: &Graph<NodeLabel, EdgeLabel>,
    src_nid: NodeId,
    dst_nid: NodeId,
    points: &mut [Point],
) {
    if let Some(src_n) = graph.node(src_nid) {
        let bbox = rusty_mermaid_core::BBox::new(src_n.x, src_n.y, src_n.width, src_n.height);
        if points.len() >= 2 {
            points[0] = intersect_rect(&bbox, points[1]);
        }
    }
    if let Some(dst_n) = graph.node(dst_nid) {
        let bbox = rusty_mermaid_core::BBox::new(dst_n.x, dst_n.y, dst_n.width, dst_n.height);
        let n = points.len();
        if n >= 2 {
            points[n - 1] = intersect_rect(&bbox, points[n - 2]);
        }
    }
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
        let r =
            parse_and_layout("requirementDiagram\n    requirement REQ {\n        id: R1\n    }");
        assert_eq!(r.nodes.len(), 1);
        assert!(r.nodes[0].width > 0.0);
    }

    #[test]
    fn requirement_and_element() {
        let r = parse_and_layout(
            "requirementDiagram\n    requirement REQ {\n        id: R1\n    }\n    element COMP {\n        type: Module\n    }\n    REQ - traces -> COMP",
        );
        assert_eq!(r.nodes.len(), 2);
        assert_eq!(r.edges.len(), 1);
    }

    #[test]
    fn contains_is_solid() {
        let r = parse_and_layout(
            "requirementDiagram\n    requirement A {\n        id: A\n    }\n    requirement B {\n        id: B\n    }\n    A - contains -> B",
        );
        assert_eq!(
            r.edges[0].edge.stroke,
            crate::common::layout::StrokeType::Normal
        );
    }

    #[test]
    fn satisfies_is_dashed() {
        let r = parse_and_layout(
            "requirementDiagram\n    requirement A {\n        id: A\n    }\n    requirement B {\n        id: B\n    }\n    A - satisfies -> B",
        );
        assert_eq!(
            r.edges[0].edge.stroke,
            crate::common::layout::StrokeType::Dotted
        );
    }
}
