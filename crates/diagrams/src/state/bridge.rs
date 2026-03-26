use std::collections::{BTreeMap, HashSet};

use rusty_mermaid_core::{
    BBox, Point, Shape, SimpleTextMeasure, Style, TextMeasure, TextStyle, intersect_circle,
    intersect_polygon, intersect_rect,
};
use rusty_mermaid_dagre::{DagreConfig, EdgeLabel, NodeLabel};
use rusty_mermaid_graph::{Graph, NodeId};

use crate::common::layout::{ArrowEnd, EdgeLayout, NodeLayout, StrokeType};

use super::center::{
    center_bullseyes, center_content, center_external_connections, fix_region_order,
};
use super::ir::{NotePosition, StateDiagram, StateKind, StateNode, StateNote, StateTransition};
use super::scope::{ScopeCtx, add_scope, collect_all_notes, region_count, resolve_state_styles};

pub(super) const PADDING_X: f64 = 16.0;
pub(super) const PADDING_Y: f64 = 8.0;
pub(super) const START_END_SIZE: f64 = 16.0;
pub(super) const FORK_JOIN_WIDTH: f64 = 70.0;
pub(super) const FORK_JOIN_HEIGHT: f64 = 7.0;
pub(super) const CHOICE_SIZE: f64 = 28.0;
const NOTE_PADDING: f64 = 10.0;
const NOTE_GAP: f64 = 10.0;
const COMPOUND_LABEL_PAD: f64 = 20.0;
const COMPOUND_HEADER_OFFSET: f64 = 24.0;
const EDGE_LABEL_FONT_SIZE: f64 = 12.0;
const EDGE_LABEL_PAD: f64 = 4.0;
pub(super) const BULLSEYE_MIN_GAP: f64 = 10.0;
/// Extra height added above compound nodes for the title + separator header.
const COMPOUND_HEADER_HEIGHT: f64 = 0.0;

/// Layout result: node positions and edge points.
#[derive(Debug)]
pub struct LayoutResult {
    pub nodes: Vec<NodeLayout>,
    pub edges: Vec<EdgeLayout>,
    pub width: f64,
    pub height: f64,
    /// Dashed divider lines between concurrent regions.
    pub dividers: Vec<DividerLine>,
    /// Dashed rectangles around each concurrent region.
    pub region_rects: Vec<RegionRect>,
}

/// Bounding box for a concurrent region (rendered as a dashed rectangle).
#[derive(Debug)]
pub struct RegionRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// A dashed line separating concurrent regions (vertical for side-by-side layout).
#[derive(Debug)]
pub struct DividerLine {
    pub start: Point,
    pub end: Point,
}

/// Layout with the default text measurer.
pub fn layout(diagram: &StateDiagram) -> LayoutResult {
    layout_with_measurer(diagram, &SimpleTextMeasure::default())
}

/// Layout with a custom text measurer.
pub fn layout_with_measurer(diagram: &StateDiagram, measurer: &impl TextMeasure) -> LayoutResult {
    let mut graph = Graph::new();
    let style = TextStyle::default();
    let mut id_map: BTreeMap<String, NodeId> = BTreeMap::new();
    let mut all_transitions: Vec<&StateTransition> = Vec::new();
    let mut synthetic_ids: HashSet<String> = HashSet::new();

    // Build graph: nodes, edges, compound hierarchy
    let mut ctx = ScopeCtx {
        graph: &mut graph,
        id_map: &mut id_map,
        synthetic_ids: &mut synthetic_ids,
        measurer,
        style: &style,
    };
    add_scope(
        &diagram.states,
        &diagram.transitions,
        None,
        &mut ctx,
        &mut all_transitions,
    );

    // Run dagre + post-layout adjustments
    run_dagre_layout(diagram, &mut graph, &id_map);

    let node_styles = resolve_state_styles(diagram);
    let nid_to_id: BTreeMap<NodeId, &str> =
        id_map.iter().map(|(id, &nid)| (nid, id.as_str())).collect();

    let mut nodes = extract_nodes(&graph, &id_map, &synthetic_ids, &node_styles, diagram);
    position_notes(diagram, measurer, &style, &mut nodes);
    adjust_compound_widths(measurer, &style, &mut nodes);
    let (mut max_x, mut max_y, x_shift) = recompute_bounds_and_shift(&mut nodes);
    let mut edges = extract_edges(
        &graph,
        &nid_to_id,
        &all_transitions,
        diagram,
        measurer,
        &style,
        x_shift,
    );
    expand_bounds_for_edges(&mut nodes, &mut edges, &mut max_x, &mut max_y);
    let (dividers, region_rects) = compute_dividers_and_regions(&nodes);

    LayoutResult {
        nodes,
        edges,
        width: max_x,
        height: max_y,
        dividers,
        region_rects,
    }
}

