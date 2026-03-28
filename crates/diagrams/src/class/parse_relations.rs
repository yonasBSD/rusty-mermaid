use winnow::prelude::*;

use super::ir::*;
use super::parser::{class_identifier, skip_horizontal_ws, take_to_eol};

pub(super) fn parse_relationship(input: &mut &str) -> ModalResult<ClassRelation> {
    let from_id = class_identifier(input)?;

    // Optional cardinality before operator: "1"
    skip_horizontal_ws(input);
    let card_from = parse_opt_cardinality(input);
    skip_horizontal_ws(input);

    // Relationship operator
    let (left_type, line_type, right_type) = parse_rel_operator(input)?;

    // Optional cardinality after operator: "many"
    skip_horizontal_ws(input);
    let card_to = parse_opt_cardinality(input);
    skip_horizontal_ws(input);

    let to_id = class_identifier(input)?;

    // Optional label after colon
    skip_horizontal_ws(input);
    let label = if input.starts_with(':') {
        ':'.parse_next(input)?;
        skip_horizontal_ws(input);
        let text = take_to_eol(input);
        if text.is_empty() {
            None
        } else {
            Some(text.to_string())
        }
    } else {
        None
    };

    Ok(ClassRelation {
        from_id: from_id.to_string(),
        to_id: to_id.to_string(),
        from_type: left_type,
        to_type: right_type,
        line_type,
        label,
        cardinality_from: card_from,
        cardinality_to: card_to,
    })
}

fn parse_rel_operator(
    input: &mut &str,
) -> ModalResult<(Option<RelationType>, LineType, Option<RelationType>)> {
    // Left marker (optional)
    let left = parse_rel_marker(input);

    // Line type: -- or ..
    let line_type = if input.starts_with("--") {
        "--".parse_next(input)?;
        LineType::Solid
    } else if input.starts_with("..") {
        "..".parse_next(input)?;
        LineType::Dotted
    } else {
        return Err(winnow::error::ErrMode::Backtrack(
            winnow::error::ContextError::new(),
        ));
    };

    // Right marker (optional)
    let right = parse_rel_marker(input);

    // Must have at least a line
    Ok((left, line_type, right))
}

fn parse_rel_marker(input: &mut &str) -> Option<RelationType> {
    if input.starts_with("<|") || input.starts_with("|>") {
        *input = &input[2..];
        Some(RelationType::Extension)
    } else if input.starts_with("()") {
        *input = &input[2..];
        Some(RelationType::Lollipop)
    } else if input.starts_with('*') {
        *input = &input[1..];
        Some(RelationType::Composition)
    } else if input.starts_with('o') && !input[1..].starts_with(|c: char| c.is_alphanumeric()) {
        *input = &input[1..];
        Some(RelationType::Aggregation)
    } else if input.starts_with('>') || input.starts_with('<') {
        *input = &input[1..];
        Some(RelationType::Dependency)
    } else {
        None
    }
}

pub(super) fn parse_opt_cardinality(input: &mut &str) -> Option<String> {
    if !input.starts_with('"') {
        return None;
    }
    *input = &input[1..]; // skip opening "
    let end = input.find('"')?;
    let text = &input[..end];
    *input = &input[end + 1..]; // skip closing "
    Some(text.to_string())
}
