use winnow::combinator::{alt, opt};
use winnow::prelude::*;
use winnow::token::take_while;

use rusty_mermaid_core::Direction;

use crate::common::error::{ParseError, ParseErrorKind};
use crate::common::styling::{class_apply_body, class_def_body, style_stmt_body};
use crate::common::tokens::{identifier, skip, ws};

use super::ir::*;

/// Parse a mermaid class diagram string into IR.
pub fn parse(input: &str) -> Result<ClassDiagram, ParseError> {
    let mut rest = input;
    parse_class_diagram(&mut rest).map_err(|_| {
        let offset = input.len() - rest.len();
        ParseError::new(ParseErrorKind::UnexpectedToken, offset..offset, input)
    })
}

fn parse_class_diagram(input: &mut &str) -> ModalResult<ClassDiagram> {
    skip.parse_next(input)?;
    header(input)?;
    skip.parse_next(input)?;

    let mut diagram = ClassDiagram::new(Direction::TB);
    parse_statements(input, &mut diagram, None)?;
    Ok(diagram)
}

fn header(input: &mut &str) -> ModalResult<()> {
    alt(("classDiagram-v2", "classDiagram")).parse_next(input)?;
    Ok(())
}

fn parse_statements(
    input: &mut &str,
    diagram: &mut ClassDiagram,
    namespace: Option<&str>,
) -> ModalResult<()> {
    loop {
        skip.parse_next(input)?;
        if input.is_empty() || input.starts_with('}') {
            break;
        }
        if !try_parse_statement(input, diagram, namespace)? {
            if !input.is_empty() {
                *input = &input[1..];
            }
        }
    }
    Ok(())
}