fn run_dagre_layout(
    diagram: &StateDiagram,
    graph: &mut Graph<NodeLabel, EdgeLabel>,
    id_map: &BTreeMap<String, NodeId>,
) {
    let config = DagreConfig {
        rankdir: diagram.direction,
        ..Default::default()
    };
    rusty_mermaid_dagre::pipeline::layout(graph, &config);
    fix_region_order(diagram, graph, id_map);
    center_content(diagram, graph, id_map);
    center_bullseyes(diagram, graph, id_map);
    center_external_connections(diagram, graph, id_map);
}

fn extract_nodes(
    graph: &Graph<NodeLabel, EdgeLabel>,
    id_map: &BTreeMap<String, NodeId>,
    synthetic_ids: &HashSet<String>,
    node_styles: &BTreeMap<&str, Style>,
    diagram: &StateDiagram,
) -> Vec<NodeLayout> {
    let mut nodes = Vec::new();
    for (id_str, &nid) in id_map {
        if synthetic_ids.contains(id_str) {
            continue;
        }
        let Some(n) = graph.node(nid) else { continue };
        let label = find_state_label(&diagram.states, id_str).unwrap_or_else(|| id_str.to_string());
        nodes.push(NodeLayout {
            id: id_str.clone(),
            label,
            x: n.x,
            y: n.y,
            width: n.width,
            height: n.height,
            is_compound: is_compound_state(&diagram.states, id_str),
            shape: node_shape(&diagram.states, id_str),
            custom_style: node_styles.get(id_str.as_str()).cloned(),
            region_count: region_count(&diagram.states, id_str),
        });
    }
    nodes
}

fn position_notes(
    diagram: &StateDiagram,
    measurer: &impl TextMeasure,
    style: &TextStyle,
    nodes: &mut Vec<NodeLayout>,
) {
    let all_notes = collect_all_notes(diagram);
    for note in &all_notes {
        let Some(state_node) = nodes.iter().find(|n| n.id == note.state_id) else {
            continue;
        };
        let ts = measurer.measure(&note.text, style);
        let note_w = ts.width + NOTE_PADDING * 2.0;
        let note_h = ts.height + NOTE_PADDING * 2.0;

        let note_x = match note.position {
            NotePosition::Right => state_node.x + state_node.width / 2.0 + NOTE_GAP + note_w / 2.0,
            NotePosition::Left => state_node.x - state_node.width / 2.0 - NOTE_GAP - note_w / 2.0,
        };
        let note_y = state_node.y;

        nodes.push(NodeLayout {
            id: format!("{}-note", note.state_id),
            label: note.text.clone(),
            x: note_x,
            y: note_y,
            width: note_w,
            height: note_h,
            is_compound: false,
            shape: Shape::Note,
            custom_style: None,
            region_count: 0,
        });
    }
}

/// Expand compound node widths to fit their label text.
fn adjust_compound_widths(
    measurer: &impl TextMeasure,
    style: &TextStyle,
    nodes: &mut [NodeLayout],
) {
    for node in nodes.iter_mut() {
        if node.is_compound {
            node.height += COMPOUND_HEADER_HEIGHT;
            node.y -= COMPOUND_HEADER_HEIGHT / 2.0;

            let label_w = measurer.measure(&node.label, style).width;
            let min_width = label_w + COMPOUND_LABEL_PAD;
            if node.width < min_width {
                node.width = min_width;
            }
        }
    }
}

/// Recompute extents after compound adjustments and notes, shift nodes into
/// positive coordinates. Returns (max_x, max_y, x_shift).
fn recompute_bounds_and_shift(nodes: &mut [NodeLayout]) -> (f64, f64, f64) {
    let mut min_x: f64 = 0.0;
    let mut max_x: f64 = 0.0;
    let mut max_y: f64 = 0.0;
    for node in nodes.iter() {
        min_x = min_x.min(node.x - node.width / 2.0);
        max_x = max_x.max(node.x + node.width / 2.0);
        max_y = max_y.max(node.y + node.height / 2.0);
    }

    let x_shift = if min_x < 0.0 { -min_x } else { 0.0 };
    if x_shift > 0.0 {
        for node in nodes.iter_mut() {
            node.x += x_shift;
        }
        max_x += x_shift;
    }
    (max_x, max_y, x_shift)
}

