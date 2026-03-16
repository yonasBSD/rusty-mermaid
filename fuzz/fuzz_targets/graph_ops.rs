#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

/// Sequence of graph operations to apply.
/// Derived via `Arbitrary` so the fuzzer generates structured operation sequences
/// rather than raw bytes — much better coverage of interesting states.
#[derive(Arbitrary, Debug)]
enum GraphOp {
    AddNode { id: u8 },
    RemoveNode { id: u8 },
    AddEdge { src: u8, dst: u8 },
    RemoveEdge { src: u8, dst: u8 },
    SetParent { child: u8, parent: u8 },
    RemoveParent { child: u8 },
    NodeCount,
    EdgeCount,
    Children { id: u8 },
}

fuzz_target!(|ops: Vec<GraphOp>| {
    // Graph must never panic regardless of operation sequence.
    // TODO: uncomment when graph crate is implemented
    // let mut g = rusty_mermaid_graph::Graph::new();
    // for op in ops { match op { ... } }
    let _ = ops;
});
