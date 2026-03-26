use std::collections::{BTreeMap, HashSet};

use rusty_mermaid_core::{
    BBox, Shape, intersect_circle, intersect_polygon, intersect_rect, Point, SimpleTextMeasure,
    Style, TextMeasure, TextStyle,
};
use rusty_mermaid_dagre::{DagreConfig, EdgeLabel, NodeLabel};
use rusty_mermaid_graph::{Graph, NodeId};

use crate::common::layout::{ArrowEnd, EdgeLayout, NodeLayout, StrokeType};

use super::ir::{NotePosition, StateDiagram, StateKind, StateNode, StateNote, StateTransition};

const PADDING_X: f64 = 16.0;
const PADDING_Y: f64 = 8.0;
const START_END_SIZE: f64 = 16.0;
const FORK_JOIN_WIDTH: f64 = 70.0;
const FORK_JOIN_HEIGHT: f64 = 7.0;
const CHOICE_SIZE: f64 = 28.0;
const NOTE_PADDING: f64 = 10.0;
const NOTE_GAP: f64 = 10.0;
const COMPOUND_LABEL_PAD: f64 = 20.0;
const COMPOUND_HEADER_OFFSET: f64 = 24.0;
const EDGE_LABEL_FONT_SIZE: f64 = 12.0;
const EDGE_LABEL_PAD: f64 = 4.0;
const BULLSEYE_MIN_GAP: f64 = 10.0;
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
    let mut g = Graph::new();
    let style = TextStyle::default();
    let mut id_map: BTreeMap<String, NodeId> = BTreeMap::new();
    let mut all_transitions: Vec<&StateTransition> = Vec::new();
    let mut synthetic_ids: HashSet<String> = HashSet::new();

    // Build graph: nodes, edges, compound hierarchy
    let mut ctx = ScopeCtx {
        g: &mut g,
        id_map: &mut id_map,
        synthetic_ids: &mut synthetic_ids,
        measurer,
        style: &style,
    };
    add_scope(&diagram.states, &diagram.transitions, None, &mut ctx, &mut all_transitions);

    // Run dagre + post-layout adjustments
    run_dagre_layout(diagram, &mut g, &id_map);

    let node_styles = resolve_state_styles(diagram);
    let nid_to_id: BTreeMap<NodeId, &str> = id_map.iter().map(|(id, &nid)| (nid, id.as_str())).collect();

    let mut nodes = extract_nodes(&g, &id_map, &synthetic_ids, &node_styles, diagram);
    position_notes(diagram, measurer, &style, &mut nodes);
    adjust_compound_widths(measurer, &style, &mut nodes);
    let (mut max_x, mut max_y, x_shift) = recompute_bounds_and_shift(&mut nodes);
    let mut edges = extract_edges(
        &g, &nid_to_id, &all_transitions, diagram, measurer, &style, x_shift,
    );
    expand_bounds_for_edges(&mut nodes, &mut edges, &mut max_x, &mut max_y);
    let (dividers, region_rects) = compute_dividers_and_regions(&nodes);

    LayoutResult { nodes, edges, width: max_x, height: max_y, dividers, region_rects }
}

fn run_dagre_layout(
    diagram: &StateDiagram,
    g: &mut Graph<NodeLabel, EdgeLabel>,
    id_map: &BTreeMap<String, NodeId>,
) {
    let config = DagreConfig {
        rankdir: diagram.direction,
        ..Default::default()
    };
    rusty_mermaid_dagre::pipeline::layout(g, &config);
    fix_region_order(diagram, g, id_map);
    center_content(diagram, g, id_map);
    center_bullseyes(diagram, g, id_map);
    center_external_connections(diagram, g, id_map);
}

