use std::collections::{HashMap, HashSet};

use rusty_mermaid_core::{
    BBox, Shape, intersect_circle, intersect_polygon, Point, SimpleTextMeasure, Style, TextMeasure,
    TextStyle,
};
use rusty_mermaid_dagre::{DagreConfig, EdgeLabel, NodeLabel};
use rusty_mermaid_graph::{Graph, NodeId};

use crate::common::layout::{ArrowEnd, EdgeLayout, NodeLayout, StrokeType};
use crate::common::rendering::apply_style_properties;
use crate::common::styling::StyleProperty;

use super::ir::{NotePosition, StateDiagram, StateKind, StateNode, StateNote, StateTransition};

const PADDING_X: f64 = 16.0;
const PADDING_Y: f64 = 8.0;
const START_END_SIZE: f64 = 16.0;
const FORK_JOIN_WIDTH: f64 = 70.0;
const FORK_JOIN_HEIGHT: f64 = 7.0;
const CHOICE_SIZE: f64 = 28.0;
const NOTE_PADDING: f64 = 10.0;
/// Extra height added above compound nodes for the title + separator header.
/// Dagre doesn't know about the header, so without this the first inner
/// child overlaps the separator line.
const COMPOUND_HEADER_HEIGHT: f64 = 4.0;

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
    let mut id_map: HashMap<String, NodeId> = HashMap::new();
    let mut all_transitions: Vec<&StateTransition> = Vec::new();
    let mut synthetic_ids: HashSet<String> = HashSet::new();

    // Build graph: nodes, edges, compound hierarchy — all in one recursive walk
    add_scope(
        &diagram.states,
        &diagram.transitions,
        None,
        &mut g,
        &mut id_map,
        &mut all_transitions,
        &mut synthetic_ids,
        measurer,
        &style,
    );

    // Configure and run layout
    let mut config = DagreConfig::default();
    config.rankdir = diagram.direction;
    rusty_mermaid_dagre::pipeline::layout(&mut g, &config);

    // Post-layout adjustments (safe — doesn't affect dagre invariants)
    fix_region_order(diagram, &mut g, &id_map);
    center_content(diagram, &mut g, &id_map);
    center_bullseyes(diagram, &mut g, &id_map);
    center_external_connections(diagram, &mut g, &id_map);

    // Resolve per-node styles
    let node_styles = resolve_state_styles(diagram);

    // Extract results
    let nid_to_id: HashMap<NodeId, &str> = id_map.iter().map(|(id, &nid)| (nid, id.as_str())).collect();

    let mut nodes = Vec::new();
    let mut max_x: f64 = 0.0;
    let mut max_y: f64 = 0.0;

    for (id_str, &nid) in &id_map {
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
        max_x = max_x.max(n.x + n.width / 2.0);
        max_y = max_y.max(n.y + n.height / 2.0);
    }

    // Position notes relative to their target state (post-layout)
    let all_notes = collect_all_notes(diagram);
    for note in &all_notes {
        let Some(state_node) = nodes.iter().find(|n| n.id == note.state_id) else { continue };
        let (tw, th) = measurer.measure(&note.text, &style);
        let note_w = tw + NOTE_PADDING * 2.0;
        let note_h = th + NOTE_PADDING * 2.0;
        let gap = 10.0;

        let note_x = match note.position {
            NotePosition::Right => state_node.x + state_node.width / 2.0 + gap + note_w / 2.0,
            NotePosition::Left => state_node.x - state_node.width / 2.0 - gap - note_w / 2.0,
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
        max_x = max_x.max(note_x + note_w / 2.0);
        max_y = max_y.max(note_y + note_h / 2.0);
    }

    // Extend compound rects upward to make room for the title + separator
    // header.  Dagre doesn't know about the header, so without this the
    // first inner child circle overlaps the separator line.
    for node in &mut nodes {
        if node.is_compound {
            node.height += COMPOUND_HEADER_HEIGHT;
            node.y -= COMPOUND_HEADER_HEIGHT / 2.0;
        }
    }

    // Recompute extents after compound adjustments and notes
    let mut min_x: f64 = 0.0;
    max_x = 0.0;
    max_y = 0.0;
    for node in &nodes {
        min_x = min_x.min(node.x - node.width / 2.0);
        max_x = max_x.max(node.x + node.width / 2.0);
        max_y = max_y.max(node.y + node.height / 2.0);
    }

    // Compute shift for notes that extend past the left edge
    let x_shift = if min_x < 0.0 { -min_x } else { 0.0 };
    if x_shift > 0.0 {
        for node in &mut nodes {
            node.x += x_shift;
        }
        max_x += x_shift;
    }

    // Only emit edges that correspond to a real transition (filters out
    // synthetic scaffold edges used for compound ranking).
    let mut edges = Vec::new();
    for eid in g.edge_ids() {
        let Some((src, dst)) = g.edge_endpoints(eid) else { continue };
        if let (Some(&src_id), Some(&dst_id)) = (nid_to_id.get(&src), nid_to_id.get(&dst)) {
            let matched = all_transitions.iter().find(|t| {
                let s = resolve_pseudo(&t.src, src_id, true);
                let d = resolve_pseudo(&t.dst, dst_id, false);
                s && d
            });
            let Some(transition) = matched else { continue };
            let Some(e) = g.edge(eid) else { continue };
            let mut points: Vec<Point> = e.points.iter().map(|p| Point::new(p.x + x_shift, p.y)).collect();

            // Re-clip edge endpoints for non-rect shapes (diamond, circle).
            if points.len() >= 2 {
                // Detect if center_bullseyes aligned all points to the same x.
                let aligned_x = {
                    let all_same = points.windows(2).all(|w| (w[0].x - w[1].x).abs() < 0.5);
                    if all_same { Some(points[0].x) } else { None }
                };

                let Some(src_node) = g.node(src) else { continue };
                let src_shape = node_shape(&diagram.states, src_id);
                let src_bbox = BBox::new(src_node.x + x_shift, src_node.y, src_node.width, src_node.height);
                if let Some(p) = state_shape_intersect(src_shape, src_bbox, points[1]) {
                    points[0] = p;
                }

                let last = points.len() - 1;
                let Some(dst_node) = g.node(dst) else { continue };
                let dst_shape = node_shape(&diagram.states, dst_id);
                let dst_bbox = BBox::new(dst_node.x + x_shift, dst_node.y, dst_node.width, dst_node.height);
                if let Some(p) = state_shape_intersect(dst_shape, dst_bbox, points[last - 1]) {
                    points[last] = p;
                }

                // Restore x-alignment if center_bullseyes straightened this edge.
                // Re-clipping against an inner pseudo-state (whose dagre position
                // differs from the compound center) can introduce x-offset that
                // would otherwise survive through Basis interpolation and compound
                // boundary clipping.
                if let Some(ax) = aligned_x {
                    points[0].x = ax;
                    points[last].x = ax;
                }
            }

            let label_size = transition.label.as_ref().map(|l| {
                let edge_style = TextStyle { font_size: 12.0, ..style.clone() };
                measurer.measure(l, &edge_style)
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
    }

    // Compute dividers + region rects together for concurrent compounds
    let mut dividers = Vec::new();
    let mut region_rects = Vec::new();
    for node in &nodes {
        if node.region_count < 2 {
            continue;
        }
        let compound_top = node.y - node.height / 2.0 + 28.0; // below header
        let compound_bottom = node.y + node.height / 2.0;
        let compound_left = node.x - node.width / 2.0;
        let compound_right = node.x + node.width / 2.0;

        // Equal-width partitions: dividers at compound_left + i * partition_width
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

        // Region rects span from compound edge → divider → compound edge
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

    LayoutResult {
        nodes,
        edges,
        width: max_x,
        height: max_y,
        dividers,
        region_rects,
    }
}

/// Enforce declaration order for concurrent regions.
/// Dagre's order phase may swap region sub-compounds. If region_0 ends up
/// to the right of region_1, mirror all descendants around the compound center.
fn fix_region_order(
    diagram: &StateDiagram,
    g: &mut Graph<NodeLabel, EdgeLabel>,
    id_map: &HashMap<String, NodeId>,
) {
    for s in &diagram.states {
        fix_region_order_for_state(s, g, id_map);
    }
}

fn fix_region_order_for_state(
    state: &StateNode,
    g: &mut Graph<NodeLabel, EdgeLabel>,
    id_map: &HashMap<String, NodeId>,
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
        if let Some(&rnid) = id_map.get(&rk) {
            if let Some(rn) = g.node(rnid) {
                region_xs.push((i, rn.x));
            }
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
        if desc_set.contains(&src) && desc_set.contains(&dst) {
            if let Some(e) = g.edge_mut(eid) {
                for pt in &mut e.points {
                    pt.x = 2.0 * cx - pt.x;
                }
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
    id_map: &HashMap<String, NodeId>,
) {
    for s in &diagram.states {
        center_content_for_state(s, g, id_map);
    }
}

fn center_content_for_state(
    state: &StateNode,
    g: &mut Graph<NodeLabel, EdgeLabel>,
    id_map: &HashMap<String, NodeId>,
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
        if *root_nid != compound_nid {
            if let Some(rn) = g.node_mut(*root_nid) {
                rn.x += dx;
            }
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
            if desc_set.contains(&src) && desc_set.contains(&dst) {
                if let Some(e) = g.edge_mut(eid) {
                    for pt in &mut e.points {
                        pt.x += dx;
                    }
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
    id_map: &HashMap<String, NodeId>,
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
    id_map: &HashMap<String, NodeId>,
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
    _states: &[StateNode],
    scope_prefix: &str,
    g: &mut Graph<NodeLabel, EdgeLabel>,
    id_map: &HashMap<String, NodeId>,
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

    if start_targets.len() == 1 {
        let start_key = format!("{scope_prefix}[*]_start");
        let Some(&start_nid) = id_map.get(&start_key) else { return };
        let Some(&target_nid) = id_map.get(start_targets[0]) else { return };
        let target_x = g.node(target_nid).map(|n| n.x).unwrap_or(0.0);

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

    if end_sources.len() == 1 {
        let end_key = format!("{scope_prefix}[*]_end");
        let Some(&end_nid) = id_map.get(&end_key) else { return };
        let Some(&source_nid) = id_map.get(end_sources[0]) else { return };
        let source_x = g.node(source_nid).map(|n| n.x).unwrap_or(0.0);

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

/// Center external nodes that connect to composite states.
/// e.g. `Active → Paused` — Paused should be centered on Active's x.
fn center_external_connections(
    diagram: &StateDiagram,
    g: &mut Graph<NodeLabel, EdgeLabel>,
    id_map: &HashMap<String, NodeId>,
) {
    center_external_in_scope(&diagram.transitions, &diagram.states, g, id_map);
    for s in &diagram.states {
        center_external_in_state(s, g, id_map);
    }
}

fn center_external_in_state(
    state: &StateNode,
    g: &mut Graph<NodeLabel, EdgeLabel>,
    id_map: &HashMap<String, NodeId>,
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
    id_map: &HashMap<String, NodeId>,
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
        if let StateKind::Composite { children, .. } = &s.kind {
            if is_compound_state(children, id) {
                return true;
            }
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
        if let StateKind::Composite { children, .. } = &s.kind {
            if let Some(label) = find_state_label(children, id) {
                return Some(label);
            }
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
        if let StateKind::Composite { children, .. } = &s.kind {
            if let Some(kind) = find_state_kind(children, id) {
                return Some(kind);
            }
        }
    }
    None
}

/// Process one scope (top-level or inside a composite): create nodes, pseudo-states,
/// edges, and compound parent relationships.
fn add_scope<'a>(
    states: &'a [super::ir::StateNode],
    transitions: &'a [StateTransition],
    parent: Option<(NodeId, &str)>, // (parent_nid, parent_id) for scoping [*] names
    g: &mut Graph<NodeLabel, EdgeLabel>,
    id_map: &mut HashMap<String, NodeId>,
    all_transitions: &mut Vec<&'a StateTransition>,
    synthetic_ids: &mut HashSet<String>,
    measurer: &impl TextMeasure,
    style: &TextStyle,
) {
    // Create [*] pseudo-states for this scope
    let scope_prefix = parent.map(|(_, id)| format!("{id}.")).unwrap_or_default();

    let has_start = transitions.iter().any(|t| t.src == "[*]");
    let has_end = transitions.iter().any(|t| t.dst == "[*]");

    if has_start {
        let key = format!("{scope_prefix}[*]_start");
        let nid = g.add_node(NodeLabel::new(START_END_SIZE, START_END_SIZE));
        if let Some((parent_nid, _)) = parent {
            g.set_parent(nid, parent_nid);
        }
        id_map.insert(key, nid);
    }
    if has_end {
        let key = format!("{scope_prefix}[*]_end");
        let nid = g.add_node(NodeLabel::new(START_END_SIZE, START_END_SIZE));
        if let Some((parent_nid, _)) = parent {
            g.set_parent(nid, parent_nid);
        }
        id_map.insert(key, nid);
    }

    // Add state nodes
    for s in states {
        let (width, height) = match &s.kind {
            StateKind::Fork | StateKind::Join => (FORK_JOIN_WIDTH, FORK_JOIN_HEIGHT),
            StateKind::Choice => (CHOICE_SIZE, CHOICE_SIZE),
            StateKind::Start | StateKind::End => (START_END_SIZE, START_END_SIZE),
            StateKind::Composite { children, transitions: inner_trans, regions, .. } => {
                let nid = g.add_node(NodeLabel::new(0.0, 0.0));
                id_map.insert(s.id.clone(), nid);
                if let Some((parent_nid, _)) = parent {
                    g.set_parent(nid, parent_nid);
                }

                if regions.is_empty() {
                    // Non-concurrent: single scope
                    add_scope(
                        children,
                        inner_trans,
                        Some((nid, &s.id)),
                        g,
                        id_map,
                        all_transitions,
                        synthetic_ids,
                        measurer,
                        style,
                    );

                    for child in children {
                        if let Some(&child_nid) = id_map.get(child.id.as_str()) {
                            if g.parent(child_nid).is_none() {
                                g.set_parent(child_nid, nid);
                            }
                        }
                    }
                } else {
                    // Concurrent: each region is a compound sub-group
                    for (i, region) in regions.iter().enumerate() {
                        let region_nid = g.add_node(NodeLabel::new(0.0, 0.0));
                        let region_key = format!("{}._region_{}", s.id, i);
                        g.set_parent(region_nid, nid);
                        synthetic_ids.insert(region_key.clone());
                        id_map.insert(region_key.clone(), region_nid);

                        add_scope(
                            &region.children,
                            &region.transitions,
                            Some((region_nid, &region_key)),
                            g,
                            id_map,
                            all_transitions,
                            synthetic_ids,
                            measurer,
                            style,
                        );

                        for child in &region.children {
                            if let Some(&child_nid) = id_map.get(child.id.as_str()) {
                                if g.parent(child_nid).is_none() {
                                    g.set_parent(child_nid, region_nid);
                                }
                            }
                        }
                    }

                    // Create compound-level entry connecting to all region starts.
                    // This centers the outer [*]_start bullseye above the compound.
                    let mut region_starts = Vec::new();
                    for (i, _) in regions.iter().enumerate() {
                        let rk = format!("{}._region_{}", s.id, i);
                        let sk = format!("{rk}.[*]_start");
                        if let Some(&sn) = id_map.get(&sk) {
                            region_starts.push(sn);
                        }
                    }
                    if !region_starts.is_empty() {
                        let entry_key = format!("{}.[*]_start", s.id);
                        if !id_map.contains_key(&entry_key) {
                            let entry_nid =
                                g.add_node(NodeLabel::new(START_END_SIZE, START_END_SIZE));
                            g.set_parent(entry_nid, nid);
                            synthetic_ids.insert(entry_key.clone());
                            id_map.insert(entry_key, entry_nid);
                            for &rs in &region_starts {
                                g.add_edge(entry_nid, rs, EdgeLabel::default());
                            }
                        }
                    }

                    // Create compound-level exit connecting from all regions' last children.
                    // This centers the outer [*]_end bullseye below the compound.
                    let is_src = transitions.iter().any(|t| t.src == s.id);
                    if is_src {
                        let exit_key = format!("{}.[*]_end", s.id);
                        if !id_map.contains_key(&exit_key) {
                            let exit_nid =
                                g.add_node(NodeLabel::new(START_END_SIZE, START_END_SIZE));
                            g.set_parent(exit_nid, nid);
                            synthetic_ids.insert(exit_key.clone());
                            id_map.insert(exit_key, exit_nid);
                            for region in regions.iter() {
                                if let Some(last) = region.children.last() {
                                    if let Some(&cn) = id_map.get(last.id.as_str()) {
                                        g.add_edge(cn, exit_nid, EdgeLabel::default());
                                    }
                                }
                            }
                        }
                    }
                }

                // If this composite is an edge source in the outer scope but has
                // no inner [*]_end, create a synthetic exit node so that dagre
                // ranks the outgoing target below the compound's children.
                let inner_end_key = format!("{}.[*]_end", s.id);
                if !id_map.contains_key(&inner_end_key) {
                    let is_source = transitions.iter().any(|t| t.src == s.id);
                    if is_source {
                        let end_nid = g.add_node(NodeLabel::new(START_END_SIZE, START_END_SIZE));
                        g.set_parent(end_nid, nid);
                        synthetic_ids.insert(inner_end_key.clone());
                        id_map.insert(inner_end_key, end_nid);

                        if let Some(last) = children.last() {
                            if let Some(&child_nid) = id_map.get(last.id.as_str()) {
                                g.add_edge(child_nid, end_nid, EdgeLabel::default());
                            }
                        }
                    }
                }

                continue; // already added the node
            }
            StateKind::History => (START_END_SIZE, START_END_SIZE),
            StateKind::Normal => {
                let text = s.label.as_deref().unwrap_or(&s.id);
                let (tw, th) = measurer.measure(text, style);
                (tw + PADDING_X * 2.0, th + PADDING_Y * 2.0)
            }
        };

        let nid = g.add_node(NodeLabel::new(width, height));
        id_map.insert(s.id.clone(), nid);

        if let Some((parent_nid, _)) = parent {
            g.set_parent(nid, parent_nid);
        }
    }

    // Add edges for this scope's transitions.
    // Edges to/from composite states are redirected to inner pseudo-states
    // so dagre assigns correct ranks (above/below the compound).
    for t in transitions {
        let mut src_key = if t.src == "[*]" {
            format!("{scope_prefix}[*]_start")
        } else {
            t.src.clone()
        };
        let mut dst_key = if t.dst == "[*]" {
            format!("{scope_prefix}[*]_end")
        } else {
            t.dst.clone()
        };

        // Redirect: edge FROM composite → use inner [*]_end
        if t.src != "[*]" {
            let inner_end = format!("{}.[*]_end", t.src);
            if id_map.contains_key(&inner_end) {
                src_key = inner_end;
            }
        }
        // Redirect: edge TO composite → use inner [*]_start
        if t.dst != "[*]" {
            let inner_start = format!("{}.[*]_start", t.dst);
            if id_map.contains_key(&inner_start) {
                dst_key = inner_start;
            }
        }

        let Some(&src) = id_map.get(&src_key) else { continue };
        let Some(&dst) = id_map.get(&dst_key) else { continue };

        let mut label = EdgeLabel::default();
        if let Some(text) = &t.label {
            let (tw, th) = measurer.measure(text, style);
            label.width = tw;
            label.height = th;
        }
        g.add_edge(src, dst, label);
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
fn resolve_state_styles(diagram: &StateDiagram) -> HashMap<&str, Style> {
    let class_map: HashMap<&str, &[StyleProperty]> = diagram
        .class_defs
        .iter()
        .map(|cd| (cd.name.as_str(), cd.styles.as_slice()))
        .collect();

    let mut result: HashMap<&str, Style> = HashMap::new();

    fn collect_states<'a>(
        states: &'a [StateNode],
        class_map: &HashMap<&str, &[StyleProperty]>,
        style_stmts: &'a [super::ir::StateStyleStmt],
        result: &mut HashMap<&'a str, Style>,
    ) {
        for s in states {
            let mut style = Style::default();
            let mut has_custom = false;

            if let Some(props) = class_map.get("default") {
                apply_style_properties(&mut style, props);
                has_custom = true;
            }
            for class_name in &s.classes {
                if let Some(props) = class_map.get(class_name.as_str()) {
                    apply_style_properties(&mut style, props);
                    has_custom = true;
                }
            }
            for stmt in style_stmts {
                if stmt.ids.iter().any(|id| id == &s.id) {
                    apply_style_properties(&mut style, &stmt.styles);
                    has_custom = true;
                }
            }
            if has_custom {
                result.insert(&s.id, style);
            }

            if let StateKind::Composite { children, .. } = &s.kind {
                collect_states(children, class_map, style_stmts, result);
            }
        }
    }

    collect_states(&diagram.states, &class_map, &diagram.style_stmts, &mut result);
    result
}

#[cfg(test)]
mod tests {
    use rusty_mermaid_core::Direction;

    use super::*;
    use crate::state::ir::*;

    #[test]
    fn composite_children_aligned() {
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    [*] --> Active\n    state Active {\n        [*] --> Idle\n        Idle --> Running\n        Running --> Idle\n    }\n    Active --> [*]"
        ).unwrap();
        let result = layout(&d);

        let idle = result.nodes.iter().find(|n| n.id == "Idle").unwrap();
        let running = result.nodes.iter().find(|n| n.id == "Running").unwrap();
        assert!(
            (idle.x - running.x).abs() < 1.0,
            "Idle (x={:.1}) and Running (x={:.1}) should be x-aligned",
            idle.x, running.x
        );
    }

    #[test]
    fn layout_simple_chain() {
        let mut d = StateDiagram::new(Direction::TB);
        d.states.push(StateNode::new("A", StateKind::Normal));
        d.states.push(StateNode::new("B", StateKind::Normal));
        d.transitions.push(StateTransition::new("A", "B"));

        let result = layout(&d);
        assert_eq!(result.nodes.len(), 2);
        assert_eq!(result.edges.len(), 1);

        let a = result.nodes.iter().find(|n| n.id == "A").unwrap();
        let b = result.nodes.iter().find(|n| n.id == "B").unwrap();
        assert!(a.y < b.y, "A should be above B in TB layout");
    }

    #[test]
    fn layout_with_start_end() {
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    [*] --> Still\n    Still --> [*]"
        ).unwrap();
        let result = layout(&d);
        // start + end + Still = 3 nodes
        assert_eq!(result.nodes.len(), 3);
        assert_eq!(result.edges.len(), 2);
    }

    #[test]
    fn layout_fork_join() {
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    state fork1 <<fork>>\n    state join1 <<join>>\n    [*] --> fork1\n    fork1 --> A\n    fork1 --> B\n    A --> join1\n    B --> join1\n    join1 --> [*]"
        ).unwrap();
        let result = layout(&d);
        let fork = result.nodes.iter().find(|n| n.id == "fork1").unwrap();
        assert!((fork.width - FORK_JOIN_WIDTH).abs() < 1.0);
    }

    #[test]
    fn layout_edge_has_points() {
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    A --> B\n    B --> C"
        ).unwrap();
        let result = layout(&d);
        for e in &result.edges {
            assert!(!e.points.is_empty(), "edge {}->{} should have points", e.src, e.dst);
        }
    }

    #[test]
    fn layout_composite_has_inner_edges() {
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    [*] --> Active\n    state Active {\n        [*] --> Idle\n        Idle --> Running\n        Running --> Idle\n    }\n    Active --> [*]"
        ).unwrap();
        let result = layout(&d);

        // Nodes: [*]_start, [*]_end, Active, Active.[*]_start, Idle, Running
        assert!(result.nodes.iter().any(|n| n.id == "Idle"), "should have Idle");
        assert!(result.nodes.iter().any(|n| n.id == "Running"), "should have Running");
        assert!(result.nodes.iter().any(|n| n.id == "Active.[*]_start"), "should have inner start");

        // Should have inner edges
        assert!(result.edges.iter().any(|e| e.src == "Active.[*]_start" && e.dst == "Idle"),
            "should have inner [*] --> Idle edge");
        assert!(result.edges.iter().any(|e| e.src == "Idle" && e.dst == "Running"),
            "should have Idle --> Running edge");

        // Active should be marked as compound
        let active = result.nodes.iter().find(|n| n.id == "Active").unwrap();
        assert!(active.is_compound, "Active should be compound");
        let idle = result.nodes.iter().find(|n| n.id == "Idle").unwrap();
        let active_left = active.x - active.width / 2.0;
        let active_right = active.x + active.width / 2.0;
        assert!(active_left <= idle.x - idle.width / 2.0,
            "Active should contain Idle: active_left={active_left} idle_left={}",
            idle.x - idle.width / 2.0);
        assert!(active_right >= idle.x + idle.width / 2.0,
            "Active should contain Idle: active_right={active_right} idle_right={}",
            idle.x + idle.width / 2.0);

        // TB layout: [*]_start should be ABOVE Active, [*]_end BELOW
        let start = result.nodes.iter().find(|n| n.id == "[*]_start").unwrap();
        let end = result.nodes.iter().find(|n| n.id == "[*]_end").unwrap();
        let active_top = active.y - active.height / 2.0;
        let active_bottom = active.y + active.height / 2.0;
        assert!(start.y < active_top,
            "[*]_start (y={}) should be above Active top (y={active_top})",
            start.y);
        assert!(end.y > active_bottom,
            "[*]_end (y={}) should be below Active bottom (y={active_bottom})",
            end.y);
    }

    #[test]
    fn choice_diamond_edges_clip_to_polygon() {
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    state check <<choice>>\n    [*] --> check\n    check --> A : yes\n    check --> B : no\n    A --> [*]\n    B --> [*]"
        ).unwrap();
        let result = layout(&d);

        let check = result.nodes.iter().find(|n| n.id == "check").unwrap();
        let check_bottom_y = check.y + check.height / 2.0;
        let check_bottom_x = check.x; // diamond's bottom vertex

        // Edges from check should start ON the diamond polygon, not at the
        // bounding box corners. For edges going down-left/down-right, the
        // start point should be on the diamond edge, not at (corner_x, bottom_y).
        let check_to_a = result.edges.iter()
            .find(|e| e.src == "check" && e.dst == "A")
            .expect("check → A edge");
        let check_to_b = result.edges.iter()
            .find(|e| e.src == "check" && e.dst == "B")
            .expect("check → B edge");

        let ax = check_to_a.points[0].x;
        let ay = check_to_a.points[0].y;
        let bx = check_to_b.points[0].x;
        let by = check_to_b.points[0].y;

        // The start points should NOT be at the exact bottom-y of the bounding
        // box (that would be rect clipping). For diagonal exits, the polygon
        // intersection hits the diamond's slanted edge, producing y < bottom_y.
        // At minimum, for points not directly below center, x should differ
        // from center AND y should be < bottom_y (on the slanted edge).
        if (ax - check_bottom_x).abs() > 1.0 {
            assert!(ay < check_bottom_y - 0.5,
                "check→A start ({ax:.1},{ay:.1}) should be on diamond edge, not bbox bottom y={check_bottom_y:.1}");
        }
        if (bx - check_bottom_x).abs() > 1.0 {
            assert!(by < check_bottom_y - 0.5,
                "check→B start ({bx:.1},{by:.1}) should be on diamond edge, not bbox bottom y={check_bottom_y:.1}");
        }
    }

    #[test]
    fn node_shapes_propagated() {
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    state fork1 <<fork>>\n    state join1 <<join>>\n    state check <<choice>>\n    [*] --> fork1\n    fork1 --> A\n    fork1 --> B\n    A --> check\n    check --> join1 : yes\n    B --> join1\n    join1 --> [*]"
        ).unwrap();
        let result = layout(&d);

        let start = result.nodes.iter().find(|n| n.id == "[*]_start").unwrap();
        assert_eq!(start.shape, Shape::StateStart);

        let end = result.nodes.iter().find(|n| n.id == "[*]_end").unwrap();
        assert_eq!(end.shape, Shape::StateEnd);

        let fork = result.nodes.iter().find(|n| n.id == "fork1").unwrap();
        assert_eq!(fork.shape, Shape::ForkJoin);

        let join = result.nodes.iter().find(|n| n.id == "join1").unwrap();
        assert_eq!(join.shape, Shape::ForkJoin);

        let choice = result.nodes.iter().find(|n| n.id == "check").unwrap();
        assert_eq!(choice.shape, Shape::Choice);

        let a = result.nodes.iter().find(|n| n.id == "A").unwrap();
        assert_eq!(a.shape, Shape::RoundedRect);
    }

    #[test]
    fn history_state_shape_is_circle() {
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    state h1 <<history>>\n    [*] --> h1\n    h1 --> A"
        ).unwrap();
        let result = layout(&d);
        let h = result.nodes.iter().find(|n| n.id == "h1").unwrap();
        assert_eq!(h.shape, Shape::History);
        // Should be sized like start/end circles
        assert!((h.width - 16.0).abs() < 1.0);
    }

    #[test]
    fn layout_choice_branches() {
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    state check <<choice>>\n    [*] --> check\n    check --> A : yes\n    check --> B : no\n    A --> [*]\n    B --> [*]"
        ).unwrap();
        let result = layout(&d);

        let check = result.nodes.iter().find(|n| n.id == "check").unwrap();
        let a = result.nodes.iter().find(|n| n.id == "A").unwrap();
        let b = result.nodes.iter().find(|n| n.id == "B").unwrap();

        // check should be above A and B
        assert!(check.y < a.y, "check should be above A");
        assert!(check.y < b.y, "check should be above B");
    }

    #[test]
    fn layout_note_right() {
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    [*] --> Still\n    note right of Still : idle state\n    Still --> [*]"
        ).unwrap();
        let result = layout(&d);

        let still = result.nodes.iter().find(|n| n.id == "Still").unwrap();
        let note = result.nodes.iter().find(|n| n.id == "Still-note").unwrap();

        assert_eq!(note.shape, Shape::Note);
        assert_eq!(note.label, "idle state");
        // Note should be to the right of the state
        assert!(note.x > still.x,
            "note (x={:.1}) should be right of Still (x={:.1})", note.x, still.x);
    }

    #[test]
    fn layout_note_left() {
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    [*] --> Still\n    note left of Still : idle state\n    Still --> [*]"
        ).unwrap();
        let result = layout(&d);

        let still = result.nodes.iter().find(|n| n.id == "Still").unwrap();
        let note = result.nodes.iter().find(|n| n.id == "Still-note").unwrap();

        assert_eq!(note.shape, Shape::Note);
        // Note should be to the left of the state
        assert!(note.x < still.x,
            "note (x={:.1}) should be left of Still (x={:.1})", note.x, still.x);
    }

    #[test]
    fn layout_concurrent_regions() {
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    state Active {\n        A --> B\n        --\n        C --> D\n    }"
        ).unwrap();
        let result = layout(&d);

        // All four states should be present
        assert!(result.nodes.iter().any(|n| n.id == "A"));
        assert!(result.nodes.iter().any(|n| n.id == "B"));
        assert!(result.nodes.iter().any(|n| n.id == "C"));
        assert!(result.nodes.iter().any(|n| n.id == "D"));

        // Active should be compound with 2 regions
        let active = result.nodes.iter().find(|n| n.id == "Active").unwrap();
        assert!(active.is_compound);
        assert_eq!(active.region_count, 2);

        // Should have at least one divider
        assert!(!result.dividers.is_empty(),
            "concurrent regions should produce divider lines");
    }

    #[test]
    fn multi_source_end_bullseye_preserves_edges() {
        // When multiple transitions target [*], each edge should connect
        // from its own source — not all get overwritten to the last source's x.
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    [*] --> Still\n    Still --> [*]\n    Still --> Moving\n    Moving --> Still\n    Moving --> Crash\n    Crash --> [*]"
        ).unwrap();
        let result = layout(&d);

        let still = result.nodes.iter().find(|n| n.id == "Still").unwrap();
        let crash = result.nodes.iter().find(|n| n.id == "Crash").unwrap();

        // Both edges to [*]_end should exist
        let still_to_end = result.edges.iter()
            .find(|e| e.src == "Still" && e.dst == "[*]_end")
            .expect("Still → [*]_end edge should exist");
        let crash_to_end = result.edges.iter()
            .find(|e| e.src == "Crash" && e.dst == "[*]_end")
            .expect("Crash → [*]_end edge should exist");

        // The Still→[*] edge's last point should be closer to Still's x than Crash's x.
        // (Before the fix, both edges would have all points at Crash's x.)
        let still_edge_start_x = still_to_end.points[0].x;
        let crash_edge_start_x = crash_to_end.points[0].x;

        // The edges should start from different x positions (their respective sources)
        assert!(
            (still_edge_start_x - crash_edge_start_x).abs() > 1.0
                || (still.x - crash.x).abs() < 1.0, // unless they happen to be x-aligned
            "Still→[*] edge start (x={:.1}) and Crash→[*] edge start (x={:.1}) should differ \
             (Still.x={:.1}, Crash.x={:.1})",
            still_edge_start_x, crash_edge_start_x, still.x, crash.x
        );
    }

    #[test]
    fn concurrent_regions_centered_in_partitions() {
        let d = crate::state::parser::parse(
            "stateDiagram-v2\n    [*] --> Active\n    state Active {\n        [*] --> NumLockOff\n        NumLockOff --> NumLockOn : EvNumLockPressed\n        NumLockOn --> NumLockOff : EvNumLockPressed\n        --\n        [*] --> CapsLockOff\n        CapsLockOff --> CapsLockOn : EvCapsLockPressed\n        CapsLockOn --> CapsLockOff : EvCapsLockPressed\n    }\n    Active --> [*]"
        ).unwrap();
        let result = layout(&d);

        let active = result.nodes.iter().find(|n| n.id == "Active").unwrap();
        let numlock = result.nodes.iter().find(|n| n.id == "NumLockOff").unwrap();
        let capslock = result.nodes.iter().find(|n| n.id == "CapsLockOff").unwrap();

        let compound_left = active.x - active.width / 2.0;
        let partition_width = active.width / 2.0;
        let p0_cx = compound_left + partition_width * 0.5;
        let p1_cx = compound_left + partition_width * 1.5;

        assert!((numlock.x - p0_cx).abs() < 30.0,
            "NumLockOff (x={:.1}) should be near partition 0 center ({:.1})",
            numlock.x, p0_cx);
        assert!((capslock.x - p1_cx).abs() < 30.0,
            "CapsLockOff (x={:.1}) should be near partition 1 center ({:.1})",
            capslock.x, p1_cx);
    }

    // ---------------------------------------------------------------
    // state_shape_intersect unit tests
    // ---------------------------------------------------------------

    fn bbox_at_origin(w: f64, h: f64) -> BBox {
        BBox::new(0.0, 0.0, w, h)
    }

    fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() < eps
    }

    fn assert_point_near(actual: Point, expected: Point, eps: f64, msg: &str) {
        assert!(
            approx_eq(actual.x, expected.x, eps) && approx_eq(actual.y, expected.y, eps),
            "{msg}: expected ({:.4}, {:.4}), got ({:.4}, {:.4})",
            expected.x, expected.y, actual.x, actual.y
        );
    }

    // --- RoundedRect, ForkJoinBar, NoteRect return None ---

    #[test]
    fn intersect_rounded_rect_returns_none() {
        let bbox = bbox_at_origin(100.0, 60.0);
        assert!(state_shape_intersect(Shape::RoundedRect, bbox, Point::new(0.0, -100.0)).is_none());
        assert!(state_shape_intersect(Shape::RoundedRect, bbox, Point::new(100.0, 0.0)).is_none());
        assert!(state_shape_intersect(Shape::RoundedRect, bbox, Point::new(50.0, 50.0)).is_none());
    }

    #[test]
    fn intersect_fork_join_bar_returns_none() {
        let bbox = bbox_at_origin(70.0, 7.0);
        assert!(state_shape_intersect(Shape::ForkJoin, bbox, Point::new(0.0, -50.0)).is_none());
        assert!(state_shape_intersect(Shape::ForkJoin, bbox, Point::new(100.0, 100.0)).is_none());
    }

    #[test]
    fn intersect_note_rect_returns_none() {
        let bbox = bbox_at_origin(80.0, 40.0);
        assert!(state_shape_intersect(Shape::Note, bbox, Point::new(0.0, -50.0)).is_none());
        assert!(state_shape_intersect(Shape::Note, bbox, Point::new(-100.0, 0.0)).is_none());
    }

    // --- Circle shapes: StartCircle, EndBullseye, HistoryCircle ---
    //
    // intersect_circle projects the target point onto the circle boundary
    // along the ray from center toward target.

    #[test]
    fn intersect_start_circle_from_above() {
        let r = 8.0;
        let bbox = bbox_at_origin(r * 2.0, r * 2.0);
        let p = state_shape_intersect(Shape::StateStart, bbox, Point::new(0.0, -100.0)).unwrap();
        assert_point_near(p, Point::new(0.0, -r), 1e-6, "start circle from above");
    }

    #[test]
    fn intersect_start_circle_from_below() {
        let r = 8.0;
        let bbox = bbox_at_origin(r * 2.0, r * 2.0);
        let p = state_shape_intersect(Shape::StateStart, bbox, Point::new(0.0, 100.0)).unwrap();
        assert_point_near(p, Point::new(0.0, r), 1e-6, "start circle from below");
    }

    #[test]
    fn intersect_start_circle_from_left() {
        let r = 8.0;
        let bbox = bbox_at_origin(r * 2.0, r * 2.0);
        let p = state_shape_intersect(Shape::StateStart, bbox, Point::new(-100.0, 0.0)).unwrap();
        assert_point_near(p, Point::new(-r, 0.0), 1e-6, "start circle from left");
    }

    #[test]
    fn intersect_start_circle_from_right() {
        let r = 8.0;
        let bbox = bbox_at_origin(r * 2.0, r * 2.0);
        let p = state_shape_intersect(Shape::StateStart, bbox, Point::new(100.0, 0.0)).unwrap();
        assert_point_near(p, Point::new(r, 0.0), 1e-6, "start circle from right");
    }

    #[test]
    fn intersect_start_circle_diagonal() {
        let r = 8.0;
        let bbox = bbox_at_origin(r * 2.0, r * 2.0);
        let p = state_shape_intersect(Shape::StateStart, bbox, Point::new(100.0, 100.0)).unwrap();
        let s = r / 2.0_f64.sqrt();
        assert_point_near(p, Point::new(s, s), 1e-6, "start circle diagonal");
        // Point should be on the circle boundary
        let dist = (p.x * p.x + p.y * p.y).sqrt();
        assert!(approx_eq(dist, r, 1e-6), "point should be on circle boundary, dist={dist}");
    }

    #[test]
    fn intersect_end_bullseye_from_above() {
        let r = 8.0;
        let bbox = bbox_at_origin(r * 2.0, r * 2.0);
        let p = state_shape_intersect(Shape::StateEnd, bbox, Point::new(0.0, -50.0)).unwrap();
        assert_point_near(p, Point::new(0.0, -r), 1e-6, "end bullseye from above");
    }

    #[test]
    fn intersect_end_bullseye_diagonal() {
        let r = 10.0;
        let bbox = bbox_at_origin(r * 2.0, r * 2.0);
        let p = state_shape_intersect(Shape::StateEnd, bbox, Point::new(-80.0, 80.0)).unwrap();
        let s = r / 2.0_f64.sqrt();
        assert_point_near(p, Point::new(-s, s), 1e-6, "end bullseye diagonal bottom-left");
    }

    #[test]
    fn intersect_history_circle_from_right() {
        let r = 8.0;
        let bbox = bbox_at_origin(r * 2.0, r * 2.0);
        let p = state_shape_intersect(Shape::History, bbox, Point::new(200.0, 0.0)).unwrap();
        assert_point_near(p, Point::new(r, 0.0), 1e-6, "history circle from right");
    }

    #[test]
    fn intersect_history_circle_diagonal() {
        let r = 8.0;
        let bbox = bbox_at_origin(r * 2.0, r * 2.0);
        let p = state_shape_intersect(Shape::History, bbox, Point::new(-50.0, -50.0)).unwrap();
        let dist = (p.x * p.x + p.y * p.y).sqrt();
        assert!(approx_eq(dist, r, 1e-6), "history circle diagonal point on boundary");
    }

    #[test]
    fn circle_uses_max_of_width_height_for_radius() {
        // When width != height, radius = max(w, h) / 2
        let bbox = BBox::new(0.0, 0.0, 20.0, 10.0);
        let r = 10.0; // max(20, 10) / 2
        let p = state_shape_intersect(Shape::StateStart, bbox, Point::new(0.0, -100.0)).unwrap();
        assert_point_near(p, Point::new(0.0, -r), 1e-6, "radius uses max dimension");
    }

    // --- ChoiceDiamond: polygon intersection ---
    //
    // Diamond vertices at (cx, cy-hh), (cx+hw, cy), (cx, cy+hh), (cx-hw, cy)
    // For a square bbox (e.g., 28x28), the diamond has vertices at
    // top=(0,-14), right=(14,0), bottom=(0,14), left=(-14,0).

    #[test]
    fn intersect_diamond_from_above() {
        let s = 28.0;
        let bbox = bbox_at_origin(s, s);
        let p = state_shape_intersect(Shape::Choice, bbox, Point::new(0.0, -100.0)).unwrap();
        // Directly above center hits the top vertex
        assert_point_near(p, Point::new(0.0, -s / 2.0), 1e-6, "diamond from above");
    }

    #[test]
    fn intersect_diamond_from_below() {
        let s = 28.0;
        let bbox = bbox_at_origin(s, s);
        let p = state_shape_intersect(Shape::Choice, bbox, Point::new(0.0, 100.0)).unwrap();
        assert_point_near(p, Point::new(0.0, s / 2.0), 1e-6, "diamond from below");
    }

    #[test]
    fn intersect_diamond_from_left() {
        let s = 28.0;
        let bbox = bbox_at_origin(s, s);
        let p = state_shape_intersect(Shape::Choice, bbox, Point::new(-100.0, 0.0)).unwrap();
        assert_point_near(p, Point::new(-s / 2.0, 0.0), 1e-6, "diamond from left");
    }

    #[test]
    fn intersect_diamond_from_right() {
        let s = 28.0;
        let bbox = bbox_at_origin(s, s);
        let p = state_shape_intersect(Shape::Choice, bbox, Point::new(100.0, 0.0)).unwrap();
        assert_point_near(p, Point::new(s / 2.0, 0.0), 1e-6, "diamond from right");
    }

    #[test]
    fn intersect_diamond_diagonal_hits_edge_not_vertex() {
        // For a square diamond, a 45-degree ray from center should hit the
        // midpoint of an edge, not a vertex.
        let s = 28.0;
        let hw = s / 2.0;
        let bbox = bbox_at_origin(s, s);
        let p = state_shape_intersect(Shape::Choice, bbox, Point::new(100.0, 100.0)).unwrap();
        // The bottom-right edge goes from (hw, 0) to (0, hw).
        // Midpoint of that edge = (hw/2, hw/2).
        // A ray at 45 degrees hits exactly the midpoint for a square diamond.
        assert_point_near(p, Point::new(hw / 2.0, hw / 2.0), 1e-6, "diamond 45-deg hits edge midpoint");
    }

    #[test]
    fn intersect_diamond_diagonal_top_left() {
        let s = 28.0;
        let hw = s / 2.0;
        let bbox = bbox_at_origin(s, s);
        let p = state_shape_intersect(Shape::Choice, bbox, Point::new(-100.0, -100.0)).unwrap();
        // Top-left edge: from (0, -hw) to (-hw, 0). Midpoint = (-hw/2, -hw/2).
        assert_point_near(p, Point::new(-hw / 2.0, -hw / 2.0), 1e-6, "diamond 45-deg top-left");
    }

    #[test]
    fn intersect_diamond_non_square_bbox() {
        // Non-square diamond: w=40, h=20 => hw=20, hh=10
        // Vertices: top=(0,-10), right=(20,0), bottom=(0,10), left=(-20,0)
        let bbox = bbox_at_origin(40.0, 20.0);
        // Ray from center straight up
        let p = state_shape_intersect(Shape::Choice, bbox, Point::new(0.0, -100.0)).unwrap();
        assert_point_near(p, Point::new(0.0, -10.0), 1e-6, "non-square diamond from above");
        // Ray from center straight right
        let p = state_shape_intersect(Shape::Choice, bbox, Point::new(100.0, 0.0)).unwrap();
        assert_point_near(p, Point::new(20.0, 0.0), 1e-6, "non-square diamond from right");
    }

    #[test]
    fn intersect_diamond_point_lies_on_edge() {
        // Verify that the returned point is on one of the four diamond edges.
        let s = 28.0;
        let hw = s / 2.0;
        let bbox = bbox_at_origin(s, s);

        // Shoot a non-axis-aligned ray (e.g., 30 degrees from horizontal)
        let angle = std::f64::consts::FRAC_PI_6; // 30 degrees
        let target = Point::new(100.0 * angle.cos(), 100.0 * angle.sin());
        let p = state_shape_intersect(Shape::Choice, bbox, target).unwrap();

        // For a square diamond centered at origin with half-width hw,
        // a point is on the boundary iff |x|/hw + |y|/hw == 1
        let boundary_check = p.x.abs() / hw + p.y.abs() / hw;
        assert!(
            approx_eq(boundary_check, 1.0, 1e-6),
            "point ({:.4}, {:.4}) should be on diamond boundary: |x|/hw + |y|/hw = {:.6}, expected 1.0",
            p.x, p.y, boundary_check
        );
    }

    #[test]
    fn intersect_diamond_many_angles_all_on_boundary() {
        // Property test: for many ray angles, the intersection should lie on
        // the diamond boundary.
        let s = 28.0;
        let hw = s / 2.0;
        let bbox = bbox_at_origin(s, s);

        for deg in (0..360).step_by(15) {
            let angle = (deg as f64).to_radians();
            let target = Point::new(100.0 * angle.cos(), 100.0 * angle.sin());
            let p = state_shape_intersect(Shape::Choice, bbox, target).unwrap();
            let boundary_check = p.x.abs() / hw + p.y.abs() / hw;
            assert!(
                approx_eq(boundary_check, 1.0, 1e-4),
                "angle={deg}deg: point ({:.4}, {:.4}) off boundary: {:.6} != 1.0",
                p.x, p.y, boundary_check
            );
        }
    }

    #[test]
    fn intersect_circle_many_angles_all_on_boundary() {
        // Property test: for many ray angles, the intersection should lie on
        // the circle boundary.
        let r = 8.0;
        let bbox = bbox_at_origin(r * 2.0, r * 2.0);

        for deg in (0..360).step_by(15) {
            let angle = (deg as f64).to_radians();
            let target = Point::new(100.0 * angle.cos(), 100.0 * angle.sin());
            let p = state_shape_intersect(Shape::StateStart, bbox, target).unwrap();
            let dist = (p.x * p.x + p.y * p.y).sqrt();
            assert!(
                approx_eq(dist, r, 1e-6),
                "angle={deg}deg: point ({:.4}, {:.4}) off circle boundary: dist={:.6} != r={r}",
                p.x, p.y, dist
            );
        }
    }

    #[test]
    fn intersect_with_offset_center() {
        // BBox centered at (50, 30), not origin
        let bbox = BBox::new(50.0, 30.0, 16.0, 16.0);
        let r = 8.0;
        // Target far above the center
        let p = state_shape_intersect(Shape::StateStart, bbox, Point::new(50.0, -100.0)).unwrap();
        assert_point_near(p, Point::new(50.0, 30.0 - r), 1e-6, "offset center from above");
        // Target to the right
        let p = state_shape_intersect(Shape::StateEnd, bbox, Point::new(200.0, 30.0)).unwrap();
        assert_point_near(p, Point::new(50.0 + r, 30.0), 1e-6, "offset center from right");
    }

    #[test]
    fn intersect_diamond_offset_center() {
        let bbox = BBox::new(100.0, 50.0, 28.0, 28.0);
        let hw = 14.0;
        // Target directly above
        let p = state_shape_intersect(Shape::Choice, bbox, Point::new(100.0, -100.0)).unwrap();
        assert_point_near(p, Point::new(100.0, 50.0 - hw), 1e-6, "offset diamond from above");
        // Target directly right
        let p = state_shape_intersect(Shape::Choice, bbox, Point::new(300.0, 50.0)).unwrap();
        assert_point_near(p, Point::new(100.0 + hw, 50.0), 1e-6, "offset diamond from right");
    }
}
