use rusty_mermaid_core::{Direction, Point};
use rusty_mermaid_graph::{Graph, NodeId};

use crate::config::DagreConfig;
use crate::labels::{DummyKind, EdgeLabel, LabelPos, NodeLabel};
use crate::{
    acyclic, border_segments, coord_system, nesting, normalize, order, parent_dummy_chains,
    position, self_edges, util,
};

/// Run the full dagre Sugiyama layout pipeline.
///
/// Mutates `graph` in place, assigning `x`/`y` coordinates to every node and
/// populating `points` on every edge. The coordinate origin is
/// (`config.marginx`, `config.marginy`).
///
/// The pipeline phases, in order:
/// 1. **Cycle removal** -- reverse back-edges to make the graph acyclic
/// 2. **Rank assignment** -- assign each node to a layer ([`crate::rank`])
/// 3. **Normalize** -- split long edges with dummy nodes ([`crate::normalize`])
/// 4. **Order** -- minimize crossings within each rank ([`crate::order`])
/// 5. **Position** -- assign x/y via Brandes-Kopf ([`crate::position`])
/// 6. **Denormalize** -- remove dummy nodes, reconstruct edge routes
///
/// # Examples
///
/// ```
/// use rusty_mermaid_graph::Graph;
/// use rusty_mermaid_dagre::config::DagreConfig;
/// use rusty_mermaid_dagre::labels::{NodeLabel, EdgeLabel};
/// use rusty_mermaid_dagre::pipeline::layout;
///
/// let mut g = Graph::new();
/// let a = g.add_node(NodeLabel::new(40.0, 20.0));
/// let b = g.add_node(NodeLabel::new(40.0, 20.0));
/// g.add_edge(a, b, EdgeLabel::default());
///
/// layout(&mut g, &DagreConfig::default());
///
/// assert!(g.node(a).unwrap().y < g.node(b).unwrap().y); // a above b in TB
/// ```
pub fn layout(graph: &mut Graph<NodeLabel, EdgeLabel>, config: &DagreConfig) {
    // Work with halved ranksep (to make room for edge labels on intermediate ranks)
    let mut cfg = config.clone();
    cfg.ranksep /= 2.0;

    make_space_for_edge_labels(graph, &cfg);
    self_edges::remove_self_edges(graph);

    acyclic::run(graph, cfg.acyclicer);

    let nesting_state = nesting::run(graph);
    crate::rank::rank(graph, cfg.ranker);
    inject_edge_label_proxies(graph);
    remove_empty_ranks(graph, nesting_state.node_rank_factor);
    nesting::cleanup(graph, &nesting_state);
    util::normalize_ranks(graph);
    border_segments::assign_rank_min_max(graph);
    remove_edge_label_proxies(graph);

    let dummy_chains = normalize::run(graph);
    parent_dummy_chains::parent_dummy_chains(graph, &dummy_chains);
    border_segments::add_border_segments(graph);
    border_segments::extend_rank_min_max(graph);

    order::order(graph);

    self_edges::insert_self_edges(graph);
    coord_system::adjust(graph, cfg.rankdir);

    position::position(graph, &cfg);

    self_edges::position_self_edges(graph);
    remove_border_nodes(graph);
    normalize::undo(graph, &dummy_chains);
    fixup_edge_label_coords(graph);
    coord_system::undo(graph, cfg.rankdir);
    translate_graph(graph, config);
    assign_node_intersects(graph);
    reverse_points_for_reversed_edges(graph);

    acyclic::undo(graph);
}

/// Double minlen and adjust edge label widths to make space for labels.
fn make_space_for_edge_labels(graph: &mut Graph<NodeLabel, EdgeLabel>, config: &DagreConfig) {
    for eid in graph.edge_ids().collect::<Vec<_>>() {
        let Some(e) = graph.edge_mut(eid) else {
            continue;
        };
        e.minlen *= 2;
        if e.labelpos != LabelPos::Center {
            if config.rankdir == Direction::TB || config.rankdir == Direction::BT {
                e.width += e.labeloffset;
            } else {
                e.height += e.labeloffset;
            }
        }
    }
}

/// Create temporary proxy nodes to preserve edge label rank positions
/// across empty-rank removal.
fn inject_edge_label_proxies(graph: &mut Graph<NodeLabel, EdgeLabel>) {
    let edges: Vec<_> = graph
        .edge_ids()
        .filter_map(|eid| {
            let e = graph.edge(eid)?;
            if e.width > 0.0 && e.height > 0.0 {
                let (src, dst) = graph.edge_endpoints(eid)?;
                let v_rank = graph.node(src)?.rank;
                let w_rank = graph.node(dst)?.rank;
                Some((eid, (w_rank - v_rank) / 2 + v_rank))
            } else {
                None
            }
        })
        .collect();

    for (eid, proxy_rank) in edges {
        let mut label = NodeLabel::new(0.0, 0.0);
        label.rank = proxy_rank;
        label.dummy = Some(DummyKind::EdgeLabel);
        label.proxy_edge = Some(eid);
        graph.add_node(label);
    }
}

