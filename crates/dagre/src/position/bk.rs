use std::collections::{HashMap, HashSet};

use rusty_mermaid_graph::{Graph, NodeId};

use crate::config::DagreConfig;
use crate::labels::{DummyKind, EdgeLabel, LabelPos, NodeLabel};
use crate::util;

type NeighborFn = dyn Fn(&Graph<NodeLabel, EdgeLabel>, NodeId) -> Vec<NodeId>;
type AdjList = HashMap<NodeId, Vec<(NodeId, f64)>>;

/// Assign x coordinates using Brandes-Köpf algorithm.
///
/// Computes 4 alignments (up-left, up-right, down-left, down-right),
/// then picks the median (or a specific alignment if `config.align` is set).
pub(crate) fn position_x(
    g: &Graph<NodeLabel, EdgeLabel>,
    config: &DagreConfig,
) -> Vec<(NodeId, f64)> {
    let layering = util::build_layer_matrix(g);
    let conflicts = find_conflicts(g, &layering);

    let mut xss: [Vec<(NodeId, f64)>; 4] = Default::default();

    for (idx, &(vert_up, horiz_left)) in
        [(true, true), (true, false), (false, true), (false, false)]
            .iter()
            .enumerate()
    {
        let mut adj_layering = if vert_up {
            layering.clone()
        } else {
            let mut rev = layering.clone();
            rev.reverse();
            rev
        };

        if !horiz_left {
            for layer in &mut adj_layering {
                layer.reverse();
            }
        }

        let neighbor_fn: Box<NeighborFn> = if vert_up {
                Box::new(|g: &Graph<NodeLabel, EdgeLabel>, v| {
                    g.in_edges(v)
                        .filter_map(|eid| g.edge_endpoints(eid).map(|(s, _)| s))
                        .collect()
                })
            } else {
                Box::new(|g: &Graph<NodeLabel, EdgeLabel>, v| {
                    g.out_edges(v)
                        .filter_map(|eid| g.edge_endpoints(eid).map(|(_, d)| d))
                        .collect()
                })
            };

        let (root, align) =
            vertical_alignment(g, &adj_layering, &conflicts, &*neighbor_fn);
        let mut xs =
            horizontal_compaction(g, &adj_layering, &root, &align, !horiz_left, config);

        if !horiz_left {
            for (_, x) in &mut xs {
                *x = -*x;
            }
        }

        xss[idx] = xs;
    }

    // Convert to HashMaps for alignment/balancing
    let mut maps: Vec<HashMap<NodeId, f64>> = xss
        .into_iter()
        .map(|xs| xs.into_iter().collect())
        .collect();

    let smallest_idx = find_smallest_width_alignment(g, &maps);
    align_coordinates(&mut maps, smallest_idx);
    balance(&maps, config)
}

// --- Conflict detection ---

type Conflicts = HashSet<(NodeId, NodeId)>;

fn add_conflict(conflicts: &mut Conflicts, v: NodeId, w: NodeId) {
    let (a, b) = if v < w { (v, w) } else { (w, v) };
    conflicts.insert((a, b));
}

fn has_conflict(conflicts: &Conflicts, v: NodeId, w: NodeId) -> bool {
    let (a, b) = if v < w { (v, w) } else { (w, v) };
    conflicts.contains(&(a, b))
}

/// Find type-1 conflicts: non-inner segment crossing an inner segment.
/// An inner segment is an edge where both endpoints are dummy nodes.
fn find_type1_conflicts(
    g: &Graph<NodeLabel, EdgeLabel>,
    layering: &[Vec<NodeId>],
) -> Conflicts {
    let mut conflicts = Conflicts::new();

    for layer_idx in 1..layering.len() {
        let prev_layer = &layering[layer_idx - 1];
        let layer = &layering[layer_idx];
        let prev_layer_len = prev_layer.len();

        // Build position cache for prev_layer
        let pos: HashMap<NodeId, usize> = prev_layer
            .iter()
            .enumerate()
            .map(|(i, &v)| (v, i))
            .collect();

        let mut k0 = 0usize;
        let mut scan_pos = 0usize;

        for (i, &v) in layer.iter().enumerate() {
            let w = find_other_inner_segment_node(g, v);
            let k1 = w.map_or(prev_layer_len, |w| {
                *pos.get(&w).unwrap_or(&prev_layer_len)
            });

            if w.is_some() || i == layer.len() - 1 {
                for &scan_node in &layer[scan_pos..=i] {
                    let predecessors: Vec<NodeId> = g
                        .in_edges(scan_node)
                        .filter_map(|eid| g.edge_endpoints(eid).map(|(s, _)| s))
                        .collect();
                    for u in predecessors {
                        let u_pos = *pos.get(&u).unwrap_or(&0);
                        let u_is_dummy = g.node(u).unwrap().dummy.is_some();
                        let scan_is_dummy = g.node(scan_node).unwrap().dummy.is_some();
                        if (u_pos < k0 || k1 < u_pos) && !(u_is_dummy && scan_is_dummy) {
                            add_conflict(&mut conflicts, u, scan_node);
                        }
                    }
                }
                scan_pos = i + 1;
                k0 = k1;
            }
        }
    }

    conflicts
}

