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

// Basic graphs
flowchart_layout!(single_node);
flowchart_layout!(hello);
flowchart_layout!(linear_3);
flowchart_layout!(diamond);
flowchart_layout!(diamond_flow);
flowchart_layout!(cycle);
flowchart_layout!(cycle_3);
flowchart_layout!(long_edge);
flowchart_layout!(long_edges);
flowchart_layout!(crossing);
flowchart_layout!(mixed_sizes);
flowchart_layout!(disconnected);
flowchart_layout!(wide_graph);
flowchart_layout!(chain_long);
flowchart_layout!(chain_branching);
flowchart_layout!(multi_edge);
flowchart_layout!(implicit_nodes);

// Directions
flowchart_layout!(linear_lr);
flowchart_layout!(linear_bt);
flowchart_layout!(linear_rl);
flowchart_layout!(directions);
flowchart_layout!(directions_all);
flowchart_layout!(directions_bt);
flowchart_layout!(directions_lr);
flowchart_layout!(directions_rl);

// Edges & labels
flowchart_layout!(edge_label);
flowchart_layout!(edge_labels);
flowchart_layout!(edge_labels_all);
flowchart_layout!(edge_lengths);
flowchart_layout!(edge_matrix);
flowchart_layout!(arrows);
flowchart_layout!(open_edges);
flowchart_layout!(self_loop);

// Shapes
flowchart_layout!(shapes);
flowchart_layout!(shapes_with_labels);
flowchart_layout!(all_shapes);

// Styling & labels
flowchart_layout!(weighted);
flowchart_layout!(minlen);
flowchart_layout!(html_labels);
flowchart_layout!(unicode_labels);
flowchart_layout!(quoted_strings);
flowchart_layout!(style_classdef);
flowchart_layout!(style_inline);
// click_bindings, comments_directives, mixed_statements: skipped (parser
// doesn't support click/accTitle/accDescr syntax yet)

// Compound / subgraphs
flowchart_layout!(compound_simple);
flowchart_layout!(nested_compound);
flowchart_layout!(subgraph);
flowchart_layout!(subgraph_deep);
flowchart_layout!(subgraph_direction);
flowchart_layout!(subgraph_empty);
flowchart_layout!(nested_subgraph);

// Real-world diagrams
flowchart_layout!(realistic_flowchart);
flowchart_layout!(ci_pipeline);
flowchart_layout!(compiler_pipeline);
flowchart_layout!(mcp_server);

// Architecture diagrams
flowchart_layout!(arch_api_gateway);
flowchart_layout!(arch_auth_flow);
flowchart_layout!(arch_caching_layers);
flowchart_layout!(arch_cicd_pipeline);
flowchart_layout!(arch_cloud_aws);
flowchart_layout!(arch_cloud_k8s);
flowchart_layout!(arch_compiler_pipeline);
flowchart_layout!(arch_component);
flowchart_layout!(arch_data_mesh);
flowchart_layout!(arch_data_pipeline_etl);
flowchart_layout!(arch_database_replication);
flowchart_layout!(arch_event_driven);
flowchart_layout!(arch_game_engine);
flowchart_layout!(arch_hexagonal);
flowchart_layout!(arch_iot_platform);
flowchart_layout!(arch_message_queue);
flowchart_layout!(arch_microservices_basic);
flowchart_layout!(arch_microservices_ecommerce);
flowchart_layout!(arch_ml_pipeline);
flowchart_layout!(arch_monitoring_stack);
flowchart_layout!(arch_multi_region);
flowchart_layout!(arch_serverless);
flowchart_layout!(arch_service_mesh);
flowchart_layout!(arch_streaming_platform);
flowchart_layout!(arch_zero_trust);

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