fn try_parse_statement(
    input: &mut &str,
    diagram: &mut ClassDiagram,
    namespace: Option<&str>,
) -> ModalResult<bool> {
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

    // style statement
    if input.starts_with("style ") {
        let cp = *input;
        *input = &input[5..];
        if let Ok(ss) = style_stmt_body.parse_next(input) {
            diagram.style_stmts.push(ss);
            return Ok(true);
        }
        *input = cp;
    }

    // class apply (cssClass)
    if input.starts_with("class ") && !peek_class_declaration(input) {
        let cp = *input;
        *input = &input[5..];
        if let Ok(ca) = class_apply_body.parse_next(input) {
            for id in &ca.ids {
                ensure_class(&mut diagram.classes, id, namespace);
                if let Some(c) = diagram.classes.iter_mut().find(|c| c.id == *id) {
                    c.css_classes.push(ca.class_name.clone());
                }
            }
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

    // namespace
    if input.starts_with("namespace") {
        let cp = *input;
        if let Ok(()) = parse_namespace(input, diagram) {
            return Ok(true);
        }
        *input = cp;
    }

    // note
    if input.starts_with("note") {
        let cp = *input;
        if let Ok(note) = parse_note(input) {
            diagram.notes.push(note);
            return Ok(true);
        }
        *input = cp;
    }

    // annotation: <<interface>> ClassName
    if input.starts_with("<<") {
        let cp = *input;
        if let Ok((annotation, class_id)) = parse_annotation(input) {
            ensure_class(&mut diagram.classes, &class_id, namespace);
            if let Some(c) = diagram.classes.iter_mut().find(|c| c.id == class_id) {
                c.annotations.push(annotation);
            }
            return Ok(true);
        }
        *input = cp;
    }

    // class declaration: `class Name ...`
    if input.starts_with("class ") {
        let cp = *input;
        if let Ok(mut node) = parse_class_declaration(input) {
            node.namespace = namespace.map(String::from);
            upsert_class(&mut diagram.classes, node);
            return Ok(true);
        }
        *input = cp;
    }

    // relationship: A <|-- B : label
    {
        let cp = *input;
        if let Ok(rel) = parse_relationship(input) {
            ensure_class(&mut diagram.classes, &rel.from_id, namespace);
            ensure_class(&mut diagram.classes, &rel.to_id, namespace);
            diagram.relationships.push(rel);
            return Ok(true);
        }
        *input = cp;
    }

    // colon member: ClassName : member
    {
        let cp = *input;
        if let Ok((class_id, member)) = parse_colon_member(input) {
            ensure_class(&mut diagram.classes, &class_id, namespace);
            if let Some(c) = diagram.classes.iter_mut().find(|c| c.id == class_id) {
                if member.is_method() {
                    c.methods.push(member);
                } else {
                    c.members.push(member);
                }
            }
            return Ok(true);
        }
        *input = cp;
    }

    Ok(false)
}

/// Peek: is this `class Name` (declaration) vs `class id1,id2 className` (apply)?
fn peek_class_declaration(input: &str) -> bool {
    let rest = input.strip_prefix("class ").unwrap_or("");
    let rest = rest.trim_start();
    // Declaration starts with identifier optionally followed by ~, [, {, <<, or newline
    // Apply starts with identifier(s) followed by a class name (no special chars)
    // Heuristic: if after first identifier there's ~, [, {, <<, newline, or EOF → declaration
    let end = rest.find(|c: char| !c.is_alphanumeric() && c != '_' && c != '-').unwrap_or(rest.len());
    if end >= rest.len() { return true; }
    let next = rest[end..].trim_start();
    next.is_empty()
        || next.starts_with('~')
        || next.starts_with('[')
        || next.starts_with('{')
        || next.starts_with("<<")
        || next.starts_with('\n')
        || next.starts_with('\r')
        || next.starts_with(":::")
}

// ── Class declaration ──

fn parse_class_declaration(input: &mut &str) -> ModalResult<ClassNode> {
    "class".parse_next(input)?;
    ws.parse_next(input)?;

    let id = class_identifier(input)?;
    let mut node = ClassNode::new(id);

    // Optional generic: ~T~ or ~KeyType, ValueType~
    if input.starts_with('~') {
        '~'.parse_next(input)?;
        let generic = take_while(1.., |c: char| c != '~').parse_next(input)?;
        '~'.parse_next(input)?;
        node.generic_type = Some(generic.to_string());
    }

    // Optional label: ["Display Label"]
    skip_horizontal_ws(input);
    if input.starts_with("[\"") {
        "[\"".parse_next(input)?;
        let label = take_while(0.., |c: char| c != '"').parse_next(input)?;
        "\"]".parse_next(input)?;
        node.label = Some(label.to_string());
    }

    // Optional inline annotation: <<interface>>
    skip_horizontal_ws(input);
    if input.starts_with("<<") {
        "<<".parse_next(input)?;
        let ann = take_while(1.., |c: char| c != '>').parse_next(input)?;
        ">>".parse_next(input)?;
        node.annotations.push(ann.to_string());
    }

    // Optional :::cssClass
    skip_horizontal_ws(input);
    if input.starts_with(":::") {
        ":::".parse_next(input)?;
        let cls = identifier.parse_next(input)?;
        node.css_classes.push(cls.to_string());
    }

    // Optional body block: { ... }
    skip_horizontal_ws(input);
    if input.starts_with('{') {
        '{'.parse_next(input)?;
        parse_class_body(input, &mut node)?;
        skip.parse_next(input)?;
        opt('}').parse_next(input)?;
    }

    Ok(node)
}

fn parse_class_body(input: &mut &str, node: &mut ClassNode) -> ModalResult<()> {
    loop {
        skip.parse_next(input)?;
        if input.is_empty() || input.starts_with('}') {
            break;
        }

        // Annotation inside body
        if input.starts_with("<<") {
            "<<".parse_next(input)?;
            let ann = take_while(1.., |c: char| c != '>').parse_next(input)?;
            ">>".parse_next(input)?;
            node.annotations.push(ann.to_string());
            continue;
        }

        // Separator lines: --, .., ==, __
        if is_separator_line(input) {
            skip_to_eol(input);
            continue;
        }

        // Member line
        let line = take_line(input);
        if line.is_empty() { continue; }
        let member = parse_member_string(line);
        if member.is_method() {
            node.methods.push(member);
        } else {
            node.members.push(member);
        }
    }
    Ok(())
}

fn is_separator_line(input: &str) -> bool {
    input.starts_with("--") && !input.starts_with("-->")
        || input.starts_with("..")
        || input.starts_with("==")
        || input.starts_with("__")
}

// ── Member parsing ──

/// Parse a member string like `+getName(id int) String` or `-count int`.
pub fn parse_member_string(raw: &str) -> ClassMember {
    let s = raw.trim();
    if s.is_empty() {
        return ClassMember::attribute("");
    }

    let mut chars = s.chars().peekable();

    // Visibility prefix
    let visibility = chars.peek()
        .and_then(|&c| Visibility::from_char(c))
        .unwrap_or(Visibility::None);
    if visibility != Visibility::None {
        chars.next();
    }

    let rest: String = chars.collect();
    let rest = rest.trim();

    // Classifier suffix
    let (rest, classifier) = if rest.ends_with('$') {
        (&rest[..rest.len()-1], Classifier::Static)
    } else if rest.ends_with('*') {
        (&rest[..rest.len()-1], Classifier::Abstract)
    } else {
        (rest, Classifier::None)
    };

    // Method: has parentheses
    if let Some(paren_start) = rest.find('(') {
        if let Some(paren_end) = rest[paren_start..].find(')') {
            let name = rest[..paren_start].trim().to_string();
            let params = rest[paren_start+1..paren_start+paren_end].trim().to_string();
            let after = rest[paren_start+paren_end+1..].trim();
            let return_type = if after.is_empty() { None } else { Some(after.to_string()) };
            return ClassMember {
                name,
                visibility,
                classifier,
                return_type,
                parameters: Some(params),
            };
        }
    }

    // Attribute: may have type before or after name
    // Patterns: `type name`, `name`, `name type`
    // Mermaid convention: last word-like token is the name if there are spaces
    let name = rest.trim().to_string();
    ClassMember {
        name,
        visibility,
        classifier,
        return_type: None,
        parameters: None,
    }
}

// ── Relationship ──

fn parse_relationship(input: &mut &str) -> ModalResult<ClassRelation> {
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
        if text.is_empty() { None } else { Some(text.to_string()) }
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

fn parse_rel_operator(input: &mut &str) -> ModalResult<(Option<RelationType>, LineType, Option<RelationType>)> {
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
        return Err(winnow::error::ErrMode::Backtrack(winnow::error::ContextError::new()));
    };

    // Right marker (optional)
    let right = parse_rel_marker(input);

    // Must have at least a line
    Ok((left, line_type, right))
}

fn parse_rel_marker(input: &mut &str) -> Option<RelationType> {
    if input.starts_with("<|") {
        *input = &input[2..];
        Some(RelationType::Extension)
    } else if input.starts_with("|>") {
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
    } else if input.starts_with('>') {
        *input = &input[1..];
        Some(RelationType::Dependency)
    } else if input.starts_with('<') {
        *input = &input[1..];
        Some(RelationType::Dependency)
    } else {
        None
    }
}

fn parse_opt_cardinality(input: &mut &str) -> Option<String> {
    if !input.starts_with('"') { return None; }
    *input = &input[1..]; // skip opening "
    let end = input.find('"')?;
    let text = &input[..end];
    *input = &input[end + 1..]; // skip closing "
    Some(text.to_string())
}

// ── Colon member ──

fn parse_colon_member(input: &mut &str) -> ModalResult<(String, ClassMember)> {
    let id = class_identifier(input)?;
    skip_horizontal_ws(input);
    ':'.parse_next(input)?;
    skip_horizontal_ws(input);
    let line = take_to_eol(input);
    if line.is_empty() {
        return Err(winnow::error::ErrMode::Backtrack(winnow::error::ContextError::new()));
    }
    let member = parse_member_string(line);
    Ok((id.to_string(), member))
}

// ── Annotation ──

fn parse_annotation(input: &mut &str) -> ModalResult<(String, String)> {
    "<<".parse_next(input)?;
    let ann = take_while(1.., |c: char| c != '>').parse_next(input)?;
    ">>".parse_next(input)?;
    skip_horizontal_ws(input);
    let id = class_identifier(input)?;
    Ok((ann.to_string(), id.to_string()))
}

// ── Namespace ──

fn parse_namespace(input: &mut &str, diagram: &mut ClassDiagram) -> ModalResult<()> {
    "namespace".parse_next(input)?;
    ws.parse_next(input)?;
    let ns_id = take_while(1.., |c: char| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
        .parse_next(input)?;
    let ns_id = ns_id.to_string();
    skip.parse_next(input)?;
    '{'.parse_next(input)?;

    let class_count_before = diagram.classes.len();
    parse_statements(input, diagram, Some(&ns_id))?;
    skip.parse_next(input)?;
    opt('}').parse_next(input)?;

    // Collect class IDs that were added inside this namespace
    let class_ids: Vec<String> = diagram.classes[class_count_before..]
        .iter()
        .map(|c| c.id.clone())
        .collect();

    diagram.namespaces.push(Namespace {
        id: ns_id,
        parent: None,
        class_ids,
    });

    Ok(())
}

// ── Note ──

fn parse_note(input: &mut &str) -> ModalResult<ClassNote> {
    "note".parse_next(input)?;
    ws.parse_next(input)?;

    let class_id = if input.starts_with("for ") {
        "for".parse_next(input)?;
        ws.parse_next(input)?;
        let id = class_identifier(input)?;
        skip_horizontal_ws(input);
        Some(id.to_string())
    } else {
        None
    };

    '"'.parse_next(input)?;
    let text = take_while(0.., |c: char| c != '"').parse_next(input)?;
    '"'.parse_next(input)?;

    Ok(ClassNote {
        text: text.to_string(),
        class_id,
    })
}

// ── Direction ──

fn parse_direction(input: &mut &str) -> ModalResult<Direction> {
    alt((
        "TB".value(Direction::TB),
        "TD".value(Direction::TB),
        "BT".value(Direction::BT),
        "LR".value(Direction::LR),
        "RL".value(Direction::RL),
    )).parse_next(input)
}

// ── Helpers ──

fn class_identifier<'i>(input: &mut &'i str) -> ModalResult<&'i str> {
    take_while(1.., |c: char| c.is_alphanumeric() || c == '_' || c == '-')
        .parse_next(input)
}

fn skip_horizontal_ws(input: &mut &str) {
    *input = input.trim_start_matches(|c: char| c == ' ' || c == '\t');
}

fn take_to_eol<'i>(input: &mut &'i str) -> &'i str {
    let end = input.find('\n').unwrap_or(input.len());
    let line = &input[..end];
    *input = &input[end..];
    line.trim()
}

