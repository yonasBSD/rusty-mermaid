use std::fs;
use std::path::Path;

use rusty_mermaid_diagrams::flowchart::parser as flowchart_parser;
use rusty_mermaid_diagrams::state::parser as state_parser;
use rusty_mermaid_diagrams::class::parser as class_parser;
use rusty_mermaid_diagrams::er::parser as er_parser;
use rusty_mermaid_diagrams::requirement::parser as req_parser;
use rusty_mermaid_diagrams::pie::parser as pie_parser;
use rusty_mermaid_diagrams::timeline::parser as timeline_parser;
use rusty_mermaid_diagrams::kanban::parser as kanban_parser;
use rusty_mermaid_diagrams::gantt::parser as gantt_parser;
use rusty_mermaid_diagrams::gitgraph::parser as gitgraph_parser;
use rusty_mermaid_diagrams::xychart::parser as xychart_parser;
use rusty_mermaid_diagrams::mindmap::parser as mindmap_parser;
use rusty_mermaid_diagrams::sankey::parser as sankey_parser;
use rusty_mermaid_diagrams::packet::parser as packet_parser;
use rusty_mermaid_diagrams::quadrant::parser as quadrant_parser;
use rusty_mermaid_diagrams::venn::parser as venn_parser;
use rusty_mermaid_diagrams::radar::parser as radar_parser;
use rusty_mermaid_diagrams::journey::parser as journey_parser;
use rusty_mermaid_diagrams::treeview::parser as treeview_parser;
use rusty_mermaid_diagrams::ishikawa::parser as ishikawa_parser;
use rusty_mermaid_diagrams::treemap::parser as treemap_parser;
use rusty_mermaid_diagrams::block::parser as block_parser;
use rusty_mermaid_diagrams::c4::parser as c4_parser;
use rusty_mermaid_diagrams::architecture::parser as arch_parser;

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
            let path = golden_dir().join("flowchart").join(concat!(stringify!($name), ".mmd"));
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
            let path = golden_dir().join("state").join(concat!(stringify!($name), ".mmd"));
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

macro_rules! parse_class {
    ($name:ident) => {
        #[test]
        fn $name() {
            let path = golden_dir().join("class").join(concat!(stringify!($name), ".mmd"));
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
            let diagram = class_parser::parse(&text)
                .unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e));
            assert!(!diagram.classes.is_empty(), "{} should have classes", stringify!($name));
        }
    };
}

parse_class!(class_basic);
parse_class!(class_members);
parse_class!(class_relationships);
parse_class!(class_annotations);
parse_class!(class_generics);
parse_class!(class_namespaces);
parse_class!(class_cardinality);
parse_class!(class_styling);
parse_class!(class_direction);
parse_class!(class_complex);

macro_rules! parse_er {
    ($name:ident) => {
        #[test]
        fn $name() {
            let path = golden_dir().join("er").join(concat!(stringify!($name), ".mmd"));
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
            let diagram = er_parser::parse(&text)
                .unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e));
            assert!(!diagram.entities.is_empty(), "{} should have entities", stringify!($name));
        }
    };
}

parse_er!(er_basic);
parse_er!(er_attributes);
parse_er!(er_cardinality);
parse_er!(er_non_identifying);
parse_er!(er_complex);
parse_er!(er_direction);

macro_rules! parse_requirement {
    ($name:ident) => {
        #[test]
        fn $name() {
            let path = golden_dir().join("requirement").join(concat!(stringify!($name), ".mmd"));
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
            let diagram = req_parser::parse(&text)
                .unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e));
            assert!(!diagram.requirements.is_empty() || !diagram.elements.is_empty(),
                "{} should have requirements or elements", stringify!($name));
        }
    };
}

parse_requirement!(req_basic);
parse_requirement!(req_all_types);
parse_requirement!(req_relationships);
parse_requirement!(req_nested);
parse_requirement!(req_direction);

// --- Pie ---

macro_rules! parse_pie {
    ($name:ident) => {
        #[test]
        fn $name() {
            let path = golden_dir().join("pie").join(concat!(stringify!($name), ".mmd"));
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
            pie_parser::parse(&text)
                .unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e));
        }
    };
}

parse_pie!(pie_basic);
parse_pie!(pie_show_data);
parse_pie!(pie_simple);
parse_pie!(pie_many_slices);
parse_pie!(pie_single_slice);

