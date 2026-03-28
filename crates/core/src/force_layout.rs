//! CoSE (Compound Spring Embedder) force-directed layout.
//!
//! Size-aware node repulsion via rectangle clipping points, overlap
//! separation, polynomial cooling schedule, convergence detection.
//!
//! Based on the cose-bilkent algorithm (Dogrusoz et al., "A layout algorithm
//! for undirected compound graphs", Information Sciences 179(7), 2009).

use std::collections::HashMap;
use std::f64::consts::TAU;

// ── Geometry ──────────────────────────────────────────────────────────

/// Axis-aligned rectangle: center + half-extents.
#[derive(Clone, Copy)]
struct Rect {
    cx: f64,
    cy: f64,
    hw: f64,
    hh: f64,
}

impl Rect {
    fn from_node(n: &ForceNode) -> Self {
        Self {
            cx: n.x,
            cy: n.y,
            hw: n.width / 2.0,
            hh: n.height / 2.0,
        }
    }

    fn overlaps(&self, other: &Self) -> bool {
        (self.cx - other.cx).abs() < self.hw + other.hw
            && (self.cy - other.cy).abs() < self.hh + other.hh
    }

    /// Where the line from this center toward `(tx, ty)` exits this rectangle.
    fn clip_point(&self, tx: f64, ty: f64) -> (f64, f64) {
        let dx = tx - self.cx;
        let dy = ty - self.cy;

        if dx.abs() < 1e-10 && dy.abs() < 1e-10 {
            return (self.cx + self.hw, self.cy);
        }
        if dx.abs() < 1e-10 {
            return (self.cx, self.cy + dy.signum() * self.hh);
        }
        if dy.abs() < 1e-10 {
            return (self.cx + dx.signum() * self.hw, self.cy);
        }

        let slope = dy / dx;
        let diag_slope = self.hh / self.hw;

        if slope.abs() <= diag_slope {
            let sx = dx.signum();
            (self.cx + sx * self.hw, self.cy + sx * self.hw * slope)
        } else {
            let sy = dy.signum();
            (self.cx + sy * self.hh / slope, self.cy + sy * self.hh)
        }
    }
}

/// Distance between border clipping points of two non-overlapping rects.
/// Returns `(dx, dy, dist)` pointing from `a`'s clip toward `b`'s clip.
fn clip_distance(a: &Rect, b: &Rect) -> (f64, f64, f64) {
    let (ax, ay) = a.clip_point(b.cx, b.cy);
    let (bx, by) = b.clip_point(a.cx, a.cy);
    let dx = bx - ax;
    let dy = by - ay;
    (dx, dy, (dx * dx + dy * dy).sqrt())
}

/// Separation vector for overlapping rects (pushes `a` by the returned amount;
/// push `b` by the negation). Includes `buffer` gap.
fn calc_separation(a: &Rect, b: &Rect, buffer: f64) -> (f64, f64) {
    let gap_x = (a.hw + b.hw + buffer) - (b.cx - a.cx).abs();
    let gap_y = (a.hh + b.hh + buffer) - (b.cy - a.cy).abs();

    if gap_x <= 0.0 || gap_y <= 0.0 {
        return (0.0, 0.0);
    }

    let dx = b.cx - a.cx;
    let dy = b.cy - a.cy;

    if dx.abs() < 1e-10 && dy.abs() < 1e-10 {
        return (-(gap_x / 2.0), 0.0);
    }

    // Separate along the center-to-center direction (slope comparison)
    let slope = dy.abs() / dx.abs().max(1e-10);
    let rect_slope = (a.hh + b.hh) / (a.hw + b.hw).max(1e-10);

    if slope <= rect_slope {
        let sign = if dx > 0.0 { -1.0 } else { 1.0 };
        (gap_x * sign / 2.0, 0.0)
    } else {
        let sign = if dy > 0.0 { -1.0 } else { 1.0 };
        (0.0, gap_y * sign / 2.0)
    }
}

// ── Spatial grid ──────────────────────────────────────────────────────

/// Grid-based spatial index for O(n·k) repulsion instead of O(n²).
/// Only nodes in the 3×3 neighborhood of each cell interact.
/// Rebuilt every `GRID_REBUILD_PERIOD` iterations.
struct SpatialGrid {
    cells: HashMap<(i32, i32), Vec<usize>>,
    cell_size: f64,
}

impl SpatialGrid {
    fn build(nodes: &[ForceNode], cell_size: f64) -> Self {
        let mut cells: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
        for (i, node) in nodes.iter().enumerate() {
            let key = Self::cell_key(node.x, node.y, cell_size);
            cells.entry(key).or_default().push(i);
        }
        Self { cells, cell_size }
    }