/// Remove ranks that contain no nodes, preserving empty ranks at multiples
/// of `node_rank_factor` (used by nesting to ensure edge labels land on
/// dedicated intermediate ranks).
fn remove_empty_ranks(graph: &mut Graph<NodeLabel, EdgeLabel>, node_rank_factor: i32) {
    let offset = graph
        .node_ids()
        .filter_map(|nid| Some(graph.node(nid)?.rank))
        .min()
        .unwrap_or(0);

    let max_rank = util::max_rank(graph);
    if max_rank < offset {
        return;
    }
    let len = (max_rank - offset + 1) as usize;

    // Build layers: which ranks are occupied?
    let mut occupied = vec![false; len];
    for nid in graph.node_ids() {
        let Some(n) = graph.node(nid) else { continue };
        let r = (n.rank - offset) as usize;
        if r < len {
            occupied[r] = true;
        }
    }

    // Compute delta shifts: remove empty ranks that are NOT at multiples
    // of node_rank_factor (matching JS dagre's removeEmptyRanks).
    let mut delta = 0i32;
    let mut shift = vec![0i32; len];
    for (i, &occ) in occupied.iter().enumerate() {
        if !occ && (i as i32) % node_rank_factor != 0 {
            delta -= 1;
        }
        shift[i] = delta;
    }

    // Apply shifts
    for nid in graph.node_ids().collect::<Vec<_>>() {
        let Some(n) = graph.node(nid) else { continue };
        let r = (n.rank - offset) as usize;
        if r < len && shift[r] != 0 {
            let Some(n) = graph.node_mut(nid) else {
                continue;
            };
            n.rank += shift[r];
        }
    }
}

/// Remove edge label proxy nodes and store their rank on the corresponding edge.
fn remove_edge_label_proxies(graph: &mut Graph<NodeLabel, EdgeLabel>) {
    let proxies: Vec<_> = graph
        .node_ids()
        .filter_map(|nid| {
            let node = graph.node(nid)?;
            if node.dummy == Some(DummyKind::EdgeLabel) {
                Some((nid, node.rank, node.proxy_edge))
            } else {
                None
            }
        })
        .collect();

    for (nid, rank, proxy_edge) in proxies {
        if let Some(eid) = proxy_edge
            && let Some(edge) = graph.edge_mut(eid)
        {
            edge.label_rank = Some(rank);
        }
        graph.remove_node(nid);
    }
}

/// Compute compound node dimensions from border node positions, then remove
/// all border dummy nodes.
///
/// Matches JS dagre's `removeBorderNodes`: each compound's size is determined
/// by its borderTop/borderBottom (y extent) and the last entries in
/// borderLeft/borderRight (x extent). Border nodes have been positioned by
/// the BK algorithm, so their coordinates define the compound's bounding box.
fn remove_border_nodes(graph: &mut Graph<NodeLabel, EdgeLabel>) {
    // First pass: size each compound from its border nodes
    let compounds: Vec<NodeId> = graph
        .node_ids()
        .filter(|&nid| graph.children(nid).next().is_some())
        .collect();

    for &nid in &compounds {
        let Some(node) = graph.node(nid) else {
            continue;
        };
        let bt = node.border_top;
        let bb = node.border_bottom;
        // Last entries = highest rank in the map (matching JS borderLeft[borderLeft.length-1])
        let bl = node
            .border_left
            .iter()
            .max_by_key(|&(&r, _)| r)
            .map(|(_, &v)| v);
        let br = node
            .border_right
            .iter()
            .max_by_key(|&(&r, _)| r)
            .map(|(_, &v)| v);

        if let (Some(t_id), Some(b_id), Some(l_id), Some(r_id)) = (bt, bb, bl, br) {
            let Some(t) = graph.node(t_id) else { continue };
            let Some(b) = graph.node(b_id) else { continue };
            let Some(l) = graph.node(l_id) else { continue };
            let Some(r) = graph.node(r_id) else { continue };

            let width = (r.x - l.x).abs();
            let height = (b.y - t.y).abs();
            let x = l.x + width / 2.0;
            let y = t.y + height / 2.0;

            let Some(n) = graph.node_mut(nid) else {
                continue;
            };
            n.width = width;
            n.height = height;
            n.x = x;
            n.y = y;
        }
    }

    // Second pass: remove border nodes
    let borders: Vec<NodeId> = graph
        .node_ids()
        .filter(|&nid| {
            graph
                .node(nid)
                .is_some_and(|n| n.dummy == Some(DummyKind::Border))
        })
        .collect();
    for nid in borders {
        graph.remove_node(nid);
    }
}

