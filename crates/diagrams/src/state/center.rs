use std::collections::{BTreeMap, HashSet};

use rusty_mermaid_dagre::{EdgeLabel, NodeLabel};
use rusty_mermaid_graph::{Graph, NodeId};

use super::bridge::{BULLSEYE_MIN_GAP, is_compound_state};
use super::ir::{StateDiagram, StateKind, StateNode, StateTransition};

/// Enforce declaration order for concurrent regions.
/// Dagre's order phase may swap region sub-compounds. If region_0 ends up
/// to the right of region_1, mirror all descendants around the compound center.
pub(super) fn fix_region_order(
    diagram: &StateDiagram,
    graph: &mut Graph<NodeLabel, EdgeLabel>,
    id_map: &BTreeMap<String, NodeId>,
) {
    for state in &diagram.states {
        fix_region_order_for_state(state, graph, id_map);
    }
}

pub(super) fn fix_region_order_for_state(
    state: &StateNode,
    graph: &mut Graph<NodeLabel, EdgeLabel>,
    id_map: &BTreeMap<String, NodeId>,
) {
    let StateKind::Composite {
        regions, children, ..
    } = &state.kind
    else {
        return;
    };

    // Recurse into children first
    for child in children {
        fix_region_order_for_state(child, graph, id_map);
    }

    if regions.len() < 2 {
        return;
    }

    // Check if regions are in declaration order (left-to-right by x)
    let mut region_xs: Vec<(usize, f64)> = Vec::new();
    for (i, _) in regions.iter().enumerate() {
        let rk = format!("{}._region_{}", state.id, i);
        if let Some(&rnid) = id_map.get(&rk)
            && let Some(rn) = graph.node(rnid)
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
    let Some(&compound_nid) = id_map.get(&state.id) else {
        return;
    };
    let Some(compound_node) = graph.node(compound_nid) else {
        return;
    };
    let cx = compound_node.x;

    // Collect all descendant node IDs
    let mut descendants = Vec::new();
    collect_descendants(graph, compound_nid, &mut descendants);

    // Mirror node positions
    for &nid in &descendants {
        if let Some(n) = graph.node_mut(nid) {
            n.x = 2.0 * cx - n.x;
        }
    }

    // Mirror edge points for edges fully within the compound
    let desc_set: HashSet<NodeId> = descendants.iter().copied().collect();
    for eid in graph.edge_ids().collect::<Vec<_>>() {
        let Some((src, dst)) = graph.edge_endpoints(eid) else {
            continue;
        };
        if desc_set.contains(&src)
            && desc_set.contains(&dst)
            && let Some(e) = graph.edge_mut(eid)
        {
            for pt in &mut e.points {
                pt.x = 2.0 * cx - pt.x;
            }
        }
    }
}

pub(super) fn collect_descendants(
    graph: &Graph<NodeLabel, EdgeLabel>,
    nid: NodeId,
    out: &mut Vec<NodeId>,
) {
    for child in graph.children(nid).collect::<Vec<_>>() {
        out.push(child);
        collect_descendants(graph, child, out);
    }
}

/// Center composite content within compound bounds.
/// Non-concurrent: centers all descendants on the compound center.
/// Concurrent: centers each region's descendants within its equal-width partition.
pub(super) fn center_content(
    diagram: &StateDiagram,
    graph: &mut Graph<NodeLabel, EdgeLabel>,
    id_map: &BTreeMap<String, NodeId>,
) {
    for state in &diagram.states {
        center_content_for_state(state, graph, id_map);
    }
}

pub(super) fn center_content_for_state(
    state: &StateNode,
    graph: &mut Graph<NodeLabel, EdgeLabel>,
    id_map: &BTreeMap<String, NodeId>,
) {
    let StateKind::Composite {
        regions, children, ..
    } = &state.kind
    else {
        return;
    };
    // Recurse into children first (handles nested composites)
    for child in children {
        center_content_for_state(child, graph, id_map);
    }

    let Some(&compound_nid) = id_map.get(&state.id) else {
        return;
    };
    let Some(compound_node) = graph.node(compound_nid) else {
        return;
    };
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
            let Some(&rnid) = id_map.get(&rk) else {
                continue;
            };
            let mut desc = Vec::new();
            collect_descendants(graph, rnid, &mut desc);
            let cx = content_bbox_cx(graph, &desc);
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
        collect_descendants(graph, compound_nid, &mut desc);
        if desc.is_empty() {
            return;
        }
        vec![(compound_nid, desc, compound_cx)]
    };

    for (root_nid, descendants, target_cx) in &partitions {
        let cx = content_bbox_cx(graph, descendants);
        let dx = target_cx - cx;
        if dx.abs() < 0.5 {
            continue;
        }

        // Shift the partition root (region compound for concurrent, skip for non-concurrent)
        if *root_nid != compound_nid
            && let Some(rn) = graph.node_mut(*root_nid)
        {
            rn.x += dx;
        }
        // Shift all descendants
        for &nid in descendants {
            if let Some(n) = graph.node_mut(nid) {
                n.x += dx;
            }
        }
        // Shift edges fully within this partition
        let desc_set: HashSet<NodeId> = std::iter::once(*root_nid)
            .chain(descendants.iter().copied())
            .collect();
        for eid in graph.edge_ids().collect::<Vec<_>>() {
            let Some((src, dst)) = graph.edge_endpoints(eid) else {
                continue;
            };
            if desc_set.contains(&src)
                && desc_set.contains(&dst)
                && let Some(e) = graph.edge_mut(eid)
            {
                for pt in &mut e.points {
                    pt.x += dx;
                }
            }
        }
    }
}

