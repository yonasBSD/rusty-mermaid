use std::collections::{BTreeMap, HashSet};

use rusty_mermaid_core::{
    BBox, Point, SimpleTextMeasure, Style, TextMeasure, TextStyle,
    intersect_circle, intersect_line_circle, intersect_polygon, intersect_rect,
};
use rusty_mermaid_dagre::{DagreConfig, EdgeLabel, NodeLabel};
use rusty_mermaid_graph::{Graph, NodeId};

use rusty_mermaid_core::Shape;

use super::ir::{ArrowEnd, FlowDiagram, StrokeType};
use crate::common::layout::{EdgeLayout, NodeLayout};
use crate::common::rendering::apply_style_properties;
use crate::common::styling::StyleProperty;
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
    let (mut g, id_map) = build_flow_graph(diagram, measurer);

    let config = DagreConfig { rankdir: diagram.direction, ..Default::default() };

    // Extract subgraphs with per-subgraph direction override.
    // Matching mermaid's dagre backend: subgraphs without external
    // connections get independent dagre layout with their own rankdir.
    let extracted = extract_directed_subgraphs(diagram, &mut g, &id_map, measurer);

    // Run layout
    rusty_mermaid_dagre::pipeline::layout(&mut g, &config);

    // Apply extracted subgraph inner positions: translate from
    // inner-relative coordinates to final absolute coordinates.
    for ex in &extracted {
        let Some(&sg_nid) = id_map.get(ex.sg_id.as_str()) else { continue };
        let Some(sg) = g.node(sg_nid) else { continue };
        let sg_cx = sg.x;
        let sg_cy = sg.y;
        for (nid, rel_x, rel_y, w, h) in &ex.inner_nodes {
            let Some(&node_nid) = id_map.get(nid.as_str()) else { continue };
            let Some(n) = g.node_mut(node_nid) else { continue };
            n.x = sg_cx + rel_x;
            n.y = sg_cy + rel_y;
            n.width = *w;
            n.height = *h;
        }
        for (sg_id, rel_x, rel_y, w, h) in &ex.inner_subgraphs {
            let Some(&nid) = id_map.get(sg_id.as_str()) else { continue };
            let Some(n) = g.node_mut(nid) else { continue };
            n.x = sg_cx + rel_x;
            n.y = sg_cy + rel_y;
            n.width = *w;
            n.height = *h;
        }
        for edge in &ex.inner_edges {
            let Some(&src) = id_map.get(edge.src.as_str()) else { continue };
            let Some(&dst) = id_map.get(edge.dst.as_str()) else { continue };
            // Re-add inner edges (removed during extraction) with translated points.
            let mut label = EdgeLabel::default();
            label.points = edge.points.iter()
                .map(|&(px, py)| Point::new(sg_cx + px, sg_cy + py))
                .collect();
            g.add_edge(src, dst, label);
        }
    }

    // Recenter compound nodes on their content.  The BK position
    // algorithm can place left/right border nodes asymmetrically,
    // causing unequal left/right padding.  Redistribute padding
    // evenly by centering each compound on its children's bounding
    // box.  Process inner-to-outer so parent compounds see the
    // updated child positions.
    for sg in diagram.subgraphs.iter().rev() {
        if extracted.iter().any(|ex| ex.sg_id == sg.id) {
            continue; // extracted subgraphs already positioned
        }
        let Some(&nid) = id_map.get(sg.id.as_str()) else {
            continue;
        };
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        for child in g.children(nid).collect::<Vec<_>>() {
            let Some(c) = g.node(child) else { continue };
            if c.width > 0.0 || c.height > 0.0 {
                min_x = min_x.min(c.x - c.width / 2.0);
                max_x = max_x.max(c.x + c.width / 2.0);
            }
        }
        if min_x.is_finite() && max_x.is_finite() && let Some(n) = g.node_mut(nid) {
            n.x = (min_x + max_x) / 2.0;
        }
    }

    // Resolve per-node styles from classDef + class + style statements.
    let node_styles = resolve_node_styles(diagram);

    // Extract results
    let nid_to_id: BTreeMap<NodeId, &str> = id_map.iter().map(|(&id, &nid)| (nid, id)).collect();

    let mut nodes = Vec::new();
    let mut max_x: f64 = 0.0;
    let mut max_y: f64 = 0.0;

    // Subgraph IDs: skip vertex rendering when a vertex ID collides with a
    // subgraph ID (the subgraph node takes precedence in the graph).
    let sg_ids: HashSet<&str> = diagram.subgraphs.iter().map(|s| s.id.as_str()).collect();

    for v in &diagram.vertices {
        if sg_ids.contains(v.id.as_str()) {
            continue; // rendered as subgraph, not vertex
        }
        if let Some(&nid) = id_map.get(v.id.as_str()) {
            let Some(n) = g.node(nid) else { continue };
            nodes.push(NodeLayout {
                id: v.id.clone(),
                label: strip_html_tags(&v.label),
                shape: v.shape,
                x: n.x,
                y: n.y,
                width: n.width,
                height: n.height,
                is_compound: false,
                custom_style: node_styles.get(v.id.as_str()).cloned(),
                region_count: 0,
            });
            max_x = max_x.max(n.x + n.width / 2.0);
            max_y = max_y.max(n.y + n.height / 2.0);
        }
    }

    let mut edges = extract_edge_layouts(diagram, &g, &nid_to_id, measurer);

    // Extract subgraph positions from dagre's compound node bounds
    // (padding and label space are already included by remove_border_nodes).
    let mut subgraphs = Vec::new();
    for sg in &diagram.subgraphs {
        if let Some(&nid) = id_map.get(sg.id.as_str()) {
            let Some(n) = g.node(nid) else { continue };
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

    // Expand bounds to include edge control points and label extents
    // (dagre routes and labels can extend past the node bounding box).
    let mut min_x: f64 = 0.0;
    let mut min_y: f64 = 0.0;
    let label_pad = 4.0;
    for edge in &edges {
        for pt in &edge.points {
            min_x = min_x.min(pt.x);
            min_y = min_y.min(pt.y);
            max_x = max_x.max(pt.x);
            max_y = max_y.max(pt.y);
        }
        if let Some(size) = edge.label_size {
            if edge.points.len() < 2 { continue; }
            let mid = edge.points[edge.points.len() / 2];
            let lw = size.0 + label_pad * 2.0;
            let lh = size.1 + label_pad * 2.0;
            min_x = min_x.min(mid.x - lw / 2.0);
            min_y = min_y.min(mid.y - lh / 2.0);
            max_x = max_x.max(mid.x + lw / 2.0);
            max_y = max_y.max(mid.y + lh / 2.0);
        }
    }

    // Shift everything if edge labels extend past the origin
    if min_x < 0.0 || min_y < 0.0 {
        let dx = if min_x < 0.0 { -min_x } else { 0.0 };
        let dy = if min_y < 0.0 { -min_y } else { 0.0 };
        for node in &mut nodes {
            node.x += dx;
            node.y += dy;
        }
        for edge in &mut edges {
            for pt in &mut edge.points {
                pt.x += dx;
                pt.y += dy;
            }
        }
        for sg in &mut subgraphs {
            sg.x += dx;
            sg.y += dy;
        }
        max_x += dx;
        max_y += dy;
    }

    LayoutResult {
        nodes,
        edges,
        subgraphs,
        width: max_x,
        height: max_y,
    }
}

/// Build the dagre graph from the flowchart IR: vertices, subgraph hierarchy, edges.
fn build_flow_graph<'a>(
    diagram: &'a FlowDiagram,
    measurer: &impl TextMeasure,
) -> (Graph<NodeLabel, EdgeLabel>, BTreeMap<&'a str, NodeId>) {
    let mut g = Graph::new();
    let style = TextStyle::default();
    let mut id_map: BTreeMap<&str, NodeId> = BTreeMap::new();

    for v in &diagram.vertices {
        let text = strip_html_tags(&v.label);
        let (tw, th) = measurer.measure(&text, &style);
        let text_w = tw + PADDING_X * 2.0;
        let text_h = th + PADDING_Y * 2.0;

        let (width, height) = match v.shape {
            Shape::Circle => {
                let d = text_w.max(text_h);
                (d, d)
            }
            Shape::DoubleCircle => {
                let d = text_w.max(text_h) + 10.0;
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
            _ => (text_w, text_h),
        };

        let nid = g.add_node(NodeLabel::new(width, height));
        id_map.insert(&v.id, nid);
    }

    // Two passes: create all subgraph nodes, then set parent-child.
    for sg in &diagram.subgraphs {
        let sg_nid = g.add_node(NodeLabel::new(0.0, 0.0));
        id_map.insert(&sg.id, sg_nid);
    }
    for sg in &diagram.subgraphs {
        let Some(&sg_nid) = id_map.get(sg.id.as_str()) else { continue };
        for child_id in &sg.node_ids {
            if let Some(&child_nid) = id_map.get(child_id.as_str())
                && g.parent(child_nid).is_none()
            {
                g.set_parent(child_nid, sg_nid);
            }
        }
        for child_sg_id in &sg.subgraph_ids {
            if let Some(&child_nid) = id_map.get(child_sg_id.as_str())
                && g.parent(child_nid).is_none()
            {
                g.set_parent(child_nid, sg_nid);
            }
        }
    }

    for e in &diagram.edges {
        let Some(&src) = id_map.get(e.src.as_str()) else { continue };
        let Some(&dst) = id_map.get(e.dst.as_str()) else { continue };
        let mut label = EdgeLabel::default();
        label.minlen = e.minlen.min(10);
        if let Some(text) = &e.label {
            let (tw, th) = measurer.measure(text, &style);
            label.width = tw;
            label.height = th;
        }
        g.add_edge(src, dst, label);
    }

    (g, id_map)
}

/// Extract edge layouts from the graph, re-clipping endpoints for non-rect shapes.
fn extract_edge_layouts(
    diagram: &FlowDiagram,
    g: &Graph<NodeLabel, EdgeLabel>,
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

    for eid in g.edge_ids() {
        let Some((src, dst)) = g.edge_endpoints(eid) else { continue };
        let (Some(&src_id), Some(&dst_id)) = (nid_to_id.get(&src), nid_to_id.get(&dst)) else {
            continue;
        };
        let Some(e) = g.edge(eid) else { continue };
        let mut points: Vec<Point> = e.points.clone();

        if points.len() >= 2 {
            let Some(src_node) = g.node(src) else { continue };
            let src_shape = vertex_shape.get(src_id).copied().unwrap_or(Shape::Rect);
            let src_bbox = BBox::new(src_node.x, src_node.y, src_node.width, src_node.height);
            if let Some(p) = shape_intersect(src_shape, src_bbox, points[1]) {
                points[0] = p;
            }

            let last = points.len() - 1;
            let Some(dst_node) = g.node(dst) else { continue };
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
            let edge_style = TextStyle { font_size: 12.0, ..Default::default() };
            measurer.measure(text, &edge_style)
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
fn collect_descendants(g: &Graph<NodeLabel, EdgeLabel>, nid: NodeId) -> HashSet<NodeId> {
    let mut result = HashSet::new();
    let mut stack: Vec<NodeId> = g.children(nid).collect();
    while let Some(v) = stack.pop() {
        if result.insert(v) {
            stack.extend(g.children(v));
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
    g: &mut Graph<NodeLabel, EdgeLabel>,
    id_map: &BTreeMap<&str, NodeId>,
    measurer: &impl TextMeasure,
) -> Vec<ExtractedLayout> {
    let style = TextStyle::default();
    let mut extracted = Vec::new();

    // Process bottom-up (inner subgraphs first).
    for sg in diagram.subgraphs.iter().rev() {
        let Some(dir) = sg.direction else { continue };
        let Some(&sg_nid) = id_map.get(sg.id.as_str()) else { continue };

        let descendants = collect_descendants(g, sg_nid);
        if descendants.is_empty() {
            continue;
        }

        // Check for external connections: any edge with one endpoint
        // inside (descendant) and the other outside.
        let has_external = g.edge_ids().any(|eid| {
            let Some((src, dst)) = g.edge_endpoints(eid) else { return false };
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
            let Some(n) = g.node(nid) else { continue };
            let inner_nid = inner_g.add_node(NodeLabel::new(n.width, n.height));
            inner_map.insert(nid, inner_nid);
        }

        // Recreate compound hierarchy within the inner graph.
        for &nid in &descendants {
            if let Some(parent) = g.parent(nid)
                && parent != sg_nid
                && let (Some(&inner_child), Some(&inner_parent)) =
                    (inner_map.get(&nid), inner_map.get(&parent))
            {
                inner_g.set_parent(inner_child, inner_parent);
            }
        }

        // Add internal edges.
        for eid in g.edge_ids().collect::<Vec<_>>() {
            let Some((src, dst)) = g.edge_endpoints(eid) else { continue };
            if descendants.contains(&src)
                && descendants.contains(&dst)
                && let (Some(&inner_src), Some(&inner_dst)) =
                    (inner_map.get(&src), inner_map.get(&dst))
            {
                let Some(label) = g.edge(eid).cloned() else { continue };
                inner_g.add_edge(inner_src, inner_dst, label);
            }
        }

        // Run dagre with the subgraph's direction.
        let inner_config = DagreConfig { rankdir: dir, ..Default::default() };
        rusty_mermaid_dagre::pipeline::layout(&mut inner_g, &inner_config);

        // Compute bounding box of inner layout.
        let reverse_map: BTreeMap<NodeId, NodeId> = inner_map.iter()
            .map(|(&outer, &inner)| (inner, outer))
            .collect();
        let nid_to_id: BTreeMap<NodeId, &str> = id_map.iter()
            .map(|(&id, &nid)| (nid, id))
            .collect();

        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for &inner_nid in inner_map.values() {
            let Some(n) = inner_g.node(inner_nid) else { continue };
            if n.width <= 0.0 && n.height <= 0.0 { continue; }
            min_x = min_x.min(n.x - n.width / 2.0);
            min_y = min_y.min(n.y - n.height / 2.0);
            max_x = max_x.max(n.x + n.width / 2.0);
            max_y = max_y.max(n.y + n.height / 2.0);
        }

        if !min_x.is_finite() { continue; }

        let center_x = (min_x + max_x) / 2.0;
        let center_y = (min_y + max_y) / 2.0;

        // Padding: border + label space (matching dagre compound style).
        let pad = 16.0;
        let label_h = if sg.label.is_some() {
            let (_, lh) = measurer.measure(sg.label.as_deref().unwrap_or(""), &style);
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
            let Some(n) = inner_g.node(inner_nid) else { continue };
            let Some(&str_id) = nid_to_id.get(&outer_nid) else { continue };
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
            let Some((src, dst)) = inner_g.edge_endpoints(eid) else { continue };
            let Some(&outer_src) = reverse_map.get(&src) else { continue };
            let Some(&outer_dst) = reverse_map.get(&dst) else { continue };
            let Some(&src_id) = nid_to_id.get(&outer_src) else { continue };
            let Some(&dst_id) = nid_to_id.get(&outer_dst) else { continue };
            let Some(e) = inner_g.edge(eid) else { continue };
            let points: Vec<(f64, f64)> = e.points.iter()
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
            g.remove_parent(nid);
        }
        // Remove internal edges from main graph.
        let internal_eids: Vec<_> = g.edge_ids().filter(|&eid| {
            g.edge_endpoints(eid).is_some_and(|(s, d)| {
                descendants.contains(&s) && descendants.contains(&d)
            })
        }).collect();
        for eid in internal_eids {
            g.remove_edge(eid);
        }
        // Set the subgraph node to a fixed-size leaf.
        if let Some(sg_node) = g.node_mut(sg_nid) {
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
    let class_map: BTreeMap<&str, &[StyleProperty]> = diagram
        .class_defs
        .iter()
        .map(|cd| (cd.name.as_str(), cd.styles.as_slice()))
        .collect();

    let mut result: BTreeMap<&str, Style> = BTreeMap::new();

    for v in &diagram.vertices {
        let mut style = Style::default();
        let mut has_custom = false;

        // 1. Apply "default" classDef to all nodes
        if let Some(props) = class_map.get("default") {
            apply_style_properties(&mut style, props);
            has_custom = true;
        }

        // 2. Apply classes (from `class` statement or `:::className`)
        for class_name in &v.classes {
            if let Some(props) = class_map.get(class_name.as_str()) {
                apply_style_properties(&mut style, props);
                has_custom = true;
            }
        }

        // 3. Apply inline `style` statement (highest priority)
        for stmt in &diagram.style_stmts {
            if stmt.ids.iter().any(|id| id == &v.id) {
                apply_style_properties(&mut style, &stmt.styles);
                has_custom = true;
            }
        }

        if has_custom {
            result.insert(&v.id, style);
        }
    }

    result
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
            // The cylinder's elliptical caps extend beyond the dagre bounding
            // box by ry. Clip to a rect matching the full visual height.
            let rx = hw;
            let ry = rx / (2.5 + w / 50.0);
            let full_h = h + ry;
            let bbox = BBox::new(cx, cy, w, full_h);
            intersect_rect(&bbox, target)
        }
        Shape::Asymmetric => {
            let notch = h / 4.0;
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

    #[test]
    fn shape_propagated_to_layout() {
        let d = crate::flowchart::parser::parse(
            "flowchart TD\n    A[Rect] --> B(Rounded) --> C{Diamond} --> D((Circle))",
        )
        .unwrap();
        let result = layout(&d);

        let a = result.nodes.iter().find(|n| n.id == "A").unwrap();
        let b = result.nodes.iter().find(|n| n.id == "B").unwrap();
        let c = result.nodes.iter().find(|n| n.id == "C").unwrap();
        let d = result.nodes.iter().find(|n| n.id == "D").unwrap();

        assert_eq!(a.shape, Shape::Rect);
        assert_eq!(b.shape, Shape::RoundedRect);
        assert_eq!(c.shape, Shape::Diamond);
        assert_eq!(d.shape, Shape::Circle);
    }

    #[test]
    fn arrow_types_propagated() {
        let d = crate::flowchart::parser::parse(
            "flowchart TD\n    A --o B\n    A --x C\n    A --- D",
        )
        .unwrap();
        let result = layout(&d);

        let ab = result.edges.iter().find(|e| e.dst == "B").unwrap();
        let ac = result.edges.iter().find(|e| e.dst == "C").unwrap();
        let ad = result.edges.iter().find(|e| e.dst == "D").unwrap();

        assert_eq!(ab.end_arrow, ArrowEnd::Circle);
        assert_eq!(ac.end_arrow, ArrowEnd::Cross);
        assert_eq!(ad.end_arrow, ArrowEnd::None);
    }

    #[test]
    fn bidirectional_arrows() {
        let d = crate::flowchart::parser::parse(
            "flowchart TD\n    A <--> B\n    C <-.-> D\n    E o--o F\n    G x--x H",
        )
        .unwrap();
        let result = layout(&d);

        let ab = result.edges.iter().find(|e| e.src == "A" && e.dst == "B").unwrap();
        assert_eq!(ab.start_arrow, ArrowEnd::Arrow);
        assert_eq!(ab.end_arrow, ArrowEnd::Arrow);

        let cd = result.edges.iter().find(|e| e.src == "C" && e.dst == "D").unwrap();
        assert_eq!(cd.start_arrow, ArrowEnd::Arrow);
        assert_eq!(cd.end_arrow, ArrowEnd::Arrow);

        let ef = result.edges.iter().find(|e| e.src == "E" && e.dst == "F").unwrap();
        assert_eq!(ef.start_arrow, ArrowEnd::Circle);
        assert_eq!(ef.end_arrow, ArrowEnd::Circle);

        let gh = result.edges.iter().find(|e| e.src == "G" && e.dst == "H").unwrap();
        assert_eq!(gh.start_arrow, ArrowEnd::Cross);
        assert_eq!(gh.end_arrow, ArrowEnd::Cross);
    }

    #[test]
    fn subgraph_direction_lr_in_td() {
        let d = crate::flowchart::parser::parse(
            "flowchart TD\n    subgraph sub1[Process]\n        direction LR\n        A[Step 1] --> B[Step 2] --> C[Step 3]\n    end\n    D[Start] --> sub1\n    sub1 --> E[End]",
        ).unwrap();
        let result = layout(&d);

        let a = result.nodes.iter().find(|n| n.id == "A").unwrap();
        let b = result.nodes.iter().find(|n| n.id == "B").unwrap();
        let c = result.nodes.iter().find(|n| n.id == "C").unwrap();

        // With direction LR inside the subgraph, A/B/C should be
        // horizontally arranged (left to right), not vertically.
        assert!(a.x < b.x, "A should be left of B: {} < {}", a.x, b.x);
        assert!(b.x < c.x, "B should be left of C: {} < {}", b.x, c.x);
        // They should be at roughly the same y.
        assert!((a.y - b.y).abs() < 5.0, "A and B should be at same y");
        assert!((b.y - c.y).abs() < 5.0, "B and C should be at same y");
    }

    #[test]
    fn subgraph_direction_skipped_with_external_edges() {
        // Edge from D directly to A (inside sub1) → external connection → direction ignored.
        let d = crate::flowchart::parser::parse(
            "flowchart TD\n    subgraph sub1[Process]\n        direction LR\n        A --> B --> C\n    end\n    D --> A",
        ).unwrap();
        let result = layout(&d);

        let a = result.nodes.iter().find(|n| n.id == "A").unwrap();
        let b = result.nodes.iter().find(|n| n.id == "B").unwrap();

        // With external connections, direction LR is ignored → defaults to TD.
        // A should be above B (or at least not horizontally arranged).
        assert!(a.y < b.y, "A should be above B (TD): {} < {}", a.y, b.y);
    }

    // ── shape_intersect tests ──────────────────────────────────────────

    const TOL: f64 = 1e-6;

    fn assert_near(actual: f64, expected: f64, msg: &str) {
        assert!(
            (actual - expected).abs() < TOL,
            "{msg}: expected {expected}, got {actual}",
        );
    }

    fn assert_point_near(actual: Point, expected: Point, msg: &str) {
        assert!(
            (actual.x - expected.x).abs() < TOL && (actual.y - expected.y).abs() < TOL,
            "{msg}: expected ({}, {}), got ({}, {})",
            expected.x, expected.y, actual.x, actual.y,
        );
    }

    /// Standard test bbox: center (100, 100), size 80x60.
    fn test_bbox() -> BBox {
        BBox::new(100.0, 100.0, 80.0, 60.0)
    }

    /// Square test bbox: center (100, 100), size 80x80.
    fn square_bbox() -> BBox {
        BBox::new(100.0, 100.0, 80.0, 80.0)
    }

    // ── Shapes that return None ──

    #[test]
    fn shape_intersect_rect_returns_none() {
        let b = test_bbox();
        assert!(shape_intersect(Shape::Rect, b, Point::new(200.0, 100.0)).is_none());
        assert!(shape_intersect(Shape::Rect, b, Point::new(100.0, 0.0)).is_none());
        assert!(shape_intersect(Shape::Rect, b, Point::new(200.0, 200.0)).is_none());
    }

    #[test]
    fn shape_intersect_rounded_rect_returns_none() {
        let b = test_bbox();
        assert!(shape_intersect(Shape::RoundedRect, b, Point::new(200.0, 100.0)).is_none());
    }

    #[test]
    fn shape_intersect_stadium_right() {
        // test_bbox: center (100,100), 80x60. r=hh=30.
        // Right cap center: (100+40-30, 100) = (110, 100), r=30.
        // Horizontal ray rightward hits circle at (140, 100).
        let b = test_bbox();
        let p = shape_intersect(Shape::Stadium, b, Point::new(200.0, 100.0)).unwrap();
        assert_point_near(p, Point::new(140.0, 100.0), "stadium right");
    }

    #[test]
    fn shape_intersect_stadium_left() {
        // Left cap center: (100-40+30, 100) = (90, 100), r=30.
        // Horizontal ray leftward hits circle at (60, 100).
        let b = test_bbox();
        let p = shape_intersect(Shape::Stadium, b, Point::new(0.0, 100.0)).unwrap();
        assert_point_near(p, Point::new(60.0, 100.0), "stadium left");
    }

    #[test]
    fn shape_intersect_stadium_top_straight() {
        // Ray straight up exits through straight top edge — returns None (dagre handles it).
        let b = test_bbox();
        assert!(shape_intersect(Shape::Stadium, b, Point::new(100.0, 0.0)).is_none());
    }

    #[test]
    fn shape_intersect_stadium_diagonal_into_cap() {
        // Ray at an angle into the right cap zone.
        // Right cap center: (110, 100), r=30.
        let b = test_bbox();
        let p = shape_intersect(Shape::Stadium, b, Point::new(200.0, 80.0)).unwrap();
        // Point should lie on the right cap circle
        let dist = Point::new(110.0, 100.0).distance_to(p);
        assert_near(dist, 30.0, "stadium diagonal should land on cap circle");
    }

    #[test]
    fn shape_intersect_wildcard_shapes_return_none() {
        let b = test_bbox();
        // Subroutine, StateStart, etc. all hit the _ arm
        assert!(shape_intersect(Shape::Subroutine, b, Point::new(200.0, 100.0)).is_none());
        assert!(shape_intersect(Shape::StateStart, b, Point::new(200.0, 100.0)).is_none());
        assert!(shape_intersect(Shape::Note, b, Point::new(200.0, 100.0)).is_none());
        assert!(shape_intersect(Shape::ClassBox, b, Point::new(200.0, 100.0)).is_none());
    }

    // ── Diamond ──

    #[test]
    fn shape_intersect_diamond_right() {
        // Diamond vertices: top (cx, cy-hh), right (cx+hw, cy), bottom (cx, cy+hh), left (cx-hw, cy)
        // Ray rightward from center hits right vertex exactly.
        let b = square_bbox(); // 80x80, hw=hh=40
        let p = shape_intersect(Shape::Diamond, b, Point::new(200.0, 100.0)).unwrap();
        assert_point_near(p, Point::new(140.0, 100.0), "diamond right");
    }

    #[test]
    fn shape_intersect_diamond_top() {
        let b = square_bbox();
        let p = shape_intersect(Shape::Diamond, b, Point::new(100.0, 0.0)).unwrap();
        assert_point_near(p, Point::new(100.0, 60.0), "diamond top");
    }

    #[test]
    fn shape_intersect_diamond_bottom() {
        let b = square_bbox();
        let p = shape_intersect(Shape::Diamond, b, Point::new(100.0, 200.0)).unwrap();
        assert_point_near(p, Point::new(100.0, 140.0), "diamond bottom");
    }

    #[test]
    fn shape_intersect_diamond_left() {
        let b = square_bbox();
        let p = shape_intersect(Shape::Diamond, b, Point::new(0.0, 100.0)).unwrap();
        assert_point_near(p, Point::new(60.0, 100.0), "diamond left");
    }

    #[test]
    fn shape_intersect_diamond_diagonal() {
        // For a square diamond (hw=hh=40), a 45-degree ray hits the midpoint
        // of the top-right edge: from (100,60) to (140,100).
        // Midpoint = (120, 80). Ray from center (100,100) at 45° up-right:
        // parametric: (100+t, 100-t). Edge: from (100,60) to (140,100),
        // parametric: (100+40s, 60+40s). Solve: 100+t=100+40s → t=40s;
        // 100-t=60+40s → 100-40s=60+40s → 40=80s → s=0.5, t=20.
        // Hit: (120, 80).
        let b = square_bbox();
        let p = shape_intersect(Shape::Diamond, b, Point::new(200.0, 0.0)).unwrap();
        assert_point_near(p, Point::new(120.0, 80.0), "diamond diagonal up-right");
    }

    // ── Circle ──

    #[test]
    fn shape_intersect_circle_right() {
        let b = BBox::new(100.0, 100.0, 60.0, 60.0); // r = 30
        let p = shape_intersect(Shape::Circle, b, Point::new(200.0, 100.0)).unwrap();
        assert_point_near(p, Point::new(130.0, 100.0), "circle right");
    }

    #[test]
    fn shape_intersect_circle_top() {
        let b = BBox::new(100.0, 100.0, 60.0, 60.0);
        let p = shape_intersect(Shape::Circle, b, Point::new(100.0, 0.0)).unwrap();
        assert_point_near(p, Point::new(100.0, 70.0), "circle top");
    }

    #[test]
    fn shape_intersect_circle_diagonal() {
        let b = BBox::new(100.0, 100.0, 60.0, 60.0); // r = 30
        let p = shape_intersect(Shape::Circle, b, Point::new(200.0, 200.0)).unwrap();
        let offset = 30.0 / 2.0_f64.sqrt();
        assert_point_near(
            p,
            Point::new(100.0 + offset, 100.0 + offset),
            "circle diagonal",
        );
    }

    // ── DoubleCircle ──

    #[test]
    fn shape_intersect_double_circle_right() {
        // DoubleCircle uses the same circle intersect, r = max(w,h)/2
        let b = BBox::new(100.0, 100.0, 70.0, 70.0); // r = 35
        let p = shape_intersect(Shape::DoubleCircle, b, Point::new(200.0, 100.0)).unwrap();
        assert_point_near(p, Point::new(135.0, 100.0), "double circle right");
    }

    #[test]
    fn shape_intersect_double_circle_left() {
        let b = BBox::new(100.0, 100.0, 70.0, 70.0);
        let p = shape_intersect(Shape::DoubleCircle, b, Point::new(0.0, 100.0)).unwrap();
        assert_point_near(p, Point::new(65.0, 100.0), "double circle left");
    }

    // ── Hexagon ──

    #[test]
    fn shape_intersect_hexagon_right() {
        // Hexagon vertices with bbox 80x60: m = h/4 = 15
        // Right vertex: (cx+hw, cy) = (140, 100)
        let b = test_bbox();
        let p = shape_intersect(Shape::Hexagon, b, Point::new(200.0, 100.0)).unwrap();
        assert_point_near(p, Point::new(140.0, 100.0), "hexagon right");
    }

    #[test]
    fn shape_intersect_hexagon_left() {
        let b = test_bbox();
        let p = shape_intersect(Shape::Hexagon, b, Point::new(0.0, 100.0)).unwrap();
        assert_point_near(p, Point::new(60.0, 100.0), "hexagon left");
    }

    #[test]
    fn shape_intersect_hexagon_top() {
        // Top edge goes from (cx-hw+m, cy-hh) to (cx+hw-m, cy-hh) = (75, 70) to (125, 70).
        // Ray straight up from center hits this horizontal segment at (100, 70).
        let b = test_bbox();
        let p = shape_intersect(Shape::Hexagon, b, Point::new(100.0, 0.0)).unwrap();
        assert_point_near(p, Point::new(100.0, 70.0), "hexagon top");
    }

    // ── Parallelogram ──

    #[test]
    fn shape_intersect_parallelogram_right() {
        // skew = h/2 = 30. Vertices:
        //   top-left:  (100-40+30, 70)  = (90, 70)
        //   top-right: (100+40+30, 70)  = (170, 70)
        //   bot-right: (100+40-30, 130) = (110, 130)
        //   bot-left:  (100-40-30, 130) = (30, 130)
        // Ray rightward from (100,100): hits right edge between top-right (170,70) and bot-right (110,130).
        let b = test_bbox();
        let p = shape_intersect(Shape::Parallelogram, b, Point::new(300.0, 100.0)).unwrap();
        // Right edge: (170,70)→(110,130). Parametric: (170-60t, 70+60t).
        // Ray: (100+s, 100). Solve: 100=70+60t → t=0.5. x=170-30=140. Hit: (140, 100).
        assert_point_near(p, Point::new(140.0, 100.0), "parallelogram right");
    }

    #[test]
    fn shape_intersect_parallelogram_top() {
        let b = test_bbox();
        let p = shape_intersect(Shape::Parallelogram, b, Point::new(100.0, 0.0)).unwrap();
        // Top edge: (90,70)→(170,70). Ray upward: (100, 100-s). Hits at y=70, x=100.
        assert_point_near(p, Point::new(100.0, 70.0), "parallelogram top");
    }

    // ── ParallelogramAlt ──

    #[test]
    fn shape_intersect_parallelogram_alt_left() {
        // skew = h/2 = 30. Vertices:
        //   top-left:  (100-40-30, 70) = (30, 70)
        //   top-right: (100+40-30, 70) = (110, 70)
        //   bot-right: (100+40+30, 130) = (170, 130)
        //   bot-left:  (100-40+30, 130) = (90, 130)
        // Ray leftward: hits left edge (30,70)→(90,130).
        let b = test_bbox();
        let p = shape_intersect(Shape::ParallelogramAlt, b, Point::new(0.0, 100.0)).unwrap();
        // Left edge: (30,70)→(90,130). Parametric: (30+60t, 70+60t).
        // Ray: (100-s, 100). y=100: 70+60t=100 → t=0.5. x=30+30=60. Hit: (60, 100).
        assert_point_near(p, Point::new(60.0, 100.0), "parallelogram alt left");
    }

    #[test]
    fn shape_intersect_parallelogram_alt_top() {
        let b = test_bbox();
        let p = shape_intersect(Shape::ParallelogramAlt, b, Point::new(100.0, 0.0)).unwrap();
        // Top edge: (30,70)→(110,70). Straight up hits at (100, 70).
        assert_point_near(p, Point::new(100.0, 70.0), "parallelogram alt top");
    }

    // ── Trapezoid ──

    #[test]
    fn shape_intersect_trapezoid_top() {
        // offset = h/2 = 30. Vertices:
        //   top-left:  (60, 70)
        //   top-right: (140, 70)
        //   bot-right: (170, 130)
        //   bot-left:  (30, 130)
        // Ray straight up hits top edge at (100, 70).
        let b = test_bbox();
        let p = shape_intersect(Shape::Trapezoid, b, Point::new(100.0, 0.0)).unwrap();
        assert_point_near(p, Point::new(100.0, 70.0), "trapezoid top");
    }

    #[test]
    fn shape_intersect_trapezoid_bottom() {
        let b = test_bbox();
        let p = shape_intersect(Shape::Trapezoid, b, Point::new(100.0, 200.0)).unwrap();
        // Bottom edge: (170,130)→(30,130). Straight down hits at (100, 130).
        assert_point_near(p, Point::new(100.0, 130.0), "trapezoid bottom");
    }

    #[test]
    fn shape_intersect_trapezoid_right() {
        let b = test_bbox();
        let p = shape_intersect(Shape::Trapezoid, b, Point::new(300.0, 100.0)).unwrap();
        // Right edge: (140,70)→(170,130). Parametric: (140+30t, 70+60t).
        // Ray: (100+s, 100). y=100: 70+60t=100 → t=0.5. x=140+15=155. Hit: (155, 100).
        assert_point_near(p, Point::new(155.0, 100.0), "trapezoid right");
    }

    // ── TrapezoidAlt ──

    #[test]
    fn shape_intersect_trapezoid_alt_top() {
        // offset = h/2 = 30. Vertices:
        //   top-left:  (30, 70)
        //   top-right: (170, 70)
        //   bot-right: (140, 130)
        //   bot-left:  (60, 130)
        // Ray straight up hits top edge at (100, 70).
        let b = test_bbox();
        let p = shape_intersect(Shape::TrapezoidAlt, b, Point::new(100.0, 0.0)).unwrap();
        assert_point_near(p, Point::new(100.0, 70.0), "trapezoid alt top");
    }

    #[test]
    fn shape_intersect_trapezoid_alt_bottom() {
        let b = test_bbox();
        let p = shape_intersect(Shape::TrapezoidAlt, b, Point::new(100.0, 200.0)).unwrap();
        // Bottom edge: (140,130)→(60,130). Straight down hits at (100, 130).
        assert_point_near(p, Point::new(100.0, 130.0), "trapezoid alt bottom");
    }

    #[test]
    fn shape_intersect_trapezoid_alt_left() {
        let b = test_bbox();
        let p = shape_intersect(Shape::TrapezoidAlt, b, Point::new(0.0, 100.0)).unwrap();
        // Left edge: (60,130)→(30,70). Parametric: (60-30t, 130-60t).
        // Ray: (100-s, 100). y=100: 130-60t=100 → t=0.5. x=60-15=45. Hit: (45, 100).
        assert_point_near(p, Point::new(45.0, 100.0), "trapezoid alt left");
    }

    // ── Cylinder ──

    #[test]
    fn shape_intersect_cylinder_right() {
        // Cylinder uses intersect_rect with expanded height.
        // bbox 80x60, rx = hw = 40, ry = 40 / (2.5 + 80/50) = 40/4.1 ≈ 9.756
        // full_h = 60 + ry ≈ 69.756. Rect intersect with this taller bbox.
        let b = test_bbox();
        let p = shape_intersect(Shape::Cylinder, b, Point::new(200.0, 100.0)).unwrap();
        // Ray rightward: hits right edge at x = cx + w/2 = 140.
        assert_point_near(p, Point::new(140.0, 100.0), "cylinder right");
    }

    #[test]
    fn shape_intersect_cylinder_top() {
        let b = test_bbox();
        let p = shape_intersect(Shape::Cylinder, b, Point::new(100.0, 0.0)).unwrap();
        // full_h = 60 + 40/4.1 ≈ 69.756. half = ≈34.878.
        // Hit at y = 100 - 34.878 ≈ 65.122.
        let rx = 40.0;
        let ry = rx / (2.5 + 80.0 / 50.0);
        let full_hh = (60.0 + ry) / 2.0;
        assert_near(p.x, 100.0, "cylinder top x");
        assert_near(p.y, 100.0 - full_hh, "cylinder top y");
    }

    #[test]
    fn shape_intersect_cylinder_bottom() {
        let b = test_bbox();
        let p = shape_intersect(Shape::Cylinder, b, Point::new(100.0, 200.0)).unwrap();
        let rx = 40.0;
        let ry = rx / (2.5 + 80.0 / 50.0);
        let full_hh = (60.0 + ry) / 2.0;
        assert_near(p.x, 100.0, "cylinder bottom x");
        assert_near(p.y, 100.0 + full_hh, "cylinder bottom y");
    }

    // ── Asymmetric ──

    #[test]
    fn shape_intersect_asymmetric_right() {
        // notch = h/4 = 15. Vertices:
        //   (60, 70), (140, 70), (140, 130), (60, 130), (75, 100)
        // Ray rightward from center (100,100) hits right edge (140,70)→(140,130) at (140,100).
        let b = test_bbox();
        let p = shape_intersect(Shape::Asymmetric, b, Point::new(200.0, 100.0)).unwrap();
        assert_point_near(p, Point::new(140.0, 100.0), "asymmetric right");
    }

    #[test]
    fn shape_intersect_asymmetric_left() {
        // Ray leftward: hits the notch edges. The left side has two edges:
        //   bottom-left (60,130) → notch (75,100) and notch (75,100) → top-left (60,70).
        // Ray from center (100,100) going left: (100-s, 100). Hits notch vertex at (75, 100).
        let b = test_bbox();
        let p = shape_intersect(Shape::Asymmetric, b, Point::new(0.0, 100.0)).unwrap();
        assert_point_near(p, Point::new(75.0, 100.0), "asymmetric left notch");
    }

    #[test]
    fn shape_intersect_asymmetric_top() {
        let b = test_bbox();
        let p = shape_intersect(Shape::Asymmetric, b, Point::new(100.0, 0.0)).unwrap();
        // Top edge: (60,70)→(140,70). Straight up hits at (100, 70).
        assert_point_near(p, Point::new(100.0, 70.0), "asymmetric top");
    }

    // ── Edge case: point on boundary returns Some for non-rect shapes ──

    #[test]
    fn shape_intersect_all_non_rect_return_some() {
        let b = test_bbox();
        let adj = Point::new(200.0, 100.0); // ray to the right
        let non_rect_shapes = [
            Shape::Diamond,
            Shape::Circle,
            Shape::DoubleCircle,
            Shape::Hexagon,
            Shape::Parallelogram,
            Shape::ParallelogramAlt,
            Shape::Trapezoid,
            Shape::TrapezoidAlt,
            Shape::Cylinder,
            Shape::Asymmetric,
            Shape::Stadium,
        ];
        for shape in non_rect_shapes {
            assert!(
                shape_intersect(shape, b, adj).is_some(),
                "{shape:?} should return Some",
            );
        }
    }

    // ── Intersection points lie on or near the shape boundary ──

    #[test]
    fn shape_intersect_diamond_boundary_distance() {
        // For a square diamond (hw=hh=40), the boundary is the set of points
        // where |x - cx|/hw + |y - cy|/hh = 1.
        let b = square_bbox();
        let (cx, cy, hw, hh) = (100.0, 100.0, 40.0, 40.0);
        let targets = [
            Point::new(200.0, 100.0),
            Point::new(100.0, 0.0),
            Point::new(0.0, 100.0),
            Point::new(100.0, 200.0),
            Point::new(200.0, 0.0),
            Point::new(0.0, 200.0),
            Point::new(200.0, 200.0),
            Point::new(0.0, 0.0),
        ];
        for t in targets {
            let p = shape_intersect(Shape::Diamond, b, t).unwrap();
            let diamond_eq = (p.x - cx).abs() / hw + (p.y - cy).abs() / hh;
            assert!(
                (diamond_eq - 1.0).abs() < TOL,
                "diamond boundary: target ({}, {}), hit ({}, {}), eq={diamond_eq}",
                t.x, t.y, p.x, p.y,
            );
        }
    }

    #[test]
    fn shape_intersect_circle_boundary_distance() {
        let b = BBox::new(100.0, 100.0, 60.0, 60.0); // r = 30
        let r = 30.0;
        let targets = [
            Point::new(200.0, 100.0),
            Point::new(100.0, 0.0),
            Point::new(0.0, 100.0),
            Point::new(100.0, 200.0),
            Point::new(200.0, 200.0),
            Point::new(0.0, 0.0),
        ];
        for t in targets {
            let p = shape_intersect(Shape::Circle, b, t).unwrap();
            let dist = Point::new(100.0, 100.0).distance_to(p);
            assert!(
                (dist - r).abs() < TOL,
                "circle boundary: target ({}, {}), hit ({}, {}), dist={dist}",
                t.x, t.y, p.x, p.y,
            );
        }
    }

    #[test]
    fn shape_intersect_hexagon_boundary_on_polygon() {
        // Verify hit points lie on the hexagon polygon edges.
        let b = test_bbox();
        let (cx, cy) = (100.0, 100.0);
        let (hw, hh) = (40.0, 30.0);
        let m = 60.0 / 4.0; // 15
        let verts = [
            Point::new(cx - hw + m, cy - hh),
            Point::new(cx + hw - m, cy - hh),
            Point::new(cx + hw, cy),
            Point::new(cx + hw - m, cy + hh),
            Point::new(cx - hw + m, cy + hh),
            Point::new(cx - hw, cy),
        ];

        let targets = [
            Point::new(200.0, 100.0),
            Point::new(0.0, 100.0),
            Point::new(100.0, 0.0),
            Point::new(100.0, 200.0),
        ];

        for t in targets {
            let p = shape_intersect(Shape::Hexagon, b, t).unwrap();
            // Point should be on one of the polygon edges.
            let on_edge = is_on_polygon_edge(&verts, p);
            assert!(
                on_edge,
                "hexagon: target ({}, {}), hit ({}, {}) not on edge",
                t.x, t.y, p.x, p.y,
            );
        }
    }

    /// Check whether a point lies on any edge of a polygon (within tolerance).
    fn is_on_polygon_edge(verts: &[Point], p: Point) -> bool {
        let n = verts.len();
        for i in 0..n {
            let a = verts[i];
            let b = verts[(i + 1) % n];
            if point_on_segment(a, b, p) {
                return true;
            }
        }
        false
    }

    /// Check whether point p lies on segment a→b (within tolerance).
    fn point_on_segment(a: Point, b: Point, p: Point) -> bool {
        let ab = ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt();
        if ab < f64::EPSILON {
            return a.distance_to(p) < TOL;
        }
        let ap = a.distance_to(p);
        let pb = p.distance_to(b);
        (ap + pb - ab).abs() < TOL
    }
}