/// Extract edges matching real transitions, clip endpoints at node shapes.
fn extract_edges(
    graph: &Graph<NodeLabel, EdgeLabel>,
    nid_to_id: &BTreeMap<NodeId, &str>,
    all_transitions: &[&StateTransition],
    diagram: &StateDiagram,
    measurer: &impl TextMeasure,
    style: &TextStyle,
    x_shift: f64,
) -> Vec<EdgeLayout> {
    let mut edges = Vec::new();
    for eid in graph.edge_ids() {
        let Some((src, dst)) = graph.edge_endpoints(eid) else {
            continue;
        };
        let (Some(&src_id), Some(&dst_id)) = (nid_to_id.get(&src), nid_to_id.get(&dst)) else {
            continue;
        };
        let matched = all_transitions.iter().find(|t| {
            resolve_pseudo(&t.src, src_id, true) && resolve_pseudo(&t.dst, dst_id, false)
        });
        let Some(transition) = matched else { continue };
        let Some(e) = graph.edge(eid) else { continue };
        let mut points: Vec<Point> = e
            .points
            .iter()
            .map(|p| Point::new(p.x + x_shift, p.y))
            .collect();

        clip_edge_endpoints(
            graph,
            &mut points,
            src,
            dst,
            src_id,
            dst_id,
            diagram,
            x_shift,
        );

        let label_size = transition.label.as_ref().map(|l| {
            let edge_style = TextStyle {
                font_size: EDGE_LABEL_FONT_SIZE,
                ..style.clone()
            };
            let ts = measurer.measure(l, &edge_style);
            (ts.width, ts.height)
        });
        edges.push(EdgeLayout {
            src: src_id.to_string(),
            dst: dst_id.to_string(),
            points,
            label: transition.label.clone(),
            label_size,
            stroke: StrokeType::Normal,
            start_arrow: ArrowEnd::None,
            end_arrow: ArrowEnd::Arrow,
            custom_style: None,
        });
    }
    edges
}

/// Re-clip edge endpoints for non-rect shapes and restore bullseye alignment.
fn clip_edge_endpoints(
    graph: &Graph<NodeLabel, EdgeLabel>,
    points: &mut [Point],
    src: NodeId,
    dst: NodeId,
    src_id: &str,
    dst_id: &str,
    diagram: &StateDiagram,
    x_shift: f64,
) {
    if points.len() < 2 {
        return;
    }
    let aligned_x = {
        let all_same = points.windows(2).all(|w| (w[0].x - w[1].x).abs() < 0.5);
        if all_same { Some(points[0].x) } else { None }
    };

    if let Some(src_node) = graph.node(src) {
        let src_shape = node_shape(&diagram.states, src_id);
        let src_bbox = BBox::new(
            src_node.x + x_shift,
            src_node.y,
            src_node.width,
            src_node.height,
        );
        if let Some(p) = state_shape_intersect(src_shape, src_bbox, points[1]) {
            points[0] = p;
        }
    }

    let last = points.len() - 1;
    if let Some(dst_node) = graph.node(dst) {
        let dst_shape = node_shape(&diagram.states, dst_id);
        let dst_bbox = BBox::new(
            dst_node.x + x_shift,
            dst_node.y,
            dst_node.width,
            dst_node.height,
        );
        if let Some(p) = state_shape_intersect(dst_shape, dst_bbox, points[last - 1]) {
            points[last] = p;
        }
    }

    if let Some(ax) = aligned_x {
        points[0].x = ax;
        points[last].x = ax;
    }
}

/// Expand bounds for edge control points and labels, shift everything into view.
fn expand_bounds_for_edges(
    nodes: &mut [NodeLayout],
    edges: &mut [EdgeLayout],
    max_x: &mut f64,
    max_y: &mut f64,
) {
    let mut min_x: f64 = 0.0;
    let mut min_y: f64 = 0.0;
    for edge in edges.iter() {
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
            let lw = size.0 + EDGE_LABEL_PAD * 2.0;
            let lh = size.1 + EDGE_LABEL_PAD * 2.0;
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
        for edge in edges.iter_mut() {
            for pt in &mut edge.points {
                pt.x += dx;
                pt.y += dy;
            }
        }
        *max_x += dx;
        *max_y += dy;
    }
}