fn extract_nodes(
    g: &Graph<NodeLabel, EdgeLabel>,
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
        let Some(n) = g.node(nid) else { continue };
        let label = find_state_label(&diagram.states, id_str)
            .unwrap_or_else(|| id_str.to_string());
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
        let Some(state_node) = nodes.iter().find(|n| n.id == note.state_id) else { continue };
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
fn adjust_compound_widths(measurer: &impl TextMeasure, style: &TextStyle, nodes: &mut [NodeLayout]) {
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
    g: &Graph<NodeLabel, EdgeLabel>,
    nid_to_id: &BTreeMap<NodeId, &str>,
    all_transitions: &[&StateTransition],
    diagram: &StateDiagram,
    measurer: &impl TextMeasure,
    style: &TextStyle,
    x_shift: f64,
) -> Vec<EdgeLayout> {
    let mut edges = Vec::new();
    for eid in g.edge_ids() {
        let Some((src, dst)) = g.edge_endpoints(eid) else { continue };
        let (Some(&src_id), Some(&dst_id)) = (nid_to_id.get(&src), nid_to_id.get(&dst)) else {
            continue;
        };
        let matched = all_transitions.iter().find(|t| {
            resolve_pseudo(&t.src, src_id, true) && resolve_pseudo(&t.dst, dst_id, false)
        });
        let Some(transition) = matched else { continue };
        let Some(e) = g.edge(eid) else { continue };
        let mut points: Vec<Point> = e.points.iter().map(|p| Point::new(p.x + x_shift, p.y)).collect();

        clip_edge_endpoints(g, &mut points, src, dst, src_id, dst_id, diagram, x_shift);

        let label_size = transition.label.as_ref().map(|l| {
            let edge_style = TextStyle { font_size: EDGE_LABEL_FONT_SIZE, ..style.clone() };
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
    g: &Graph<NodeLabel, EdgeLabel>,
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

    if let Some(src_node) = g.node(src) {
        let src_shape = node_shape(&diagram.states, src_id);
        let src_bbox = BBox::new(src_node.x + x_shift, src_node.y, src_node.width, src_node.height);
        if let Some(p) = state_shape_intersect(src_shape, src_bbox, points[1]) {
            points[0] = p;
        }
    }

    let last = points.len() - 1;
    if let Some(dst_node) = g.node(dst) {
        let dst_shape = node_shape(&diagram.states, dst_id);
        let dst_bbox = BBox::new(dst_node.x + x_shift, dst_node.y, dst_node.width, dst_node.height);
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
            if edge.points.len() < 2 { continue; }
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

/// Enforce declaration order for concurrent regions.
/// Dagre's order phase may swap region sub-compounds. If region_0 ends up
/// to the right of region_1, mirror all descendants around the compound center.
fn fix_region_order(
    diagram: &StateDiagram,
    g: &mut Graph<NodeLabel, EdgeLabel>,
    id_map: &BTreeMap<String, NodeId>,
) {
    for s in &diagram.states {
        fix_region_order_for_state(s, g, id_map);
    }
}

fn fix_region_order_for_state(
    state: &StateNode,
    g: &mut Graph<NodeLabel, EdgeLabel>,
    id_map: &BTreeMap<String, NodeId>,
) {
    let StateKind::Composite { regions, children, .. } = &state.kind else { return };

    // Recurse into children first
    for child in children {
        fix_region_order_for_state(child, g, id_map);
    }

    if regions.len() < 2 {
        return;
    }

    // Check if regions are in declaration order (left-to-right by x)
    let mut region_xs: Vec<(usize, f64)> = Vec::new();
    for (i, _) in regions.iter().enumerate() {
        let rk = format!("{}._region_{}", state.id, i);
        if let Some(&rnid) = id_map.get(&rk)
            && let Some(rn) = g.node(rnid)
        {
            region_xs.push((i, rn.x));
        }
    }
    if region_xs.len() < 2 {
        return;
    }

    // Check if sorted by x matches declaration order
    let in_order = region_xs.windows(2).all(|w| w[0].1 <= w[1].1);
    if in_order {
        return;
    }

    // Need to mirror: flip all descendants' x around compound center
    let Some(&compound_nid) = id_map.get(&state.id) else { return };
    let Some(compound_node) = g.node(compound_nid) else { return };
    let cx = compound_node.x;

    // Collect all descendant node IDs
    let mut descendants = Vec::new();
    collect_descendants(g, compound_nid, &mut descendants);

    // Mirror node positions
    for &nid in &descendants {
        if let Some(n) = g.node_mut(nid) {
            n.x = 2.0 * cx - n.x;
        }
    }

    // Mirror edge points for edges fully within the compound
    let desc_set: HashSet<NodeId> = descendants.iter().copied().collect();
    for eid in g.edge_ids().collect::<Vec<_>>() {
        let Some((src, dst)) = g.edge_endpoints(eid) else { continue };
        if desc_set.contains(&src)
            && desc_set.contains(&dst)
            && let Some(e) = g.edge_mut(eid)
        {
            for pt in &mut e.points {
                pt.x = 2.0 * cx - pt.x;
            }
        }
    }
}

fn collect_descendants(g: &Graph<NodeLabel, EdgeLabel>, nid: NodeId, out: &mut Vec<NodeId>) {
    for child in g.children(nid).collect::<Vec<_>>() {
        out.push(child);
        collect_descendants(g, child, out);
    }
}

/// Center composite content within compound bounds.
/// Non-concurrent: centers all descendants on the compound center.
/// Concurrent: centers each region's descendants within its equal-width partition.
fn center_content(
    diagram: &StateDiagram,
    g: &mut Graph<NodeLabel, EdgeLabel>,
    id_map: &BTreeMap<String, NodeId>,
) {
    for s in &diagram.states {
        center_content_for_state(s, g, id_map);
    }
}

fn center_content_for_state(
    state: &StateNode,
    g: &mut Graph<NodeLabel, EdgeLabel>,
    id_map: &BTreeMap<String, NodeId>,
) {
    let StateKind::Composite { regions, children, .. } = &state.kind else { return };
    // Recurse into children first (handles nested composites)
    for child in children {
        center_content_for_state(child, g, id_map);
    }

    let Some(&compound_nid) = id_map.get(&state.id) else { return };
    let Some(compound_node) = g.node(compound_nid) else { return };
    let compound_cx = compound_node.x;
    let compound_left = compound_cx - compound_node.width / 2.0;
    let compound_right = compound_cx + compound_node.width / 2.0;

    // Build list of (root_nid, descendants, target_cx) for each partition.
    // Non-concurrent: one partition = entire compound.
    // Concurrent: one partition per region, equal-width.
    let partitions: Vec<(NodeId, Vec<NodeId>, f64)> = if regions.len() >= 2 {
        let n = regions.len() as f64;
        let pw = (compound_right - compound_left) / n;

        let mut parts: Vec<(NodeId, Vec<NodeId>, f64, f64)> = Vec::new();
        for (i, _) in regions.iter().enumerate() {
            let rk = format!("{}._region_{}", state.id, i);
            let Some(&rnid) = id_map.get(&rk) else { continue };
            let mut desc = Vec::new();
            collect_descendants(g, rnid, &mut desc);
            let cx = content_bbox_cx(g, &desc);
            parts.push((rnid, desc, cx, cx)); // cx used for sorting
        }
        if parts.len() < 2 {
            return;
        }
        parts.sort_by(|a, b| a.3.total_cmp(&b.3));
        parts
            .into_iter()
            .enumerate()
            .map(|(idx, (rnid, desc, _, _))| {
                let target = compound_left + pw * (idx as f64 + 0.5);
                (rnid, desc, target)
            })
            .collect()
    } else {
        let mut desc = Vec::new();
        collect_descendants(g, compound_nid, &mut desc);
        if desc.is_empty() {
            return;
        }
        vec![(compound_nid, desc, compound_cx)]
    };

    for (root_nid, descendants, target_cx) in &partitions {
        let cx = content_bbox_cx(g, descendants);
        let dx = target_cx - cx;
        if dx.abs() < 0.5 {
            continue;
        }

        // Shift the partition root (region compound for concurrent, skip for non-concurrent)
        if *root_nid != compound_nid
            && let Some(rn) = g.node_mut(*root_nid)
        {
            rn.x += dx;
        }
        // Shift all descendants
        for &nid in descendants {
            if let Some(n) = g.node_mut(nid) {
                n.x += dx;
            }
        }
        // Shift edges fully within this partition
        let desc_set: HashSet<NodeId> = std::iter::once(*root_nid)
            .chain(descendants.iter().copied())
            .collect();
        for eid in g.edge_ids().collect::<Vec<_>>() {
            let Some((src, dst)) = g.edge_endpoints(eid) else { continue };
            if desc_set.contains(&src)
                && desc_set.contains(&dst)
                && let Some(e) = g.edge_mut(eid)
            {
                for pt in &mut e.points {
                    pt.x += dx;
                }
            }
        }
    }
}

/// Compute the horizontal center of a group of nodes' bounding box.
fn content_bbox_cx(g: &Graph<NodeLabel, EdgeLabel>, nodes: &[NodeId]) -> f64 {
    let (mut min_x, mut max_x) = (f64::MAX, f64::MIN);
    for &nid in nodes {
        if let Some(n) = g.node(nid) {
            min_x = min_x.min(n.x - n.width / 2.0);
            max_x = max_x.max(n.x + n.width / 2.0);
        }
    }
    (min_x + max_x) / 2.0
}

/// Center outer [*]_start / [*]_end bullseyes on their connected compound.
/// After dagre layout, the bullseye x may not align with the compound center.
fn center_bullseyes(
    diagram: &StateDiagram,
    g: &mut Graph<NodeLabel, EdgeLabel>,
    id_map: &BTreeMap<String, NodeId>,
) {
    center_bullseyes_in_scope(
        &diagram.transitions,
        &diagram.states,
        "",
        g,
        id_map,
    );
    // Recurse into composites
    for s in &diagram.states {
        center_bullseyes_in_state(s, g, id_map);
    }
}

fn center_bullseyes_in_state(
    state: &StateNode,
    g: &mut Graph<NodeLabel, EdgeLabel>,
    id_map: &BTreeMap<String, NodeId>,
) {
    let StateKind::Composite { transitions, children, .. } = &state.kind else { return };
    let prefix = format!("{}.", state.id);
    center_bullseyes_in_scope(transitions, children, &prefix, g, id_map);
    for child in children {
        center_bullseyes_in_state(child, g, id_map);
    }
}

fn center_bullseyes_in_scope(
    transitions: &[StateTransition],
    states: &[StateNode],
    scope_prefix: &str,
    g: &mut Graph<NodeLabel, EdgeLabel>,
    id_map: &BTreeMap<String, NodeId>,
) {
    // Only center+straighten when exactly one transition connects to the
    // pseudo-state. With multiple sources/targets, dagre's layout is better
    // than forcing everything to one x coordinate (which overwrites earlier
    // positioning — the bug this fixes).
    let start_targets: Vec<&str> = transitions
        .iter()
        .filter(|t| t.src == "[*]")
        .map(|t| t.dst.as_str())
        .collect();
    let end_sources: Vec<&str> = transitions
        .iter()
        .filter(|t| t.dst == "[*]")
        .map(|t| t.src.as_str())
        .collect();

    // Collect non-compound peer node IDs for overlap checks.
    // Compound nodes are containers — pseudo-states naturally share their space.
    let peer_nids: Vec<NodeId> = states
        .iter()
        .filter(|s| !s.is_composite())
        .filter_map(|s| id_map.get(s.id.as_str()).copied())
        .collect();

    if start_targets.len() == 1 {
        let start_key = format!("{scope_prefix}[*]_start");
        let Some(&start_nid) = id_map.get(&start_key) else { return };
        let Some(&target_nid) = id_map.get(start_targets[0]) else { return };
        let target_x = g.node(target_nid).map(|n| n.x).unwrap_or(0.0);

        if !would_overlap(g, start_nid, target_x, &peer_nids) {
            if let Some(n) = g.node_mut(start_nid) {
                n.x = target_x;
            }
            for eid in g.out_edges(start_nid).collect::<Vec<_>>() {
                if let Some(e) = g.edge_mut(eid) {
                    for pt in &mut e.points {
                        pt.x = target_x;
                    }
                }
            }
        }
    }

    if end_sources.len() == 1 {
        let end_key = format!("{scope_prefix}[*]_end");
        let Some(&end_nid) = id_map.get(&end_key) else { return };
        let Some(&source_nid) = id_map.get(end_sources[0]) else { return };
        let source_x = g.node(source_nid).map(|n| n.x).unwrap_or(0.0);

        if !would_overlap(g, end_nid, source_x, &peer_nids) {
            if let Some(n) = g.node_mut(end_nid) {
                n.x = source_x;
            }
            for eid in g.in_edges(end_nid).collect::<Vec<_>>() {
                if let Some(e) = g.edge_mut(eid) {
                    for pt in &mut e.points {
                        pt.x = source_x;
                    }
                }
            }
        }
    }
}

/// Check if moving `nid` to `new_x` would cause it to overlap with any peer node.
fn would_overlap(
    g: &Graph<NodeLabel, EdgeLabel>,
    nid: NodeId,
    new_x: f64,
    peers: &[NodeId],
) -> bool {
    let Some(node) = g.node(nid) else { return false };
    let half_w = node.width / 2.0;
    let half_h = node.height / 2.0;
    let min_gap = BULLSEYE_MIN_GAP;

    for &pid in peers {
        if pid == nid {
            continue;
        }
        let Some(peer) = g.node(pid) else { continue };
        let x_overlap = (new_x - peer.x).abs() < half_w + peer.width / 2.0 + min_gap;
        let y_overlap = (node.y - peer.y).abs() < half_h + peer.height / 2.0 + min_gap;
        if x_overlap && y_overlap {
            return true;
        }
    }
    false
}

/// Center external nodes that connect to composite states.
/// e.g. `Active → Paused` — Paused should be centered on Active's x.
fn center_external_connections(
    diagram: &StateDiagram,
    g: &mut Graph<NodeLabel, EdgeLabel>,
    id_map: &BTreeMap<String, NodeId>,
) {
    center_external_in_scope(&diagram.transitions, &diagram.states, g, id_map);
    for s in &diagram.states {
        center_external_in_state(s, g, id_map);
    }
}

fn center_external_in_state(
    state: &StateNode,
    g: &mut Graph<NodeLabel, EdgeLabel>,
    id_map: &BTreeMap<String, NodeId>,
) {
    let StateKind::Composite { transitions, children, .. } = &state.kind else { return };
    center_external_in_scope(transitions, children, g, id_map);
    for child in children {
        center_external_in_state(child, g, id_map);
    }
}

fn center_external_in_scope(
    transitions: &[StateTransition],
    states: &[StateNode],
    g: &mut Graph<NodeLabel, EdgeLabel>,
    id_map: &BTreeMap<String, NodeId>,
) {
    // Collect which external nodes need centering and their target x
    let mut centered: HashSet<NodeId> = HashSet::new();

    for t in transitions {
        if t.src == "[*]" || t.dst == "[*]" {
            continue;
        }

        let src_is_composite = is_compound_state(states, &t.src);
        let dst_is_composite = is_compound_state(states, &t.dst);

        // Composite → external: center external node on composite's x
        if src_is_composite && !dst_is_composite {
            let Some(&comp_nid) = id_map.get(&t.src) else { continue };
            let Some(&ext_nid) = id_map.get(&t.dst) else { continue };
            if centered.contains(&ext_nid) {
                continue;
            }
            let comp_x = g.node(comp_nid).map(|n| n.x).unwrap_or(0.0);
            let old_x = g.node(ext_nid).map(|n| n.x).unwrap_or(0.0);
            let dx = comp_x - old_x;
            if dx.abs() < 0.5 {
                continue;
            }
            if let Some(n) = g.node_mut(ext_nid) {
                n.x = comp_x;
            }
            // Shift edge points by dx (preserves dagre curve shape)
            for eid in g.in_edges(ext_nid).chain(g.out_edges(ext_nid)).collect::<Vec<_>>() {
                if let Some(e) = g.edge_mut(eid) {
                    for pt in &mut e.points {
                        pt.x += dx;
                    }
                }
            }
            centered.insert(ext_nid);
        }
        // External → composite: center external node on composite's x
        if dst_is_composite && !src_is_composite {
            let Some(&comp_nid) = id_map.get(&t.dst) else { continue };
            let Some(&ext_nid) = id_map.get(&t.src) else { continue };
            if centered.contains(&ext_nid) {
                continue;
            }
            let comp_x = g.node(comp_nid).map(|n| n.x).unwrap_or(0.0);
            let old_x = g.node(ext_nid).map(|n| n.x).unwrap_or(0.0);
            let dx = comp_x - old_x;
            if dx.abs() < 0.5 {
                continue;
            }
            if let Some(n) = g.node_mut(ext_nid) {
                n.x = comp_x;
            }
            for eid in g.in_edges(ext_nid).chain(g.out_edges(ext_nid)).collect::<Vec<_>>() {
                if let Some(e) = g.edge_mut(eid) {
                    for pt in &mut e.points {
                        pt.x += dx;
                    }
                }
            }
            centered.insert(ext_nid);
        }
    }
}

/// Check if a transition endpoint matches a resolved node ID.
/// `is_source` = true when matching the source side of a transition,
/// false for the destination side.  This prevents false matches like
/// `Active → [*]` matching a synthetic entry edge `Active.[*]_start → …`.
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
fn is_compound_state(states: &[super::ir::StateNode], id: &str) -> bool {
    for s in states {
        if s.id == id {
            return s.is_composite();
        }
        if let StateKind::Composite { children, .. } = &s.kind
            && is_compound_state(children, id)
        {
            return true;
        }
    }
    false
}

/// Recursively find a state's label by ID across all nesting levels.
fn find_state_label(states: &[super::ir::StateNode], id: &str) -> Option<String> {
    for s in states {
        if s.id == id {
            return s.label.clone().or_else(|| Some(s.id.clone()));
        }
        if let StateKind::Composite { children, .. } = &s.kind
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
        Shape::RoundedRect | Shape::ForkJoin | Shape::Note => {
            intersect_rect(&bbox, target)
        }
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
    for s in states {
        if s.id == id {
            return Some(&s.kind);
        }
        if let StateKind::Composite { children, .. } = &s.kind
            && let Some(kind) = find_state_kind(children, id)
        {
            return Some(kind);
        }
    }
    None
}

/// Process one scope (top-level or inside a composite): create nodes, pseudo-states,
/// edges, and compound parent relationships.
#[allow(clippy::too_many_arguments)]
/// Mutable state threaded through recursive scope building.
struct ScopeCtx<'c, M: TextMeasure> {
    g: &'c mut Graph<NodeLabel, EdgeLabel>,
    id_map: &'c mut BTreeMap<String, NodeId>,
    synthetic_ids: &'c mut HashSet<String>,
    measurer: &'c M,
    style: &'c TextStyle,
}

fn add_scope<'a, M: TextMeasure>(
    states: &'a [StateNode],
    transitions: &'a [StateTransition],
    parent: Option<(NodeId, &str)>,
    ctx: &mut ScopeCtx<'_, M>,
    all_transitions: &mut Vec<&'a StateTransition>,
) {
    let scope_prefix = parent.map(|(_, id)| format!("{id}.")).unwrap_or_default();
    let start_key = format!("{scope_prefix}[*]_start");
    let end_key = format!("{scope_prefix}[*]_end");

    add_pseudo_states(transitions, parent, &start_key, &end_key, ctx);
    add_state_nodes(states, transitions, parent, ctx, all_transitions);
    wire_edges(transitions, &start_key, &end_key, ctx, all_transitions);
}

fn add_pseudo_states<M: TextMeasure>(
    transitions: &[StateTransition],
    parent: Option<(NodeId, &str)>,
    start_key: &str, end_key: &str,
    ctx: &mut ScopeCtx<'_, M>,
) {
    for (key, has) in [
        (start_key, transitions.iter().any(|t| t.src == "[*]")),
        (end_key, transitions.iter().any(|t| t.dst == "[*]")),
    ] {
        if has {
            let nid = ctx.g.add_node(NodeLabel::new(START_END_SIZE, START_END_SIZE));
            if let Some((parent_nid, _)) = parent {
                ctx.g.set_parent(nid, parent_nid);
            }
            ctx.id_map.insert(key.to_string(), nid);
        }
    }
}

fn add_state_nodes<'a, M: TextMeasure>(
    states: &'a [StateNode],
    transitions: &'a [StateTransition],
    parent: Option<(NodeId, &str)>,
    ctx: &mut ScopeCtx<'_, M>,
    all_transitions: &mut Vec<&'a StateTransition>,
) {
    for s in states {
        let (width, height) = match &s.kind {
            StateKind::Fork | StateKind::Join => (FORK_JOIN_WIDTH, FORK_JOIN_HEIGHT),
            StateKind::Choice => (CHOICE_SIZE, CHOICE_SIZE),
            StateKind::Start | StateKind::End => (START_END_SIZE, START_END_SIZE),
            StateKind::History => (START_END_SIZE, START_END_SIZE),
            StateKind::Normal => {
                let text = s.label.as_deref().unwrap_or(&s.id);
                let ts = ctx.measurer.measure(text, ctx.style);
                (ts.width + PADDING_X * 2.0, ts.height + PADDING_Y * 2.0)
            }
            StateKind::Composite { children, transitions: inner_trans, regions, .. } => {
                add_composite_state(s, children, inner_trans, regions, transitions, parent, ctx, all_transitions);
                continue;
            }
        };

        let nid = ctx.g.add_node(NodeLabel::new(width, height));
        ctx.id_map.insert(s.id.clone(), nid);
        if let Some((parent_nid, _)) = parent {
            ctx.g.set_parent(nid, parent_nid);
        }
    }
}

fn add_composite_state<'a, M: TextMeasure>(
    s: &'a StateNode,
    children: &'a [StateNode],
    inner_trans: &'a [StateTransition],
    regions: &'a [super::ir::ConcurrentRegion],
    outer_transitions: &'a [StateTransition],
    parent: Option<(NodeId, &str)>,
    ctx: &mut ScopeCtx<'_, M>,
    all_transitions: &mut Vec<&'a StateTransition>,
) {
    let nid = ctx.g.add_node(NodeLabel::new(0.0, 0.0));
    ctx.id_map.insert(s.id.clone(), nid);
    if let Some((parent_nid, _)) = parent {
        ctx.g.set_parent(nid, parent_nid);
    }

    let inner_start_key = format!("{}.[*]_start", s.id);
    let inner_end_key = format!("{}.[*]_end", s.id);

    if regions.is_empty() {
        add_scope(children, inner_trans, Some((nid, &s.id)), ctx, all_transitions);
        for child in children {
            if let Some(&child_nid) = ctx.id_map.get(child.id.as_str()) {
                if ctx.g.parent(child_nid).is_none() {
                    ctx.g.set_parent(child_nid, nid);
                }
            }
        }
    } else {
        add_concurrent_regions(s, regions, nid, &inner_start_key, &inner_end_key, outer_transitions, ctx, all_transitions);
    }

    // Synthetic exit if composite is an edge source but has no inner [*]_end
    if !ctx.id_map.contains_key(&inner_end_key)
        && outer_transitions.iter().any(|t| t.src == s.id)
    {
        let end_nid = ctx.g.add_node(NodeLabel::new(START_END_SIZE, START_END_SIZE));
        ctx.g.set_parent(end_nid, nid);
        ctx.synthetic_ids.insert(inner_end_key.clone());
        ctx.id_map.insert(inner_end_key, end_nid);
        if let Some(last) = children.last() {
            if let Some(&child_nid) = ctx.id_map.get(last.id.as_str()) {
                ctx.g.add_edge(child_nid, end_nid, EdgeLabel::default());
            }
        }
    }
}

fn add_concurrent_regions<'a, M: TextMeasure>(
    s: &'a StateNode,
    regions: &'a [super::ir::ConcurrentRegion],
    compound_nid: NodeId,
    inner_start_key: &str, inner_end_key: &str,
    outer_transitions: &'a [StateTransition],
    ctx: &mut ScopeCtx<'_, M>,
    all_transitions: &mut Vec<&'a StateTransition>,
) {
    for (i, region) in regions.iter().enumerate() {
        let region_nid = ctx.g.add_node(NodeLabel::new(0.0, 0.0));
        let region_key = format!("{}._region_{}", s.id, i);
        ctx.g.set_parent(region_nid, compound_nid);
        ctx.synthetic_ids.insert(region_key.clone());
        ctx.id_map.insert(region_key.clone(), region_nid);

        add_scope(&region.children, &region.transitions, Some((region_nid, &region_key)), ctx, all_transitions);

        for child in &region.children {
            if let Some(&child_nid) = ctx.id_map.get(child.id.as_str()) {
                if ctx.g.parent(child_nid).is_none() {
                    ctx.g.set_parent(child_nid, region_nid);
                }
            }
        }
    }

    // Compound-level entry connecting to all region starts
    let region_starts: Vec<NodeId> = (0..regions.len())
        .filter_map(|i| {
            let sk = format!("{}._region_{}.[*]_start", s.id, i);
            ctx.id_map.get(&sk).copied()
        })
        .collect();
    if !region_starts.is_empty() && !ctx.id_map.contains_key(inner_start_key) {
        let entry_nid = ctx.g.add_node(NodeLabel::new(START_END_SIZE, START_END_SIZE));
        ctx.g.set_parent(entry_nid, compound_nid);
        ctx.synthetic_ids.insert(inner_start_key.to_string());
        ctx.id_map.insert(inner_start_key.to_string(), entry_nid);
        for &rs in &region_starts {
            ctx.g.add_edge(entry_nid, rs, EdgeLabel::default());
        }
    }

    // Compound-level exit connecting from all regions' last children
    let is_src = outer_transitions.iter().any(|t| t.src == s.id);
    if is_src && !ctx.id_map.contains_key(inner_end_key) {
        let exit_nid = ctx.g.add_node(NodeLabel::new(START_END_SIZE, START_END_SIZE));
        ctx.g.set_parent(exit_nid, compound_nid);
        ctx.synthetic_ids.insert(inner_end_key.to_string());
        ctx.id_map.insert(inner_end_key.to_string(), exit_nid);
        for region in regions {
            if let Some(last) = region.children.last() {
                if let Some(&cn) = ctx.id_map.get(last.id.as_str()) {
                    ctx.g.add_edge(cn, exit_nid, EdgeLabel::default());
                }
            }
        }
    }
}

fn wire_edges<'a, M: TextMeasure>(
    transitions: &'a [StateTransition],
    start_key: &str, end_key: &str,
    ctx: &mut ScopeCtx<'_, M>,
    all_transitions: &mut Vec<&'a StateTransition>,
) {
    for t in transitions {
        let mut src_key = if t.src == "[*]" { start_key.to_string() } else { t.src.clone() };
        let mut dst_key = if t.dst == "[*]" { end_key.to_string() } else { t.dst.clone() };

        // Redirect: edge FROM composite → use inner [*]_end
        if t.src != "[*]" {
            let inner_end = format!("{}.[*]_end", t.src);
            if ctx.id_map.contains_key(&inner_end) { src_key = inner_end; }
        }
        // Redirect: edge TO composite → use inner [*]_start
        if t.dst != "[*]" {
            let inner_start = format!("{}.[*]_start", t.dst);
            if ctx.id_map.contains_key(&inner_start) { dst_key = inner_start; }
        }

        let Some(&src) = ctx.id_map.get(&src_key) else { continue };
        let Some(&dst) = ctx.id_map.get(&dst_key) else { continue };

        let mut label = EdgeLabel::default();
        if let Some(text) = &t.label {
            let ts = ctx.measurer.measure(text, ctx.style);
            label.width = ts.width;
            label.height = ts.height;
        }
        ctx.g.add_edge(src, dst, label);
        all_transitions.push(t);
    }
}

/// Return the number of concurrent regions for a state (0 if not concurrent).
fn region_count(states: &[super::ir::StateNode], id: &str) -> usize {
    for s in states {
        if s.id == id {
            if let StateKind::Composite { regions, .. } = &s.kind {
                return regions.len();
            }
            return 0;
        }
        if let StateKind::Composite { children, .. } = &s.kind {
            let c = region_count(children, id);
            if c > 0 { return c; }
        }
    }
    0
}

/// Collect all notes from the diagram, including those inside composites.
fn collect_all_notes(diagram: &StateDiagram) -> Vec<&StateNote> {
    let mut result = Vec::new();
    for note in &diagram.notes {
        result.push(note);
    }
    fn collect_from_states<'a>(states: &'a [StateNode], result: &mut Vec<&'a StateNote>) {
        for s in states {
            if let StateKind::Composite { notes, children, .. } = &s.kind {
                for note in notes {
                    result.push(note);
                }
                collect_from_states(children, result);
            }
        }
    }
    collect_from_states(&diagram.states, &mut result);
    result
}

/// Resolve classDef + class + style into a per-state Style map.
fn resolve_state_styles(diagram: &StateDiagram) -> BTreeMap<&str, Style> {
    fn flatten_states(states: &[StateNode]) -> Vec<(&str, &[String])> {
        let mut out = Vec::new();
        for s in states {
            out.push((s.id.as_str(), s.classes.as_slice()));
            if let StateKind::Composite { children, .. } = &s.kind {
                out.extend(flatten_states(children));
            }
        }
        out
    }
    let entities = flatten_states(&diagram.states);
    crate::common::rendering::resolve_entity_styles(entities.into_iter(), &diagram.class_defs, &diagram.style_stmts)
}

#[cfg(test)]
#[path = "bridge_tests.rs"]
mod bridge_tests;