/// Find type-2 conflicts: dummy-to-dummy edges crossing border boundaries.
fn find_type2_conflicts(
    g: &Graph<NodeLabel, EdgeLabel>,
    layering: &[Vec<NodeId>],
) -> Conflicts {
    let mut conflicts = Conflicts::new();

    for layer_idx in 1..layering.len() {
        let north = &layering[layer_idx - 1];
        let south = &layering[layer_idx];

        // Build position cache for north
        let north_pos: HashMap<NodeId, usize> = north
            .iter()
            .enumerate()
            .map(|(i, &v)| (v, i))
            .collect();

        let mut prev_north_border: i64 = -1;
        let mut south_pos = 0usize;

        for (south_lookahead, &v) in south.iter().enumerate() {
            if g.node(v).unwrap().dummy == Some(DummyKind::Border) {
                let preds: Vec<NodeId> = g
                    .in_edges(v)
                    .filter_map(|eid| g.edge_endpoints(eid).map(|(s, _)| s))
                    .collect();
                if let Some(&pred) = preds.first() {
                    let next_north_pos =
                        *north_pos.get(&pred).unwrap_or(&0) as i64;
                    scan_type2(
                        g,
                        &north_pos,
                        south,
                        south_pos,
                        south_lookahead,
                        prev_north_border,
                        next_north_pos,
                        &mut conflicts,
                    );
                    south_pos = south_lookahead;
                    prev_north_border = next_north_pos;
                }
            }
        }
        // Final scan to end of south layer
        scan_type2(
            g,
            &north_pos,
            south,
            south_pos,
            south.len(),
            prev_north_border,
            north.len() as i64,
            &mut conflicts,
        );
    }

    conflicts
}

#[allow(clippy::too_many_arguments)]
fn scan_type2(
    g: &Graph<NodeLabel, EdgeLabel>,
    north_pos: &HashMap<NodeId, usize>,
    south: &[NodeId],
    start: usize,
    end: usize,
    prev_north_border: i64,
    next_north_border: i64,
    conflicts: &mut Conflicts,
) {
    for &v in &south[start..end] {
        if g.node(v).unwrap().dummy.is_some() {
            let preds: Vec<NodeId> = g
                .in_edges(v)
                .filter_map(|eid| g.edge_endpoints(eid).map(|(s, _)| s))
                .collect();
            for u in preds {
                if g.node(u).unwrap().dummy.is_some() {
                    let u_order = *north_pos.get(&u).unwrap_or(&0) as i64;
                    if u_order < prev_north_border || u_order > next_north_border {
                        add_conflict(conflicts, u, v);
                    }
                }
            }
        }
    }
}

fn find_other_inner_segment_node(
    g: &Graph<NodeLabel, EdgeLabel>,
    v: NodeId,
) -> Option<NodeId> {
    if g.node(v).unwrap().dummy.is_some() {
        g.in_edges(v)
            .filter_map(|eid| g.edge_endpoints(eid).map(|(s, _)| s))
            .find(|&u| g.node(u).unwrap().dummy.is_some())
    } else {
        None
    }
}

fn find_conflicts(
    g: &Graph<NodeLabel, EdgeLabel>,
    layering: &[Vec<NodeId>],
) -> Conflicts {
    let mut c = find_type1_conflicts(g, layering);
    c.extend(find_type2_conflicts(g, layering));
    c
}

// --- Vertical alignment ---