/// Fix up edge label coordinates based on label position.
fn fixup_edge_label_coords(graph: &mut Graph<NodeLabel, EdgeLabel>) {
    for eid in graph.edge_ids().collect::<Vec<_>>() {
        let Some(e) = graph.edge(eid) else { continue };
        if e.x == 0.0 && e.y == 0.0 {
            continue;
        }
        let lp = e.labelpos;
        let lo = e.labeloffset;

        let Some(e) = graph.edge_mut(eid) else {
            continue;
        };
        match lp {
            LabelPos::Left => {
                e.width -= lo;
                e.x -= e.width / 2.0 + lo;
            }
            LabelPos::Right => {
                e.width -= lo;
                e.x += e.width / 2.0 + lo;
            }
            LabelPos::Center => {}
        }
    }
}

/// Translate the entire graph so its bounding box starts at (marginx, marginy).
fn translate_graph(graph: &mut Graph<NodeLabel, EdgeLabel>, config: &DagreConfig) {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for nid in graph.node_ids() {
        let Some(n) = graph.node(nid) else { continue };
        min_x = min_x.min(n.x - n.width / 2.0);
        max_x = max_x.max(n.x + n.width / 2.0);
        min_y = min_y.min(n.y - n.height / 2.0);
        max_y = max_y.max(n.y + n.height / 2.0);
    }

    for eid in graph.edge_ids() {
        let Some(e) = graph.edge(eid) else { continue };
        if e.x != 0.0 || e.y != 0.0 {
            min_x = min_x.min(e.x - e.width / 2.0);
            max_x = max_x.max(e.x + e.width / 2.0);
            min_y = min_y.min(e.y - e.height / 2.0);
            max_y = max_y.max(e.y + e.height / 2.0);
        }
    }

    let dx = config.marginx - min_x;
    let dy = config.marginy - min_y;

    for nid in graph.node_ids().collect::<Vec<_>>() {
        let Some(n) = graph.node_mut(nid) else {
            continue;
        };
        n.x += dx;
        n.y += dy;
    }

    for eid in graph.edge_ids().collect::<Vec<_>>() {
        let Some(e) = graph.edge_mut(eid) else {
            continue;
        };
        for p in &mut e.points {
            p.x += dx;
            p.y += dy;
        }
        if e.x != 0.0 || e.y != 0.0 {
            e.x += dx;
            e.y += dy;
        }
    }
}

/// Assign edge-node intersection points as first/last edge points.
fn assign_node_intersects(graph: &mut Graph<NodeLabel, EdgeLabel>) {
    for eid in graph.edge_ids().collect::<Vec<_>>() {
        let Some((src, dst)) = graph.edge_endpoints(eid) else {
            continue;
        };
        let Some(node_v) = graph.node(src) else {
            continue;
        };
        let Some(node_w) = graph.node(dst) else {
            continue;
        };

        let Some(e) = graph.edge(eid) else { continue };
        let (p1, p2) = if e.points.is_empty() {
            (
                Point {
                    x: node_w.x,
                    y: node_w.y,
                },
                Point {
                    x: node_v.x,
                    y: node_v.y,
                },
            )
        } else {
            (e.points[0], e.points[e.points.len() - 1])
        };

        let v_bbox = rusty_mermaid_core::BBox {
            x: node_v.x,
            y: node_v.y,
            width: node_v.width,
            height: node_v.height,
        };
        let w_bbox = rusty_mermaid_core::BBox {
            x: node_w.x,
            y: node_w.y,
            width: node_w.width,
            height: node_w.height,
        };

        let start = rusty_mermaid_core::intersect_rect(&v_bbox, p1);
        let end = rusty_mermaid_core::intersect_rect(&w_bbox, p2);

        let Some(e) = graph.edge_mut(eid) else {
            continue;
        };
        e.points.insert(0, start);
        e.points.push(end);
    }
}