// --- Timeline ---

macro_rules! parse_timeline {
    ($name:ident) => {
        #[test]
        fn $name() {
            let path = golden_dir().join("timeline").join(concat!(stringify!($name), ".mmd"));
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
            timeline_parser::parse(&text)
                .unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e));
        }
    };
}

parse_timeline!(timeline_basic);
parse_timeline!(timeline_sections);
parse_timeline!(timeline_simple);
parse_timeline!(timeline_tb);

// --- Kanban ---

macro_rules! parse_kanban {
    ($name:ident) => {
        #[test]
        fn $name() {
            let path = golden_dir().join("kanban").join(concat!(stringify!($name), ".mmd"));
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
            kanban_parser::parse(&text)
                .unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e));
        }
    };
}

parse_kanban!(kanban_basic);
parse_kanban!(kanban_metadata);
parse_kanban!(kanban_simple);
parse_kanban!(kanban_many_columns);
parse_kanban!(kanban_single_column);

// --- Gantt ---

macro_rules! parse_gantt {
    ($name:ident) => {
        #[test]
        fn $name() {
            let path = golden_dir().join("gantt").join(concat!(stringify!($name), ".mmd"));
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
            gantt_parser::parse(&text)
                .unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e));
        }
    };
}

parse_gantt!(gantt_basic);
parse_gantt!(gantt_dependencies);
parse_gantt!(gantt_simple);
parse_gantt!(gantt_milestones);
parse_gantt!(gantt_many_tasks);

// --- Gitgraph ---

macro_rules! parse_gitgraph {
    ($name:ident) => {
        #[test]
        fn $name() {
            let path = golden_dir().join("gitgraph").join(concat!(stringify!($name), ".mmd"));
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
            gitgraph_parser::parse(&text)
                .unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e));
        }
    };
}

parse_gitgraph!(git_basic);
parse_gitgraph!(git_feature_branches);
parse_gitgraph!(git_commit_types);
parse_gitgraph!(git_many_branches);
parse_gitgraph!(git_tags);

// --- XY Chart ---

macro_rules! parse_xychart {
    ($name:ident) => {
        #[test]
        fn $name() {
            let path = golden_dir().join("xychart").join(concat!(stringify!($name), ".mmd"));
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
            xychart_parser::parse(&text)
                .unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e));
        }
    };
}

parse_xychart!(xychart_bar);
parse_xychart!(xychart_line);
parse_xychart!(xychart_mixed);
parse_xychart!(xychart_multi_series);

// --- Mindmap ---

macro_rules! parse_mindmap {
    ($name:ident) => {
        #[test]
        fn $name() {
            let path = golden_dir().join("mindmap").join(concat!(stringify!($name), ".mmd"));
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
            mindmap_parser::parse(&text)
                .unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e));
        }
    };
}

parse_mindmap!(mindmap_basic);
parse_mindmap!(mindmap_shapes);
parse_mindmap!(mindmap_deep);
parse_mindmap!(mindmap_wide);
parse_mindmap!(mindmap_study);
parse_mindmap!(mindmap_single);
parse_mindmap!(mindmap_asymmetric);

// --- Sankey ---

macro_rules! parse_sankey {
    ($name:ident) => {
        #[test]
        fn $name() {
            let path = golden_dir().join("sankey").join(concat!(stringify!($name), ".mmd"));
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
            sankey_parser::parse(&text)
                .unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e));
        }
    };
}

parse_sankey!(sankey_basic);
parse_sankey!(sankey_energy);
parse_sankey!(sankey_multi);
parse_sankey!(sankey_quoted);

// --- Packet ---

macro_rules! parse_packet {
    ($name:ident) => {
        #[test]
        fn $name() {
            let path = golden_dir().join("packet").join(concat!(stringify!($name), ".mmd"));
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
            packet_parser::parse(&text)
                .unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e));
        }
    };
}

parse_packet!(packet_tcp);
parse_packet!(packet_udp);
parse_packet!(packet_ipv4);
parse_packet!(packet_single_field);
parse_packet!(packet_many_fields);

// --- Quadrant ---

macro_rules! parse_quadrant {
    ($name:ident) => {
        #[test]
        fn $name() {
            let path = golden_dir().join("quadrant").join(concat!(stringify!($name), ".mmd"));
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
            quadrant_parser::parse(&text)
                .unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e));
        }
    };
}