    fn cell_key(x: f64, y: f64, cell_size: f64) -> (i32, i32) {
        (
            (x / cell_size).floor() as i32,
            (y / cell_size).floor() as i32,
        )
    }

    /// Iterate all node indices in the 3×3 neighborhood of a position.
    fn neighbors(&self, x: f64, y: f64) -> impl Iterator<Item = usize> + '_ {
        let (cx, cy) = Self::cell_key(x, y, self.cell_size);
        (-1i32..=1).flat_map(move |dx| {
            (-1i32..=1).flat_map(move |dy| {
                self.cells
                    .get(&(cx + dx, cy + dy))
                    .into_iter()
                    .flat_map(|v| v.iter().copied())
            })
        })
    }
}

/// Grid rebuild frequency (iterations).
const GRID_REBUILD_PERIOD: usize = 10;
/// Below this node count, brute-force O(n²) is faster than grid overhead.
const GRID_NODE_THRESHOLD: usize = 50;

// ── Adaptive scaling ─────────────────────────────────────────────────

const ADAPT_LOWER: usize = 1000;
const ADAPT_UPPER: usize = 5000;

/// For large graphs, cool faster so we don't waste iterations.
fn adaptation_factor(n: usize) -> f64 {
    if n >= ADAPT_UPPER {
        3.0
    } else if n > ADAPT_LOWER {
        1.0 + 2.0 * (n - ADAPT_LOWER) as f64 / (ADAPT_UPPER - ADAPT_LOWER) as f64
    } else {
        1.0
    }
}

// ── Public types ──────────────────────────────────────────────────────

/// A node in the force-directed graph.
#[derive(Debug, Clone)]
pub struct ForceNode {
    pub id: usize,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    /// If true, this node's position is fixed (forces don't move it).
    pub fixed: bool,
    fx: f64,
    fy: f64,
}

impl ForceNode {
    pub fn new(id: usize) -> Self {
        Self {
            id,
            x: 0.0,
            y: 0.0,
            width: 40.0,
            height: 40.0,
            fixed: false,
            fx: 0.0,
            fy: 0.0,
        }
    }

    pub fn with_position(mut self, x: f64, y: f64) -> Self {
        self.x = x;
        self.y = y;
        self
    }

    pub fn with_size(mut self, w: f64, h: f64) -> Self {
        self.width = w;
        self.height = h;
        self
    }
}

/// An edge (spring) between two nodes.
#[derive(Debug, Clone)]
pub struct ForceEdge {
    pub source: usize,
    pub target: usize,
}

/// Configuration for the CoSE layout.
#[derive(Debug, Clone)]
pub struct ForceConfig {
    /// Hard cap on simulation iterations (convergence may stop earlier).
    pub max_iterations: usize,
    /// Repulsion strength (inverse-square constant between all node pairs).
    pub repulsion: f64,
    /// Spring constant for edges (Hooke).
    pub attraction: f64,
    /// Rest length of edge springs (distance measured between rect borders).
    pub ideal_length: f64,
    /// Gravity strength pulling outliers toward center.
    pub gravity: f64,
    /// Gravity only kicks in beyond `estimated_graph_size × gravity_range`.
    pub gravity_range: f64,
    /// Minimum repulsion distance (avoids division by zero).
    pub min_distance: f64,
    /// Maximum displacement per node per iteration.
    pub max_displacement: f64,
}

impl Default for ForceConfig {
    fn default() -> Self {
        Self {
            max_iterations: 2500,
            repulsion: 4500.0,
            attraction: 0.45,
            ideal_length: 50.0,
            gravity: 0.25,
            gravity_range: 3.8,
            min_distance: 5.0,
            max_displacement: 300.0,
        }
    }
}

impl ForceConfig {
    /// Preset tuned for tree/mindmap layouts.
    pub fn tree() -> Self {
        Self {
            ideal_length: 80.0,
            ..Self::default()
        }
    }
}

/// The force-directed graph: nodes + edges.
#[derive(Debug, Clone, Default)]
pub struct ForceGraph {
    pub nodes: Vec<ForceNode>,
    pub edges: Vec<ForceEdge>,
}


impl ForceGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: ForceNode) -> usize {
        let id = self.nodes.len();
        debug_assert!(node.id == id, "node id must match insertion index");
        self.nodes.push(node);
        id
    }

    pub fn add_edge(&mut self, source: usize, target: usize) {
        self.edges.push(ForceEdge { source, target });
    }
}

// ── Layout algorithm ──────────────────────────────────────────────────

const CONVERGENCE_PERIOD: usize = 100;
const DISPLACEMENT_PER_NODE: f64 = 1.5;

