use std::fs;
use std::path::Path;

use rusty_mermaid_diagrams::flowchart;
use rusty_mermaid_diagrams::state;

fn golden_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/golden/mmd")
}

/// Flowchart golden layout tests: parse → bridge → layout → verify non-empty.
macro_rules! flowchart_layout {
    ($name:ident) => {
        #[test]
        fn $name() {
            let path = golden_dir().join(concat!(stringify!($name), ".mmd"));
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
            let diagram = flowchart::parser::parse(&text)
                .unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e));
            let result = flowchart::bridge::layout(&diagram);
            assert!(
                !result.nodes.is_empty(),
                "{} layout should produce nodes",
                stringify!($name)
            );
            for e in &result.edges {
                assert!(
                    !e.points.is_empty(),
                    "{} edge {}->{} should have points",
                    stringify!($name),
                    e.src,
                    e.dst
                );
            }
        }
    };
}

flowchart_layout!(single_node);
flowchart_layout!(linear_3);
flowchart_layout!(diamond);
flowchart_layout!(linear_lr);
flowchart_layout!(linear_bt);
flowchart_layout!(linear_rl);
flowchart_layout!(cycle_3);
flowchart_layout!(long_edge);
flowchart_layout!(crossing);
flowchart_layout!(mixed_sizes);
flowchart_layout!(disconnected);
flowchart_layout!(edge_label);
flowchart_layout!(self_loop);
flowchart_layout!(weighted);
flowchart_layout!(minlen);
flowchart_layout!(html_labels);
flowchart_layout!(realistic_flowchart);
flowchart_layout!(compound_simple);
flowchart_layout!(nested_compound);
flowchart_layout!(compiler_pipeline);
flowchart_layout!(mcp_server);

/// State diagram golden layout tests: parse → bridge → layout → verify non-empty.
macro_rules! state_layout {
    ($name:ident) => {
        #[test]
        fn $name() {
            let path = golden_dir().join(concat!(stringify!($name), ".mmd"));
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
            let diagram = state::parser::parse(&text)
                .unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e));
            let result = state::bridge::layout(&diagram);
            assert!(
                !result.nodes.is_empty(),
                "{} layout should produce nodes",
                stringify!($name)
            );
            for e in &result.edges {
                assert!(
                    !e.points.is_empty(),
                    "{} edge {}->{} should have points",
                    stringify!($name),
                    e.src,
                    e.dst
                );
            }
        }
    };
}

state_layout!(state_simple);
state_layout!(state_fork_join);
state_layout!(state_composite);
state_layout!(state_choice);

/// Helper: assert every node declared in a subgraph is geometrically inside
/// that subgraph's bounding box. Uses the IR to know which nodes belong to
/// which subgraph (first-wins, matching bridge.rs semantics).
fn assert_subgraph_containment(
    name: &str,
    diagram: &flowchart::ir::FlowDiagram,
    result: &flowchart::bridge::LayoutResult,
) {
    use std::collections::HashSet;

    // Build first-wins membership: node_id → first subgraph that claims it
    let mut claimed: HashSet<&str> = HashSet::new();
    let mut membership: Vec<(&str, &str)> = Vec::new(); // (node_id, sg_id)
    for sg in &diagram.subgraphs {
        for nid in &sg.node_ids {
            if claimed.insert(nid.as_str()) {
                membership.push((nid.as_str(), sg.id.as_str()));
            }
        }
    }

    for (node_id, sg_id) in &membership {
        let Some(sg) = result.subgraphs.iter().find(|s| s.id == *sg_id) else {
            continue;
        };
        let Some(node) = result.nodes.iter().find(|n| n.id == *node_id) else {
            continue;
        };

        let sg_left = sg.x - sg.width / 2.0;
        let sg_right = sg.x + sg.width / 2.0;
        let sg_top = sg.y - sg.height / 2.0;
        let sg_bottom = sg.y + sg.height / 2.0;
        let n_left = node.x - node.width / 2.0;
        let n_right = node.x + node.width / 2.0;
        let n_top = node.y - node.height / 2.0;
        let n_bottom = node.y + node.height / 2.0;

        assert!(
            sg_left <= n_left && n_right <= sg_right
                && sg_top <= n_top && n_bottom <= sg_bottom,
            "{name}: node '{node_id}' not inside subgraph '{sg_id}'\n\
             node: [{n_left:.1}, {n_top:.1}] - [{n_right:.1}, {n_bottom:.1}]\n\
             sg:   [{sg_left:.1}, {sg_top:.1}] - [{sg_right:.1}, {sg_bottom:.1}]",
        );
    }
}

#[test]
fn mcp_server_subgraph_containment() {
    let path = golden_dir().join("mcp_server.mmd");
    let text = fs::read_to_string(&path).unwrap();
    let diagram = flowchart::parser::parse(&text).unwrap();
    let result = flowchart::bridge::layout(&diagram);
    assert_subgraph_containment("mcp_server", &diagram, &result);
}

#[test]
fn compiler_pipeline_subgraph_containment() {
    let path = golden_dir().join("compiler_pipeline.mmd");
    let text = fs::read_to_string(&path).unwrap();
    let diagram = flowchart::parser::parse(&text).unwrap();
    let result = flowchart::bridge::layout(&diagram);
    assert_subgraph_containment("compiler_pipeline", &diagram, &result);
}

#[test]
fn nested_compound_subgraph_containment() {
    let path = golden_dir().join("nested_compound.mmd");
    let text = fs::read_to_string(&path).unwrap();
    let diagram = flowchart::parser::parse(&text).unwrap();
    let result = flowchart::bridge::layout(&diagram);
    assert_subgraph_containment("nested_compound", &diagram, &result);
}

#[test]
fn compound_simple_subgraph_containment() {
    let path = golden_dir().join("compound_simple.mmd");
    let text = fs::read_to_string(&path).unwrap();
    let diagram = flowchart::parser::parse(&text).unwrap();
    let result = flowchart::bridge::layout(&diagram);
    assert_subgraph_containment("compound_simple", &diagram, &result);
}
