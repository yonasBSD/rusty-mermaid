use std::collections::{BTreeMap, HashSet};

use rusty_mermaid_core::{
    BBox, Point, SimpleTextMeasure, Style, TextMeasure, TextStyle, intersect_circle,
    intersect_line_circle, intersect_line_ellipse, intersect_polygon, intersect_rect,
};
use rusty_mermaid_dagre::{DagreConfig, EdgeLabel, NodeLabel};
use rusty_mermaid_graph::{Graph, NodeId};

use rusty_mermaid_core::Shape;

use super::ir::{ArrowEnd, FlowDiagram, StrokeType};
use crate::common::layout::{EdgeLayout, NodeLayout};
use crate::common::rendering::apply_style_properties;
use crate::common::tokens::strip_html_tags;

const PADDING_X: f64 = 16.0;
const PADDING_Y: f64 = 8.0;
const DOUBLE_CIRCLE_PAD: f64 = 10.0;
const EDGE_LABEL_FONT_SIZE: f64 = 12.0;

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
    let (mut graph, id_map) = build_flow_graph(diagram, measurer);

    let config = DagreConfig {
        rankdir: diagram.direction,
        ..Default::default()
    };
    let extracted = extract_directed_subgraphs(diagram, &mut graph, &id_map, measurer);

    rusty_mermaid_dagre::pipeline::layout(&mut graph, &config);

    apply_subgraph_positions(&extracted, &id_map, &mut graph);
    recenter_compounds(diagram, &extracted, &id_map, &mut graph);

    let node_styles = resolve_node_styles(diagram);
    let nid_to_id: BTreeMap<NodeId, &str> = id_map.iter().map(|(&id, &nid)| (nid, id)).collect();

    let (mut nodes, mut max_x, mut max_y) =
        extract_flow_nodes(diagram, &graph, &id_map, &node_styles);
    let mut edges = extract_edge_layouts(diagram, &graph, &nid_to_id, measurer);
    let (mut subgraphs, sg_max_x, sg_max_y) = extract_subgraph_layouts(diagram, &graph, &id_map);
    max_x = max_x.max(sg_max_x);
    max_y = max_y.max(sg_max_y);

    expand_bounds_and_shift(&edges, &mut nodes, &mut subgraphs, &mut max_x, &mut max_y);

    // Apply shift to edges separately (needs mutable borrow on edges)
    let (min_x, min_y) = compute_edge_bounds_min(&edges);
    if min_x < 0.0 || min_y < 0.0 {
        let dx = if min_x < 0.0 { -min_x } else { 0.0 };
        let dy = if min_y < 0.0 { -min_y } else { 0.0 };
        for edge in &mut edges {
            for pt in &mut edge.points {
                pt.x += dx;
                pt.y += dy;
            }
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

fn apply_subgraph_positions(
    extracted: &[ExtractedLayout],
    id_map: &BTreeMap<&str, NodeId>,
    graph: &mut Graph<NodeLabel, EdgeLabel>,
) {
    for ex in extracted {
        let Some(&sg_nid) = id_map.get(ex.sg_id.as_str()) else {
            continue;
        };
        let Some(sg) = graph.node(sg_nid) else {
            continue;
        };
        let sg_cx = sg.x;
        let sg_cy = sg.y;
        for (nid, rel_x, rel_y, w, h) in &ex.inner_nodes {
            let Some(&node_nid) = id_map.get(nid.as_str()) else {
                continue;
            };
            let Some(n) = graph.node_mut(node_nid) else {
                continue;
            };
            n.x = sg_cx + rel_x;
            n.y = sg_cy + rel_y;
            n.width = *w;
            n.height = *h;
        }
        for (sg_id, rel_x, rel_y, w, h) in &ex.inner_subgraphs {
            let Some(&nid) = id_map.get(sg_id.as_str()) else {
                continue;
            };
            let Some(n) = graph.node_mut(nid) else {
                continue;
            };
            n.x = sg_cx + rel_x;
            n.y = sg_cy + rel_y;
            n.width = *w;
            n.height = *h;
        }
        for edge in &ex.inner_edges {
            let Some(&src) = id_map.get(edge.src.as_str()) else {
                continue;
            };
            let Some(&dst) = id_map.get(edge.dst.as_str()) else {
                continue;
            };
            let mut label = EdgeLabel::default();
            label.points = edge
                .points
                .iter()
                .map(|&(px, py)| Point::new(sg_cx + px, sg_cy + py))
                .collect();
            graph.add_edge(src, dst, label);
        }
    }
}

fn recenter_compounds(
    diagram: &FlowDiagram,
    extracted: &[ExtractedLayout],
    id_map: &BTreeMap<&str, NodeId>,
    graph: &mut Graph<NodeLabel, EdgeLabel>,
) {
    for sg in diagram.subgraphs.iter().rev() {
        if extracted.iter().any(|ex| ex.sg_id == sg.id) {
            continue;
        }
        let Some(&nid) = id_map.get(sg.id.as_str()) else {
            continue;
        };
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        for child in graph.children(nid).collect::<Vec<_>>() {
            let Some(c) = graph.node(child) else { continue };
            if c.width > 0.0 || c.height > 0.0 {
                min_x = min_x.min(c.x - c.width / 2.0);
                max_x = max_x.max(c.x + c.width / 2.0);
            }
        }
        if min_x.is_finite()
            && max_x.is_finite()
            && let Some(n) = graph.node_mut(nid)
        {
            n.x = (min_x + max_x) / 2.0;
        }
    }
}

fn extract_flow_nodes(
    diagram: &FlowDiagram,
    graph: &Graph<NodeLabel, EdgeLabel>,
    id_map: &BTreeMap<&str, NodeId>,
    node_styles: &BTreeMap<&str, Style>,
) -> (Vec<NodeLayout>, f64, f64) {
    let sg_ids: HashSet<&str> = diagram.subgraphs.iter().map(|s| s.id.as_str()).collect();
    let mut nodes = Vec::new();
    let mut max_x: f64 = 0.0;
    let mut max_y: f64 = 0.0;

    for vertex in &diagram.vertices {
        if sg_ids.contains(vertex.id.as_str()) {
            continue;
        }
        if let Some(&nid) = id_map.get(vertex.id.as_str()) {
            let Some(n) = graph.node(nid) else { continue };
            nodes.push(NodeLayout {
                id: vertex.id.clone(),
                label: strip_html_tags(&vertex.label),
                shape: vertex.shape,
                x: n.x,
                y: n.y,
                width: n.width,
                height: n.height,
                is_compound: false,
                custom_style: node_styles.get(vertex.id.as_str()).cloned(),
                region_count: 0,
            });
            max_x = max_x.max(n.x + n.width / 2.0);
            max_y = max_y.max(n.y + n.height / 2.0);
        }
    }

    (nodes, max_x, max_y)
}

fn extract_subgraph_layouts(
    diagram: &FlowDiagram,
    graph: &Graph<NodeLabel, EdgeLabel>,
    id_map: &BTreeMap<&str, NodeId>,
) -> (Vec<SubgraphLayout>, f64, f64) {
    let mut subgraphs = Vec::new();
    let mut max_x: f64 = 0.0;
    let mut max_y: f64 = 0.0;

    for sg in &diagram.subgraphs {
        if let Some(&nid) = id_map.get(sg.id.as_str()) {
            let Some(n) = graph.node(nid) else { continue };
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

    (subgraphs, max_x, max_y)
}

fn compute_edge_bounds_min(edges: &[EdgeLayout]) -> (f64, f64) {
    let label_pad = 4.0;
    let mut min_x: f64 = 0.0;
    let mut min_y: f64 = 0.0;
    for edge in edges {
        for pt in &edge.points {
            min_x = min_x.min(pt.x);
            min_y = min_y.min(pt.y);
        }
        if let Some(size) = edge.label_size {
            if edge.points.len() < 2 {
                continue;
            }
            let mid = edge.points[edge.points.len() / 2];
            let lw = size.0 + label_pad * 2.0;
            let lh = size.1 + label_pad * 2.0;
            min_x = min_x.min(mid.x - lw / 2.0);
            min_y = min_y.min(mid.y - lh / 2.0);
        }
    }
    (min_x, min_y)
}

fn expand_bounds_and_shift(
    edges: &[EdgeLayout],
    nodes: &mut [NodeLayout],
    subgraphs: &mut [SubgraphLayout],
    max_x: &mut f64,
    max_y: &mut f64,
) {
    let label_pad = 4.0;
    let mut min_x: f64 = 0.0;
    let mut min_y: f64 = 0.0;
    for edge in edges {
        for pt in &edge.points {
            min_x = min_x.min(pt.x);
            min_y = min_y.min(pt.y);
            *max_x = max_x.max(pt.x);
            *max_y = max_y.max(pt.y);
        }
        if let Some(size) = edge.label_size {
            if edge.points.len() < 2 {
                continue;
            }
            let mid = edge.points[edge.points.len() / 2];
            let lw = size.0 + label_pad * 2.0;
            let lh = size.1 + label_pad * 2.0;
            min_x = min_x.min(mid.x - lw / 2.0);
            min_y = min_y.min(mid.y - lh / 2.0);
            *max_x = max_x.max(mid.x + lw / 2.0);
            *max_y = max_y.max(mid.y + lh / 2.0);
        }
    }

    if min_x < 0.0 || min_y < 0.0 {
        let dx = if min_x < 0.0 { -min_x } else { 0.0 };
        let dy = if min_y < 0.0 { -min_y } else { 0.0 };
        for node in nodes.iter_mut() {
            node.x += dx;
            node.y += dy;
        }
        for sg in subgraphs.iter_mut() {
            sg.x += dx;
            sg.y += dy;
        }
        *max_x += dx;
        *max_y += dy;
    }
}

/// Build the dagre graph from the flowchart IR: vertices, subgraph hierarchy, edges.
fn build_flow_graph<'a>(
    diagram: &'a FlowDiagram,
    measurer: &impl TextMeasure,
) -> (Graph<NodeLabel, EdgeLabel>, BTreeMap<&'a str, NodeId>) {
    let mut graph = Graph::new();
    let style = TextStyle::default();
    let mut id_map: BTreeMap<&str, NodeId> = BTreeMap::new();

    for vertex in &diagram.vertices {
        let text = strip_html_tags(&vertex.label);
        let ts = measurer.measure(&text, &style);
        let text_w = ts.width + PADDING_X * 2.0;
        let text_h = ts.height + PADDING_Y * 2.0;

        let (width, height) = match vertex.shape {
            Shape::Circle => {
                let d = text_w.max(text_h);
                (d, d)
            }
            Shape::DoubleCircle => {
                let d = text_w.max(text_h) + DOUBLE_CIRCLE_PAD;
                (d, d)
            }
            Shape::Diamond => {
                let s = text_w + text_h;
                (s, s)
            }
            Shape::Cylinder => {
                let rx = text_w / 2.0;
                let ry = rx / (2.5 + text_w / 50.0);
                (text_w, text_h + ry * 2.0)
            }
            Shape::Subroutine => {
                // Include the 8px decorative bar offset on each side so dagre's
                // intersection calculation uses the full visual boundary.
                (text_w + 16.0, text_h)
            }
            _ => (text_w, text_h),
        };

        let nid = graph.add_node(NodeLabel::new(width, height));
        id_map.insert(&vertex.id, nid);
    }

    // Two passes: create all subgraph nodes, then set parent-child.
    for sg in &diagram.subgraphs {
        let sg_nid = graph.add_node(NodeLabel::new(0.0, 0.0));
        id_map.insert(&sg.id, sg_nid);
    }
    for sg in &diagram.subgraphs {
        let Some(&sg_nid) = id_map.get(sg.id.as_str()) else {
            continue;
        };
        for child_id in &sg.node_ids {
            if let Some(&child_nid) = id_map.get(child_id.as_str())
                && graph.parent(child_nid).is_none()
            {
                graph.set_parent(child_nid, sg_nid);
            }
        }
        for child_sg_id in &sg.subgraph_ids {
            if let Some(&child_nid) = id_map.get(child_sg_id.as_str())
                && graph.parent(child_nid).is_none()
            {
                graph.set_parent(child_nid, sg_nid);
            }
        }
    }

    for edge in &diagram.edges {
        let Some(&src) = id_map.get(edge.src.as_str()) else {
            continue;
        };
        let Some(&dst) = id_map.get(edge.dst.as_str()) else {
            continue;
        };
        let mut label = EdgeLabel::default();
        label.minlen = edge.minlen.min(10);
        if let Some(text) = &edge.label {
            let ts = measurer.measure(text, &style);
            label.width = ts.width;
            label.height = ts.height;
        }
        graph.add_edge(src, dst, label);
    }

    (graph, id_map)
}

/// Extract edge layouts from the graph, re-clipping endpoints for non-rect shapes.
fn extract_edge_layouts(
    diagram: &FlowDiagram,
    graph: &Graph<NodeLabel, EdgeLabel>,
    nid_to_id: &BTreeMap<NodeId, &str>,
    measurer: &impl TextMeasure,
) -> Vec<EdgeLayout> {
    let vertex_shape: BTreeMap<&str, Shape> = diagram
        .vertices
        .iter()
        .map(|v| (v.id.as_str(), v.shape))
        .collect();
    let edge_styles = resolve_edge_styles(diagram);
    let mut used_edge_idx = vec![false; diagram.edges.len()];
    let mut edges = Vec::new();

    for eid in graph.edge_ids() {
        let Some((src, dst)) = graph.edge_endpoints(eid) else {
            continue;
        };
        let (Some(&src_id), Some(&dst_id)) = (nid_to_id.get(&src), nid_to_id.get(&dst)) else {
            continue;
        };
        let Some(e) = graph.edge(eid) else { continue };
        let mut points: Vec<Point> = e.points.clone();

        if points.len() >= 2 {
            let Some(src_node) = graph.node(src) else {
                continue;
            };
            let src_shape = vertex_shape.get(src_id).copied().unwrap_or(Shape::Rect);
            let src_bbox = BBox::new(src_node.x, src_node.y, src_node.width, src_node.height);
            if let Some(p) = shape_intersect(src_shape, src_bbox, points[1]) {
                points[0] = p;
            }

            let last = points.len() - 1;
            let Some(dst_node) = graph.node(dst) else {
                continue;
            };
            let dst_shape = vertex_shape.get(dst_id).copied().unwrap_or(Shape::Rect);
            let dst_bbox = BBox::new(dst_node.x, dst_node.y, dst_node.width, dst_node.height);
            if let Some(p) = shape_intersect(dst_shape, dst_bbox, points[last - 1]) {
                points[last] = p;
            }
        }

        let edge_idx = diagram.edges.iter().enumerate().find_map(|(i, fe)| {
            if !used_edge_idx[i] && fe.src == src_id && fe.dst == dst_id {
                Some(i)
            } else {
                None
            }
        });
        if let Some(idx) = edge_idx {
            used_edge_idx[idx] = true;
        }
        let flow_edge = edge_idx.map(|i| &diagram.edges[i]);
        let label = flow_edge.and_then(|fe| fe.label.clone());
        let label_size = label.as_ref().map(|text| {
            let edge_style = TextStyle {
                font_size: EDGE_LABEL_FONT_SIZE,
                ..Default::default()
            };
            let ts = measurer.measure(text, &edge_style);
            (ts.width, ts.height)
        });
        let stroke = flow_edge.map_or(StrokeType::Normal, |fe| fe.stroke);
        let start_arrow = flow_edge.map_or(ArrowEnd::None, |fe| fe.start_arrow);
        let end_arrow = flow_edge.map_or(ArrowEnd::Arrow, |fe| fe.end_arrow);
        let custom_style = edge_idx.and_then(|i| edge_styles.get(&i).cloned());
        edges.push(EdgeLayout {
            src: src_id.to_string(),
            dst: dst_id.to_string(),
            points,
            label,
            label_size,
            stroke,
            start_arrow,
            end_arrow,
            custom_style,
        });
    }

    edges
}

/// Inner edge data from an extracted subgraph layout.
struct InnerEdgeLayout {
    src: String,
    dst: String,
    /// Points relative to inner bounding-box center.
    points: Vec<(f64, f64)>,
}

/// Layout data for a subgraph that was extracted for independent layout.
struct ExtractedLayout {
    sg_id: String,
    /// (node_id, rel_x, rel_y, width, height) — positions relative to inner center.
    inner_nodes: Vec<(String, f64, f64, f64, f64)>,
    /// (sg_id, rel_x, rel_y, width, height) — nested subgraph positions relative to inner center.
    inner_subgraphs: Vec<(String, f64, f64, f64, f64)>,
    inner_edges: Vec<InnerEdgeLayout>,
}

/// Collect all descendants (children, grandchildren, ...) of a node.
fn collect_descendants(graph: &Graph<NodeLabel, EdgeLabel>, nid: NodeId) -> HashSet<NodeId> {
    let mut result = HashSet::new();
    let mut stack: Vec<NodeId> = graph.children(nid).collect();
    while let Some(v) = stack.pop() {
        if result.insert(v) {
            stack.extend(graph.children(v));
        }
    }
    result
}

/// Extract subgraphs with per-subgraph direction for independent layout.
///
/// Matches mermaid's dagre backend: only subgraphs without external connections
/// (no edges crossing the boundary between descendants and outside nodes) are
/// extracted. Returns layout data to be applied after the main dagre pass.
fn extract_directed_subgraphs(
    diagram: &FlowDiagram,
    graph: &mut Graph<NodeLabel, EdgeLabel>,
    id_map: &BTreeMap<&str, NodeId>,
    measurer: &impl TextMeasure,
) -> Vec<ExtractedLayout> {
    let style = TextStyle::default();
    let mut extracted = Vec::new();

    // Process bottom-up (inner subgraphs first).
    for sg in diagram.subgraphs.iter().rev() {
        let Some(dir) = sg.direction else { continue };
        let Some(&sg_nid) = id_map.get(sg.id.as_str()) else {
            continue;
        };

        let descendants = collect_descendants(graph, sg_nid);
        if descendants.is_empty() {
            continue;
        }

        // Check for external connections: any edge with one endpoint
        // inside (descendant) and the other outside.
        let has_external = graph.edge_ids().any(|eid| {
            let Some((src, dst)) = graph.edge_endpoints(eid) else {
                return false;
            };
            let s_in = descendants.contains(&src);
            let d_in = descendants.contains(&dst);
            s_in ^ d_in
        });
        if has_external {
            continue;
        }

        // Build independent graph for this subgraph.
        let mut inner_g = Graph::new();
        let mut inner_map: BTreeMap<NodeId, NodeId> = BTreeMap::new();

        // Add descendant nodes.
        for &nid in &descendants {
            let Some(n) = graph.node(nid) else { continue };
            let inner_nid = inner_g.add_node(NodeLabel::new(n.width, n.height));
            inner_map.insert(nid, inner_nid);
        }

        // Recreate compound hierarchy within the inner graph.
        for &nid in &descendants {
            if let Some(parent) = graph.parent(nid)
                && parent != sg_nid
                && let (Some(&inner_child), Some(&inner_parent)) =
                    (inner_map.get(&nid), inner_map.get(&parent))
            {
                inner_g.set_parent(inner_child, inner_parent);
            }
        }

        // Add internal edges.
        for eid in graph.edge_ids().collect::<Vec<_>>() {
            let Some((src, dst)) = graph.edge_endpoints(eid) else {
                continue;
            };
            if descendants.contains(&src)
                && descendants.contains(&dst)
                && let (Some(&inner_src), Some(&inner_dst)) =
                    (inner_map.get(&src), inner_map.get(&dst))
            {
                let Some(label) = graph.edge(eid).cloned() else {
                    continue;
                };
                inner_g.add_edge(inner_src, inner_dst, label);
            }
        }

        // Run dagre with the subgraph's direction.
        let inner_config = DagreConfig {
            rankdir: dir,
            ..Default::default()
        };
        rusty_mermaid_dagre::pipeline::layout(&mut inner_g, &inner_config);

        // Compute bounding box of inner layout.
        let reverse_map: BTreeMap<NodeId, NodeId> = inner_map
            .iter()
            .map(|(&outer, &inner)| (inner, outer))
            .collect();
        let nid_to_id: BTreeMap<NodeId, &str> =
            id_map.iter().map(|(&id, &nid)| (nid, id)).collect();

        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for &inner_nid in inner_map.values() {
            let Some(n) = inner_g.node(inner_nid) else {
                continue;
            };
            if n.width <= 0.0 && n.height <= 0.0 {
                continue;
            }
            min_x = min_x.min(n.x - n.width / 2.0);
            min_y = min_y.min(n.y - n.height / 2.0);
            max_x = max_x.max(n.x + n.width / 2.0);
            max_y = max_y.max(n.y + n.height / 2.0);
        }

        if !min_x.is_finite() {
            continue;
        }

        let center_x = (min_x + max_x) / 2.0;
        let center_y = (min_y + max_y) / 2.0;

        // Padding: border + label space (matching dagre compound style).
        let pad = 16.0;
        let label_h = if sg.label.is_some() {
            let lh = measurer
                .measure(sg.label.as_deref().unwrap_or(""), &style)
                .height;
            lh + 8.0
        } else {
            0.0
        };
        let total_w = (max_x - min_x) + pad * 2.0;
        let total_h = (max_y - min_y) + pad * 2.0 + label_h;
        // Shift center_y down by half the label height so children sit below label.
        let adj_center_y = center_y - label_h / 2.0;

        // Store inner node positions relative to center.
        let mut inner_nodes = Vec::new();
        let mut inner_subgraphs = Vec::new();

        for (&outer_nid, &inner_nid) in &inner_map {
            let Some(n) = inner_g.node(inner_nid) else {
                continue;
            };
            let Some(&str_id) = nid_to_id.get(&outer_nid) else {
                continue;
            };
            let rel_x = n.x - center_x;
            let rel_y = n.y - adj_center_y;

            // Is this a subgraph node or a vertex node?
            if diagram.subgraphs.iter().any(|s| s.id == str_id) {
                if n.width > 0.0 || n.height > 0.0 {
                    inner_subgraphs.push((str_id.to_string(), rel_x, rel_y, n.width, n.height));
                }
            } else {
                inner_nodes.push((str_id.to_string(), rel_x, rel_y, n.width, n.height));
            }
        }

        // Store inner edge points relative to center.
        let mut inner_edges = Vec::new();
        for eid in inner_g.edge_ids() {
            let Some((src, dst)) = inner_g.edge_endpoints(eid) else {
                continue;
            };
            let Some(&outer_src) = reverse_map.get(&src) else {
                continue;
            };
            let Some(&outer_dst) = reverse_map.get(&dst) else {
                continue;
            };
            let Some(&src_id) = nid_to_id.get(&outer_src) else {
                continue;
            };
            let Some(&dst_id) = nid_to_id.get(&outer_dst) else {
                continue;
            };
            let Some(e) = inner_g.edge(eid) else { continue };
            let points: Vec<(f64, f64)> = e
                .points
                .iter()
                .map(|p| (p.x - center_x, p.y - adj_center_y))
                .collect();
            inner_edges.push(InnerEdgeLayout {
                src: src_id.to_string(),
                dst: dst_id.to_string(),
                points,
            });
        }

        // Modify main graph: remove children from compound, set fixed size.
        for &nid in &descendants {
            graph.remove_parent(nid);
        }
        // Remove internal edges from main graph.
        let internal_eids: Vec<_> = graph
            .edge_ids()
            .filter(|&eid| {
                graph
                    .edge_endpoints(eid)
                    .is_some_and(|(s, d)| descendants.contains(&s) && descendants.contains(&d))
            })
            .collect();
        for eid in internal_eids {
            graph.remove_edge(eid);
        }
        // Set the subgraph node to a fixed-size leaf.
        if let Some(sg_node) = graph.node_mut(sg_nid) {
            sg_node.width = total_w;
            sg_node.height = total_h;
        }

        extracted.push(ExtractedLayout {
            sg_id: sg.id.clone(),
            inner_nodes,
            inner_subgraphs,
            inner_edges,
        });
    }

    extracted
}

/// Resolve all style sources into a single `Style` per node.
/// Priority (last wins): classDef "default" → classDef via class/:::class → style statement.
fn resolve_node_styles(diagram: &FlowDiagram) -> BTreeMap<&str, Style> {
    let entities = diagram
        .vertices
        .iter()
        .map(|v| (v.id.as_str(), v.classes.as_slice()));
    crate::common::rendering::resolve_entity_styles(
        entities,
        &diagram.class_defs,
        &diagram.style_stmts,
    )
}

/// Resolve linkStyle statements into a per-edge-index Style map.
/// Priority: linkStyle default → linkStyle by index (last wins).
fn resolve_edge_styles(diagram: &FlowDiagram) -> BTreeMap<usize, Style> {
    let mut result: BTreeMap<usize, Style> = BTreeMap::new();
    let edge_count = diagram.edges.len();

    for ls in &diagram.link_styles {
        if ls.is_default {
            // Apply to all edges
            for i in 0..edge_count {
                let style = result.entry(i).or_default();
                apply_style_properties(style, &ls.styles);
            }
        } else {
            for &idx in &ls.indices {
                if idx < edge_count {
                    let style = result.entry(idx).or_default();
                    apply_style_properties(style, &ls.styles);
                }
            }
        }
    }

    result
}

/// Compute shape-specific edge intersection point.
/// Returns `None` for rect-like shapes (dagre's default clipping is correct).
/// For inscribed shapes (diamond, circle, hexagon, etc.), returns the point
/// where the ray from node center toward `adj` crosses the shape perimeter.
fn shape_intersect(shape: Shape, bbox: BBox, adj: Point) -> Option<Point> {
    let (cx, cy, w, h) = (bbox.x, bbox.y, bbox.width, bbox.height);
    let center = Point::new(cx, cy);
    let target = adj;
    let hw = w / 2.0;
    let hh = h / 2.0;

    let p = match shape {
        Shape::Diamond => {
            let verts = [
                Point::new(cx, cy - hh),
                Point::new(cx + hw, cy),
                Point::new(cx, cy + hh),
                Point::new(cx - hw, cy),
            ];
            intersect_polygon(&verts, center, target)
        }
        Shape::Circle | Shape::DoubleCircle => {
            let r = w.max(h) / 2.0;
            intersect_circle(center, r, target)
        }
        Shape::Hexagon => {
            let m = h / 4.0;
            let verts = [
                Point::new(cx - hw + m, cy - hh),
                Point::new(cx + hw - m, cy - hh),
                Point::new(cx + hw, cy),
                Point::new(cx + hw - m, cy + hh),
                Point::new(cx - hw + m, cy + hh),
                Point::new(cx - hw, cy),
            ];
            intersect_polygon(&verts, center, target)
        }
        Shape::Parallelogram => {
            let skew = h / 2.0;
            let verts = [
                Point::new(cx - hw + skew, cy - hh),
                Point::new(cx + hw + skew, cy - hh),
                Point::new(cx + hw - skew, cy + hh),
                Point::new(cx - hw - skew, cy + hh),
            ];
            intersect_polygon(&verts, center, target)
        }
        Shape::ParallelogramAlt => {
            let skew = h / 2.0;
            let verts = [
                Point::new(cx - hw - skew, cy - hh),
                Point::new(cx + hw - skew, cy - hh),
                Point::new(cx + hw + skew, cy + hh),
                Point::new(cx - hw + skew, cy + hh),
            ];
            intersect_polygon(&verts, center, target)
        }
        Shape::Trapezoid => {
            let offset = h / 2.0;
            let verts = [
                Point::new(cx - hw, cy - hh),
                Point::new(cx + hw, cy - hh),
                Point::new(cx + hw + offset, cy + hh),
                Point::new(cx - hw - offset, cy + hh),
            ];
            intersect_polygon(&verts, center, target)
        }
        Shape::TrapezoidAlt => {
            let offset = h / 2.0;
            let verts = [
                Point::new(cx - hw - offset, cy - hh),
                Point::new(cx + hw + offset, cy - hh),
                Point::new(cx + hw, cy + hh),
                Point::new(cx - hw, cy + hh),
            ];
            intersect_polygon(&verts, center, target)
        }
        Shape::Cylinder => {
            // Composite intersection: rect for straight sides, ellipse for caps.
            let rx = hw;
            let ry = rx / (2.5 + w / 50.0);
            let body_h = h - ry;
            let top_cap_cy = cy - body_h / 2.0;
            let bot_cap_cy = cy + body_h / 2.0;
            let full_bbox = BBox::new(cx, cy, w, h + ry);
            let rect_hit = intersect_rect(&full_bbox, target);

            if rect_hit.y <= top_cap_cy {
                intersect_line_ellipse(center, target, Point::new(cx, top_cap_cy), rx, ry)
            } else if rect_hit.y >= bot_cap_cy {
                intersect_line_ellipse(center, target, Point::new(cx, bot_cap_cy), rx, ry)
            } else {
                rect_hit
            }
        }
        Shape::Asymmetric => {
            let notch = (h / 4.0).min(hw * 0.8);
            let verts = [
                Point::new(cx - hw, cy - hh),
                Point::new(cx + hw, cy - hh),
                Point::new(cx + hw, cy + hh),
                Point::new(cx - hw, cy + hh),
                Point::new(cx - hw + notch, cy),
            ];
            intersect_polygon(&verts, center, target)
        }
        Shape::Stadium => {
            let r = hh;
            let left_cx = cx - hw + r;
            let right_cx = cx + hw - r;

            if left_cx >= right_cx {
                // Degenerate: width ≤ height, treat as circle
                intersect_circle(center, r, target)
            } else {
                let rect_hit = intersect_rect(&bbox, target);
                if rect_hit.x <= left_cx {
                    intersect_line_circle(center, target, Point::new(left_cx, cy), r)
                } else if rect_hit.x >= right_cx {
                    intersect_line_circle(center, target, Point::new(right_cx, cy), r)
                } else {
                    // Straight top/bottom edge — dagre's rect clipping is correct
                    return None;
                }
            }
        }
        // Rect-like shapes: dagre's intersect_rect is already correct
        _ => return None,
    };

    Some(p)
}

#[cfg(test)]
#[path = "bridge_tests.rs"]
mod bridge_tests;