/// Polynomial cooling exponent for the given schedule parameters.
/// Produces a value such that `1.0 - max_cycle^exponent / 100 ≈ final_temp`.
fn cooling_exponent(max_cycle: usize, final_temp: f64) -> f64 {
    if max_cycle > 1 {
        (100.0 * (1.0 - final_temp)).max(1.0).ln() / (max_cycle as f64).ln()
    } else {
        1.0
    }
}

/// Compute the cooling factor for a given cycle.
///
/// `adapt` scales the effective cycle (> 1.0 for large graphs → cools faster).
/// Returns a value in `[final_temp, 1.0]` that decays polynomially.
fn compute_cooling(
    cooling_cycle: usize,
    adapt: f64,
    exponent: f64,
    max_cycle: usize,
    final_temp: f64,
) -> f64 {
    let effective = (cooling_cycle as f64 * adapt).min(max_cycle as f64);
    (1.0 - effective.powf(exponent) / 100.0).max(final_temp)
}

/// Run the CoSE force-directed layout.
///
/// Modifies node positions in-place. Uses polynomial cooling with early
/// convergence detection (displacement threshold + oscillation).
/// For graphs with > 50 nodes, repulsion uses a spatial grid (O(n·k)
/// instead of O(n²)), rebuilt every 10 iterations.
pub fn layout(graph: &mut ForceGraph, config: &ForceConfig) {
    let n = graph.nodes.len();
    if n <= 1 {
        return;
    }

    // Seed initial positions if all at origin
    if graph.nodes.iter().all(|n| n.x == 0.0 && n.y == 0.0) {
        seed_positions(graph);
    }

    let use_grid = n > GRID_NODE_THRESHOLD;
    let cell_size = config.ideal_length * 3.0;
    let adapt = adaptation_factor(n);

    let max_cycle = (config.max_iterations / CONVERGENCE_PERIOD).max(1);
    let final_temp = CONVERGENCE_PERIOD as f64 / config.max_iterations.max(1) as f64;
    let threshold = DISPLACEMENT_PER_NODE * n as f64;
    let exponent = cooling_exponent(max_cycle, final_temp);

    let mut cooling_cycle = 0usize;
    let mut prev_displacement = f64::MAX;
    let mut grid: Option<SpatialGrid> = None;

    for iter in 0..config.max_iterations {
        for node in &mut graph.nodes {
            node.fx = 0.0;
            node.fy = 0.0;
        }

        // Rebuild spatial grid periodically
        if use_grid && (grid.is_none() || iter % GRID_REBUILD_PERIOD == 0) {
            grid = Some(SpatialGrid::build(&graph.nodes, cell_size));
        }

        if let Some(ref g) = grid {
            apply_repulsion_grid(graph, config, g);
        } else {
            apply_repulsion_brute(graph, config);
        }
        apply_springs(graph, config);
        apply_gravity(graph, config);

        let cooling = compute_cooling(cooling_cycle, adapt, exponent, max_cycle, final_temp);
        let displacement = apply_displacements(graph, cooling, config);

        if (iter + 1) % CONVERGENCE_PERIOD == 0 {
            cooling_cycle += 1;

            if displacement < threshold {
                break;
            }
            if iter > config.max_iterations / 3 && (displacement - prev_displacement).abs() < 2.0 {
                break;
            }
            prev_displacement = displacement;
        }
    }
}

/// Deterministic circular seeding.
fn seed_positions(graph: &mut ForceGraph) {
    let n = graph.nodes.len();
    let radius = (n as f64 * 30.0).max(80.0);
    for (i, node) in graph.nodes.iter_mut().enumerate() {
        let angle = TAU * i as f64 / n as f64;
        node.x = radius * angle.cos();
        node.y = radius * angle.sin();
    }
}

/// Compute repulsion force between two rects.
/// Returns (fx, fy) applied to node `i` (negate for node `j`).
fn repulse_pair(ri: &Rect, rj: &Rect, config: &ForceConfig) -> (f64, f64) {
    let buffer = config.ideal_length / 2.0;

    if ri.overlaps(rj) {
        calc_separation(ri, rj, buffer)
    } else {
        let (dx, dy, dist) = clip_distance(ri, rj);
        let dist = dist.max(config.min_distance);
        let force = config.repulsion / (dist * dist);
        (-force * dx / dist, -force * dy / dist)
    }
}

/// O(n²) brute-force repulsion (used for small graphs).
fn apply_repulsion_brute(graph: &mut ForceGraph, config: &ForceConfig) {
    let n = graph.nodes.len();
    for i in 0..n {
        for j in (i + 1)..n {
            let ri = Rect::from_node(&graph.nodes[i]);
            let rj = Rect::from_node(&graph.nodes[j]);
            let (fx, fy) = repulse_pair(&ri, &rj, config);
            graph.nodes[i].fx += fx;
            graph.nodes[i].fy += fy;
            graph.nodes[j].fx -= fx;
            graph.nodes[j].fy -= fy;
        }
    }
}

