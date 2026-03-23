use winnow::combinator::{alt, opt};
use winnow::prelude::*;
use winnow::token::take_while;

use rusty_mermaid_core::Direction;

use crate::common::error::{ParseError, ParseErrorKind};
use crate::common::styling::{class_def_body, style_stmt_body};
use crate::common::tokens::{identifier, skip, ws};

use super::ir::*;

/// Parse a mermaid requirement diagram string into IR.
pub fn parse(input: &str) -> Result<RequirementDiagram, ParseError> {
    let mut rest = input;
    parse_req_diagram(&mut rest).map_err(|_| {
        let offset = input.len() - rest.len();
        ParseError::new(ParseErrorKind::UnexpectedToken, offset..offset, input)
    })
}

fn parse_req_diagram(input: &mut &str) -> ModalResult<RequirementDiagram> {
    skip.parse_next(input)?;
    "requirementDiagram".parse_next(input)?;
    skip.parse_next(input)?;

    let mut diagram = RequirementDiagram::new(Direction::TB);
    parse_statements(input, &mut diagram)?;
    Ok(diagram)
}

fn parse_statements(input: &mut &str, diagram: &mut RequirementDiagram) -> ModalResult<()> {
    loop {
        skip.parse_next(input)?;
        if input.is_empty() { break; }
        if !try_parse_statement(input, diagram)? {
            if !input.is_empty() {
                *input = &input[1..];
            }
        }
    }
    Ok(())
}

fn try_parse_statement(input: &mut &str, diagram: &mut RequirementDiagram) -> ModalResult<bool> {
    // classDef
    if input.starts_with("classDef") {
        let cp = *input;
        *input = &input[8..];
        if let Ok(cd) = class_def_body.parse_next(input) {
            diagram.class_defs.push(cd);
            return Ok(true);
        }
        *input = cp;
    }

    // style
    if input.starts_with("style ") {
        let cp = *input;
        *input = &input[5..];
        if let Ok(ss) = style_stmt_body.parse_next(input) {
            diagram.style_stmts.push(ss);
            return Ok(true);
        }
        *input = cp;
    }

    // direction
    if input.starts_with("direction") {
        let cp = *input;
        *input = &input[9..];
        if let Ok(_) = ws.parse_next(input) {
            if let Ok(dir) = parse_direction(input) {
                diagram.direction = dir;
                return Ok(true);
            }
        }
        *input = cp;
    }

    // requirement/element block
    if let Some(req_type) = try_parse_req_type(input) {
        let cp = *input;
        if let Ok(req) = parse_requirement_block(input, req_type) {
            diagram.requirements.push(req);
            return Ok(true);
        }
        *input = cp;
    }

    if input.starts_with("element") {
        let cp = *input;
        *input = &input[7..];
        if let Ok(elem) = parse_element_block(input) {
            diagram.elements.push(elem);
            return Ok(true);
        }
        *input = cp;
    }

    // relationship: A - type -> B  or  B <- type - A
    {
        let cp = *input;
        if let Ok(rel) = parse_relationship(input) {
            diagram.relationships.push(rel);
            return Ok(true);
        }
        *input = cp;
    }

    Ok(false)
}

fn try_parse_req_type(input: &mut &str) -> Option<RequirementType> {
    for (prefix, rt) in [
        ("functionalRequirement", RequirementType::FunctionalRequirement),
        ("interfaceRequirement", RequirementType::InterfaceRequirement),
        ("performanceRequirement", RequirementType::PerformanceRequirement),
        ("physicalRequirement", RequirementType::PhysicalRequirement),
        ("designConstraint", RequirementType::DesignConstraint),
        ("requirement", RequirementType::Requirement),
    ] {
        if input.starts_with(prefix) {
            let after = &input[prefix.len()..];
            if after.starts_with(|c: char| c == ' ' || c == '\t' || c == ':') {
                *input = &input[prefix.len()..];
                return Some(rt);
            }
        }
    }
    None
}

fn parse_requirement_block(input: &mut &str, req_type: RequirementType) -> ModalResult<Requirement> {
    ws.parse_next(input)?;
    let name = req_identifier(input)?;
    let mut req = Requirement::new(name, req_type);

    // Optional :::cssClass
    skip_horizontal_ws(input);
    if input.starts_with(":::") {
        *input = &input[3..];
        let cls = identifier.parse_next(input)?;
        req.css_classes.push(cls.to_string());
    }

    skip.parse_next(input)?;
    '{'.parse_next(input)?;

    loop {
        skip.parse_next(input)?;
        if input.is_empty() || input.starts_with('}') { break; }

        let line = take_line(input);
        if let Some((key, val)) = parse_kv(line) {
            match key {
                "id" => req.id = Some(val.to_string()),
                "text" => req.text = Some(strip_quotes(val)),
                "risk" => req.risk = parse_risk(val),
                "verifymethod" => req.verify_method = parse_verify(val),
                _ => {}
            }
        }
    }
    opt('}').parse_next(input)?;

    Ok(req)
}

fn parse_element_block(input: &mut &str) -> ModalResult<DesignElement> {
    ws.parse_next(input)?;
    let name = req_identifier(input)?;
    let mut elem = DesignElement::new(name);

    skip_horizontal_ws(input);
    if input.starts_with(":::") {
        *input = &input[3..];
        let cls = identifier.parse_next(input)?;
        elem.css_classes.push(cls.to_string());
    }

    skip.parse_next(input)?;
    '{'.parse_next(input)?;

    loop {
        skip.parse_next(input)?;
        if input.is_empty() || input.starts_with('}') { break; }

        let line = take_line(input);
        if let Some((key, val)) = parse_kv(line) {
            match key {
                "type" => elem.elem_type = Some(val.to_string()),
                "docref" => elem.docref = Some(strip_quotes(val)),
                _ => {}
            }
        }
    }
    opt('}').parse_next(input)?;

    Ok(elem)
}