/// Compute the horizontal center of a group of nodes' bounding box.
pub(super) fn content_bbox_cx(graph: &Graph<NodeLabel, EdgeLabel>, nodes: &[NodeId]) -> f64 {
    let (mut min_x, mut max_x) = (f64::MAX, f64::MIN);
    for &nid in nodes {
        if let Some(n) = graph.node(nid) {
            min_x = min_x.min(n.x - n.width / 2.0);
            max_x = max_x.max(n.x + n.width / 2.0);
        }
    }
    (min_x + max_x) / 2.0
}

/// Center outer [*]_start / [*]_end bullseyes on their connected compound.
/// After dagre layout, the bullseye x may not align with the compound center.
pub(super) fn center_bullseyes(
    diagram: &StateDiagram,
    graph: &mut Graph<NodeLabel, EdgeLabel>,
    id_map: &BTreeMap<String, NodeId>,
) {
    center_bullseyes_in_scope(&diagram.transitions, &diagram.states, "", graph, id_map);
    // Recurse into composites
    for state in &diagram.states {
        center_bullseyes_in_state(state, graph, id_map);
    }
}

pub(super) fn center_bullseyes_in_state(
    state: &StateNode,
    graph: &mut Graph<NodeLabel, EdgeLabel>,
    id_map: &BTreeMap<String, NodeId>,
) {
    let StateKind::Composite {
        transitions,
        children,
        ..
    } = &state.kind
    else {
        return;
    };
    let prefix = format!("{}.", state.id);
    center_bullseyes_in_scope(transitions, children, &prefix, graph, id_map);
    for child in children {
        center_bullseyes_in_state(child, graph, id_map);
    }
}

pub(super) fn center_bullseyes_in_scope(
    transitions: &[StateTransition],
    states: &[StateNode],
    scope_prefix: &str,
    graph: &mut Graph<NodeLabel, EdgeLabel>,
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
        let Some(&start_nid) = id_map.get(&start_key) else {
            return;
        };
        let Some(&target_nid) = id_map.get(start_targets[0]) else {
            return;
        };
        let target_x = graph.node(target_nid).map(|n| n.x).unwrap_or(0.0);

        if !would_overlap(graph, start_nid, target_x, &peer_nids) {
            if let Some(n) = graph.node_mut(start_nid) {
                n.x = target_x;
            }
            for eid in graph.out_edges(start_nid).collect::<Vec<_>>() {
                if let Some(e) = graph.edge_mut(eid) {
                    for pt in &mut e.points {
                        pt.x = target_x;
                    }
                }
            }
        }
    }

    if end_sources.len() == 1 {
        let end_key = format!("{scope_prefix}[*]_end");
        let Some(&end_nid) = id_map.get(&end_key) else {
            return;
        };
        let Some(&source_nid) = id_map.get(end_sources[0]) else {
            return;
        };
        let source_x = graph.node(source_nid).map(|n| n.x).unwrap_or(0.0);

        if !would_overlap(graph, end_nid, source_x, &peer_nids) {
            if let Some(n) = graph.node_mut(end_nid) {
                n.x = source_x;
            }
            for eid in graph.in_edges(end_nid).collect::<Vec<_>>() {
                if let Some(e) = graph.edge_mut(eid) {
                    for pt in &mut e.points {
                        pt.x = source_x;
                    }
                }
            }
        }
    }
}