/// Grid-accelerated repulsion: only checks 3×3 neighborhood per node.
fn apply_repulsion_grid(graph: &mut ForceGraph, config: &ForceConfig, grid: &SpatialGrid) {
    let n = graph.nodes.len();
    for i in 0..n {
        let ni = &graph.nodes[i];
        for j in grid.neighbors(ni.x, ni.y) {
            if j <= i {
                continue;
            }
            let ri = Rect::from_node(&graph.nodes[i]);
            let rj = Rect::from_node(&graph.nodes[j]);
            let (fx, fy) = repulse_pair(&ri, &rj, config);
            graph.nodes[i].fx += fx;
            graph.nodes[i].fy += fy;
            graph.nodes[j].fx -= fx;
            graph.nodes[j].fy -= fy;
        }
    }
}

/// Spring (Hooke) attraction along edges, measured from clip points.
/// Skipped when endpoints overlap (handled by repulsion separation).
fn apply_springs(graph: &mut ForceGraph, config: &ForceConfig) {
    for edge_idx in 0..graph.edges.len() {
        let s = graph.edges[edge_idx].source;
        let t = graph.edges[edge_idx].target;
        if s >= graph.nodes.len() || t >= graph.nodes.len() || s == t {
            continue;
        }

        let rs = Rect::from_node(&graph.nodes[s]);
        let rt = Rect::from_node(&graph.nodes[t]);
        if rs.overlaps(&rt) {
            continue;
        }

        let (dx, dy, dist) = clip_distance(&rs, &rt);
        let dist = dist.max(1.0);
        let force = config.attraction * (dist - config.ideal_length);
        let fx = force * dx / dist;
        let fy = force * dy / dist;

        graph.nodes[s].fx += fx;
        graph.nodes[s].fy += fy;
        graph.nodes[t].fx -= fx;
        graph.nodes[t].fy -= fy;
    }
}

/// Gravity toward graph center, only outside `estimated_size × gravity_range`.
fn apply_gravity(graph: &mut ForceGraph, config: &ForceConfig) {
    let (mut min_x, mut max_x) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY);
    for n in graph.nodes.iter() {
        min_x = min_x.min(n.x - n.width / 2.0);
        max_x = max_x.max(n.x + n.width / 2.0);
        min_y = min_y.min(n.y - n.height / 2.0);
        max_y = max_y.max(n.y + n.height / 2.0);
    }
    let estimated = ((max_x - min_x) + (max_y - min_y)) / 2.0;
    let range = estimated * config.gravity_range;
    let gx = (min_x + max_x) / 2.0;
    let gy = (min_y + max_y) / 2.0;

    for node in &mut graph.nodes {
        let dx = node.x - gx;
        let dy = node.y - gy;
        if dx.abs() > range || dy.abs() > range {
            node.fx -= config.gravity * dx;
            node.fy -= config.gravity * dy;
        }
    }
}

