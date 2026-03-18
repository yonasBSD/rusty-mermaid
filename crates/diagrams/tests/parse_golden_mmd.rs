use std::fs;
use std::path::Path;

use rusty_mermaid_diagrams::flowchart::parser as flowchart_parser;
use rusty_mermaid_diagrams::state::parser as state_parser;

fn golden_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/golden/mmd")
}

macro_rules! parse_flowchart {
    ($name:ident) => {
        #[test]
        fn $name() {
            let path = golden_dir().join(concat!(stringify!($name), ".mmd"));
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
            let diagram = flowchart_parser::parse(&text)
                .unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e));
            assert!(!diagram.vertices.is_empty(), "{} should have vertices", stringify!($name));
        }
    };
}

macro_rules! parse_state {
    ($name:ident) => {
        #[test]
        fn $name() {
            let path = golden_dir().join(concat!(stringify!($name), ".mmd"));
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
            let diagram = state_parser::parse(&text)
                .unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e));
            assert!(!diagram.states.is_empty(), "{} should have states", stringify!($name));
        }
    };
}

parse_flowchart!(single_node);
parse_flowchart!(linear_3);
parse_flowchart!(diamond);
parse_flowchart!(linear_lr);
parse_flowchart!(linear_bt);
parse_flowchart!(linear_rl);
parse_flowchart!(cycle_3);
parse_flowchart!(long_edge);
parse_flowchart!(crossing);
parse_flowchart!(mixed_sizes);
parse_flowchart!(disconnected);
parse_flowchart!(edge_label);
parse_flowchart!(self_loop);
parse_flowchart!(weighted);
parse_flowchart!(minlen);
parse_flowchart!(html_labels);
parse_flowchart!(realistic_flowchart);
parse_flowchart!(compound_simple);
parse_flowchart!(nested_compound);
parse_flowchart!(compiler_pipeline);
parse_flowchart!(mcp_server);

// Styling combinations
parse_flowchart!(style_combined);
parse_flowchart!(style_multiple_classes);
parse_flowchart!(style_default_override);
parse_flowchart!(style_linkstyle_all);
parse_flowchart!(style_opacity);

// Text variations
parse_flowchart!(text_html_tags);
parse_flowchart!(text_very_long);
parse_flowchart!(text_single_char);
parse_flowchart!(text_special_chars);
parse_flowchart!(text_unicode_emoji);

// Layout stress tests
parse_flowchart!(layout_two_nodes_no_edge);
parse_flowchart!(layout_binary_tree);
parse_flowchart!(layout_parallel_chains);
parse_flowchart!(layout_deep_nesting);
parse_flowchart!(layout_cross_subgraph_edges);

// Combined features
parse_flowchart!(combo_styled_shapes);
parse_flowchart!(combo_all_arrows_labeled);
parse_flowchart!(combo_subgraph_styled);

parse_state!(state_simple);
parse_state!(state_fork_join);
parse_state!(state_composite);
parse_state!(state_choice);