/// Check if moving `nid` to `new_x` would cause it to overlap with any peer node.
pub(super) fn would_overlap(
    graph: &Graph<NodeLabel, EdgeLabel>,
    nid: NodeId,
    new_x: f64,
    peers: &[NodeId],
) -> bool {
    let Some(node) = graph.node(nid) else {
        return false;
    };
    let half_w = node.width / 2.0;
    let half_h = node.height / 2.0;
    let min_gap = BULLSEYE_MIN_GAP;

    for &pid in peers {
        if pid == nid {
            continue;
        }
        let Some(peer) = graph.node(pid) else {
            continue;
        };
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
pub(super) fn center_external_connections(
    diagram: &StateDiagram,
    graph: &mut Graph<NodeLabel, EdgeLabel>,
    id_map: &BTreeMap<String, NodeId>,
) {
    center_external_in_scope(&diagram.transitions, &diagram.states, graph, id_map);
    for state in &diagram.states {
        center_external_in_state(state, graph, id_map);
    }
}

pub(super) fn center_external_in_state(
    state: &StateNode,
    graph: &mut Graph<NodeLabel, EdgeLabel>,
    id_map: &BTreeMap<String, NodeId>,
) {
    let StateKind::Composite {
        transitions,
        children,
        ..
    } = &state.kind
    else {
        return;
    };
    center_external_in_scope(transitions, children, graph, id_map);
    for child in children {
        center_external_in_state(child, graph, id_map);
    }
}

pub(super) fn center_external_in_scope(
    transitions: &[StateTransition],
    states: &[StateNode],
    graph: &mut Graph<NodeLabel, EdgeLabel>,
    id_map: &BTreeMap<String, NodeId>,
) {
    // Collect which external nodes need centering and their target x
    let mut centered: HashSet<NodeId> = HashSet::new();

    for trans in transitions {
        if trans.src == "[*]" || trans.dst == "[*]" {
            continue;
        }

        let src_is_composite = is_compound_state(states, &trans.src);
        let dst_is_composite = is_compound_state(states, &trans.dst);

        // Composite → external: center external node on composite's x
        if src_is_composite && !dst_is_composite {
            let Some(&comp_nid) = id_map.get(&trans.src) else {
                continue;
            };
            let Some(&ext_nid) = id_map.get(&trans.dst) else {
                continue;
            };
            if centered.contains(&ext_nid) {
                continue;
            }
            let comp_x = graph.node(comp_nid).map(|n| n.x).unwrap_or(0.0);
            let old_x = graph.node(ext_nid).map(|n| n.x).unwrap_or(0.0);
            let dx = comp_x - old_x;
            if dx.abs() < 0.5 {
                continue;
            }
            if let Some(n) = graph.node_mut(ext_nid) {
                n.x = comp_x;
            }
            // Shift edge points by dx (preserves dagre curve shape)
            for eid in graph
                .in_edges(ext_nid)
                .chain(graph.out_edges(ext_nid))
                .collect::<Vec<_>>()
            {
                if let Some(e) = graph.edge_mut(eid) {
                    for pt in &mut e.points {
                        pt.x += dx;
                    }
                }
            }
            centered.insert(ext_nid);
        }
        // External → composite: center external node on composite's x
        if dst_is_composite && !src_is_composite {
            let Some(&comp_nid) = id_map.get(&trans.dst) else {
                continue;
            };
            let Some(&ext_nid) = id_map.get(&trans.src) else {
                continue;
            };
            if centered.contains(&ext_nid) {
                continue;
            }
            let comp_x = graph.node(comp_nid).map(|n| n.x).unwrap_or(0.0);
            let old_x = graph.node(ext_nid).map(|n| n.x).unwrap_or(0.0);
            let dx = comp_x - old_x;
            if dx.abs() < 0.5 {
                continue;
            }
            if let Some(n) = graph.node_mut(ext_nid) {
                n.x = comp_x;
            }
            for eid in graph
                .in_edges(ext_nid)
                .chain(graph.out_edges(ext_nid))
                .collect::<Vec<_>>()
            {
                if let Some(e) = graph.edge_mut(eid) {
                    for pt in &mut e.points {
                        pt.x += dx;
                    }
                }
            }
            centered.insert(ext_nid);
        }
    }
}
