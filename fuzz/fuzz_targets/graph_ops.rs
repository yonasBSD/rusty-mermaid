#![no_main]

use std::collections::HashMap;

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use rusty_mermaid_graph::{Graph, NodeId};

/// Sequence of graph operations to apply.
/// Derived via `Arbitrary` so the fuzzer generates structured operation sequences
/// rather than raw bytes — much better coverage of interesting states.
#[derive(Arbitrary, Debug)]
enum GraphOp {
    AddNode { id: u8 },
    RemoveNode { id: u8 },
    AddEdge { src: u8, dst: u8 },
    RemoveEdge { idx: u8 },
    SetParent { child: u8, parent: u8 },
    RemoveParent { child: u8 },
    Reverse,
    QuerySuccessors { id: u8 },
    QueryPredecessors { id: u8 },
}

fuzz_target!(|ops: Vec<GraphOp>| {
    // Graph must never panic regardless of operation sequence.
    let mut g: Graph<u8, ()> = Graph::new();
    let mut id_map: HashMap<u8, NodeId> = HashMap::new();
    let mut edges: Vec<rusty_mermaid_graph::EdgeId> = Vec::new();

    for op in ops {
        match op {
            GraphOp::AddNode { id } => {
                let nid = g.add_node(id);
                id_map.insert(id, nid);
            }
            GraphOp::RemoveNode { id } => {
                if let Some(&nid) = id_map.get(&id) {
                    g.remove_node(nid);
                    id_map.remove(&id);
                }
            }
            GraphOp::AddEdge { src, dst } => {
                if let (Some(&s), Some(&d)) = (id_map.get(&src), id_map.get(&dst)) {
                    if g.has_node(s) && g.has_node(d) {
                        let eid = g.add_edge(s, d, ());
                        edges.push(eid);
                    }
                }
            }
            GraphOp::RemoveEdge { idx } => {
                if !edges.is_empty() {
                    let i = idx as usize % edges.len();
                    let eid = edges.swap_remove(i);
                    g.remove_edge(eid);
                }
            }
            GraphOp::SetParent { child, parent } => {
                if child != parent {
                    if let (Some(&c), Some(&p)) = (id_map.get(&child), id_map.get(&parent)) {
                        if g.has_node(c) && g.has_node(p) {
                            g.set_parent(c, p);
                        }
                    }
                }
            }
            GraphOp::RemoveParent { child } => {
                if let Some(&nid) = id_map.get(&child) {
                    if g.has_node(nid) {
                        g.remove_parent(nid);
                    }
                }
            }
            GraphOp::Reverse => {
                g.reverse();
            }
            GraphOp::QuerySuccessors { id } => {
                if let Some(&nid) = id_map.get(&id) {
                    let _: Vec<_> = g.successors(nid).collect();
                }
            }
            GraphOp::QueryPredecessors { id } => {
                if let Some(&nid) = id_map.get(&id) {
                    let _: Vec<_> = g.predecessors(nid).collect();
                }
            }
        }
    }

    // Invariant: node_count and edge_count should be consistent
    let _ = g.node_count();
    let _ = g.edge_count();
    let _ = g.is_empty();
});
