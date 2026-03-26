use std::collections::BTreeMap;

use rusty_mermaid_core::{Point, SimpleTextMeasure, Style, TextMeasure, TextStyle, intersect_rect};
use rusty_mermaid_dagre::{DagreConfig, EdgeLabel, NodeLabel};
use rusty_mermaid_graph::{Graph, NodeId};

use crate::common::layout::EdgeLayout;
use crate::common::rendering::resolve_entity_styles;

use super::ir::*;

const PADDING_X: f64 = 16.0;
const TITLE_PADDING_Y: f64 = 8.0;
const SECTION_GAP: f64 = 4.0;
const MIN_CLASS_WIDTH: f64 = 80.0;

/// Layout result for class diagrams.
#[derive(Debug)]
pub struct LayoutResult {
    pub classes: Vec<ClassLayout>,
    pub edges: Vec<ClassEdgeLayout>,
    pub namespaces: Vec<NamespaceLayout>,
    pub width: f64,
    pub height: f64,
}

/// A positioned class box with computed section heights.
#[derive(Debug)]
pub struct ClassLayout {
    pub id: String,
    pub label: String,
    pub generic_type: Option<String>,
    pub annotations: Vec<String>,
    pub members: Vec<ClassMember>,
    pub methods: Vec<ClassMember>,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub title_height: f64,
    pub members_height: f64,
    pub methods_height: f64,
    pub custom_style: Option<Style>,
}

/// A positioned relationship edge.
#[derive(Debug)]
pub struct ClassEdgeLayout {
    pub edge: EdgeLayout,
    pub from_type: Option<RelationType>,
    pub to_type: Option<RelationType>,
    pub cardinality_from: Option<String>,
    pub cardinality_to: Option<String>,
}

