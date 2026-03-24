//! Force-directed graph layout via physics simulation.
//!
//! Coulomb repulsion + Hooke spring attraction + center gravity.
//! General-purpose: works for trees, cyclic graphs, disconnected components.
//! Deterministic via position seeding from node index.

use std::collections::HashMap;

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
    // Internal: accumulated force per iteration
    fx: f64,
    fy: f64,
}

impl ForceNode {
    pub fn new(id: usize) -> Self {
        Self {
            id, x: 0.0, y: 0.0, width: 40.0, height: 40.0,
            fixed: false, fx: 0.0, fy: 0.0,
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

/// Configuration for the force simulation.
#[derive(Debug, Clone)]
pub struct ForceConfig {
    /// Number of simulation iterations.
    pub iterations: usize,
    /// Repulsion strength (Coulomb constant). Higher = more spread.
    pub repulsion: f64,
    /// Spring attraction strength. Higher = tighter edges.
    pub attraction: f64,
    /// Ideal spring length (rest distance for edges).
    pub ideal_length: f64,
    /// Gravity toward center. Prevents drift.
    pub gravity: f64,
    /// Minimum distance between nodes to avoid division by zero.
    pub min_distance: f64,
    /// Starting temperature (max displacement per step).
    pub initial_temp: f64,
    /// Temperature decay rate per iteration (0 < rate < 1).
    pub cooling_rate: f64,
}

impl Default for ForceConfig {
    fn default() -> Self {
        Self {
            iterations: 200,
            repulsion: 5000.0,
            attraction: 0.01,
            ideal_length: 80.0,
            gravity: 0.02,
            min_distance: 1.0,
            initial_temp: 100.0,
            cooling_rate: 0.97,
        }
    }
}

impl ForceConfig {
    /// Preset tuned for tree/mindmap layouts: stronger gravity, moderate repulsion.
    pub fn tree() -> Self {
        Self {
            iterations: 150,
            repulsion: 4000.0,
            attraction: 0.015,
            ideal_length: 100.0,
            gravity: 0.04,
            min_distance: 1.0,
            initial_temp: 80.0,
            cooling_rate: 0.96,
        }
    }
}

/// The force-directed graph: nodes + edges.
#[derive(Debug, Clone)]
pub struct ForceGraph {
    pub nodes: Vec<ForceNode>,
    pub edges: Vec<ForceEdge>,
}

impl ForceGraph {
    pub fn new() -> Self {
        Self { nodes: Vec::new(), edges: Vec::new() }
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

/// Run the force-directed layout simulation.
///
/// Modifies node positions in-place. After return, `graph.nodes[i].x/y`
/// contain the final positions.
pub fn layout(graph: &mut ForceGraph, config: &ForceConfig) {
    let n = graph.nodes.len();
    if n == 0 { return; }

    // Seed initial positions if all at origin
    if graph.nodes.iter().all(|n| n.x == 0.0 && n.y == 0.0) {
        seed_positions(graph);
    }

    let mut temp = config.initial_temp;

    for _iter in 0..config.iterations {
        // Reset forces
        for node in &mut graph.nodes {
            node.fx = 0.0;
            node.fy = 0.0;
        }

        // All-pairs repulsion (Coulomb)
        apply_repulsion(graph, config);

        // Edge spring attraction (Hooke)
        apply_attraction(graph, config);

        // Center gravity
        apply_gravity(graph, config);

        // Apply forces with temperature-limited displacement
        apply_forces(graph, temp, config.min_distance);

        // Cool
        temp *= config.cooling_rate;
    }
}

/// Seed positions in a circular arrangement.
fn seed_positions(graph: &mut ForceGraph) {
    let n = graph.nodes.len();
    if n == 1 {
        graph.nodes[0].x = 0.0;
        graph.nodes[0].y = 0.0;
        return;
    }
    let radius = (n as f64 * 20.0).max(50.0);
    for (i, node) in graph.nodes.iter_mut().enumerate() {
        let angle = std::f64::consts::TAU * i as f64 / n as f64;
        node.x = radius * angle.cos();
        node.y = radius * angle.sin();
    }
}

/// All-pairs repulsion: each node repels every other node.
fn apply_repulsion(graph: &mut ForceGraph, config: &ForceConfig) {
    let n = graph.nodes.len();
    for i in 0..n {
        for j in (i + 1)..n {
            let dx = graph.nodes[j].x - graph.nodes[i].x;
            let dy = graph.nodes[j].y - graph.nodes[i].y;
            let dist_sq = (dx * dx + dy * dy).max(config.min_distance * config.min_distance);
            let dist = dist_sq.sqrt();

            // Coulomb force: F = repulsion / dist²
            let force = config.repulsion / dist_sq;
            let fx = force * dx / dist;
            let fy = force * dy / dist;

            graph.nodes[i].fx -= fx;
            graph.nodes[i].fy -= fy;
            graph.nodes[j].fx += fx;
            graph.nodes[j].fy += fy;
        }
    }
}

/// Edge spring attraction: connected nodes attract.
fn apply_attraction(graph: &mut ForceGraph, config: &ForceConfig) {
    for edge in &graph.edges {
        let (s, t) = (edge.source, edge.target);
        if s >= graph.nodes.len() || t >= graph.nodes.len() { continue; }

        let dx = graph.nodes[t].x - graph.nodes[s].x;
        let dy = graph.nodes[t].y - graph.nodes[s].y;
        let dist = (dx * dx + dy * dy).sqrt().max(config.min_distance);

        // Hooke's law: F = attraction × (dist - ideal_length)
        let force = config.attraction * (dist - config.ideal_length);
        let fx = force * dx / dist;
        let fy = force * dy / dist;

        graph.nodes[s].fx += fx;
        graph.nodes[s].fy += fy;
        graph.nodes[t].fx -= fx;
        graph.nodes[t].fy -= fy;
    }
}

/// Center gravity: all nodes pulled toward (0, 0).
fn apply_gravity(graph: &mut ForceGraph, config: &ForceConfig) {
    for node in &mut graph.nodes {
        node.fx -= config.gravity * node.x;
        node.fy -= config.gravity * node.y;
    }
}

/// Apply accumulated forces, limiting displacement by temperature.
fn apply_forces(graph: &mut ForceGraph, temp: f64, min_dist: f64) {
    for node in &mut graph.nodes {
        if node.fixed { continue; }

        let force_mag = (node.fx * node.fx + node.fy * node.fy).sqrt().max(min_dist);
        // Clamp displacement to temperature
        let scale = temp.min(force_mag) / force_mag;
        node.x += node.fx * scale;
        node.y += node.fy * scale;

        // Ensure coordinates stay finite
        if !node.x.is_finite() { node.x = 0.0; }
        if !node.y.is_finite() { node.y = 0.0; }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_graph_is_noop() {
        let mut g = ForceGraph::new();
        layout(&mut g, &ForceConfig::default());
        assert!(g.nodes.is_empty());
    }

    #[test]
    fn single_node_stays_at_center() {
        let mut g = ForceGraph::new();
        g.add_node(ForceNode::new(0));
        layout(&mut g, &ForceConfig::default());
        assert!((g.nodes[0].x).abs() < 1.0);
        assert!((g.nodes[0].y).abs() < 1.0);
    }

    #[test]
    fn two_connected_nodes_reach_equilibrium() {
        let mut g = ForceGraph::new();
        g.add_node(ForceNode::new(0).with_position(-50.0, 0.0));
        g.add_node(ForceNode::new(1).with_position(50.0, 0.0));
        g.add_edge(0, 1);
        layout(&mut g, &ForceConfig::default());

        let dist = ((g.nodes[1].x - g.nodes[0].x).powi(2) + (g.nodes[1].y - g.nodes[0].y).powi(2)).sqrt();
        // Should be near ideal_length (80)
        assert!(dist > 30.0, "connected nodes too close: {dist}");
        assert!(dist < 200.0, "connected nodes too far: {dist}");
    }

    #[test]
    fn unconnected_nodes_repel() {
        let mut g = ForceGraph::new();
        g.add_node(ForceNode::new(0).with_position(0.0, 0.0));
        g.add_node(ForceNode::new(1).with_position(1.0, 0.0));
        // No edge — only repulsion
        layout(&mut g, &ForceConfig::default());

        let dist = ((g.nodes[1].x - g.nodes[0].x).powi(2) + (g.nodes[1].y - g.nodes[0].y).powi(2)).sqrt();
        assert!(dist > 10.0, "unconnected nodes should repel: dist={dist}");
    }

    #[test]
    fn triangle_converges() {
        let mut g = ForceGraph::new();
        g.add_node(ForceNode::new(0));
        g.add_node(ForceNode::new(1));
        g.add_node(ForceNode::new(2));
        g.add_edge(0, 1);
        g.add_edge(1, 2);
        g.add_edge(2, 0);
        layout(&mut g, &ForceConfig::default());

        // All nodes should have finite positions
        for n in &g.nodes {
            assert!(n.x.is_finite(), "node {} x is not finite", n.id);
            assert!(n.y.is_finite(), "node {} y is not finite", n.id);
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
    fn disconnected_components_dont_overlap() {
        let mut g = ForceGraph::new();
        // Component 1: 0-1-2
        g.add_node(ForceNode::new(0));
        g.add_node(ForceNode::new(1));
        g.add_node(ForceNode::new(2));
        g.add_edge(0, 1);
        g.add_edge(1, 2);
        // Component 2: 3-4
        g.add_node(ForceNode::new(3));
        g.add_node(ForceNode::new(4));
        g.add_edge(3, 4);

        layout(&mut g, &ForceConfig::default());

        // Centers of components should be separated
        let c1_x = (g.nodes[0].x + g.nodes[1].x + g.nodes[2].x) / 3.0;
        let c1_y = (g.nodes[0].y + g.nodes[1].y + g.nodes[2].y) / 3.0;
        let c2_x = (g.nodes[3].x + g.nodes[4].x) / 2.0;
        let c2_y = (g.nodes[3].y + g.nodes[4].y) / 2.0;
        let comp_dist = ((c2_x - c1_x).powi(2) + (c2_y - c1_y).powi(2)).sqrt();
        assert!(comp_dist > 10.0, "components should separate: dist={comp_dist}");
    }

    #[test]
    fn large_graph_converges_without_nan() {
        let mut g = ForceGraph::new();
        let n = 50;
        for i in 0..n {
            g.add_node(ForceNode::new(i));
        }
        // Chain: 0-1-2-...-49
        for i in 0..n - 1 {
            g.add_edge(i, i + 1);
        }
        layout(&mut g, &ForceConfig::default());

        for node in &g.nodes {
            assert!(node.x.is_finite(), "node {} x is NaN/Inf", node.id);
            assert!(node.y.is_finite(), "node {} y is NaN/Inf", node.id);
        }
    }

    #[test]
    fn tree_config_converges() {
        let mut g = ForceGraph::new();
        // Star: root 0 connected to 1,2,3,4
        g.add_node(ForceNode::new(0));
        for i in 1..5 {
            g.add_node(ForceNode::new(i));
            g.add_edge(0, i);
        }
        layout(&mut g, &ForceConfig::tree());

        for node in &g.nodes {
            assert!(node.x.is_finite());
            assert!(node.y.is_finite());
        }
        // Children should be spread around root
        let root = &g.nodes[0];
        for child in &g.nodes[1..] {
            let dist = ((child.x - root.x).powi(2) + (child.y - root.y).powi(2)).sqrt();
            assert!(dist > 20.0, "child {} too close to root: {dist}", child.id);
        }
    }

    #[test]
    fn deterministic_same_input_same_output() {
        let make_graph = || {
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

        let mut g1 = make_graph();
        let mut g2 = make_graph();
        let config = ForceConfig::default();
        layout(&mut g1, &config);
        layout(&mut g2, &config);

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
        g.add_edge(0, 1); // 0-1 connected, 2 isolated

        layout(&mut g, &ForceConfig::default());

        let d01 = ((g.nodes[1].x - g.nodes[0].x).powi(2) + (g.nodes[1].y - g.nodes[0].y).powi(2)).sqrt();
        let d02 = ((g.nodes[2].x - g.nodes[0].x).powi(2) + (g.nodes[2].y - g.nodes[0].y).powi(2)).sqrt();
        assert!(d01 < d02, "connected pair (d={d01}) should be closer than unconnected (d={d02})");
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
                if s != t {
                    g.add_edge(s, t);
                }
            }
            layout(&mut g, &ForceConfig::default());
            for node in &g.nodes {
                prop_assert!(node.x.is_finite(), "node {} x not finite", node.id);
                prop_assert!(node.y.is_finite(), "node {} y not finite", node.id);
            }
        }
    }
}