fn parse_relationship(input: &mut &str) -> ModalResult<ReqRelation> {
    let first = req_identifier(input)?;
    skip_horizontal_ws(input);

    // Forward: A - type -> B
    if input.starts_with('-') {
        '-'.parse_next(input)?;
        skip_horizontal_ws(input);
        let rel_type = parse_rel_type(input)?;
        skip_horizontal_ws(input);
        "->".parse_next(input)?;
        skip_horizontal_ws(input);
        let second = req_identifier(input)?;
        return Ok(ReqRelation {
            src: first.to_string(),
            dst: second.to_string(),
            rel_type,
        });
    }

    // Reverse: B <- type - A
    if input.starts_with("<-") {
        "<-".parse_next(input)?;
        skip_horizontal_ws(input);
        let rel_type = parse_rel_type(input)?;
        skip_horizontal_ws(input);
        '-'.parse_next(input)?;
        skip_horizontal_ws(input);
        let second = req_identifier(input)?;
        return Ok(ReqRelation {
            src: second.to_string(),
            dst: first.to_string(),
            rel_type,
        });
    }

    Err(winnow::error::ErrMode::Backtrack(winnow::error::ContextError::new()))
}

fn parse_rel_type(input: &mut &str) -> ModalResult<RelationshipType> {
    for (keyword, rt) in [
        ("contains", RelationshipType::Contains),
        ("copies", RelationshipType::Copies),
        ("derives", RelationshipType::Derives),
        ("satisfies", RelationshipType::Satisfies),
        ("verifies", RelationshipType::Verifies),
        ("refines", RelationshipType::Refines),
        ("traces", RelationshipType::Traces),
    ] {
        if input.starts_with(keyword) {
            let after = &input[keyword.len()..];
            if after.is_empty() || after.starts_with(|c: char| c == ' ' || c == '\t' || c == '-') {
                *input = &input[keyword.len()..];
                return Ok(rt);
            }
        }
    }
    Err(winnow::error::ErrMode::Backtrack(winnow::error::ContextError::new()))
}

// ── Helpers ──

fn parse_direction(input: &mut &str) -> ModalResult<Direction> {
    alt((
        "TB".value(Direction::TB),
        "TD".value(Direction::TB),
        "BT".value(Direction::BT),
        "LR".value(Direction::LR),
        "RL".value(Direction::RL),
    )).parse_next(input)
}

fn parse_kv(line: &str) -> Option<(&str, &str)> {
    let (key, rest) = line.split_once(':')?;
    Some((key.trim(), rest.trim()))
}

fn parse_risk(s: &str) -> Option<RiskLevel> {
    match s.trim().to_ascii_lowercase().as_str() {
        "low" => Some(RiskLevel::Low),
        "medium" => Some(RiskLevel::Medium),
        "high" => Some(RiskLevel::High),
        _ => None,
    }
}

fn parse_verify(s: &str) -> Option<VerifyMethod> {
    match s.trim().to_ascii_lowercase().as_str() {
        "analysis" => Some(VerifyMethod::Analysis),
        "demonstration" => Some(VerifyMethod::Demonstration),
        "inspection" => Some(VerifyMethod::Inspection),
        "test" => Some(VerifyMethod::Test),
        _ => None,
    }
}

fn strip_quotes(s: &str) -> String {
    s.trim().trim_matches('"').to_string()
}

fn req_identifier<'i>(input: &mut &'i str) -> ModalResult<&'i str> {
    take_while(1.., |c: char| c.is_alphanumeric() || c == '_' || c == '-')
        .parse_next(input)
}

fn skip_horizontal_ws(input: &mut &str) {
    *input = input.trim_start_matches(|c: char| c == ' ' || c == '\t');
}

fn take_line<'i>(input: &mut &'i str) -> &'i str {
    let end = input.find('\n').unwrap_or(input.len());
    let line = input[..end].trim();
    *input = if end < input.len() { &input[end + 1..] } else { "" };
    line
}

#[cfg(test)]
mod tests {
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
        assert_eq!(d.requirements[0].req_type, RequirementType::FunctionalRequirement);
    }

    #[test]
    fn parse_design_constraint() {
        let d = parse("requirementDiagram\n    designConstraint DC_01 {\n        id: DC_001\n        text: \"Budget limit\"\n        risk: low\n        verifymethod: inspection\n    }").unwrap();
        assert_eq!(d.requirements[0].req_type, RequirementType::DesignConstraint);
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
            let input = format!("requirementDiagram\n    requirement A {{\n        id: A\n    }}\n    requirement B {{\n        id: B\n    }}\n    A - {keyword} -> B");
            let d = parse(&input).unwrap();
            assert_eq!(d.relationships[0].rel_type, expected, "failed for {keyword}");
        }
    }

    #[test]
    fn parse_direction() {
        let d = parse("requirementDiagram\n    direction LR").unwrap();
        assert_eq!(d.direction, Direction::LR);
    }

    #[test]
    fn parse_css_class() {
        let d = parse("requirementDiagram\n    requirement REQ:::highlight {\n        id: R1\n    }").unwrap();
        assert_eq!(d.requirements[0].css_classes, vec!["highlight"]);
    }

    #[test]
    fn parse_comments() {
        let d = parse("requirementDiagram\n    %% comment\n    requirement R {\n        id: R1\n    }").unwrap();
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
}
