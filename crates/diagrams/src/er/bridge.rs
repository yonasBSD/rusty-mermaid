use std::collections::BTreeMap;

use rusty_mermaid_core::{Point, SimpleTextMeasure, Style, TextMeasure, TextStyle, intersect_rect};
use rusty_mermaid_dagre::{DagreConfig, EdgeLabel, NodeLabel};
use rusty_mermaid_graph::{Graph, NodeId};

use crate::common::layout::EdgeLayout;
use crate::common::rendering::resolve_entity_styles;

use super::ir::*;

const PADDING_X: f64 = 16.0;
const ATTR_FONT_SCALE: f64 = 0.85;
const ROW_PADDING: f64 = 4.0;
const MIN_ENTITY_WIDTH: f64 = 100.0;
const TITLE_PADDING_Y: f64 = 10.0;

/// Layout result for ER diagrams.
#[derive(Debug)]
pub struct LayoutResult {
    pub entities: Vec<EntityLayout>,
    pub edges: Vec<ErEdgeLayout>,
    pub width: f64,
    pub height: f64,
}

/// A positioned entity box with computed dimensions.
#[derive(Debug)]
pub struct EntityLayout {
    pub id: String,
    pub display_name: String,
    pub attributes: Vec<Attribute>,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub title_height: f64,
    pub row_height: f64,
    pub custom_style: Option<Style>,
}

/// A positioned relationship edge with cardinality info.
#[derive(Debug)]
pub struct ErEdgeLayout {
    pub edge: EdgeLayout,
    pub cardinality_a: Cardinality,
    pub cardinality_b: Cardinality,
    pub identification: Identification,
}

/// Layout with default text measurer.
pub fn layout(diagram: &ErDiagram) -> LayoutResult {
    layout_with_measurer(diagram, &SimpleTextMeasure::default())
}

/// Layout with custom text measurer.
pub fn layout_with_measurer(diagram: &ErDiagram, measurer: &impl TextMeasure) -> LayoutResult {
    let style = TextStyle::default();
    let line_height = measurer.measure("X", &style).height;
    let row_height = line_height * ATTR_FONT_SCALE + ROW_PADDING * 2.0;
    let (mut graph, id_map, entity_dims) =
        build_er_graph(diagram, measurer, &style, line_height, row_height);

    let config = DagreConfig {
        rankdir: diagram.direction,
        ..Default::default()
    };
    rusty_mermaid_dagre::pipeline::layout(&mut graph, &config);

    let style_entities = diagram
        .entities
        .iter()
        .map(|e| (e.id.as_str(), e.css_classes.as_slice()));
    let node_styles =
        resolve_entity_styles(style_entities, &diagram.class_defs, &diagram.style_stmts);

    let (entities, mut max_x, mut max_y) = extract_er_entities(
        diagram,
        &graph,
        &id_map,
        &entity_dims,
        &node_styles,
        row_height,
    );
    let edges = extract_er_edges(diagram, &graph, &id_map, measurer, &style);

    for edge in &edges {
        for pt in &edge.edge.points {
            max_x = max_x.max(pt.x);
            max_y = max_y.max(pt.y);
        }
    }

    LayoutResult {
        entities,
        edges,
        width: max_x,
        height: max_y,
    }
}

