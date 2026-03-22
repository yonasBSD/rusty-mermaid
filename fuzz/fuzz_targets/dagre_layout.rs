#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use rusty_mermaid_core::Direction;
use rusty_mermaid_dagre::DagreConfig;
use rusty_mermaid_graph::Graph;

#[derive(Arbitrary, Debug)]
struct FuzzGraph {
    num_nodes: u8,
    edges: Vec<(u8, u8)>,
    rankdir: u8,
    nodesep: u16,
    ranksep: u16,
}

fuzz_target!(|input: FuzzGraph| {
    let num_nodes = (input.num_nodes % 32) as usize;
    if num_nodes == 0 { return; }

    let mut g: Graph<rusty_mermaid_dagre::NodeLabel, rusty_mermaid_dagre::EdgeLabel> = Graph::new();
    let mut ids = Vec::with_capacity(num_nodes);

    for _ in 0..num_nodes {
        ids.push(g.add_node(rusty_mermaid_dagre::NodeLabel::new(40.0, 20.0)));
    }

    for &(src, dst) in &input.edges {
        let s = (src as usize) % num_nodes;
        let d = (dst as usize) % num_nodes;
        g.add_edge(ids[s], ids[d], rusty_mermaid_dagre::EdgeLabel::new());
    }

    let config = DagreConfig {
        rankdir: match input.rankdir % 4 {
            0 => Direction::TB,
            1 => Direction::BT,
            2 => Direction::LR,
            _ => Direction::RL,
        },
        nodesep: (input.nodesep % 200) as f64 + 1.0,
        ranksep: (input.ranksep % 200) as f64 + 1.0,
        ..Default::default()
    };

    rusty_mermaid_dagre::pipeline::layout(&mut g, &config);
});