fn take_line<'i>(input: &mut &'i str) -> &'i str {
    let end = input.find('\n').unwrap_or(input.len());
    let line = input[..end].trim();
    *input = if end < input.len() { &input[end+1..] } else { "" };
    line
}

fn skip_to_eol(input: &mut &str) {
    let end = input.find('\n').unwrap_or(input.len());
    *input = if end < input.len() { &input[end+1..] } else { "" };
}

fn ensure_class(classes: &mut Vec<ClassNode>, id: &str, namespace: Option<&str>) {
    if !classes.iter().any(|c| c.id == id) {
        let mut node = ClassNode::new(id);
        node.namespace = namespace.map(String::from);
        classes.push(node);
    }
}

fn upsert_class(classes: &mut Vec<ClassNode>, node: ClassNode) {
    if let Some(existing) = classes.iter_mut().find(|c| c.id == node.id) {
        if node.label.is_some() { existing.label = node.label; }
        if node.generic_type.is_some() { existing.generic_type = node.generic_type; }
        if !node.annotations.is_empty() { existing.annotations.extend(node.annotations); }
        if !node.css_classes.is_empty() { existing.css_classes.extend(node.css_classes); }
        if node.namespace.is_some() { existing.namespace = node.namespace; }
        existing.members.extend(node.members);
        existing.methods.extend(node.methods);
    } else {
        classes.push(node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Positive tests ──

    #[test]
    fn parse_empty_diagram() {
        let d = parse("classDiagram").unwrap();
        assert!(d.classes.is_empty());
        assert!(d.relationships.is_empty());
    }

    #[test]
    fn parse_single_class() {
        let d = parse("classDiagram\n    class Animal").unwrap();
        assert_eq!(d.classes.len(), 1);
        assert_eq!(d.classes[0].id, "Animal");
    }

    #[test]
    fn parse_class_with_generic() {
        let d = parse("classDiagram\n    class List~T~").unwrap();
        assert_eq!(d.classes[0].generic_type.as_deref(), Some("T"));
    }

    #[test]
    fn parse_class_with_multi_generic() {
        let d = parse("classDiagram\n    class Map~KeyType, ValueType~").unwrap();
        assert_eq!(d.classes[0].generic_type.as_deref(), Some("KeyType, ValueType"));
    }

    #[test]
    fn parse_class_with_label() {
        let d = parse("classDiagram\n    class C1[\"My Class\"]").unwrap();
        assert_eq!(d.classes[0].label.as_deref(), Some("My Class"));
    }

    #[test]
    fn parse_class_with_body() {
        let d = parse("classDiagram\n    class Animal {\n        +String name\n        +getAge() int\n    }").unwrap();
        assert_eq!(d.classes[0].members.len(), 1);
        assert_eq!(d.classes[0].methods.len(), 1);
        assert_eq!(d.classes[0].members[0].name, "String name");
        assert_eq!(d.classes[0].methods[0].name, "getAge");
    }

    #[test]
    fn parse_class_body_annotation() {
        let d = parse("classDiagram\n    class Shape {\n        <<interface>>\n        +draw()\n    }").unwrap();
        assert_eq!(d.classes[0].annotations, vec!["interface"]);
        assert_eq!(d.classes[0].methods.len(), 1);
    }

    #[test]
    fn parse_class_inline_annotation() {
        let d = parse("classDiagram\n    class Shape <<abstract>>").unwrap();
        assert_eq!(d.classes[0].annotations, vec!["abstract"]);
    }

    #[test]
    fn parse_standalone_annotation() {
        let d = parse("classDiagram\n    class Shape\n    <<interface>> Shape").unwrap();
        assert_eq!(d.classes[0].annotations, vec!["interface"]);
    }

    #[test]
    fn parse_class_css_class() {
        let d = parse("classDiagram\n    class Foo:::highlight").unwrap();
        assert_eq!(d.classes[0].css_classes, vec!["highlight"]);
    }

    #[test]
    fn parse_visibility_members() {
        let d = parse("classDiagram\n    class Foo {\n        +publicField\n        -privateField\n        #protectedField\n        ~packageField\n    }").unwrap();
        assert_eq!(d.classes[0].members[0].visibility, Visibility::Public);
        assert_eq!(d.classes[0].members[1].visibility, Visibility::Private);
        assert_eq!(d.classes[0].members[2].visibility, Visibility::Protected);
        assert_eq!(d.classes[0].members[3].visibility, Visibility::Package);
    }

    #[test]
    fn parse_classifier_members() {
        let d = parse("classDiagram\n    class Foo {\n        +staticMethod()$\n        +abstractMethod()*\n    }").unwrap();
        assert_eq!(d.classes[0].methods[0].classifier, Classifier::Static);
        assert_eq!(d.classes[0].methods[1].classifier, Classifier::Abstract);
    }

    #[test]
    fn parse_method_with_return_type() {
        let d = parse("classDiagram\n    class Foo {\n        +getName() String\n    }").unwrap();
        let m = &d.classes[0].methods[0];
        assert_eq!(m.name, "getName");
        assert_eq!(m.return_type.as_deref(), Some("String"));
        assert_eq!(m.parameters.as_deref(), Some(""));
    }

    #[test]
    fn parse_method_with_params() {
        let d = parse("classDiagram\n    class Foo {\n        +setName(name String)\n    }").unwrap();
        let m = &d.classes[0].methods[0];
        assert_eq!(m.parameters.as_deref(), Some("name String"));
    }

    #[test]
    fn parse_extension_relationship() {
        let d = parse("classDiagram\n    Animal <|-- Dog").unwrap();
        assert_eq!(d.relationships.len(), 1);
        assert_eq!(d.relationships[0].from_id, "Animal");
        assert_eq!(d.relationships[0].to_id, "Dog");
        assert_eq!(d.relationships[0].from_type, Some(RelationType::Extension));
        assert_eq!(d.relationships[0].line_type, LineType::Solid);
    }

    #[test]
    fn parse_composition_relationship() {
        let d = parse("classDiagram\n    Car *-- Wheel").unwrap();
        assert_eq!(d.relationships[0].from_type, Some(RelationType::Composition));
    }

    #[test]
    fn parse_aggregation_relationship() {
        let d = parse("classDiagram\n    Fleet o-- Car").unwrap();
        assert_eq!(d.relationships[0].from_type, Some(RelationType::Aggregation));
    }

    #[test]
    fn parse_dependency_relationship() {
        let d = parse("classDiagram\n    Class1 --> Class2").unwrap();
        assert_eq!(d.relationships[0].to_type, Some(RelationType::Dependency));
    }

    #[test]
    fn parse_dotted_extension() {
        let d = parse("classDiagram\n    Shape ..|> Circle").unwrap();
        assert_eq!(d.relationships[0].to_type, Some(RelationType::Extension));
        assert_eq!(d.relationships[0].line_type, LineType::Dotted);
    }

    #[test]
    fn parse_plain_association() {
        let d = parse("classDiagram\n    A -- B").unwrap();
        assert!(d.relationships[0].from_type.is_none() && d.relationships[0].to_type.is_none());
        assert_eq!(d.relationships[0].line_type, LineType::Solid);
    }

    #[test]
    fn parse_relationship_with_label() {
        let d = parse("classDiagram\n    Animal <|-- Dog : inherits").unwrap();
        assert_eq!(d.relationships[0].label.as_deref(), Some("inherits"));
    }

    #[test]
    fn parse_relationship_with_cardinality() {
        let d = parse("classDiagram\n    Car \"1\" *-- \"many\" Wheel : has").unwrap();
        assert_eq!(d.relationships[0].cardinality_from.as_deref(), Some("1"));
        assert_eq!(d.relationships[0].cardinality_to.as_deref(), Some("many"));
        assert_eq!(d.relationships[0].label.as_deref(), Some("has"));
    }

    #[test]
    fn parse_colon_member_attribute() {
        let d = parse("classDiagram\n    class Animal\n    Animal : +String name").unwrap();
        assert_eq!(d.classes[0].members.len(), 1);
        assert_eq!(d.classes[0].members[0].visibility, Visibility::Public);
    }

    #[test]
    fn parse_colon_member_method() {
        let d = parse("classDiagram\n    class Animal\n    Animal : +eat() void").unwrap();
        assert_eq!(d.classes[0].methods.len(), 1);
        assert!(d.classes[0].methods[0].is_method());
    }

    #[test]
    fn parse_namespace_basic() {
        let d = parse("classDiagram\n    namespace MyApp {\n        class User\n        class Admin\n    }").unwrap();
        assert_eq!(d.namespaces.len(), 1);
        assert_eq!(d.namespaces[0].id, "MyApp");
        assert_eq!(d.namespaces[0].class_ids.len(), 2);
        assert_eq!(d.classes[0].namespace.as_deref(), Some("MyApp"));
    }

    #[test]
    fn parse_namespace_dotted() {
        let d = parse("classDiagram\n    namespace Com.Example {\n        class Foo\n    }").unwrap();
        assert_eq!(d.namespaces[0].id, "Com.Example");
    }

    #[test]
    fn parse_note_standalone() {
        let d = parse("classDiagram\n    note \"Important info\"").unwrap();
        assert_eq!(d.notes.len(), 1);
        assert_eq!(d.notes[0].text, "Important info");
        assert!(d.notes[0].class_id.is_none());
    }

    #[test]
    fn parse_note_for_class() {
        let d = parse("classDiagram\n    class Animal\n    note for Animal \"Represents all animals\"").unwrap();
        assert_eq!(d.notes[0].class_id.as_deref(), Some("Animal"));
    }

    #[test]
    fn parse_direction_lr() {
        let d = parse("classDiagram\n    direction LR\n    class A").unwrap();
        assert_eq!(d.direction, Direction::LR);
    }

    #[test]
    fn parse_comments_ignored() {
        let d = parse("classDiagram\n    %% comment\n    class Animal\n    %% another").unwrap();
        assert_eq!(d.classes.len(), 1);
    }

    #[test]
    fn parse_class_def_styling() {
        let d = parse("classDiagram\n    class A\n    classDef highlight fill:#f9f,stroke:#333").unwrap();
        assert_eq!(d.class_defs.len(), 1);
    }

    #[test]
    fn parse_multiple_relationships() {
        let input = "classDiagram\n    A <|-- B\n    B *-- C\n    C o-- D\n    D --> E";
        let d = parse(input).unwrap();
        assert_eq!(d.relationships.len(), 4);
    }

    #[test]
    fn parse_body_separators_ignored() {
        let d = parse("classDiagram\n    class Foo {\n        field1\n        --\n        method1()\n        ..\n        method2()\n    }").unwrap();
        assert_eq!(d.classes[0].members.len(), 1);
        assert_eq!(d.classes[0].methods.len(), 2);
    }

    #[test]
    fn parse_complex_diagram() {
        let input = "\
classDiagram
    class Animal {
        +String name
        +int age
        +makeSound()*
    }
    class Dog {
        +String breed
        +bark() void
    }
    class Cat {
        +String color
        +purr() void
    }
    Animal <|-- Dog : extends
    Animal <|-- Cat : extends
    Dog \"1\" o-- \"many\" Toy : plays with";
        let d = parse(input).unwrap();
        assert_eq!(d.classes.len(), 4, "Animal, Dog, Cat, Toy");
        assert_eq!(d.relationships.len(), 3);
        assert_eq!(d.classes[0].methods[0].classifier, Classifier::Abstract);
    }

    // ── Negative tests ──

    #[test]
    fn reject_empty_input() {
        assert!(parse("").is_err());
    }

    #[test]
    fn reject_wrong_header() {
        assert!(parse("flowchart TD\n    A --> B").is_err());
    }

    #[test]
    fn reject_whitespace_only() {
        assert!(parse("   \n\n  ").is_err());
    }

    #[test]
    fn parse_v2_header() {
        let d = parse("classDiagram-v2\n    class Foo").unwrap();
        assert_eq!(d.classes.len(), 1);
    }

    #[test]
    fn auto_create_classes_from_relationship() {
        let d = parse("classDiagram\n    A <|-- B").unwrap();
        assert_eq!(d.classes.len(), 2);
        assert!(d.class("A").is_some());
        assert!(d.class("B").is_some());
    }

    #[test]
    fn member_display_text_roundtrip() {
        let m = parse_member_string("+getName(id int) String");
        assert_eq!(m.visibility, Visibility::Public);
        assert_eq!(m.name, "getName");
        assert_eq!(m.parameters.as_deref(), Some("id int"));
        assert_eq!(m.return_type.as_deref(), Some("String"));
        assert_eq!(m.display_text(), "+getName(id int) : String");
    }
}