parse_quadrant!(quadrant_priority);
parse_quadrant!(quadrant_gartner);
parse_quadrant!(quadrant_many_points);
parse_quadrant!(quadrant_minimal);

// --- Venn ---

macro_rules! parse_venn {
    ($name:ident) => {
        #[test]
        fn $name() {
            let path = golden_dir().join("venn").join(concat!(stringify!($name), ".mmd"));
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
            venn_parser::parse(&text)
                .unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e));
        }
    };
}

parse_venn!(venn_two_sets);
parse_venn!(venn_three_sets);
parse_venn!(venn_four_sets);
parse_venn!(venn_single_set);

// --- Radar ---

macro_rules! parse_radar {
    ($name:ident) => {
        #[test]
        fn $name() {
            let path = golden_dir().join("radar").join(concat!(stringify!($name), ".mmd"));
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
            radar_parser::parse(&text)
                .unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e));
        }
    };
}

parse_radar!(radar_skills);
parse_radar!(radar_performance);
parse_radar!(radar_single_curve);
parse_radar!(radar_many_curves);

// --- Journey ---

macro_rules! parse_journey {
    ($name:ident) => {
        #[test]
        fn $name() {
            let path = golden_dir().join("journey").join(concat!(stringify!($name), ".mmd"));
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
            journey_parser::parse(&text)
                .unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e));
        }
    };
}

parse_journey!(journey_workday);
parse_journey!(journey_ecommerce);
parse_journey!(journey_multi_actor);
parse_journey!(journey_single_section);

// --- Treeview ---

macro_rules! parse_treeview {
    ($name:ident) => {
        #[test]
        fn $name() {
            let path = golden_dir().join("treeview").join(concat!(stringify!($name), ".mmd"));
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
            treeview_parser::parse(&text)
                .unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e));
        }
    };
}

parse_treeview!(treeview_project);
parse_treeview!(treeview_org);
parse_treeview!(treeview_deep);
parse_treeview!(treeview_wide);

// --- Ishikawa ---

macro_rules! parse_ishikawa {
    ($name:ident) => {
        #[test]
        fn $name() {
            let path = golden_dir().join("ishikawa").join(concat!(stringify!($name), ".mmd"));
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
            ishikawa_parser::parse(&text)
                .unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e));
        }
    };
}

parse_ishikawa!(ishikawa_quality);
parse_ishikawa!(ishikawa_bug);
parse_ishikawa!(ishikawa_many_categories);
parse_ishikawa!(ishikawa_minimal);

// --- Treemap ---

macro_rules! parse_treemap {
    ($name:ident) => {
        #[test]
        fn $name() {
            let path = golden_dir().join("treemap").join(concat!(stringify!($name), ".mmd"));
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
            treemap_parser::parse(&text)
                .unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e));
        }
    };
}

parse_treemap!(treemap_budget);
parse_treemap!(treemap_disk);
parse_treemap!(treemap_deep);
parse_treemap!(treemap_flat);

// --- Block ---

macro_rules! parse_block {
    ($name:ident) => {
        #[test]
        fn $name() {
            let path = golden_dir().join("block").join(concat!(stringify!($name), ".mmd"));
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
            block_parser::parse(&text)
                .unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e));
        }
    };
}

parse_block!(block_grid);
parse_block!(block_shapes);
parse_block!(block_spanning);
parse_block!(block_many_shapes);
parse_block!(block_edges);

// --- C4 ---

macro_rules! parse_c4 {
    ($name:ident) => {
        #[test]
        fn $name() {
            let path = golden_dir().join("c4").join(concat!(stringify!($name), ".mmd"));
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
            c4_parser::parse(&text)
                .unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e));
        }
    };
}

parse_c4!(c4_context);
parse_c4!(c4_container);
parse_c4!(c4_dynamic);
parse_c4!(c4_boundary);
parse_c4!(c4_single_element);

// --- Architecture ---

macro_rules! parse_architecture {
    ($name:ident) => {
        #[test]
        fn $name() {
            let path = golden_dir().join("architecture").join(concat!(stringify!($name), ".mmd"));
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
            arch_parser::parse(&text)
                .unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e));
        }
    };
}

parse_architecture!(arch_api_gateway);
parse_architecture!(arch_network);
parse_architecture!(arch_multi_group);
parse_architecture!(arch_junctions);
parse_architecture!(arch_single_service);
