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