/// Helper: assert peer subgraphs (same parent level) don't overlap.
///
/// Two subgraphs are peers if neither is listed in the other's `subgraph_ids`
/// and they share the same parent (or are both root-level). Overlap means their
/// bounding boxes intersect, which would cause rendering artifacts.
fn assert_peer_subgraphs_no_overlap(
    name: &str,
    diagram: &flowchart::ir::FlowDiagram,
    result: &flowchart::bridge::LayoutResult,
) {
    use std::collections::HashMap;

    // Build parent map: sg_id → parent_sg_id
    let mut parent_of: HashMap<&str, &str> = HashMap::new();
    for sg in &diagram.subgraphs {
        for child_id in &sg.subgraph_ids {
            parent_of.insert(child_id.as_str(), sg.id.as_str());
        }
    }

    // Group subgraphs by parent (None = root level)
    let mut peer_groups: HashMap<Option<&str>, Vec<&str>> = HashMap::new();
    for sg in &diagram.subgraphs {
        let parent = parent_of.get(sg.id.as_str()).copied();
        peer_groups.entry(parent).or_default().push(sg.id.as_str());
    }

    // Check all pairs within each peer group
    for (parent, peers) in &peer_groups {
        for i in 0..peers.len() {
            for j in (i + 1)..peers.len() {
                let Some(a) = result.subgraphs.iter().find(|s| s.id == peers[i]) else {
                    continue;
                };
                let Some(b) = result.subgraphs.iter().find(|s| s.id == peers[j]) else {
                    continue;
                };

                let a_left = a.x - a.width / 2.0;
                let a_right = a.x + a.width / 2.0;
                let a_top = a.y - a.height / 2.0;
                let a_bottom = a.y + a.height / 2.0;
                let b_left = b.x - b.width / 2.0;
                let b_right = b.x + b.width / 2.0;
                let b_top = b.y - b.height / 2.0;
                let b_bottom = b.y + b.height / 2.0;

                let overlaps = a_left < b_right
                    && b_left < a_right
                    && a_top < b_bottom
                    && b_top < a_bottom;

                assert!(
                    !overlaps,
                    "{name}: peer subgraphs '{}' and '{}' overlap (parent: {:?})\n\
                     '{}': [{a_left:.1}, {a_top:.1}] - [{a_right:.1}, {a_bottom:.1}]\n\
                     '{}': [{b_left:.1}, {b_top:.1}] - [{b_right:.1}, {b_bottom:.1}]",
                    peers[i], peers[j], parent,
                    peers[i], peers[j],
                );
            }
        }
    }
}

#[test]
fn compiler_pipeline_peer_subgraphs_no_overlap() {
    let path = golden_dir().join("compiler_pipeline.mmd");
    let text = fs::read_to_string(&path).unwrap();
    let diagram = flowchart::parser::parse(&text).unwrap();
    let result = flowchart::bridge::layout(&diagram);
    assert_peer_subgraphs_no_overlap("compiler_pipeline", &diagram, &result);
}

#[test]
fn mcp_server_peer_subgraphs_no_overlap() {
    let path = golden_dir().join("mcp_server.mmd");
    let text = fs::read_to_string(&path).unwrap();
    let diagram = flowchart::parser::parse(&text).unwrap();
    let result = flowchart::bridge::layout(&diagram);
    assert_peer_subgraphs_no_overlap("mcp_server", &diagram, &result);
}

#[test]
fn nested_compound_peer_subgraphs_no_overlap() {
    let path = golden_dir().join("nested_compound.mmd");
    let text = fs::read_to_string(&path).unwrap();
    let diagram = flowchart::parser::parse(&text).unwrap();
    let result = flowchart::bridge::layout(&diagram);
    assert_peer_subgraphs_no_overlap("nested_compound", &diagram, &result);
}

#[test]
fn arch_component_subgraph_containment() {
    let path = golden_dir().join("arch_component.mmd");
    let text = fs::read_to_string(&path).unwrap();
    let diagram = flowchart::parser::parse(&text).unwrap();
    let result = flowchart::bridge::layout(&diagram);
    assert_subgraph_containment("arch_component", &diagram, &result);
}

#[test]
fn arch_component_peer_subgraphs_no_overlap() {
    let path = golden_dir().join("arch_component.mmd");
    let text = fs::read_to_string(&path).unwrap();
    let diagram = flowchart::parser::parse(&text).unwrap();
    let result = flowchart::bridge::layout(&diagram);
    assert_peer_subgraphs_no_overlap("arch_component", &diagram, &result);
}