/// A positioned namespace container.
#[derive(Debug)]
pub struct NamespaceLayout {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Layout with default text measurer.
pub fn layout(diagram: &ClassDiagram) -> LayoutResult {
    layout_with_measurer(diagram, &SimpleTextMeasure::default())
}

/// Layout with custom text measurer.
pub fn layout_with_measurer(diagram: &ClassDiagram, measurer: &impl TextMeasure) -> LayoutResult {
    let style = TextStyle::default();
    let line_height = measurer.measure("X", &style).height;
    let (mut graph, id_map, class_dims) = build_class_graph(diagram, measurer, &style, line_height);

    let config = DagreConfig {
        rankdir: diagram.direction,
        ..Default::default()
    };
    rusty_mermaid_dagre::pipeline::layout(&mut graph, &config);

    let entities = diagram
        .classes
        .iter()
        .map(|c| (c.id.as_str(), c.css_classes.as_slice()));
    let node_styles = resolve_entity_styles(entities, &diagram.class_defs, &diagram.style_stmts);

    let (classes, mut max_x, mut max_y) =
        extract_class_layouts(diagram, &graph, &id_map, &class_dims, &node_styles);
    let edges = extract_class_edges(diagram, &graph, &id_map, measurer, &style);
    let (namespaces, ns_max_x, ns_max_y) = extract_namespace_layouts(diagram, &graph, &id_map);
    max_x = max_x.max(ns_max_x);
    max_y = max_y.max(ns_max_y);

    LayoutResult {
        classes,
        edges,
        namespaces,
        width: max_x,
        height: max_y,
    }
}

fn build_class_graph(
    diagram: &ClassDiagram,
    measurer: &impl TextMeasure,
    style: &TextStyle,
    line_height: f64,
) -> (
    Graph<NodeLabel, EdgeLabel>,
    BTreeMap<String, NodeId>,
    BTreeMap<String, ClassDims>,
) {
    let mut graph: Graph<NodeLabel, EdgeLabel> = Graph::new();
    let mut id_map: BTreeMap<String, NodeId> = BTreeMap::new();
    let mut class_dims: BTreeMap<String, ClassDims> = BTreeMap::new();

    for class in &diagram.classes {
        let dims = compute_class_dims(class, measurer, style, line_height);
        let nid = graph.add_node(NodeLabel::new(dims.width, dims.height));
        id_map.insert(class.id.clone(), nid);
        class_dims.insert(class.id.clone(), dims);
    }

    // Namespaces as compound nodes
    for ns in &diagram.namespaces {
        let nid = graph.add_node(NodeLabel::new(0.0, 0.0));
        id_map.insert(ns.id.clone(), nid);
        for cid in &ns.class_ids {
            if let Some(&child_nid) = id_map.get(cid.as_str()) {
                graph.set_parent(child_nid, nid);
            }
        }
    }

    for rel in &diagram.relationships {
        let Some(&src) = id_map.get(&rel.from_id) else {
            continue;
        };
        let Some(&dst) = id_map.get(&rel.to_id) else {
            continue;
        };
        let mut label = EdgeLabel::default();
        if let Some(text) = &rel.label {
            let ts = measurer.measure(text, style);
            label.width = ts.width;
            label.height = ts.height;
        }
        graph.add_edge(src, dst, label);
    }

    (graph, id_map, class_dims)
}

fn extract_class_layouts(
    diagram: &ClassDiagram,
    graph: &Graph<NodeLabel, EdgeLabel>,
    id_map: &BTreeMap<String, NodeId>,
    class_dims: &BTreeMap<String, ClassDims>,
    node_styles: &BTreeMap<&str, Style>,
) -> (Vec<ClassLayout>, f64, f64) {
    let mut classes = Vec::new();
    let mut max_x: f64 = 0.0;
    let mut max_y: f64 = 0.0;

    for class in &diagram.classes {
        let Some(&nid) = id_map.get(&class.id) else {
            continue;
        };
        let Some(n) = graph.node(nid) else { continue };
        let dims = &class_dims[&class.id];

        classes.push(ClassLayout {
            id: class.id.clone(),
            label: class.label.clone().unwrap_or_else(|| class.id.clone()),
            generic_type: class.generic_type.clone(),
            annotations: class.annotations.clone(),
            members: class.members.clone(),
            methods: class.methods.clone(),
            x: n.x,
            y: n.y,
            width: n.width.max(dims.width),
            height: n.height.max(dims.height),
            title_height: dims.title_height,
            members_height: dims.members_height,
            methods_height: dims.methods_height,
            custom_style: node_styles.get(class.id.as_str()).cloned(),
        });
        max_x = max_x.max(n.x + n.width / 2.0);
        max_y = max_y.max(n.y + n.height / 2.0);
    }

    (classes, max_x, max_y)
}

fn extract_class_edges(
    diagram: &ClassDiagram,
    graph: &Graph<NodeLabel, EdgeLabel>,
    id_map: &BTreeMap<String, NodeId>,
    measurer: &impl TextMeasure,
    style: &TextStyle,
) -> Vec<ClassEdgeLayout> {
    let mut edges = Vec::new();
    for rel in &diagram.relationships {
        let Some(&src_nid) = id_map.get(&rel.from_id) else {
            continue;
        };
        let Some(&dst_nid) = id_map.get(&rel.to_id) else {
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

        clip_class_edge_endpoints(graph, src_nid, dst_nid, &mut points);

        let label_size = rel.label.as_ref().map(|l| {
            let ts = measurer.measure(l, style);
            (ts.width, ts.height)
        });

        edges.push(ClassEdgeLayout {
            edge: EdgeLayout {
                src: rel.from_id.clone(),
                dst: rel.to_id.clone(),
                points,
                label: rel.label.clone(),
                label_size,
                stroke: match rel.line_type {
                    LineType::Solid => crate::common::layout::StrokeType::Normal,
                    LineType::Dotted => crate::common::layout::StrokeType::Dotted,
                },
                start_arrow: crate::common::layout::ArrowEnd::None,
                end_arrow: crate::common::layout::ArrowEnd::None,
                custom_style: None,
            },
            from_type: rel.from_type,
            to_type: rel.to_type,
            cardinality_from: rel.cardinality_from.clone(),
            cardinality_to: rel.cardinality_to.clone(),
        });
    }
    edges
}

fn extract_namespace_layouts(
    diagram: &ClassDiagram,
    graph: &Graph<NodeLabel, EdgeLabel>,
    id_map: &BTreeMap<String, NodeId>,
) -> (Vec<NamespaceLayout>, f64, f64) {
    let mut namespaces = Vec::new();
    let mut max_x: f64 = 0.0;
    let mut max_y: f64 = 0.0;

    for ns in &diagram.namespaces {
        if let Some(&nid) = id_map.get(&ns.id) {
            if let Some(n) = graph.node(nid) {
                if n.width > 0.0 && n.height > 0.0 {
                    namespaces.push(NamespaceLayout {
                        id: ns.id.clone(),
                        x: n.x,
                        y: n.y,
                        width: n.width,
                        height: n.height,
                    });
                    max_x = max_x.max(n.x + n.width / 2.0);
                    max_y = max_y.max(n.y + n.height / 2.0);
                }
            }
        }
    }

    (namespaces, max_x, max_y)
}

fn clip_class_edge_endpoints(
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

/// Pre-computed dimensions for a class box.
struct ClassDims {
    width: f64,
    height: f64,
    title_height: f64,
    members_height: f64,
    methods_height: f64,
}

fn compute_class_dims(
    class: &ClassNode,
    measurer: &impl TextMeasure,
    style: &TextStyle,
    line_height: f64,
) -> ClassDims {
    // Title: class name + optional generic + optional annotation
    let title_text = class.label.as_deref().unwrap_or(&class.id);
    let mut title_w = measurer.measure(title_text, style).width;
    if let Some(generic) = &class.generic_type {
        title_w += measurer.measure(&format!("<{generic}>"), style).width;
    }
    let title_height = line_height + TITLE_PADDING_Y * 2.0;

    // Annotation line above title (if any)
    let annotation_height = if class.annotations.is_empty() {
        0.0
    } else {
        line_height
    };

    // Members
    let members_height = if class.members.is_empty() {
        SECTION_GAP
    } else {
        class.members.len() as f64 * line_height + SECTION_GAP * 2.0
    };

    // Methods
    let methods_height = if class.methods.is_empty() {
        SECTION_GAP
    } else {
        class.methods.len() as f64 * line_height + SECTION_GAP * 2.0
    };

    // Width: max of all sections
    let member_widths = class
        .members
        .iter()
        .map(|m| measurer.measure(&m.display_text(), style).width)
        .fold(0.0f64, f64::max);
    let method_widths = class
        .methods
        .iter()
        .map(|m| measurer.measure(&m.display_text(), style).width)
        .fold(0.0f64, f64::max);
    // Measure annotation at its actual render size (font_size_small = 11px).
    // Uses measure_raw to avoid strip_markup eating <<>> as HTML tags.
    let small_style = TextStyle {
        font_size: 11.0,
        ..style.clone()
    };
    let annotation_w = class
        .annotations
        .first()
        .map(|a| SimpleTextMeasure::measure_raw(&format!("<<{a}>>"), &small_style).width)
        .unwrap_or(0.0);

    let content_w = title_w
        .max(member_widths)
        .max(method_widths)
        .max(annotation_w);
    let width = (content_w + PADDING_X * 2.0).max(MIN_CLASS_WIDTH);
    let height = annotation_height + title_height + members_height + methods_height;

    ClassDims {
        width,
        height,
        title_height: annotation_height + title_height,
        members_height,
        methods_height,
    }
}

#[cfg(test)]
#[path = "bridge_tests.rs"]
mod bridge_tests;