/// Reverse edge points for edges that were reversed during cycle removal.
fn reverse_points_for_reversed_edges(graph: &mut Graph<NodeLabel, EdgeLabel>) {
    for eid in graph.edge_ids().collect::<Vec<_>>() {
        if graph.edge(eid).is_some_and(|e| e.reversed)
            && let Some(e) = graph.edge_mut(eid)
        {
            e.points.reverse();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_layout_assigns_coordinates() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(40.0, 20.0));
        let b = g.add_node(NodeLabel::new(40.0, 20.0));
        let c = g.add_node(NodeLabel::new(40.0, 20.0));
        g.add_edge(a, b, EdgeLabel::default());
        g.add_edge(b, c, EdgeLabel::default());

        let config = DagreConfig::default();
        layout(&mut g, &config);

        // All nodes should have non-zero positions
        for nid in g.node_ids() {
            let n = g.node(nid).unwrap();
            assert!(n.x >= 0.0, "node x should be >= 0");
            assert!(n.y >= 0.0, "node y should be >= 0");
        }

        // Edges should have points
        for eid in g.edge_ids() {
            let e = g.edge(eid).unwrap();
            assert!(!e.points.is_empty(), "edge should have points");
        }
    }

    #[test]
    fn diamond_layout() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(40.0, 20.0));
        let b = g.add_node(NodeLabel::new(40.0, 20.0));
        let c = g.add_node(NodeLabel::new(40.0, 20.0));
        let d = g.add_node(NodeLabel::new(40.0, 20.0));
        g.add_edge(a, b, EdgeLabel::default());
        g.add_edge(a, c, EdgeLabel::default());
        g.add_edge(b, d, EdgeLabel::default());
        g.add_edge(c, d, EdgeLabel::default());

        let config = DagreConfig::default();
        layout(&mut g, &config);

        // a should be above b,c which should be above d
        let a_y = g.node(a).unwrap().y;
        let b_y = g.node(b).unwrap().y;
        let c_y = g.node(c).unwrap().y;
        let d_y = g.node(d).unwrap().y;
        assert!(a_y < b_y);
        assert!(a_y < c_y);
        assert!(b_y < d_y);
        assert!(c_y < d_y);
    }

    #[test]
    fn lr_layout_swaps_axes() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(40.0, 20.0));
        let b = g.add_node(NodeLabel::new(40.0, 20.0));
        g.add_edge(a, b, EdgeLabel::default());

        let mut config = DagreConfig::default();
        config.rankdir = Direction::LR;
        layout(&mut g, &config);

        // In LR, a should be to the left of b
        assert!(g.node(a).unwrap().x < g.node(b).unwrap().x);
    }

    #[test]
    fn single_node_layout() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(40.0, 20.0));

        let config = DagreConfig::default();
        layout(&mut g, &config);

        assert!(g.node(a).unwrap().x >= 0.0);
        assert!(g.node(a).unwrap().y >= 0.0);
    }

    #[test]
    fn nested_compound_inner_contains_children() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(80.0, 32.0));
        let b = g.add_node(NodeLabel::new(80.0, 32.0));
        let c = g.add_node(NodeLabel::new(80.0, 32.0));
        let d = g.add_node(NodeLabel::new(80.0, 32.0));

        let inner = g.add_node(NodeLabel::new(0.0, 0.0));
        let outer = g.add_node(NodeLabel::new(0.0, 0.0));

        g.set_parent(a, inner);
        g.set_parent(b, inner);
        g.set_parent(inner, outer);
        g.set_parent(c, outer);

        g.add_edge(a, b, EdgeLabel::default());
        g.add_edge(c, d, EdgeLabel::default());

        let config = DagreConfig::default();
        layout(&mut g, &config);

        let inner_n = g.node(inner).unwrap();
        let a_n = g.node(a).unwrap();
        let b_n = g.node(b).unwrap();

        let inner_left = inner_n.x - inner_n.width / 2.0;
        let inner_right = inner_n.x + inner_n.width / 2.0;

        eprintln!(
            "inner: x={:.1} w={:.1} [{:.1}, {:.1}]",
            inner_n.x, inner_n.width, inner_left, inner_right
        );
        eprintln!("a: x={:.1} w={:.1}", a_n.x, a_n.width);
        eprintln!("b: x={:.1} w={:.1}", b_n.x, b_n.width);

        assert!(
            inner_left <= a_n.x - a_n.width / 2.0,
            "inner should contain A: inner_left={inner_left} a_left={}",
            a_n.x - a_n.width / 2.0
        );
        assert!(
            inner_right >= a_n.x + a_n.width / 2.0,
            "inner should contain A: inner_right={inner_right} a_right={}",
            a_n.x + a_n.width / 2.0
        );
    }

    #[test]
    fn self_edge_preserved() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(40.0, 20.0));
        g.add_edge(a, a, EdgeLabel::default());

        let config = DagreConfig::default();
        layout(&mut g, &config);

        // Self-edge should be restored with points
        let has_self_edge = g
            .edge_ids()
            .any(|eid| g.edge_endpoints(eid).is_some_and(|(s, d)| s == d));
        assert!(has_self_edge);
    }
}
