use std::collections::BTreeMap;

use rusty_mermaid_core::{
    intersect_rect, Point, SimpleTextMeasure, Style, TextMeasure, TextStyle,
};
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
    let mut g: Graph<NodeLabel, EdgeLabel> = Graph::new();
    let mut id_map: BTreeMap<String, NodeId> = BTreeMap::new();

    // Compute dimensions and add nodes
    let mut class_dims: BTreeMap<String, ClassDims> = BTreeMap::new();
    for c in &diagram.classes {
        let dims = compute_class_dims(c, measurer, &style, line_height);
        let nid = g.add_node(NodeLabel::new(dims.width, dims.height));
        id_map.insert(c.id.clone(), nid);
        class_dims.insert(c.id.clone(), dims);
    }

    // Namespaces as compound nodes
    for ns in &diagram.namespaces {
        let nid = g.add_node(NodeLabel::new(0.0, 0.0));
        id_map.insert(ns.id.clone(), nid);
        for cid in &ns.class_ids {
            if let Some(&child_nid) = id_map.get(cid.as_str()) {
                g.set_parent(child_nid, nid);
            }
        }
    }

    // Add edges
    for rel in &diagram.relationships {
        let Some(&src) = id_map.get(&rel.from_id) else { continue };
        let Some(&dst) = id_map.get(&rel.to_id) else { continue };
        let mut label = EdgeLabel::default();
        if let Some(text) = &rel.label {
            let ts = measurer.measure(text, &style);
            label.width = ts.width;
            label.height = ts.height;
        }
        g.add_edge(src, dst, label);
    }

    // Run dagre
    let config = DagreConfig {
        rankdir: diagram.direction,
        ..Default::default()
    };
    rusty_mermaid_dagre::pipeline::layout(&mut g, &config);

    // Resolve styles
    let entities = diagram.classes.iter().map(|c| (c.id.as_str(), c.css_classes.as_slice()));
    let node_styles = resolve_entity_styles(
        entities,
        &diagram.class_defs,
        &diagram.style_stmts,
    );

    // Extract positioned classes
    let mut classes = Vec::new();
    let mut max_x: f64 = 0.0;
    let mut max_y: f64 = 0.0;

    for c in &diagram.classes {
        let Some(&nid) = id_map.get(&c.id) else { continue };
        let Some(n) = g.node(nid) else { continue };
        let dims = &class_dims[&c.id];

        classes.push(ClassLayout {
            id: c.id.clone(),
            label: c.label.clone().unwrap_or_else(|| c.id.clone()),
            generic_type: c.generic_type.clone(),
            annotations: c.annotations.clone(),
            members: c.members.clone(),
            methods: c.methods.clone(),
            x: n.x,
            y: n.y,
            width: n.width.max(dims.width),
            height: n.height.max(dims.height),
            title_height: dims.title_height,
            members_height: dims.members_height,
            methods_height: dims.methods_height,
            custom_style: node_styles.get(c.id.as_str()).cloned(),
        });
        max_x = max_x.max(n.x + n.width / 2.0);
        max_y = max_y.max(n.y + n.height / 2.0);
    }

    // Extract edges with intersection routing
    // Note: start_arrow/end_arrow are None — class relationship markers are
    // handled by the renderer via ClassEdgeLayout.from_type/to_type → MarkerType.
    let mut edges = Vec::new();
    for rel in &diagram.relationships {
        let Some(&src_nid) = id_map.get(&rel.from_id) else { continue };
        let Some(&dst_nid) = id_map.get(&rel.to_id) else { continue };

        // Find the dagre edge
        let edge_id = g.edge_ids()
            .find(|&eid| g.edge_endpoints(eid).is_some_and(|(s, d)| s == src_nid && d == dst_nid));
        let Some(eid) = edge_id else { continue };
        let Some(edge_label) = g.edge(eid) else { continue };

        let mut points = edge_label.points.clone();
        if points.is_empty() {
            let (Some(src_n), Some(dst_n)) = (g.node(src_nid), g.node(dst_nid)) else { continue };
            points = vec![Point::new(src_n.x, src_n.y), Point::new(dst_n.x, dst_n.y)];
        }

        // Clip endpoints at node boundaries
        if let Some(src_n) = g.node(src_nid) {
            let bbox = rusty_mermaid_core::BBox::new(src_n.x, src_n.y, src_n.width, src_n.height);
            if points.len() >= 2 {
                points[0] = intersect_rect(&bbox, points[1]);
            }
        }
        if let Some(dst_n) = g.node(dst_nid) {
            let bbox = rusty_mermaid_core::BBox::new(dst_n.x, dst_n.y, dst_n.width, dst_n.height);
            let n = points.len();
            if n >= 2 {
                points[n - 1] = intersect_rect(&bbox, points[n - 2]);
            }
        }

        let label_size = rel.label.as_ref().map(|l| {
            let ts = measurer.measure(l, &style);
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

    // Namespace positions
    let mut namespaces = Vec::new();
    for ns in &diagram.namespaces {
        if let Some(&nid) = id_map.get(&ns.id) {
            if let Some(n) = g.node(nid) {
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

    LayoutResult {
        classes,
        edges,
        namespaces,
        width: max_x,
        height: max_y,
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
    if let Some(g) = &class.generic_type {
        title_w += measurer.measure(&format!("<{g}>"), style).width;
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
    let member_widths = class.members.iter()
        .map(|m| measurer.measure(&m.display_text(), style).width)
        .fold(0.0f64, f64::max);
    let method_widths = class.methods.iter()
        .map(|m| measurer.measure(&m.display_text(), style).width)
        .fold(0.0f64, f64::max);
    // Measure annotation at its actual render size (font_size_small = 11px).
    // Uses measure_raw to avoid strip_markup eating <<>> as HTML tags.
    let small_style = TextStyle { font_size: 11.0, ..style.clone() };
    let annotation_w = class.annotations.first()
        .map(|a| SimpleTextMeasure::measure_raw(&format!("<<{a}>>"), &small_style).width)
        .unwrap_or(0.0);

    let content_w = title_w.max(member_widths).max(method_widths).max(annotation_w);
    let width = (content_w + PADDING_X * 2.0).max(MIN_CLASS_WIDTH);
    let height = annotation_height + title_height + members_height + methods_height;

    ClassDims { width, height, title_height: annotation_height + title_height, members_height, methods_height }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_and_layout(input: &str) -> LayoutResult {
        let diagram = super::super::parser::parse(input).unwrap();
        layout(&diagram)
    }

    #[test]
    fn single_class_produces_one_node() {
        let r = parse_and_layout("classDiagram\n    class Animal");
        assert_eq!(r.classes.len(), 1);
        assert_eq!(r.classes[0].id, "Animal");
        assert!(r.classes[0].width > 0.0);
        assert!(r.classes[0].height > 0.0);
    }

    #[test]
    fn two_classes_with_relationship() {
        let r = parse_and_layout("classDiagram\n    Animal <|-- Dog");
        assert_eq!(r.classes.len(), 2);
        assert_eq!(r.edges.len(), 1);
        assert_eq!(r.edges[0].from_type, Some(RelationType::Extension));
    }

    #[test]
    fn class_with_members_taller() {
        let r1 = parse_and_layout("classDiagram\n    class Foo");
        let r2 = parse_and_layout("classDiagram\n    class Foo {\n        +a\n        +b\n        +c\n    }");
        assert!(r2.classes[0].height > r1.classes[0].height,
            "class with 3 members should be taller");
    }

    #[test]
    fn class_with_methods_taller() {
        let r1 = parse_and_layout("classDiagram\n    class Foo");
        let r2 = parse_and_layout("classDiagram\n    class Foo {\n        +doA()\n        +doB()\n    }");
        assert!(r2.classes[0].height > r1.classes[0].height,
            "class with 2 methods should be taller");
    }

    #[test]
    fn min_width_enforced() {
        let r = parse_and_layout("classDiagram\n    class A");
        assert!(r.classes[0].width >= MIN_CLASS_WIDTH);
    }

    #[test]
    fn namespace_produces_compound() {
        let r = parse_and_layout("classDiagram\n    namespace MyApp {\n        class User\n        class Admin\n    }");
        assert_eq!(r.classes.len(), 2);
        assert_eq!(r.namespaces.len(), 1);
        assert_eq!(r.namespaces[0].id, "MyApp");
    }

    #[test]
    fn edge_routing_clips_at_boundary() {
        let r = parse_and_layout("classDiagram\n    A <|-- B");
        let edge = &r.edges[0].edge;
        // Edge points should not be at the center of nodes
        let a = &r.classes.iter().find(|c| c.id == "A").unwrap();
        let first = edge.points.first().unwrap();
        let at_center = (first.x - a.x).abs() < 0.1 && (first.y - a.y).abs() < 0.1;
        assert!(!at_center, "edge start should be clipped to node boundary, not center");
    }

    #[test]
    fn section_heights_computed() {
        let r = parse_and_layout("classDiagram\n    class Foo {\n        +field1\n        +method1()\n    }");
        let c = &r.classes[0];
        assert!(c.title_height > 0.0);
        assert!(c.members_height > 0.0);
        assert!(c.methods_height > 0.0);
        let total = c.title_height + c.members_height + c.methods_height;
        assert!((total - c.height).abs() < 1.0, "section heights should sum to total height");
    }

    #[test]
    fn annotation_widens_box() {
        let r = parse_and_layout("classDiagram\n    class Color {\n        <<enumeration>>\n        RED\n    }");
        let c = &r.classes[0];
        // <<enumeration>> (17 chars) should force box wider than just "Color" (5 chars) + "RED" (3 chars)
        let measurer = SimpleTextMeasure::default();
        let style = TextStyle::default();
        let ann_w = measurer.measure("<<enumeration>>", &style).width;
        assert!(c.width >= ann_w, "box width {} should contain annotation width {ann_w}", c.width);
    }

    #[test]
    fn cardinality_preserved() {
        let r = parse_and_layout("classDiagram\n    A \"1\" *-- \"many\" B : has");
        assert_eq!(r.edges[0].cardinality_from.as_deref(), Some("1"));
        assert_eq!(r.edges[0].cardinality_to.as_deref(), Some("many"));
    }

    #[test]
    fn direction_lr() {
        let r = parse_and_layout("classDiagram\n    direction LR\n    A <|-- B");
        // In LR layout, nodes should be side by side (x differs more than y)
        let a = r.classes.iter().find(|c| c.id == "A").unwrap();
        let b = r.classes.iter().find(|c| c.id == "B").unwrap();
        let dx = (a.x - b.x).abs();
        let dy = (a.y - b.y).abs();
        assert!(dx > dy, "LR layout: horizontal distance ({dx}) should exceed vertical ({dy})");
    }
}
