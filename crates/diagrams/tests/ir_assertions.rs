//! IR content assertions: verify parsed structure matches expected
//! node counts, edge counts, shapes, and connectivity.

use rusty_mermaid_core::Shape;
use rusty_mermaid_diagrams::class::parser as class_parser;
use rusty_mermaid_diagrams::er::parser as er_parser;
use rusty_mermaid_diagrams::requirement::parser as req_parser;
use rusty_mermaid_diagrams::flowchart::parser as flowchart_parser;
use rusty_mermaid_diagrams::sequence::parser as sequence_parser;
use rusty_mermaid_diagrams::state::parser as state_parser;
use rusty_mermaid_diagrams::pie::parser as pie_parser;
use rusty_mermaid_diagrams::sankey::parser as sankey_parser;
use rusty_mermaid_diagrams::packet::parser as packet_parser;
use rusty_mermaid_diagrams::mindmap::parser as mindmap_parser;
use rusty_mermaid_diagrams::quadrant::parser as quadrant_parser;
use rusty_mermaid_diagrams::venn::parser as venn_parser;
use rusty_mermaid_diagrams::radar::parser as radar_parser;
use rusty_mermaid_diagrams::journey::parser as journey_parser;
use rusty_mermaid_diagrams::treeview::parser as treeview_parser;
use rusty_mermaid_diagrams::treemap::parser as treemap_parser;
use rusty_mermaid_diagrams::block::parser as block_parser;
use rusty_mermaid_diagrams::c4::parser as c4_parser;
use rusty_mermaid_diagrams::architecture::parser as arch_parser;
use rusty_mermaid_diagrams::gantt::parser as gantt_parser;
use rusty_mermaid_diagrams::gitgraph::parser as gitgraph_parser;
use rusty_mermaid_diagrams::timeline::parser as timeline_parser;
use rusty_mermaid_diagrams::kanban::parser as kanban_parser;
use rusty_mermaid_diagrams::xychart::parser as xychart_parser;
use rusty_mermaid_diagrams::ishikawa::parser as ishikawa_parser;

use std::fs;
use std::path::Path;

fn golden_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/golden/mmd")
}