/// Group nodes into vertical blocks by aligning each with its median neighbor.
fn vertical_alignment(
    g: &Graph<NodeLabel, EdgeLabel>,
    layering: &[Vec<NodeId>],
    conflicts: &Conflicts,
    neighbor_fn: &NeighborFn,
) -> (HashMap<NodeId, NodeId>, HashMap<NodeId, NodeId>) {
    let mut root: HashMap<NodeId, NodeId> = HashMap::new();
    let mut align: HashMap<NodeId, NodeId> = HashMap::new();

    // Position cache based on the adjusted layering
    let mut pos: HashMap<NodeId, usize> = HashMap::new();
    for layer in layering {
        for (order, &v) in layer.iter().enumerate() {
            root.insert(v, v);
            align.insert(v, v);
            pos.insert(v, order);
        }
    }

    for layer in layering {
        let mut prev_idx: i64 = -1;
        for &v in layer {
            let mut ws = neighbor_fn(g, v);
            if ws.is_empty() {
                continue;
            }
            ws.sort_by_key(|&w| *pos.get(&w).unwrap_or(&0));

            let mp = (ws.len() as f64 - 1.0) / 2.0;
            let lo = mp.floor() as usize;
            let hi = mp.ceil() as usize;

            for &w in &ws[lo..=hi] {
                if align[&v] == v
                    && prev_idx < *pos.get(&w).unwrap_or(&0) as i64
                    && !has_conflict(conflicts, v, w)
                {
                    align.insert(w, v);
                    let rw = root[&w];
                    root.insert(v, rw);
                    align.insert(v, rw);
                    prev_idx = *pos.get(&w).unwrap_or(&0) as i64;
                }
            }
        }
    }

    (root, align)
}

// --- Horizontal compaction ---

/// Compute minimum separation between two adjacent nodes.
fn sep(
    g: &Graph<NodeLabel, EdgeLabel>,
    v: NodeId,
    w: NodeId,
    reverse_sep: bool,
    config: &DagreConfig,
) -> f64 {
    let v_label = g.node(v).unwrap();
    let w_label = g.node(w).unwrap();
    let node_sep = config.nodesep;
    let edge_sep = config.edgesep;

    let mut sum = v_label.width / 2.0;

    // Label position offset for v
    if let Some(lp) = v_label.label_pos {
        let delta = match lp {
            LabelPos::Left => -v_label.width / 2.0,
            LabelPos::Right => v_label.width / 2.0,
            LabelPos::Center => 0.0,
        };
        sum += if reverse_sep { delta } else { -delta };
    }

    sum += if v_label.dummy.is_some() {
        edge_sep
    } else {
        node_sep
    } / 2.0;
    sum += if w_label.dummy.is_some() {
        edge_sep
    } else {
        node_sep
    } / 2.0;

    sum += w_label.width / 2.0;

    if let Some(lp) = w_label.label_pos {
        let delta = match lp {
            LabelPos::Left => w_label.width / 2.0,
            LabelPos::Right => -w_label.width / 2.0,
            LabelPos::Center => 0.0,
        };
        sum += if reverse_sep { delta } else { -delta };
    }

    sum
}

/// Build a block graph: nodes are block roots, edges represent
/// minimum separation constraints between adjacent blocks.
fn build_block_graph(
    g: &Graph<NodeLabel, EdgeLabel>,
    layering: &[Vec<NodeId>],
    root: &HashMap<NodeId, NodeId>,
    reverse_sep: bool,
    config: &DagreConfig,
) -> (Vec<NodeId>, AdjList, AdjList) {
    // Block graph as adjacency lists
    let mut block_nodes: HashSet<NodeId> = HashSet::new();
    let mut fwd_edges: HashMap<NodeId, Vec<(NodeId, f64)>> = HashMap::new();
    let mut rev_edges: HashMap<NodeId, Vec<(NodeId, f64)>> = HashMap::new();

    for layer in layering {
        let mut prev: Option<NodeId> = None;
        for &v in layer {
            let v_root = root[&v];
            block_nodes.insert(v_root);
            if let Some(u) = prev {
                let u_root = root[&u];
                let separation = sep(g, v, u, reverse_sep, config);
                let entry = fwd_edges.entry(u_root).or_default();
                // Keep max separation between same block pair
                if let Some(e) = entry.iter_mut().find(|(dst, _)| *dst == v_root) {
                    e.1 = e.1.max(separation);
                } else {
                    entry.push((v_root, separation));
                }
                let rev_entry = rev_edges.entry(v_root).or_default();
                if let Some(e) = rev_entry.iter_mut().find(|(src, _)| *src == u_root) {
                    e.1 = e.1.max(separation);
                } else {
                    rev_entry.push((u_root, separation));
                }
            }
            prev = Some(v);
        }
    }

    let nodes: Vec<NodeId> = block_nodes.into_iter().collect();
    (nodes, fwd_edges, rev_edges)
}