fn compute_dividers_and_regions(nodes: &[NodeLayout]) -> (Vec<DividerLine>, Vec<RegionRect>) {
    let mut dividers = Vec::new();
    let mut region_rects = Vec::new();
    for node in nodes {
        if node.region_count < 2 {
            continue;
        }
        let compound_top = node.y - node.height / 2.0 + COMPOUND_HEADER_OFFSET;
        let compound_bottom = node.y + node.height / 2.0;
        let compound_left = node.x - node.width / 2.0;
        let compound_right = node.x + node.width / 2.0;

        let n = node.region_count as f64;
        let partition_width = (compound_right - compound_left) / n;
        let mut div_xs = Vec::new();
        for i in 1..node.region_count {
            let div_x = compound_left + partition_width * i as f64;
            div_xs.push(div_x);
            dividers.push(DividerLine {
                start: Point::new(div_x, compound_top),
                end: Point::new(div_x, compound_bottom),
            });
        }

        let mut boundaries = vec![compound_left];
        boundaries.extend_from_slice(&div_xs);
        boundaries.push(compound_right);
        for w in boundaries.windows(2) {
            region_rects.push(RegionRect {
                x: w[0],
                y: compound_top,
                width: w[1] - w[0],
                height: compound_bottom - compound_top,
            });
        }
    }
    (dividers, region_rects)
}

fn resolve_pseudo(transition_id: &str, node_id: &str, is_source: bool) -> bool {
    if transition_id == "[*]" {
        // [*] as source → only matches _start pseudo-states
        // [*] as destination → only matches _end pseudo-states
        if is_source {
            node_id.ends_with("[*]_start")
        } else {
            node_id.ends_with("[*]_end")
        }
    } else if transition_id == node_id {
        true
    } else {
        // Composite redirect: src "Active" → "Active.[*]_end" (exit)
        //                     dst "Active" → "Active.[*]_start" (entry)
        let prefix = format!("{transition_id}.");
        if !node_id.starts_with(&prefix) {
            return false;
        }
        let suffix = &node_id[prefix.len()..];
        if is_source {
            suffix == "[*]_end"
        } else {
            suffix == "[*]_start"
        }
    }
}

/// Check if a state ID refers to a composite (compound) state.
pub(super) fn is_compound_state(states: &[super::ir::StateNode], id: &str) -> bool {
    for state in states {
        if state.id == id {
            return state.is_composite();
        }
        if let StateKind::Composite { children, .. } = &state.kind
            && is_compound_state(children, id)
        {
            return true;
        }
    }
    false
}

/// Recursively find a state's label by ID across all nesting levels.
fn find_state_label(states: &[super::ir::StateNode], id: &str) -> Option<String> {
    for state in states {
        if state.id == id {
            return state.label.clone().or_else(|| Some(state.id.clone()));
        }
        if let StateKind::Composite { children, .. } = &state.kind
            && let Some(label) = find_state_label(children, id)
        {
            return Some(label);
        }
    }
    None
}

/// Determine the rendering shape for a node based on its ID and IR kind.
/// Re-clip an edge endpoint to the actual node shape instead of dagre's
/// default bounding-box rectangle.
fn state_shape_intersect(shape: Shape, bbox: BBox, adj: Point) -> Option<Point> {
    let (cx, cy) = (bbox.x, bbox.y);
    let center = Point::new(cx, cy);
    let target = adj;
    let (hw, hh) = (bbox.width / 2.0, bbox.height / 2.0);

    let p = match shape {
        Shape::Choice => {
            let verts = [
                Point::new(cx, cy - hh),
                Point::new(cx + hw, cy),
                Point::new(cx, cy + hh),
                Point::new(cx - hw, cy),
            ];
            intersect_polygon(&verts, center, target)
        }
        Shape::StateStart | Shape::StateEnd | Shape::History => {
            let r = bbox.width.max(bbox.height) / 2.0;
            intersect_circle(center, r, target)
        }
        Shape::RoundedRect | Shape::ForkJoin | Shape::Note => intersect_rect(&bbox, target),
        _ => return None,
    };

    Some(p)
}

fn node_shape(states: &[super::ir::StateNode], id: &str) -> Shape {
    if id.ends_with("[*]_start") {
        return Shape::StateStart;
    }
    if id.ends_with("[*]_end") {
        return Shape::StateEnd;
    }
    match find_state_kind(states, id) {
        Some(StateKind::Fork | StateKind::Join) => Shape::ForkJoin,
        Some(StateKind::Choice) => Shape::Choice,
        Some(StateKind::Start) => Shape::StateStart,
        Some(StateKind::End) => Shape::StateEnd,
        Some(StateKind::History) => Shape::History,
        _ => Shape::RoundedRect,
    }
}

/// Recursively find a state's kind by ID across all nesting levels.
fn find_state_kind<'a>(states: &'a [super::ir::StateNode], id: &str) -> Option<&'a StateKind> {
    for state in states {
        if state.id == id {
            return Some(&state.kind);
        }
        if let StateKind::Composite { children, .. } = &state.kind
            && let Some(kind) = find_state_kind(children, id)
        {
            return Some(kind);
        }
    }
    None
}

#[cfg(test)]
#[path = "bridge_tests.rs"]
mod bridge_tests;