/// Apply accumulated forces scaled by cooling factor.
/// Returns total displacement (for convergence check).
fn apply_displacements(graph: &mut ForceGraph, cooling: f64, config: &ForceConfig) -> f64 {
    let max = cooling * config.max_displacement;
    let mut total = 0.0;

    for node in &mut graph.nodes {
        if node.fixed {
            continue;
        }

        let mut dx = cooling * node.fx;
        let mut dy = cooling * node.fy;

        if dx.abs() > max {
            dx = max * dx.signum();
        }
        if dy.abs() > max {
            dy = max * dy.signum();
        }

        node.x += dx;
        node.y += dy;

        if !node.x.is_finite() {
            node.x = 0.0;
        }
        if !node.y.is_finite() {
            node.y = 0.0;
        }

        total += dx.abs() + dy.abs();
    }

    total
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Geometry unit tests ──

    #[test]
    fn clip_point_right_edge() {
        let r = Rect {
            cx: 0.0,
            cy: 0.0,
            hw: 20.0,
            hh: 10.0,
        };
        let (x, y) = r.clip_point(100.0, 0.0);
        assert!((x - 20.0).abs() < 0.01);
        assert!(y.abs() < 0.01);
    }

    #[test]
    fn clip_point_top_edge() {
        let r = Rect {
            cx: 0.0,
            cy: 0.0,
            hw: 20.0,
            hh: 10.0,
        };
        let (x, y) = r.clip_point(0.0, -100.0);
        assert!(x.abs() < 0.01);
        assert!((y - -10.0).abs() < 0.01);
    }

    #[test]
    fn clip_point_diagonal() {
        let r = Rect {
            cx: 0.0,
            cy: 0.0,
            hw: 20.0,
            hh: 20.0,
        };
        let (x, y) = r.clip_point(100.0, 100.0);
        assert!((x - 20.0).abs() < 0.01);
        assert!((y - 20.0).abs() < 0.01);
    }

    #[test]
    fn overlap_detection() {
        let a = Rect {
            cx: 0.0,
            cy: 0.0,
            hw: 20.0,
            hh: 10.0,
        };
        let b = Rect {
            cx: 30.0,
            cy: 0.0,
            hw: 20.0,
            hh: 10.0,
        };
        assert!(a.overlaps(&b), "rects touching should overlap");

        let c = Rect {
            cx: 50.0,
            cy: 0.0,
            hw: 20.0,
            hh: 10.0,
        };
        assert!(!a.overlaps(&c), "separated rects should not overlap");
    }

    #[test]
    fn separation_pushes_apart() {
        let a = Rect {
            cx: 0.0,
            cy: 0.0,
            hw: 20.0,
            hh: 10.0,
        };
        let b = Rect {
            cx: 10.0,
            cy: 0.0,
            hw: 20.0,
            hh: 10.0,
        };
        let (sx, sy) = calc_separation(&a, &b, 5.0);
        assert!(sx < 0.0, "a should be pushed left");
        assert!(sy.abs() < 0.01, "no vertical separation needed");
    }

    // ── Layout behavioral tests ──

    #[test]
    fn empty_graph_is_noop() {
        let mut g = ForceGraph::new();
        layout(&mut g, &ForceConfig::default());
        assert!(g.nodes.is_empty());
    }

    #[test]
    fn single_node_stays_put() {
        let mut g = ForceGraph::new();
        g.add_node(ForceNode::new(0));
        layout(&mut g, &ForceConfig::default());
        assert!((g.nodes[0].x).abs() < 1.0);
        assert!((g.nodes[0].y).abs() < 1.0);
    }

    #[test]
    fn two_connected_nodes_reach_equilibrium() {
        let mut g = ForceGraph::new();
        g.add_node(ForceNode::new(0).with_position(-80.0, 0.0));
        g.add_node(ForceNode::new(1).with_position(80.0, 0.0));
        g.add_edge(0, 1);
        layout(&mut g, &ForceConfig::default());

        let dist =
            ((g.nodes[1].x - g.nodes[0].x).powi(2) + (g.nodes[1].y - g.nodes[0].y).powi(2)).sqrt();
        assert!(dist > 20.0, "connected nodes too close: {dist}");
        assert!(dist < 300.0, "connected nodes too far: {dist}");
    }

    #[test]
    fn unconnected_nodes_repel() {
        let mut g = ForceGraph::new();
        g.add_node(ForceNode::new(0).with_position(0.0, 0.0));
        g.add_node(ForceNode::new(1).with_position(5.0, 0.0));
        layout(&mut g, &ForceConfig::default());

        let dist =
            ((g.nodes[1].x - g.nodes[0].x).powi(2) + (g.nodes[1].y - g.nodes[0].y).powi(2)).sqrt();
        assert!(dist > 10.0, "unconnected nodes should repel: dist={dist}");
    }

    #[test]
    fn triangle_converges() {
        let mut g = ForceGraph::new();
        for i in 0..3 {
            g.add_node(ForceNode::new(i));
        }
        g.add_edge(0, 1);
        g.add_edge(1, 2);
        g.add_edge(2, 0);
        layout(&mut g, &ForceConfig::default());

        for n in &g.nodes {
            assert!(n.x.is_finite(), "node {} x not finite", n.id);
            assert!(n.y.is_finite(), "node {} y not finite", n.id);
        }
    }

    #[test]
    fn fixed_nodes_dont_move() {
        let mut g = ForceGraph::new();
        g.add_node(ForceNode::new(0).with_position(100.0, 100.0));
        g.nodes[0].fixed = true;
        g.add_node(ForceNode::new(1));
        g.add_edge(0, 1);
        layout(&mut g, &ForceConfig::default());

        assert!((g.nodes[0].x - 100.0).abs() < f64::EPSILON);
        assert!((g.nodes[0].y - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn disconnected_components_separate() {
        let mut g = ForceGraph::new();
        for i in 0..5 {
            g.add_node(ForceNode::new(i));
        }
        g.add_edge(0, 1);
        g.add_edge(1, 2);
        g.add_edge(3, 4);

        layout(&mut g, &ForceConfig::default());

        let c1x = (g.nodes[0].x + g.nodes[1].x + g.nodes[2].x) / 3.0;
        let c1y = (g.nodes[0].y + g.nodes[1].y + g.nodes[2].y) / 3.0;
        let c2x = (g.nodes[3].x + g.nodes[4].x) / 2.0;
        let c2y = (g.nodes[3].y + g.nodes[4].y) / 2.0;
        let d = ((c2x - c1x).powi(2) + (c2y - c1y).powi(2)).sqrt();
        assert!(d > 10.0, "components should separate: dist={d}");
    }

    #[test]
    fn large_chain_converges() {
        let mut g = ForceGraph::new();
        let n = 50;
        for i in 0..n {
            g.add_node(ForceNode::new(i));
        }
        for i in 0..n - 1 {
            g.add_edge(i, i + 1);
        }
        layout(&mut g, &ForceConfig::default());

        for node in &g.nodes {
            assert!(node.x.is_finite(), "node {} x NaN/Inf", node.id);
            assert!(node.y.is_finite(), "node {} y NaN/Inf", node.id);
        }
    }

    #[test]
    fn tree_config_spreads_children() {
        let mut g = ForceGraph::new();
        g.add_node(ForceNode::new(0));
        for i in 1..6 {
            g.add_node(ForceNode::new(i));
            g.add_edge(0, i);
        }
        layout(&mut g, &ForceConfig::tree());

        let root = &g.nodes[0];
        for child in &g.nodes[1..] {
            let d = ((child.x - root.x).powi(2) + (child.y - root.y).powi(2)).sqrt();
            assert!(d > 20.0, "child {} too close to root: {d}", child.id);
        }
    }

    #[test]
    fn deterministic() {
        let make = || {
            let mut g = ForceGraph::new();
            for i in 0..5 {
                g.add_node(ForceNode::new(i));
            }
            g.add_edge(0, 1);
            g.add_edge(1, 2);
            g.add_edge(2, 3);
            g.add_edge(3, 4);
            g
        };

        let mut g1 = make();
        let mut g2 = make();
        let cfg = ForceConfig::default();
        layout(&mut g1, &cfg);
        layout(&mut g2, &cfg);

        for (a, b) in g1.nodes.iter().zip(g2.nodes.iter()) {
            assert!((a.x - b.x).abs() < 1e-10, "node {} x differs", a.id);
            assert!((a.y - b.y).abs() < 1e-10, "node {} y differs", a.id);
        }
    }

    #[test]
    fn connected_closer_than_unconnected() {
        let mut g = ForceGraph::new();
        g.add_node(ForceNode::new(0));
        g.add_node(ForceNode::new(1));
        g.add_node(ForceNode::new(2));
        g.add_edge(0, 1);

        layout(&mut g, &ForceConfig::default());

        let d01 =
            ((g.nodes[1].x - g.nodes[0].x).powi(2) + (g.nodes[1].y - g.nodes[0].y).powi(2)).sqrt();
        let d02 =
            ((g.nodes[2].x - g.nodes[0].x).powi(2) + (g.nodes[2].y - g.nodes[0].y).powi(2)).sqrt();
        assert!(
            d01 < d02,
            "connected ({d01}) should be closer than unconnected ({d02})"
        );
    }

    #[test]
    fn sized_nodes_dont_overlap() {
        let mut g = ForceGraph::new();
        for i in 0..6 {
            g.add_node(ForceNode::new(i).with_size(80.0, 40.0));
        }
        g.add_edge(0, 1);
        g.add_edge(0, 2);
        g.add_edge(0, 3);
        g.add_edge(0, 4);
        g.add_edge(0, 5);
        layout(&mut g, &ForceConfig::default());

        for i in 0..6 {
            for j in (i + 1)..6 {
                let ri = Rect::from_node(&g.nodes[i]);
                let rj = Rect::from_node(&g.nodes[j]);
                assert!(
                    !ri.overlaps(&rj),
                    "nodes {} and {} overlap after layout",
                    i,
                    j
                );
            }
        }
    }

    #[test]
    fn convergence_exits_early() {
        let mut g = ForceGraph::new();
        g.add_node(ForceNode::new(0).with_position(-25.0, 0.0));
        g.add_node(ForceNode::new(1).with_position(25.0, 0.0));
        g.add_edge(0, 1);

        // Should converge well before 2500 iterations
        let cfg = ForceConfig {
            max_iterations: 2500,
            ..ForceConfig::default()
        };
        layout(&mut g, &cfg);

        for n in &g.nodes {
            assert!(n.x.is_finite());
            assert!(n.y.is_finite());
        }
    }

    // ── Grid + scaling tests ──

    #[test]
    fn grid_matches_brute_force_when_all_nearby() {
        // Cell size large enough that all nodes fit in one 3×3 neighborhood
        let make = || {
            let mut g = ForceGraph::new();
            for i in 0..10 {
                g.add_node(
                    ForceNode::new(i)
                        .with_size(60.0, 30.0)
                        .with_position(i as f64 * 20.0, 0.0),
                );
            }
            g
        };

        let cfg = ForceConfig::default();
        let huge_cell = 10000.0; // one cell covers everything

        let mut g_brute = make();
        apply_repulsion_brute(&mut g_brute, &cfg);

        let mut g_grid = make();
        let grid = SpatialGrid::build(&g_grid.nodes, huge_cell);
        apply_repulsion_grid(&mut g_grid, &cfg, &grid);

        for (a, b) in g_brute.nodes.iter().zip(g_grid.nodes.iter()) {
            assert!(
                (a.fx - b.fx).abs() < 1e-6,
                "node {} fx: brute={} grid={}",
                a.id,
                a.fx,
                b.fx,
            );
        }
    }

    #[test]
    fn grid_layout_produces_valid_result() {
        // Force the grid path and verify behavioral correctness
        let mut g = ForceGraph::new();
        let n = 60; // above GRID_NODE_THRESHOLD
        for i in 0..n {
            g.add_node(ForceNode::new(i).with_size(50.0, 25.0));
        }
        for i in 0..n - 1 {
            g.add_edge(i, i + 1);
        }
        layout(&mut g, &ForceConfig::default());

        for node in &g.nodes {
            assert!(node.x.is_finite(), "node {} x not finite", node.id);
            assert!(node.y.is_finite(), "node {} y not finite", node.id);
        }
    }

    // ── Cooling schedule + adaptation tests ──

    #[test]
    fn cooling_starts_at_one_ends_at_final_temp() {
        let max_cycle = 25;
        let final_temp = 0.04;
        let exp = cooling_exponent(max_cycle, final_temp);

        let first = compute_cooling(0, 1.0, exp, max_cycle, final_temp);
        assert!(
            (first - 1.0).abs() < 1e-10,
            "cycle 0 should be 1.0, got {first}"
        );

        let last = compute_cooling(max_cycle, 1.0, exp, max_cycle, final_temp);
        assert!(
            (last - final_temp).abs() < 0.01,
            "cycle {max_cycle} should be ~{final_temp}, got {last}"
        );
    }

    #[test]
    fn cooling_decays_monotonically() {
        let max_cycle = 25;
        let final_temp = 0.04;
        let exp = cooling_exponent(max_cycle, final_temp);

        let mut prev = 2.0;
        for cycle in 0..=max_cycle {
            let c = compute_cooling(cycle, 1.0, exp, max_cycle, final_temp);
            assert!(
                c <= prev,
                "cooling must decay: cycle {cycle} ({c}) > prev ({prev})"
            );
            assert!(c >= final_temp, "cooling must stay >= final_temp");
            assert!(c <= 1.0, "cooling must stay <= 1.0");
            prev = c;
        }
    }

    #[test]
    fn cooling_never_below_final_temp() {
        let max_cycle = 25;
        let final_temp = 0.04;
        let exp = cooling_exponent(max_cycle, final_temp);

        // Even with huge cycle overshoot
        for cycle in 0..100 {
            let c = compute_cooling(cycle, 1.0, exp, max_cycle, final_temp);
            assert!(c >= final_temp, "cycle {cycle}: {c} < {final_temp}");
        }
    }

    #[test]
    fn adaptation_reaches_final_temp_sooner() {
        let max_cycle = 25;
        let final_temp = 0.04;
        let exp = cooling_exponent(max_cycle, final_temp);

        // Find first cycle where cooling <= final_temp + 0.01 (effectively converged)
        let near_final = |adapt: f64| -> usize {
            (0..=max_cycle)
                .find(|&c| {
                    compute_cooling(c, adapt, exp, max_cycle, final_temp) < final_temp + 0.01
                })
                .unwrap_or(max_cycle)
        };

        let normal = near_final(1.0);
        let adapted = near_final(3.0);
        assert!(
            adapted < normal,
            "adapt=3.0 should converge sooner: adapted={adapted} >= normal={normal}"
        );
    }

    #[test]
    fn adaptation_preserves_bounds() {
        let max_cycle = 25;
        let final_temp = 0.04;
        let exp = cooling_exponent(max_cycle, final_temp);

        for adapt in [1.0, 2.0, 3.0] {
            for cycle in 0..=max_cycle {
                let c = compute_cooling(cycle, adapt, exp, max_cycle, final_temp);
                assert!(c >= final_temp, "adapt={adapt} cycle={cycle}: {c} < final");
                assert!(c <= 1.0, "adapt={adapt} cycle={cycle}: {c} > 1.0");
            }
        }
    }

    #[test]
    fn adaptation_factor_thresholds() {
        assert!((adaptation_factor(0) - 1.0).abs() < f64::EPSILON);
        assert!((adaptation_factor(999) - 1.0).abs() < f64::EPSILON);
        assert!((adaptation_factor(1000) - 1.0).abs() < f64::EPSILON);
        assert!(adaptation_factor(3000) > 1.0);
        assert!(adaptation_factor(3000) < 3.0);
        assert!((adaptation_factor(5000) - 3.0).abs() < f64::EPSILON);
        assert!((adaptation_factor(10000) - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn adapted_layout_preserves_invariants() {
        // 200-node graph: uses grid + mild adaptation path
        let mut g = ForceGraph::new();
        let n = 200;
        for i in 0..n {
            g.add_node(ForceNode::new(i).with_size(40.0, 20.0));
        }
        // Tree: 0→1..9, 1→10..19, etc.
        for i in 0..n {
            for c in 0..3 {
                let child = i * 3 + c + 1;
                if child < n {
                    g.add_edge(i, child);
                }
            }
        }

        let cfg = ForceConfig {
            max_iterations: 500,
            ..ForceConfig::default()
        };
        layout(&mut g, &cfg);

        // All finite
        for node in &g.nodes {
            assert!(node.x.is_finite(), "node {} x not finite", node.id);
            assert!(node.y.is_finite(), "node {} y not finite", node.id);
        }

        // Connected pairs should be closer on average than random pairs
        let mut connected_sum = 0.0;
        let mut connected_count = 0;
        for edge in &g.edges {
            let dx = g.nodes[edge.target].x - g.nodes[edge.source].x;
            let dy = g.nodes[edge.target].y - g.nodes[edge.source].y;
            connected_sum += (dx * dx + dy * dy).sqrt();
            connected_count += 1;
        }
        let avg_connected = connected_sum / connected_count as f64;

        // Sample some non-connected pairs
        let mut random_sum = 0.0;
        let pairs = [(0, n / 2), (1, n - 1), (n / 4, 3 * n / 4), (2, n - 2)];
        for &(a, b) in &pairs {
            let dx = g.nodes[b].x - g.nodes[a].x;
            let dy = g.nodes[b].y - g.nodes[a].y;
            random_sum += (dx * dx + dy * dy).sqrt();
        }
        let avg_random = random_sum / pairs.len() as f64;

        assert!(
            avg_connected < avg_random,
            "connected avg ({avg_connected:.1}) should be < random avg ({avg_random:.1})"
        );
    }

    // ── Property tests ──

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn all_coordinates_finite(n in 2usize..20, edges in 1usize..30) {
            let mut g = ForceGraph::new();
            for i in 0..n {
                g.add_node(ForceNode::new(i));
            }
            for e in 0..edges {
                let s = e % n;
                let t = (e * 7 + 3) % n;
                if s != t { g.add_edge(s, t); }
            }
            layout(&mut g, &ForceConfig::default());
            for node in &g.nodes {
                prop_assert!(node.x.is_finite(), "node {} x not finite", node.id);
                prop_assert!(node.y.is_finite(), "node {} y not finite", node.id);
            }
        }

        #[test]
        fn cooling_bounded_across_params(
            max_cycle in 1usize..50,
            adapt in 1.0f64..4.0,
        ) {
            let final_temp = 0.04;
            let exp = cooling_exponent(max_cycle, final_temp);

            let mut prev = 2.0;
            for cycle in 0..=max_cycle {
                let c = compute_cooling(cycle, adapt, exp, max_cycle, final_temp);
                prop_assert!(c >= final_temp, "below final: cycle={cycle} c={c}");
                prop_assert!(c <= 1.0, "above 1.0: cycle={cycle} c={c}");
                prop_assert!(c <= prev + 1e-10, "non-monotonic: cycle={cycle}");
                prev = c;
            }
        }

        #[test]
        fn sized_nodes_stay_finite(
            n in 2usize..15,
            w in 20.0f64..120.0,
            h in 15.0f64..60.0,
        ) {
            let mut g = ForceGraph::new();
            for i in 0..n {
                g.add_node(ForceNode::new(i).with_size(w, h));
            }
            for i in 0..n - 1 { g.add_edge(i, i + 1); }
            layout(&mut g, &ForceConfig::tree());
            for node in &g.nodes {
                prop_assert!(node.x.is_finite(), "node {} x not finite", node.id);
                prop_assert!(node.y.is_finite(), "node {} y not finite", node.id);
            }
        }
    }
}
