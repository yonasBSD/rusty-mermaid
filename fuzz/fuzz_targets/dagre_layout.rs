#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

/// A structured graph input for layout fuzzing.
/// Using `Arbitrary` lets the fuzzer explore meaningful graph topologies
/// (varying node counts, edge patterns, configs) instead of random bytes.
#[derive(Arbitrary, Debug)]
struct FuzzGraph {
    num_nodes: u8,
    edges: Vec<(u8, u8)>,
    rankdir: u8,
    nodesep: u16,
    ranksep: u16,
}

fuzz_target!(|input: FuzzGraph| {
    // Layout must never panic, even on degenerate inputs:
    // empty graph, self-loops, disconnected components, huge fan-out, etc.
    //
    // TODO: uncomment when dagre crate is implemented
    // let num_nodes = (input.num_nodes % 64) as usize; // cap at 64 nodes
    // let mut g = build_fuzz_graph(num_nodes, &input.edges);
    // let config = DagreConfig {
    //     rankdir: match input.rankdir % 4 { 0 => TB, 1 => BT, 2 => LR, _ => RL },
    //     nodesep: (input.nodesep % 200) as f64 + 1.0,
    //     ranksep: (input.ranksep % 200) as f64 + 1.0,
    //     ..Default::default()
    // };
    // let _ = dagre::layout(&mut g, &config, &SimpleTextMeasure);
    let _ = input;
});