/// Assign x coordinates by compacting blocks.
///
/// Two passes: first assigns smallest valid coordinates, second
/// moves blocks right to fill unused space.
fn horizontal_compaction(
    g: &Graph<NodeLabel, EdgeLabel>,
    layering: &[Vec<NodeId>],
    root: &HashMap<NodeId, NodeId>,
    _align: &HashMap<NodeId, NodeId>,
    reverse_sep: bool,
    config: &DagreConfig,
) -> Vec<(NodeId, f64)> {
    let (block_nodes, fwd_edges, rev_edges) =
        build_block_graph(g, layering, root, reverse_sep, config);

    let border_type = if reverse_sep {
        crate::labels::BorderType::Left
    } else {
        crate::labels::BorderType::Right
    };

    let mut xs: HashMap<NodeId, f64> = HashMap::new();

    // Pass 1: assign smallest coordinates (process in topo order via DFS post-order)
    dfs_iterate(&block_nodes, &rev_edges, |elem| {
        let x = rev_edges
            .get(&elem)
            .map_or(0.0, |preds| {
                preds
                    .iter()
                    .map(|&(pred, sep)| xs.get(&pred).unwrap_or(&0.0) + sep)
                    .fold(0.0f64, f64::max)
            });
        xs.insert(elem, x);
    });

    // Pass 2: assign greatest coordinates (compact right)
    dfs_iterate(&block_nodes, &fwd_edges, |elem| {
        let min = fwd_edges
            .get(&elem)
            .map_or(f64::INFINITY, |succs| {
                succs
                    .iter()
                    .map(|&(succ, sep)| xs.get(&succ).unwrap_or(&0.0) - sep)
                    .fold(f64::INFINITY, f64::min)
            });

        if min != f64::INFINITY
            && g.node(elem)
                .unwrap()
                .border_type
                .is_none_or(|bt| bt != border_type)
        {
            let cur = xs[&elem];
            xs.insert(elem, cur.max(min));
        }
    });

    // Every node's x = its block root's x
    let mut result: Vec<(NodeId, f64)> = Vec::new();
    for (&v, &r) in root {
        if let Some(&x) = xs.get(&r) {
            result.push((v, x));
        }
    }

    result
}

/// DFS post-order iteration over a graph given as adjacency list.
/// Processes each node after all its predecessors (per `pred_edges`).
fn dfs_iterate(
    nodes: &[NodeId],
    pred_edges: &HashMap<NodeId, Vec<(NodeId, f64)>>,
    mut process: impl FnMut(NodeId),
) {
    let mut visited: HashSet<NodeId> = HashSet::new();
    let mut stack: Vec<(NodeId, bool)> = Vec::new();

    for &n in nodes {
        if !visited.contains(&n) {
            stack.push((n, false));
        }
        while let Some((elem, processed)) = stack.pop() {
            if processed {
                process(elem);
            } else if visited.insert(elem) {
                stack.push((elem, true));
                if let Some(preds) = pred_edges.get(&elem) {
                    for &(pred, _) in preds {
                        if !visited.contains(&pred) {
                            stack.push((pred, false));
                        }
                    }
                }
            }
        }
    }
}

// --- Alignment selection ---

fn find_smallest_width_alignment(
    g: &Graph<NodeLabel, EdgeLabel>,
    maps: &[HashMap<NodeId, f64>],
) -> usize {
    let mut best_idx = 0;
    let mut best_width = f64::INFINITY;

    for (idx, xs) in maps.iter().enumerate() {
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for (&v, &x) in xs {
            let hw = g.node(v).unwrap().width / 2.0;
            min = min.min(x - hw);
            max = max.max(x + hw);
        }
        let width = max - min;
        if width < best_width {
            best_width = width;
            best_idx = idx;
        }
    }

    best_idx
}

fn align_coordinates(maps: &mut [HashMap<NodeId, f64>], align_to_idx: usize) {
    let align_to = &maps[align_to_idx];
    let align_min = align_to.values().copied().fold(f64::INFINITY, f64::min);
    let align_max = align_to.values().copied().fold(f64::NEG_INFINITY, f64::max);

    // Compute deltas first (idx 0=ul, 1=ur, 2=dl, 3=dr)
    let deltas: Vec<f64> = (0..4)
        .map(|idx| {
            if idx == align_to_idx || maps[idx].is_empty() {
                return 0.0;
            }
            let is_right = idx % 2 == 1;
            if is_right {
                align_max - maps[idx].values().copied().fold(f64::NEG_INFINITY, f64::max)
            } else {
                align_min - maps[idx].values().copied().fold(f64::INFINITY, f64::min)
            }
        })
        .collect();

    for (map, &delta) in maps.iter_mut().zip(&deltas) {
        if delta.abs() > f64::EPSILON {
            for x in map.values_mut() {
                *x += delta;
            }
        }
    }
}

