//! IR content assertions: verify parsed structure matches expected
//! node counts, edge counts, shapes, and connectivity.

use rusty_mermaid_core::Shape;
use rusty_mermaid_diagrams::class::parser as class_parser;
use rusty_mermaid_diagrams::er::parser as er_parser;
use rusty_mermaid_diagrams::requirement::parser as req_parser;
use rusty_mermaid_diagrams::flowchart::parser as flowchart_parser;
use rusty_mermaid_diagrams::sequence::parser as sequence_parser;
use rusty_mermaid_diagrams::state::parser as state_parser;

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
