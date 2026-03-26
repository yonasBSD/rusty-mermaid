use super::*;

#[test]
fn parse_empty() {
    let d = parse("requirementDiagram").unwrap();
    assert!(d.requirements.is_empty());
}

#[test]
fn parse_requirement_block() {
    let d = parse("requirementDiagram\n    requirement REQ_01 {\n        id: SYS_001\n        text: \"Must process data\"\n        risk: high\n        verifymethod: test\n    }").unwrap();
    assert_eq!(d.requirements.len(), 1);
    let r = &d.requirements[0];
    assert_eq!(r.name, "REQ_01");
    assert_eq!(r.req_type, RequirementType::Requirement);
    assert_eq!(r.id.as_deref(), Some("SYS_001"));
    assert_eq!(r.text.as_deref(), Some("Must process data"));
    assert_eq!(r.risk, Some(RiskLevel::High));
    assert_eq!(r.verify_method, Some(VerifyMethod::Test));
}

#[test]
fn parse_functional_requirement() {
    let d = parse("requirementDiagram\n    functionalRequirement FR_01 {\n        id: FR_001\n        text: \"UI response\"\n        risk: medium\n        verifymethod: analysis\n    }").unwrap();
    assert_eq!(
        d.requirements[0].req_type,
        RequirementType::FunctionalRequirement
    );
}

#[test]
fn parse_design_constraint() {
    let d = parse("requirementDiagram\n    designConstraint DC_01 {\n        id: DC_001\n        text: \"Budget limit\"\n        risk: low\n        verifymethod: inspection\n    }").unwrap();
    assert_eq!(
        d.requirements[0].req_type,
        RequirementType::DesignConstraint
    );
}

#[test]
fn parse_element() {
    let d = parse("requirementDiagram\n    element COMP_01 {\n        type: Component\n        docref: \"spec.pdf\"\n    }").unwrap();
    assert_eq!(d.elements.len(), 1);
    assert_eq!(d.elements[0].name, "COMP_01");
    assert_eq!(d.elements[0].elem_type.as_deref(), Some("Component"));
    assert_eq!(d.elements[0].docref.as_deref(), Some("spec.pdf"));
}

#[test]
fn parse_relationship_forward() {
    let d = parse("requirementDiagram\n    requirement A {\n        id: A\n    }\n    requirement B {\n        id: B\n    }\n    A - contains -> B").unwrap();
    assert_eq!(d.relationships.len(), 1);
    assert_eq!(d.relationships[0].src, "A");
    assert_eq!(d.relationships[0].dst, "B");
    assert_eq!(d.relationships[0].rel_type, RelationshipType::Contains);
}

#[test]
fn parse_relationship_reverse() {
    let d = parse("requirementDiagram\n    requirement A {\n        id: A\n    }\n    requirement B {\n        id: B\n    }\n    B <- satisfies - A").unwrap();
    assert_eq!(d.relationships[0].src, "A");
    assert_eq!(d.relationships[0].dst, "B");
    assert_eq!(d.relationships[0].rel_type, RelationshipType::Satisfies);
}

#[test]
fn parse_all_relationship_types() {
    for (keyword, expected) in [
        ("contains", RelationshipType::Contains),
        ("copies", RelationshipType::Copies),
        ("derives", RelationshipType::Derives),
        ("satisfies", RelationshipType::Satisfies),
        ("verifies", RelationshipType::Verifies),
        ("refines", RelationshipType::Refines),
        ("traces", RelationshipType::Traces),
    ] {
        let input = format!(
            "requirementDiagram\n    requirement A {{\n        id: A\n    }}\n    requirement B {{\n        id: B\n    }}\n    A - {keyword} -> B"
        );
        let d = parse(&input).unwrap();
        assert_eq!(
            d.relationships[0].rel_type, expected,
            "failed for {keyword}"
        );
    }
}

#[test]
fn parse_direction() {
    let d = parse("requirementDiagram\n    direction LR").unwrap();
    assert_eq!(d.direction, Direction::LR);
}

#[test]
fn parse_css_class() {
    let d = parse("requirementDiagram\n    requirement REQ:::highlight {\n        id: R1\n    }")
        .unwrap();
    assert_eq!(d.requirements[0].css_classes, vec!["highlight"]);
}

#[test]
fn parse_comments() {
    let d = parse("requirementDiagram\n    %% comment\n    requirement R {\n        id: R1\n    }")
        .unwrap();
    assert_eq!(d.requirements.len(), 1);
}

#[test]
fn reject_empty() {
    assert!(parse("").is_err());
}

#[test]
fn reject_wrong_header() {
    assert!(parse("classDiagram\n    class Foo").is_err());
}
