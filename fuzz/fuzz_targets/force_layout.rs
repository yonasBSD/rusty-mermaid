#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use rusty_mermaid_core::force_layout::{ForceConfig, ForceGraph, ForceNode};

#[derive(Arbitrary, Debug)]
struct FuzzInput {
    num_nodes: u8,
    edges: Vec<(u8, u8)>,
    repulsion: u16,
    attraction: u16,
}

fuzz_target!(|input: FuzzInput| {
    let n = (input.num_nodes % 30) as usize;
    if n == 0 { return; }

    let mut g = ForceGraph::new();
    for i in 0..n {
        g.add_node(ForceNode::new(i));
    }
    for &(s, t) in &input.edges {
        let s = (s as usize) % n;
        let t = (t as usize) % n;
        if s != t {
            g.add_edge(s, t);
        }
    }

    let config = ForceConfig {
        iterations: 50, // fewer for fuzzing speed
        repulsion: (input.repulsion % 10000) as f64 + 1.0,
        attraction: (input.attraction % 100) as f64 / 1000.0 + 0.001,
        ..ForceConfig::default()
    };

    rusty_mermaid_core::force_layout::layout(&mut g, &config);

    // All coordinates must be finite
    for node in &g.nodes {
        assert!(node.x.is_finite(), "node {} x not finite", node.id);
        assert!(node.y.is_finite(), "node {} y not finite", node.id);
    }
});