fn read_golden(subdir: &str, name: &str) -> String {
    let path = golden_dir().join(subdir).join(format!("{name}.mmd"));
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

// ── Flowchart IR assertions ─────────────────────────────────────────

#[test]
fn flowchart_all_shapes_nodes_and_edges() {
    let d = flowchart_parser::parse(&read_golden("flowchart", "all_shapes")).unwrap();
    assert_eq!(d.vertices.len(), 14, "14 distinct shapes");
    assert_eq!(d.edges.len(), 13, "linear chain of 14 nodes = 13 edges");

    // Verify each shape is parsed correctly
    let shapes: Vec<_> = d.vertices.iter().map(|v| (&*v.id, v.shape)).collect();
    assert_eq!(shapes[0], ("rect", Shape::Rect));
    assert_eq!(shapes[1], ("rounded", Shape::RoundedRect));
    assert_eq!(shapes[2], ("stadium", Shape::Stadium));
    assert_eq!(shapes[3], ("diamond", Shape::Diamond));
    assert_eq!(shapes[4], ("circle", Shape::Circle));
    assert_eq!(shapes[5], ("hex", Shape::Hexagon));
    assert_eq!(shapes[6], ("para", Shape::Trapezoid));
    assert_eq!(shapes[7], ("paraAlt", Shape::TrapezoidAlt));
    assert_eq!(shapes[8], ("trap", Shape::Trapezoid));
    assert_eq!(shapes[9], ("trapAlt", Shape::TrapezoidAlt));
    assert_eq!(shapes[10], ("dblCircle", Shape::DoubleCircle));
    assert_eq!(shapes[11], ("sub", Shape::Subroutine));
    assert_eq!(shapes[12], ("cyl", Shape::Cylinder));
    assert_eq!(shapes[13], ("asym", Shape::Asymmetric));
}

#[test]
fn flowchart_linear_3_structure() {
    let d = flowchart_parser::parse(&read_golden("flowchart", "linear_3")).unwrap();
    assert_eq!(d.vertices.len(), 3);
    assert_eq!(d.edges.len(), 2);
    assert_eq!(d.edges[0].src, "A");
    assert_eq!(d.edges[0].dst, "B");
    assert_eq!(d.edges[1].src, "B");
    assert_eq!(d.edges[1].dst, "C");
}

#[test]
fn flowchart_diamond_flow_branching() {
    let d = flowchart_parser::parse(&read_golden("flowchart", "diamond_flow")).unwrap();
    assert_eq!(d.vertices.len(), 7, "A B C D E F G");
    assert_eq!(d.edges.len(), 8);
    assert_eq!(d.vertex("B").unwrap().shape, Shape::Diamond);
    assert_eq!(d.vertex("C").unwrap().shape, Shape::Diamond);
    // B branches to C and D
    let b_dsts: Vec<_> = d.edges.iter().filter(|e| e.src == "B").map(|e| &*e.dst).collect();
    assert!(b_dsts.contains(&"C"));
    assert!(b_dsts.contains(&"D"));
}

#[test]
fn flowchart_edge_labels_present() {
    let d = flowchart_parser::parse(&read_golden("flowchart", "edge_labels")).unwrap();
    assert_eq!(d.vertices.len(), 7);
    assert_eq!(d.edges.len(), 7);
    // First 3 edges have labels
    assert_eq!(d.edges[0].label.as_deref(), Some("Place order"));
    assert_eq!(d.edges[1].label.as_deref(), Some("Success"));
    assert_eq!(d.edges[2].label.as_deref(), Some("Failed"));
}

#[test]
fn flowchart_arrows_all_types() {
    let d = flowchart_parser::parse(&read_golden("flowchart", "arrows")).unwrap();
    assert_eq!(d.vertices.len(), 10, "A through J");
    assert_eq!(d.edges.len(), 9);
    use rusty_mermaid_diagrams::flowchart::ir::{ArrowEnd, StrokeType};
    // A --> B (normal arrow)
    assert_eq!(d.edges[0].stroke, StrokeType::Normal);
    assert_eq!(d.edges[0].end_arrow, ArrowEnd::Arrow);
    // A --- C (no arrow)
    assert_eq!(d.edges[1].end_arrow, ArrowEnd::None);
    // A -.-> D (dotted)
    assert_eq!(d.edges[2].stroke, StrokeType::Dotted);
    // A ==> E (thick)
    assert_eq!(d.edges[3].stroke, StrokeType::Thick);
    // B --o F (circle end)
    assert_eq!(d.edges[4].end_arrow, ArrowEnd::Circle);
    // B --x G (cross end)
    assert_eq!(d.edges[5].end_arrow, ArrowEnd::Cross);
}

#[test]
fn flowchart_self_loop_edge() {
    let d = flowchart_parser::parse(&read_golden("flowchart", "self_loop")).unwrap();
    let self_edges: Vec<_> = d.edges.iter().filter(|e| e.src == e.dst).collect();
    assert_eq!(self_edges.len(), 1, "C --> C is one self-loop");
    assert_eq!(self_edges[0].src, "C");
}

#[test]
fn flowchart_chain_branching_fan_out() {
    let d = flowchart_parser::parse(&read_golden("flowchart", "chain_branching")).unwrap();
    assert_eq!(d.vertices.len(), 7, "A B C D E F G");
    assert_eq!(d.edges.len(), 8);
    // A fans out to B and C
    let a_dsts: Vec<_> = d.edges.iter().filter(|e| e.src == "A").map(|e| &*e.dst).collect();
    assert_eq!(a_dsts.len(), 2);
    // D fans out to E and F
    let d_dsts: Vec<_> = d.edges.iter().filter(|e| e.src == "D").map(|e| &*e.dst).collect();
    assert_eq!(d_dsts.len(), 2);
}

#[test]
fn flowchart_compound_subgraph() {
    let d = flowchart_parser::parse(&read_golden("flowchart", "compound_simple")).unwrap();
    assert_eq!(d.subgraphs.len(), 1);
    assert_eq!(d.subgraphs[0].id, "cluster");
    assert!(d.subgraphs[0].node_ids.contains(&"A".to_string()));
    assert!(d.subgraphs[0].node_ids.contains(&"B".to_string()));
}

#[test]
fn flowchart_style_classdef_resolution() {
    let d = flowchart_parser::parse(&read_golden("flowchart", "style_classdef")).unwrap();
    assert_eq!(d.vertices.len(), 5, "A B C D E");
    assert_eq!(d.class_defs.len(), 2, "highlight + dimmed");
    // A and C have class "highlight", B has "dimmed"
    assert!(d.vertex("A").unwrap().classes.contains(&"highlight".to_string()));
    assert!(d.vertex("C").unwrap().classes.contains(&"highlight".to_string()));
    assert!(d.vertex("B").unwrap().classes.contains(&"dimmed".to_string()));
    // D has inline :::highlight
    assert!(d.vertex("D").unwrap().classes.contains(&"highlight".to_string()));
}

#[test]
fn flowchart_combo_subgraph_styled() {
    let d = flowchart_parser::parse(&read_golden("flowchart", "combo_subgraph_styled")).unwrap();
    assert_eq!(d.subgraphs.len(), 3, "inputs, processing, outputs");
    assert_eq!(d.class_defs.len(), 5, "primary, warning, danger, success, info");
    assert!(!d.link_styles.is_empty(), "linkStyle statements present");
}

// ── State IR assertions ─────────────────────────────────────────────

#[test]
fn state_simple_transitions() {
    let text = &read_golden("state", "state_linear_chain");
    let d = state_parser::parse(text).unwrap();
    assert!(!d.states.is_empty());
    assert!(!d.transitions.is_empty());
    // Every transition should have non-empty src and dst
    for t in &d.transitions {
        assert!(!t.src.is_empty(), "transition src must not be empty");
        assert!(!t.dst.is_empty(), "transition dst must not be empty");
    }
}

#[test]
fn state_pseudostates_present() {
    // state_linear_chain should have [*] → first state transitions
    let text = &read_golden("state", "state_linear_chain");
    let d = state_parser::parse(text).unwrap();
    let has_start = d.transitions.iter().any(|t| t.src == "[*]");
    assert!(has_start, "should have [*] start transition");
}

#[test]
fn state_composite_nesting() {
    let text = &read_golden("state", "state_composite");
    let d = state_parser::parse(text).unwrap();
    // At least one composite state
    use rusty_mermaid_diagrams::state::ir::StateKind;
    let composites: Vec<_> = d.states.iter().filter(|s| matches!(s.kind, StateKind::Composite { .. })).collect();
    assert!(!composites.is_empty(), "should have at least one composite state");
}

#[test]
fn state_concurrent_regions() {
    let text = &read_golden("state", "state_concurrent");
    let d = state_parser::parse(text).unwrap();
    // Concurrent state has regions separated by --
    use rusty_mermaid_diagrams::state::ir::StateKind;
    let concurrent: Vec<_> = d.states.iter().filter(|s| {
        if let StateKind::Composite { regions, .. } = &s.kind { regions.len() > 1 } else { false }
    }).collect();
    assert!(!concurrent.is_empty(), "should have a concurrent composite with >1 region");
}

#[test]
fn state_choice_and_fork() {
    let text = &read_golden("state", "state_all_types");
    let d = state_parser::parse(text).unwrap();
    use rusty_mermaid_diagrams::state::ir::StateKind;
    let has_choice = d.states.iter().any(|s| matches!(s.kind, StateKind::Choice));
    let has_fork = d.states.iter().any(|s| matches!(s.kind, StateKind::Fork));
    assert!(has_choice, "should have a <<choice>> state");
    assert!(has_fork, "should have a <<fork>> state");
}

// ── Sequence IR assertions ──────────────────────────────────────────

#[test]
fn sequence_basic_structure() {
    let text = &read_golden("sequence", "seq_basic");
    let d = sequence_parser::parse(text).unwrap();
    assert!(d.participants.len() >= 2, "at least 2 participants");
    assert!(!d.items.is_empty(), "should have messages");
}

#[test]
fn sequence_all_arrow_types() {
    let text = &read_golden("sequence", "seq_arrows");
    let d = sequence_parser::parse(text).unwrap();
    use rusty_mermaid_diagrams::sequence::ir::{ArrowHead, LineStyle, SequenceItem};
    let messages: Vec<_> = d.items.iter().filter_map(|item| {
        if let SequenceItem::Message(m) = item { Some(m) } else { None }
    }).collect();
    assert!(messages.len() >= 4, "should have multiple arrow type messages");
    // Check we have both solid and dotted lines
    let has_solid = messages.iter().any(|m| m.arrow.line == LineStyle::Solid);
    let has_dotted = messages.iter().any(|m| m.arrow.line == LineStyle::Dotted);
    assert!(has_solid, "should have solid line messages");
    assert!(has_dotted, "should have dotted line messages");
    // Check we have different arrow heads
    let has_filled = messages.iter().any(|m| m.arrow.head == ArrowHead::Filled);
    let has_open = messages.iter().any(|m| m.arrow.head == ArrowHead::Open);
    assert!(has_filled, "should have filled arrowhead");
    assert!(has_open, "should have open arrowhead");
}

#[test]
fn sequence_self_message() {
    let text = &read_golden("sequence", "seq_self_msg");
    let d = sequence_parser::parse(text).unwrap();
    use rusty_mermaid_diagrams::sequence::ir::SequenceItem;
    let self_msgs: Vec<_> = d.items.iter().filter_map(|item| {
        if let SequenceItem::Message(m) = item {
            if m.from == m.to { Some(m) } else { None }
        } else { None }
    }).collect();
    assert!(!self_msgs.is_empty(), "should have at least one self-message");
}

#[test]
fn sequence_notes() {
    let text = &read_golden("sequence", "seq_notes");
    let d = sequence_parser::parse(text).unwrap();
    use rusty_mermaid_diagrams::sequence::ir::SequenceItem;
    let notes: Vec<_> = d.items.iter().filter(|item| matches!(item, SequenceItem::Note(_))).collect();
    assert!(!notes.is_empty(), "should have note items");
}

#[test]
fn sequence_fragments() {
    let text = &read_golden("sequence", "seq_loop");
    let d = sequence_parser::parse(text).unwrap();
    use rusty_mermaid_diagrams::sequence::ir::SequenceItem;
    let fragments: Vec<_> = d.items.iter().filter(|item| matches!(item, SequenceItem::Fragment(_))).collect();
    assert!(!fragments.is_empty(), "should have loop fragment");
}

#[test]
fn sequence_activation() {
    let text = &read_golden("sequence", "seq_activation");
    let d = sequence_parser::parse(text).unwrap();
    use rusty_mermaid_diagrams::sequence::ir::SequenceItem;
    // Should have activate/deactivate items or messages with +/- suffixes
    let has_activation = d.items.iter().any(|item| {
        matches!(item, SequenceItem::Activation(_)) ||
        matches!(item, SequenceItem::Message(m) if m.activate)
    });
    assert!(has_activation, "should have activation items");
}

#[test]
fn sequence_autonumber() {
    let text = &read_golden("sequence", "seq_autonumber");
    let d = sequence_parser::parse(text).unwrap();
    assert!(d.autonumber.is_some(), "autonumber should be enabled");
}

// ── Class IR assertions ─────────────────────────────────────────────

#[test]
fn class_basic_structure() {
    let d = class_parser::parse(&read_golden("class", "class_basic")).unwrap();
    assert_eq!(d.classes.len(), 2, "Animal + Dog");
    assert_eq!(d.relationships.len(), 1);
    assert!(!d.classes[0].members.is_empty());
    assert!(!d.classes[0].methods.is_empty());
}

#[test]
fn class_all_relationship_types() {
    let d = class_parser::parse(&read_golden("class", "class_relationships")).unwrap();
    assert!(d.relationships.len() >= 5, "should have all relationship types");
}

#[test]
fn class_generics_parsed() {
    let d = class_parser::parse(&read_golden("class", "class_generics")).unwrap();
    let list = d.class("List").unwrap();
    assert!(list.generic_type.is_some(), "List should have generic type");
}

#[test]
fn class_namespaces_parsed() {
    let d = class_parser::parse(&read_golden("class", "class_namespaces")).unwrap();
    assert!(!d.namespaces.is_empty(), "should have namespaces");
}

// ── ER IR assertions ────────────────────────────────────────────────

#[test]
fn er_basic_structure() {
    let d = er_parser::parse(&read_golden("er", "er_basic")).unwrap();
    assert_eq!(d.entities.len(), 2, "CUSTOMER + ORDER");
    assert_eq!(d.relationships.len(), 1);
    assert!(!d.entities[0].attributes.is_empty());
}

#[test]
fn er_all_cardinality_types() {
    let d = er_parser::parse(&read_golden("er", "er_cardinality")).unwrap();
    assert_eq!(d.relationships.len(), 4);
    use rusty_mermaid_diagrams::er::ir::Cardinality;
    let cards: Vec<_> = d.relationships.iter()
        .flat_map(|r| [r.cardinality_a, r.cardinality_b])
        .collect();
    assert!(cards.contains(&Cardinality::ExactlyOne));
    assert!(cards.contains(&Cardinality::ZeroOrMore));
    assert!(cards.contains(&Cardinality::OneOrMore));
    assert!(cards.contains(&Cardinality::ZeroOrOne));
}

#[test]
fn er_attributes_with_keys() {
    let d = er_parser::parse(&read_golden("er", "er_attributes")).unwrap();
    let product = d.entity("PRODUCT").unwrap();
    use rusty_mermaid_diagrams::er::ir::KeyType;
    let has_pk = product.attributes.iter().any(|a| a.keys.contains(&KeyType::PrimaryKey));
    let has_fk = product.attributes.iter().any(|a| a.keys.contains(&KeyType::ForeignKey));
    let has_uk = product.attributes.iter().any(|a| a.keys.contains(&KeyType::UniqueKey));
    assert!(has_pk, "should have PK attribute");
    assert!(has_fk, "should have FK attribute");
    assert!(has_uk, "should have UK attribute");
}

#[test]
fn er_complex_entity_count() {
    let d = er_parser::parse(&read_golden("er", "er_complex")).unwrap();
    assert_eq!(d.entities.len(), 5, "CUSTOMER, ORDER, LINE-ITEM, PRODUCT, CATEGORY");
    assert_eq!(d.relationships.len(), 4);
}

// ── Requirement IR assertions ───────────────────────────────────────

#[test]
fn req_basic_structure() {
    let d = req_parser::parse(&read_golden("requirement", "req_basic")).unwrap();
    assert_eq!(d.requirements.len(), 2);
    assert_eq!(d.relationships.len(), 1);
    assert_eq!(d.requirements[0].risk, Some(rusty_mermaid_diagrams::requirement::ir::RiskLevel::High));
}

#[test]
fn req_all_types_coverage() {
    let d = req_parser::parse(&read_golden("requirement", "req_all_types")).unwrap();
    assert!(d.requirements.len() >= 4, "should have multiple requirement types");
    assert_eq!(d.elements.len(), 1, "should have one element");
    assert!(d.relationships.len() >= 5, "should have multiple relationship types");
}

#[test]
fn req_all_relationship_types() {
    let d = req_parser::parse(&read_golden("requirement", "req_relationships")).unwrap();
    assert_eq!(d.relationships.len(), 7, "all 7 relationship types");
    use rusty_mermaid_diagrams::requirement::ir::RelationshipType;
    let types: Vec<_> = d.relationships.iter().map(|r| r.rel_type).collect();
    assert!(types.contains(&RelationshipType::Contains));
    assert!(types.contains(&RelationshipType::Copies));
    assert!(types.contains(&RelationshipType::Derives));
    assert!(types.contains(&RelationshipType::Satisfies));
    assert!(types.contains(&RelationshipType::Verifies));
    assert!(types.contains(&RelationshipType::Refines));
    assert!(types.contains(&RelationshipType::Traces));
}

// ── Pie IR assertions ───────────────────────────────────────────────

#[test]
fn pie_basic_slices_and_title() {
    let d = pie_parser::parse(&read_golden("pie", "pie_basic")).unwrap();
    assert_eq!(d.title.as_deref(), Some("Pet Adoption"));
    assert_eq!(d.slices.len(), 5, "Dogs, Cats, Birds, Fish, Reptiles");
    assert!(!d.show_data);
}

#[test]
fn pie_basic_slice_values() {
    let d = pie_parser::parse(&read_golden("pie", "pie_basic")).unwrap();
    assert_eq!(d.slices[0].label, "Dogs");
    assert!((d.slices[0].value - 386.0).abs() < f64::EPSILON);
    assert_eq!(d.slices[4].label, "Reptiles");
    assert!((d.slices[4].value - 10.0).abs() < f64::EPSILON);
    assert!((d.total() - 531.0).abs() < f64::EPSILON);
}

#[test]
fn pie_show_data_flag() {
    let d = pie_parser::parse(&read_golden("pie", "pie_show_data")).unwrap();
    assert!(d.show_data, "showData flag should be set");
    assert_eq!(d.title.as_deref(), Some("Browser Market Share"));
    assert_eq!(d.slices.len(), 5);
}

// ── Sankey IR assertions ────────────────────────────────────────────

#[test]
fn sankey_basic_links_and_nodes() {
    let d = sankey_parser::parse(&read_golden("sankey", "sankey_basic")).unwrap();
    assert_eq!(d.links.len(), 4);
    let names = d.node_names();
    assert!(names.contains(&"Source A".to_string()));
    assert!(names.contains(&"Target X".to_string()));
    assert!(names.contains(&"Target Y".to_string()));
    assert_eq!(names.len(), 4, "Source A, Source B, Target X, Target Y");
}

#[test]
fn sankey_energy_cascade() {
    let d = sankey_parser::parse(&read_golden("sankey", "sankey_energy")).unwrap();
    assert_eq!(d.links.len(), 9);
    let names = d.node_names();
    assert!(names.contains(&"Electricity".to_string()));
    assert!(names.contains(&"Coal".to_string()));
    assert!(names.contains(&"Residential".to_string()));
}

#[test]
fn sankey_quoted_node_names() {
    let d = sankey_parser::parse(&read_golden("sankey", "sankey_quoted")).unwrap();
    assert_eq!(d.links.len(), 6);
    let names = d.node_names();
    assert!(names.contains(&"Heating & Cooling".to_string()), "quoted names with special chars");
    assert!(names.contains(&"Office Buildings".to_string()));
}

// ── Packet IR assertions ────────────────────────────────────────────

#[test]
fn packet_tcp_fields() {
    let d = packet_parser::parse(&read_golden("packet", "packet_tcp")).unwrap();
    assert_eq!(d.title.as_deref(), Some("TCP Header"));
    assert_eq!(d.fields.len(), 15);
    assert_eq!(d.fields[0].label, "Source Port");
    assert_eq!(d.fields[0].start, 0);
    assert_eq!(d.fields[0].end, 15);
    assert_eq!(d.fields[0].bits(), 16);
}

#[test]
fn packet_tcp_single_bit_flags() {
    let d = packet_parser::parse(&read_golden("packet", "packet_tcp")).unwrap();
    // URG, ACK, PSH, RST, SYN, FIN are single-bit fields (106-111)
    let flags: Vec<_> = d.fields.iter().filter(|f| f.bits() == 1).collect();
    assert_eq!(flags.len(), 6, "6 single-bit TCP flags");
    assert_eq!(flags[0].label, "URG");
    assert_eq!(flags[5].label, "FIN");
}

#[test]
fn packet_udp_fields() {
    let d = packet_parser::parse(&read_golden("packet", "packet_udp")).unwrap();
    assert_eq!(d.title.as_deref(), Some("UDP Header"));
    assert_eq!(d.fields.len(), 4, "Source Port, Dest Port, Length, Checksum");
    // All 16-bit fields
    for f in &d.fields {
        assert_eq!(f.bits(), 16, "{} should be 16 bits", f.label);
    }
}

// ── Mindmap IR assertions ───────────────────────────────────────────

#[test]
fn mindmap_basic_structure() {
    let d = mindmap_parser::parse(&read_golden("mindmap", "mindmap_basic")).unwrap();
    assert_eq!(d.root.text, "Programming");
    assert_eq!(d.root.children.len(), 3, "Languages, Paradigms, Tools");
    assert_eq!(d.root.count(), 13, "root + 3 branches x 3 leaves + 3 branch nodes");
}

#[test]
fn mindmap_shapes_parsed() {
    use rusty_mermaid_diagrams::mindmap::ir::MindmapShape;
    let d = mindmap_parser::parse(&read_golden("mindmap", "mindmap_shapes")).unwrap();
    assert_eq!(d.root.text, "Central Idea");
    assert_eq!(d.root.children.len(), 6);
    let shapes: Vec<_> = d.root.children.iter().map(|c| c.shape).collect();
    assert!(shapes.contains(&MindmapShape::Rect));
    assert!(shapes.contains(&MindmapShape::RoundedRect));
    assert!(shapes.contains(&MindmapShape::Circle));
    assert!(shapes.contains(&MindmapShape::Cloud));
    assert!(shapes.contains(&MindmapShape::Bang));
    assert!(shapes.contains(&MindmapShape::Hexagon));
}

#[test]
fn mindmap_single_node() {
    let d = mindmap_parser::parse(&read_golden("mindmap", "mindmap_single")).unwrap();
    assert_eq!(d.root.text, "Just One Node");
    assert!(d.root.children.is_empty());
    assert_eq!(d.root.count(), 1);
}

// ── Quadrant IR assertions ──────────────────────────────────────────

#[test]
fn quadrant_priority_points() {
    let d = quadrant_parser::parse(&read_golden("quadrant", "quadrant_priority")).unwrap();
    assert_eq!(d.title.as_deref(), Some("Priority Matrix"));
    assert_eq!(d.points.len(), 6);
    assert_eq!(d.quadrants[0].as_deref(), Some("Do First"));
    assert_eq!(d.quadrants[3].as_deref(), Some("Eliminate"));
}

#[test]
fn quadrant_priority_point_values() {
    let d = quadrant_parser::parse(&read_golden("quadrant", "quadrant_priority")).unwrap();
    let bug = d.points.iter().find(|p| p.label == "Critical Bug").unwrap();
    assert!((bug.x - 0.9).abs() < 0.01);
    assert!((bug.y - 0.95).abs() < 0.01);
}

#[test]
fn quadrant_gartner_points() {
    let d = quadrant_parser::parse(&read_golden("quadrant", "quadrant_gartner")).unwrap();
    assert_eq!(d.title.as_deref(), Some("Analytics Platforms"));
    assert_eq!(d.points.len(), 8);
    assert_eq!(d.quadrants[0].as_deref(), Some("Leaders"));
    assert_eq!(d.quadrants[1].as_deref(), Some("Challengers"));
}

// ── Venn IR assertions ──────────────────────────────────────────────

#[test]
fn venn_two_sets_structure() {
    let d = venn_parser::parse(&read_golden("venn", "venn_two_sets")).unwrap();
    assert_eq!(d.title.as_deref(), Some("Skills"));
    assert_eq!(d.sets.len(), 2, "A and B");
    assert_eq!(d.unions.len(), 1, "A,B union");
    assert_eq!(d.sets[0].label, "Frontend");
    assert_eq!(d.sets[1].label, "Backend");
}

#[test]
fn venn_three_sets_unions() {
    let d = venn_parser::parse(&read_golden("venn", "venn_three_sets")).unwrap();
    assert_eq!(d.sets.len(), 3, "A, B, C");
    assert_eq!(d.unions.len(), 3, "A-B, B-C, A-B-C");
    let triple = d.unions.iter().find(|u| u.set_ids.len() == 3).unwrap();
    assert_eq!(triple.label.as_deref(), Some("Core"));
}

#[test]
fn venn_two_sets_sizes() {
    let d = venn_parser::parse(&read_golden("venn", "venn_two_sets")).unwrap();
    assert!((d.sets[0].size - 25.0).abs() < f64::EPSILON);
    assert!((d.sets[1].size - 20.0).abs() < f64::EPSILON);
    assert!((d.unions[0].size - 8.0).abs() < f64::EPSILON);
}

// ── Radar IR assertions ─────────────────────────────────────────────

#[test]
fn radar_skills_axes_and_curves() {
    let d = radar_parser::parse(&read_golden("radar", "radar_skills")).unwrap();
    assert_eq!(d.title.as_deref(), Some("Developer Skills"));
    assert_eq!(d.axes.len(), 6, "Rust, TypeScript, Python, SQL, DevOps, Design");
    assert_eq!(d.curves.len(), 2, "Alice and Bob");
}

#[test]
fn radar_skills_curve_values() {
    let d = radar_parser::parse(&read_golden("radar", "radar_skills")).unwrap();
    let alice = d.curves.iter().find(|c| c.label == "Alice").unwrap();
    assert_eq!(alice.values, vec![8.0, 6.0, 7.0, 5.0, 4.0, 3.0]);
    let bob = d.curves.iter().find(|c| c.label == "Bob").unwrap();
    assert_eq!(bob.values, vec![4.0, 9.0, 5.0, 7.0, 8.0, 6.0]);
}

#[test]
fn radar_performance_explicit_max() {
    let d = radar_parser::parse(&read_golden("radar", "radar_performance")).unwrap();
    assert_eq!(d.axes.len(), 5, "Speed, Comfort, Safety, Economy, Style");
    assert_eq!(d.curves.len(), 3, "Sports Car, Family Sedan, SUV");
    assert!((d.effective_max() - 10.0).abs() < f64::EPSILON, "explicit max 10");
    assert_eq!(d.ticks, 4);
}

// ── Journey IR assertions ───────────────────────────────────────────

#[test]
fn journey_workday_sections_and_tasks() {
    let d = journey_parser::parse(&read_golden("journey", "journey_workday")).unwrap();
    assert_eq!(d.title.as_deref(), Some("My Working Day"));
    assert_eq!(d.sections.len(), 2, "Go to work, Go home");
    assert_eq!(d.sections[0].tasks.len(), 3, "Make tea, Go upstairs, Do work");
    assert_eq!(d.sections[1].tasks.len(), 2, "Go downstairs, Sit down");
}

#[test]
fn journey_workday_scores() {
    let d = journey_parser::parse(&read_golden("journey", "journey_workday")).unwrap();
    assert_eq!(d.sections[0].tasks[0].name, "Make tea");
    assert_eq!(d.sections[0].tasks[0].score, 5);
    assert_eq!(d.sections[0].tasks[2].name, "Do work");
    assert_eq!(d.sections[0].tasks[2].score, 1);
}

#[test]
fn journey_ecommerce_actors() {
    let d = journey_parser::parse(&read_golden("journey", "journey_ecommerce")).unwrap();
    assert_eq!(d.sections.len(), 3, "Browse, Purchase, Delivery");
    let total_tasks: usize = d.sections.iter().map(|s| s.tasks.len()).sum();
    assert_eq!(total_tasks, 9);
    let actors = d.all_actors();
    assert!(actors.contains(&"Customer".to_string()));
    assert!(actors.contains(&"Support".to_string()));
}

// ── Treeview IR assertions ──────────────────────────────────────────

#[test]
fn treeview_project_roots_and_total() {
    let d = treeview_parser::parse(&read_golden("treeview", "treeview_project")).unwrap();
    assert_eq!(d.roots.len(), 4, "src, tests, Cargo.toml, README.md");
    assert_eq!(d.node_count(), 14);
}

#[test]
fn treeview_project_nesting() {
    let d = treeview_parser::parse(&read_golden("treeview", "treeview_project")).unwrap();
    let src = &d.roots[0];
    assert_eq!(src.name, "src");
    assert_eq!(src.children.len(), 3, "main.rs, lib.rs, modules");
}

#[test]
fn treeview_org_structure() {
    let d = treeview_parser::parse(&read_golden("treeview", "treeview_org")).unwrap();
    assert_eq!(d.roots.len(), 1, "Company is single root");
    assert_eq!(d.roots[0].name, "Company");
    assert_eq!(d.roots[0].children.len(), 3, "Engineering, Product, Marketing");
    assert_eq!(d.node_count(), 9);
}

// ── Treemap IR assertions ───────────────────────────────────────────

#[test]
fn treemap_budget_roots_and_values() {
    let d = treemap_parser::parse(&read_golden("treemap", "treemap_budget")).unwrap();
    assert_eq!(d.roots.len(), 3, "Operations, Marketing, R&D");
    let total: f64 = d.roots.iter().map(|r| r.total_value()).sum();
    assert!((total - 2000.0).abs() < f64::EPSILON);
}

#[test]
fn treemap_budget_children() {
    let d = treemap_parser::parse(&read_golden("treemap", "treemap_budget")).unwrap();
    let ops = &d.roots[0];
    assert_eq!(ops.name, "Operations");
    assert_eq!(ops.children.len(), 3, "Salaries, Equipment, Supplies");
    assert!((ops.total_value() - 1000.0).abs() < f64::EPSILON);
}

#[test]
fn treemap_disk_mixed_leaves() {
    let d = treemap_parser::parse(&read_golden("treemap", "treemap_disk")).unwrap();
    assert_eq!(d.roots.len(), 4, "Documents, Media, Code, System");
    // Documents is a leaf with value 300
    assert!(d.roots[0].is_leaf());
    assert!((d.roots[0].total_value() - 300.0).abs() < f64::EPSILON);
    // Media is a section with children
    assert!(!d.roots[1].is_leaf());
    assert_eq!(d.roots[1].children.len(), 3);
}

// ── Block IR assertions ─────────────────────────────────────────────

#[test]
fn block_grid_blocks_and_edges() {
    let d = block_parser::parse(&read_golden("block", "block_grid")).unwrap();
    assert_eq!(d.columns, 3);
    // Frontend, API Gateway, Auth Service, Database, (space), Cache = 5 real blocks + 1 space
    assert_eq!(d.blocks.len(), 6, "5 named blocks + 1 space");
    assert_eq!(d.edges.len(), 4, "a->b, b->c, b->d, d->e");
}

#[test]
fn block_shapes_variety() {
    use rusty_mermaid_diagrams::block::ir::BlockShape;
    let d = block_parser::parse(&read_golden("block", "block_shapes")).unwrap();
    assert_eq!(d.columns, 4);
    assert_eq!(d.blocks.len(), 6);
    let shapes: Vec<_> = d.blocks.iter().map(|b| b.shape).collect();
    assert!(shapes.contains(&BlockShape::Rect));
    assert!(shapes.contains(&BlockShape::Round));
    assert!(shapes.contains(&BlockShape::Diamond));
    assert!(shapes.contains(&BlockShape::Circle));
}

#[test]
fn block_spanning_columns() {
    let d = block_parser::parse(&read_golden("block", "block_spanning")).unwrap();
    assert_eq!(d.columns, 3);
    let header = d.blocks.iter().find(|b| b.label == "Header").unwrap();
    assert_eq!(header.span, 3, "Header spans all 3 columns");
    let footer = d.blocks.iter().find(|b| b.label == "Footer").unwrap();
    assert_eq!(footer.span, 2, "Footer spans 2 columns");
}

// ── C4 IR assertions ────────────────────────────────────────────────

#[test]
fn c4_context_elements_and_rels() {
    use rusty_mermaid_diagrams::c4::ir::C4Level;
    let d = c4_parser::parse(&read_golden("c4", "c4_context")).unwrap();
    assert_eq!(d.level, C4Level::Context);
    assert_eq!(d.elements.len(), 4, "customer + banking + email + core");
    assert_eq!(d.relationships.len(), 3);
    assert_eq!(d.boundaries.len(), 0);
}

#[test]
fn c4_context_external_flag() {
    let d = c4_parser::parse(&read_golden("c4", "c4_context")).unwrap();
    let email = d.elements.iter().find(|e| e.alias == "email").unwrap();
    assert!(email.external, "E-mail System is System_Ext");
    let banking = d.elements.iter().find(|e| e.alias == "banking").unwrap();
    assert!(!banking.external, "Banking is internal System");
}

#[test]
fn c4_container_boundary() {
    use rusty_mermaid_diagrams::c4::ir::C4Level;
    let d = c4_parser::parse(&read_golden("c4", "c4_container")).unwrap();
    assert_eq!(d.level, C4Level::Container);
    assert_eq!(d.boundaries.len(), 1, "Banking System boundary");
    assert_eq!(d.boundaries[0].alias, "bank");
    // 1 Person + 3 containers inside boundary = 4 elements
    assert_eq!(d.elements.len(), 4);
    assert_eq!(d.relationships.len(), 3);
}

// ── Architecture IR assertions ──────────────────────────────────────

#[test]
fn arch_api_gateway_services_and_edges() {
    let d = arch_parser::parse(&read_golden("architecture", "arch_api_gateway")).unwrap();
    assert_eq!(d.groups.len(), 1, "api group");
    assert_eq!(d.services.len(), 4, "db, disk1, disk2, server");
    assert_eq!(d.edges.len(), 3);
}

#[test]
fn arch_api_gateway_grouping() {
    let d = arch_parser::parse(&read_golden("architecture", "arch_api_gateway")).unwrap();
    // All services belong to api group
    for svc in &d.services {
        assert_eq!(svc.group.as_deref(), Some("api"), "{} should be in api group", svc.id);
    }
}

#[test]
fn arch_network_multiple_groups() {
    let d = arch_parser::parse(&read_golden("architecture", "arch_network")).unwrap();
    assert_eq!(d.groups.len(), 2, "cloud + internal");
    assert_eq!(d.services.len(), 5, "lb, app1, app2, cache, db");
    assert_eq!(d.edges.len(), 5);
    let cloud_svcs: Vec<_> = d.services.iter().filter(|s| s.group.as_deref() == Some("cloud")).collect();
    assert_eq!(cloud_svcs.len(), 3, "lb, app1, app2 in cloud");
}

// ── Gantt IR assertions ─────────────────────────────────────────────

#[test]
fn gantt_basic_sections_and_tasks() {
    let d = gantt_parser::parse(&read_golden("gantt", "gantt_basic")).unwrap();
    assert_eq!(d.title.as_deref(), Some("Project Schedule"));
    assert_eq!(d.sections.len(), 3, "Design, Development, Testing");
    assert_eq!(d.sections[0].tasks.len(), 2, "Wireframes + Mockups");
    assert_eq!(d.sections[1].tasks.len(), 2, "Backend API + Frontend UI");
    assert_eq!(d.sections[2].tasks.len(), 2, "Integration + UAT");
}

#[test]
fn gantt_simple_no_sections() {
    let d = gantt_parser::parse(&read_golden("gantt", "gantt_simple")).unwrap();
    assert_eq!(d.title.as_deref(), Some("Weekly Tasks"));
    assert_eq!(d.sections.len(), 1, "tasks in default unnamed section");
    assert!(d.sections[0].name.is_none());
    assert_eq!(d.sections[0].tasks.len(), 4, "Research, Writing, Review, Publish");
}

#[test]
fn gantt_dependencies_task_tags() {
    use rusty_mermaid_diagrams::gantt::ir::TaskTag;
    let d = gantt_parser::parse(&read_golden("gantt", "gantt_dependencies")).unwrap();
    assert_eq!(d.sections.len(), 3, "Sprint 1, Sprint 2, Milestones");
    let design = &d.sections[0].tasks[0];
    assert!(design.tags.contains(&TaskTag::Done), "Design should be done");
    let dev = &d.sections[1].tasks[0];
    assert!(dev.tags.contains(&TaskTag::Crit), "Development should be crit");
}

// ── Gitgraph IR assertions ──────────────────────────────────────────

#[test]
fn gitgraph_basic_commits() {
    use rusty_mermaid_diagrams::gitgraph::ir::GitStatement;
    let d = gitgraph_parser::parse(&read_golden("gitgraph", "git_basic")).unwrap();
    let commits: Vec<_> = d.statements.iter().filter(|s| matches!(s, GitStatement::Commit { .. })).collect();
    assert_eq!(commits.len(), 6, "6 commit statements");
    let merges: Vec<_> = d.statements.iter().filter(|s| matches!(s, GitStatement::Merge { .. })).collect();
    assert_eq!(merges.len(), 1, "1 merge");
}

#[test]
fn gitgraph_feature_branches() {
    use rusty_mermaid_diagrams::gitgraph::ir::GitStatement;
    let d = gitgraph_parser::parse(&read_golden("gitgraph", "git_feature_branches")).unwrap();
    let branches: Vec<_> = d.statements.iter().filter_map(|s| {
        if let GitStatement::Branch { name, .. } = s { Some(name.as_str()) } else { None }
    }).collect();
    assert_eq!(branches.len(), 2, "feature-auth + feature-api");
    assert!(branches.contains(&"feature-auth"));
    assert!(branches.contains(&"feature-api"));
}

#[test]
fn gitgraph_commit_types() {
    use rusty_mermaid_diagrams::gitgraph::ir::{GitStatement, CommitType};
    let d = gitgraph_parser::parse(&read_golden("gitgraph", "git_commit_types")).unwrap();
    let commits: Vec<_> = d.statements.iter().filter_map(|s| {
        if let GitStatement::Commit { commit_type, .. } = s { Some(*commit_type) } else { None }
    }).collect();
    assert!(commits.contains(&CommitType::Normal));
    assert!(commits.contains(&CommitType::Reverse));
    assert!(commits.contains(&CommitType::Highlight));
}

// ── Timeline IR assertions ──────────────────────────────────────────

#[test]
fn timeline_basic_periods() {
    let d = timeline_parser::parse(&read_golden("timeline", "timeline_basic")).unwrap();
    assert_eq!(d.title.as_deref(), Some("History of Computing"));
    assert_eq!(d.sections.len(), 1, "single default section");
    assert_eq!(d.sections[0].tasks.len(), 5, "1940s through 1980s");
    assert_eq!(d.sections[0].tasks[0].name, "1940s");
    assert_eq!(d.sections[0].tasks[0].events, vec!["ENIAC", "Colossus"]);
}

#[test]
fn timeline_sections_structure() {
    let d = timeline_parser::parse(&read_golden("timeline", "timeline_sections")).unwrap();
    assert_eq!(d.title.as_deref(), Some("Company History"));
    assert_eq!(d.sections.len(), 3, "Foundation, Growth, Maturity");
    assert_eq!(d.sections[0].name.as_deref(), Some("Foundation"));
    assert_eq!(d.sections[0].tasks.len(), 2);
    assert_eq!(d.sections[1].tasks.len(), 2);
    assert_eq!(d.sections[2].tasks.len(), 2);
}

#[test]
fn timeline_simple_no_title() {
    let d = timeline_parser::parse(&read_golden("timeline", "timeline_simple")).unwrap();
    assert!(d.title.is_none());
    let total_tasks: usize = d.sections.iter().map(|s| s.tasks.len()).sum();
    assert_eq!(total_tasks, 3, "Morning, Afternoon, Evening");
}

// ── Kanban IR assertions ────────────────────────────────────────────

#[test]
fn kanban_basic_columns_and_cards() {
    let d = kanban_parser::parse(&read_golden("kanban", "kanban_basic")).unwrap();
    assert_eq!(d.columns.len(), 4, "Backlog, In Progress, Review, Done");
    assert_eq!(d.columns[0].cards.len(), 3, "3 backlog cards");
    assert_eq!(d.columns[1].cards.len(), 2, "2 in progress");
    assert_eq!(d.columns[2].cards.len(), 1, "1 in review");
    assert_eq!(d.columns[3].cards.len(), 2, "2 done");
}

#[test]
fn kanban_metadata_priorities() {
    use rusty_mermaid_diagrams::kanban::ir::Priority;
    let d = kanban_parser::parse(&read_golden("kanban", "kanban_metadata")).unwrap();
    assert_eq!(d.columns.len(), 3, "Todo, In Progress, Done");
    let t1 = &d.columns[0].cards[0];
    assert_eq!(t1.priority, Some(Priority::High));
    assert_eq!(t1.assigned.as_deref(), Some("alice"));
    assert_eq!(t1.ticket.as_deref(), Some("BUG-101"));
}

#[test]
fn kanban_simple_columns() {
    let d = kanban_parser::parse(&read_golden("kanban", "kanban_simple")).unwrap();
    assert_eq!(d.columns.len(), 3, "To Do, Doing, Done");
    let total_cards: usize = d.columns.iter().map(|c| c.cards.len()).sum();
    assert_eq!(total_cards, 6);
}

// ── XyChart IR assertions ───────────────────────────────────────────

#[test]
fn xychart_bar_series() {
    use rusty_mermaid_diagrams::xychart::ir::PlotType;
    let d = xychart_parser::parse(&read_golden("xychart", "xychart_bar")).unwrap();
    assert_eq!(d.title.as_deref(), Some("Monthly Revenue"));
    assert_eq!(d.plots.len(), 1);
    assert_eq!(d.plots[0].plot_type, PlotType::Bar);
    assert_eq!(d.plots[0].values.len(), 6);
}

#[test]
fn xychart_line_series() {
    use rusty_mermaid_diagrams::xychart::ir::PlotType;
    let d = xychart_parser::parse(&read_golden("xychart", "xychart_line")).unwrap();
    assert_eq!(d.plots.len(), 2, "High and Low temperature lines");
    assert!(d.plots.iter().all(|p| p.plot_type == PlotType::Line));
    assert_eq!(d.plots[0].values.len(), 7, "7 days of data");
}

#[test]
fn xychart_mixed_series() {
    use rusty_mermaid_diagrams::xychart::ir::PlotType;
    let d = xychart_parser::parse(&read_golden("xychart", "xychart_mixed")).unwrap();
    assert_eq!(d.title.as_deref(), Some("Sales vs Target"));
    assert_eq!(d.plots.len(), 2);
    let has_bar = d.plots.iter().any(|p| p.plot_type == PlotType::Bar);
    let has_line = d.plots.iter().any(|p| p.plot_type == PlotType::Line);
    assert!(has_bar, "should have bar series");
    assert!(has_line, "should have line series");
}

// ── Ishikawa IR assertions ──────────────────────────────────────────

#[test]
fn ishikawa_quality_categories() {
    let d = ishikawa_parser::parse(&read_golden("ishikawa", "ishikawa_quality")).unwrap();
    assert_eq!(d.effect, "Low Quality Output");
    assert_eq!(d.categories.len(), 5, "People, Process, Equipment, Materials, Environment");
    assert_eq!(d.categories[0].name, "People");
    assert_eq!(d.categories[0].causes.len(), 3, "Lack of training, High turnover, Fatigue");
}

#[test]
fn ishikawa_quality_subcauses() {
    let d = ishikawa_parser::parse(&read_golden("ishikawa", "ishikawa_quality")).unwrap();
    let equipment = d.categories.iter().find(|c| c.name == "Equipment").unwrap();
    assert_eq!(equipment.causes.len(), 2, "Outdated tools, Calibration drift");
    let cal = equipment.causes.iter().find(|c| c.name == "Calibration drift").unwrap();
    assert_eq!(cal.subcauses.len(), 2, "Sensor age, No maintenance");
    assert_eq!(equipment.total_causes(), 4, "2 causes + 2 subcauses");
}

#[test]
fn ishikawa_bug_effect_and_categories() {
    let d = ishikawa_parser::parse(&read_golden("ishikawa", "ishikawa_bug")).unwrap();
    assert_eq!(d.effect, "Production Bug");
    assert_eq!(d.categories.len(), 3, "Code, Testing, Deploy");
    for cat in &d.categories {
        assert_eq!(cat.causes.len(), 2, "each category has 2 causes");
    }
}