fn build_er_graph(
    diagram: &ErDiagram,
    measurer: &impl TextMeasure,
    style: &TextStyle,
    line_height: f64,
    row_height: f64,
) -> (
    Graph<NodeLabel, EdgeLabel>,
    BTreeMap<String, NodeId>,
    BTreeMap<String, EntityDims>,
) {
    let mut graph: Graph<NodeLabel, EdgeLabel> = Graph::new();
    let mut id_map: BTreeMap<String, NodeId> = BTreeMap::new();
    let mut entity_dims: BTreeMap<String, EntityDims> = BTreeMap::new();

    for entity in &diagram.entities {
        let dims = compute_entity_dims(entity, measurer, style, line_height, row_height);
        let nid = graph.add_node(NodeLabel::new(dims.width, dims.height));
        id_map.insert(entity.id.clone(), nid);
        entity_dims.insert(entity.id.clone(), dims);
    }

    for rel in &diagram.relationships {
        let Some(&src) = id_map.get(&rel.entity_a) else {
            continue;
        };
        let Some(&dst) = id_map.get(&rel.entity_b) else {
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

    (graph, id_map, entity_dims)
}

fn extract_er_entities(
    diagram: &ErDiagram,
    graph: &Graph<NodeLabel, EdgeLabel>,
    id_map: &BTreeMap<String, NodeId>,
    entity_dims: &BTreeMap<String, EntityDims>,
    node_styles: &BTreeMap<&str, Style>,
    row_height: f64,
) -> (Vec<EntityLayout>, f64, f64) {
    let mut entities = Vec::new();
    let mut max_x: f64 = 0.0;
    let mut max_y: f64 = 0.0;

    for entity in &diagram.entities {
        let Some(&nid) = id_map.get(&entity.id) else {
            continue;
        };
        let Some(n) = graph.node(nid) else { continue };
        let dims = &entity_dims[&entity.id];

        entities.push(EntityLayout {
            id: entity.id.clone(),
            display_name: entity.display_name().to_string(),
            attributes: entity.attributes.clone(),
            x: n.x,
            y: n.y,
            width: n.width.max(dims.width),
            height: n.height.max(dims.height),
            title_height: dims.title_height,
            row_height,
            custom_style: node_styles.get(entity.id.as_str()).cloned(),
        });
        max_x = max_x.max(n.x + n.width / 2.0);
        max_y = max_y.max(n.y + n.height / 2.0);
    }

    (entities, max_x, max_y)
}

fn extract_er_edges(
    diagram: &ErDiagram,
    graph: &Graph<NodeLabel, EdgeLabel>,
    id_map: &BTreeMap<String, NodeId>,
    measurer: &impl TextMeasure,
    style: &TextStyle,
) -> Vec<ErEdgeLayout> {
    let mut edges = Vec::new();
    for rel in &diagram.relationships {
        let Some(&src_nid) = id_map.get(&rel.entity_a) else {
            continue;
        };
        let Some(&dst_nid) = id_map.get(&rel.entity_b) else {
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

        clip_edge_endpoints(graph, src_nid, dst_nid, &mut points);

        let label_size = rel.label.as_ref().map(|l| {
            let ts = measurer.measure(l, style);
            (ts.width, ts.height)
        });

        edges.push(ErEdgeLayout {
            edge: EdgeLayout {
                src: rel.entity_a.clone(),
                dst: rel.entity_b.clone(),
                points,
                label: rel.label.clone(),
                label_size,
                stroke: match rel.identification {
                    Identification::Identifying => crate::common::layout::StrokeType::Normal,
                    Identification::NonIdentifying => crate::common::layout::StrokeType::Dotted,
                },
                start_arrow: crate::common::layout::ArrowEnd::None,
                end_arrow: crate::common::layout::ArrowEnd::None,
                custom_style: None,
            },
            cardinality_a: rel.cardinality_a,
            cardinality_b: rel.cardinality_b,
            identification: rel.identification,
        });
    }
    edges
}

fn clip_edge_endpoints(
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

struct EntityDims {
    width: f64,
    height: f64,
    title_height: f64,
}

fn compute_entity_dims(
    entity: &Entity,
    measurer: &impl TextMeasure,
    style: &TextStyle,
    line_height: f64,
    row_height: f64,
) -> EntityDims {
    let title_w = measurer.measure(entity.display_name(), style).width;
    let title_height = line_height + TITLE_PADDING_Y * 2.0;

    // Attribute row widths: type + name + keys + comment
    let attr_style = TextStyle {
        font_size: style.font_size * ATTR_FONT_SCALE,
        ..style.clone()
    };
    let attr_max_w = entity
        .attributes
        .iter()
        .map(|a| {
            let mut text = format!("{} {}", a.attr_type, a.name);
            if !a.keys.is_empty() {
                let keys: Vec<&str> = a.keys.iter().map(|k| k.label()).collect();
                text.push_str(&format!(" {}", keys.join(",")));
            }
            if let Some(c) = &a.comment {
                text.push_str(&format!(" \"{}\"", c));
            }
            measurer.measure(&text, &attr_style).width
        })
        .fold(0.0f64, f64::max);

    let attrs_height = if entity.attributes.is_empty() {
        0.0
    } else {
        entity.attributes.len() as f64 * row_height
    };

    let content_w = title_w.max(attr_max_w);
    let width = (content_w + PADDING_X * 2.0).max(MIN_ENTITY_WIDTH);
    let height = title_height + attrs_height;

    EntityDims {
        width,
        height,
        title_height,
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
    fn single_entity() {
        let r = parse_and_layout("erDiagram\n    CUSTOMER {\n        string name\n    }");
        assert_eq!(r.entities.len(), 1);
        assert!(r.entities[0].width > 0.0);
        assert!(r.entities[0].height > 0.0);
    }

    #[test]
    fn entity_with_attrs_taller() {
        let r1 = parse_and_layout("erDiagram\n    A {\n        int x\n    }");
        let r2 = parse_and_layout(
            "erDiagram\n    A {\n        int x\n        int y\n        int z\n    }",
        );
        assert!(r2.entities[0].height > r1.entities[0].height);
    }

    #[test]
    fn min_width_enforced() {
        let r = parse_and_layout("erDiagram\n    A {\n        int x\n    }");
        assert!(r.entities[0].width >= MIN_ENTITY_WIDTH);
    }

    #[test]
    fn relationship_creates_edge() {
        let r = parse_and_layout("erDiagram\n    A ||--o{ B : has");
        assert_eq!(r.entities.len(), 2);
        assert_eq!(r.edges.len(), 1);
        assert_eq!(r.edges[0].cardinality_a, Cardinality::ExactlyOne);
        assert_eq!(r.edges[0].cardinality_b, Cardinality::ZeroOrMore);
    }

    #[test]
    fn edge_clipped_at_boundary() {
        let r = parse_and_layout("erDiagram\n    A ||--|| B : is");
        let edge = &r.edges[0].edge;
        let a = r.entities.iter().find(|e| e.id == "A").unwrap();
        let first = edge.points.first().unwrap();
        let at_center = (first.x - a.x).abs() < 0.1 && (first.y - a.y).abs() < 0.1;
        assert!(!at_center, "edge should be clipped to boundary");
    }

    #[test]
    fn non_identifying_is_dotted() {
        let r = parse_and_layout("erDiagram\n    A }|..|{ B : has");
        assert_eq!(
            r.edges[0].edge.stroke,
            crate::common::layout::StrokeType::Dotted
        );
    }

    #[test]
    fn title_height_positive() {
        let r = parse_and_layout("erDiagram\n    A {\n        int x\n    }");
        assert!(r.entities[0].title_height > 0.0);
    }
}