fn balance(
    maps: &[HashMap<NodeId, f64>],
    config: &DagreConfig,
) -> Vec<(NodeId, f64)> {
    if let Some(align) = config.align {
        let idx = match align {
            crate::config::Align::UL => 0,
            crate::config::Align::UR => 1,
            crate::config::Align::DL => 2,
            crate::config::Align::DR => 3,
        };
        return maps[idx].iter().map(|(&v, &x)| (v, x)).collect();
    }

    // Median of 4 alignments (take middle two, average)
    let all_nodes: HashSet<NodeId> = maps.iter().flat_map(|m| m.keys().copied()).collect();
    let mut result = Vec::with_capacity(all_nodes.len());

    for v in all_nodes {
        let mut xs: Vec<f64> = maps
            .iter()
            .filter_map(|m| m.get(&v).copied())
            .collect();
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let x = if xs.len() >= 4 {
            (xs[1] + xs[2]) / 2.0
        } else if xs.len() >= 2 {
            (xs[0] + xs[xs.len() - 1]) / 2.0
        } else {
            xs.first().copied().unwrap_or(0.0)
        };
        result.push((v, x));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_graph() -> Graph<NodeLabel, EdgeLabel> {
        // a -> b -> c, each at rank 0,1,2, order 0
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(40.0, 20.0));
        let b = g.add_node(NodeLabel::new(40.0, 20.0));
        let c = g.add_node(NodeLabel::new(40.0, 20.0));
        g.add_edge(a, b, EdgeLabel::default());
        g.add_edge(b, c, EdgeLabel::default());
        g.node_mut(a).unwrap().rank = 0;
        g.node_mut(b).unwrap().rank = 1;
        g.node_mut(c).unwrap().rank = 2;
        g.node_mut(a).unwrap().order = 0;
        g.node_mut(b).unwrap().order = 0;
        g.node_mut(c).unwrap().order = 0;
        g
    }

    #[test]
    fn linear_chain_all_aligned() {
        let g = simple_graph();
        let config = DagreConfig::default();
        let xs = position_x(&g, &config);
        // All nodes should have the same x (single column)
        let x_vals: Vec<f64> = xs.iter().map(|(_, x)| *x).collect();
        assert!(x_vals.windows(2).all(|w| (w[0] - w[1]).abs() < 1.0));
    }

    #[test]
    fn two_nodes_same_layer_separated() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(40.0, 20.0));
        let b = g.add_node(NodeLabel::new(40.0, 20.0));
        g.node_mut(a).unwrap().rank = 0;
        g.node_mut(b).unwrap().rank = 0;
        g.node_mut(a).unwrap().order = 0;
        g.node_mut(b).unwrap().order = 1;

        let config = DagreConfig::default();
        let xs = position_x(&g, &config);
        let xs_map: HashMap<NodeId, f64> = xs.into_iter().collect();
        // b should be to the right of a by at least nodesep
        let diff = xs_map[&b] - xs_map[&a];
        assert!(diff >= config.nodesep);
    }

    #[test]
    fn conflict_detection_no_panic() {
        let g = simple_graph();
        let layering = util::build_layer_matrix(&g);
        let conflicts = find_conflicts(&g, &layering);
        // Simple chain: no conflicts
        assert!(conflicts.is_empty());
    }

    #[test]
    fn vertical_alignment_chain() {
        let g = simple_graph();
        let layering = util::build_layer_matrix(&g);
        let conflicts = Conflicts::new();
        let neighbor_fn = |g: &Graph<NodeLabel, EdgeLabel>, v: NodeId| -> Vec<NodeId> {
            g.in_edges(v)
                .filter_map(|eid| g.edge_endpoints(eid).map(|(s, _)| s))
                .collect()
        };
        let (root, _align) = vertical_alignment(&g, &layering, &conflicts, &neighbor_fn);
        // In a simple chain, all nodes should share the same root
        let roots: HashSet<NodeId> = root.values().copied().collect();
        assert_eq!(roots.len(), 1);
    }
}
