use winnow::combinator::{alt, opt};
use winnow::prelude::*;
use winnow::token::take_while;

use rusty_mermaid_core::Direction;

use crate::common::error::{ParseError, ParseErrorKind};
use crate::common::styling::{class_apply_body, class_def_body, style_stmt_body, ClassDef};
use crate::common::tokens::{identifier, skip, unescape_unicode, ws};

use super::ir::*;

/// Parse a mermaid state diagram string into IR.
pub fn parse(input: &str) -> Result<StateDiagram, ParseError> {
    let mut rest = input;
    parse_state_diagram(&mut rest).map_err(|_| {
        let offset = input.len() - rest.len();
        ParseError::new(ParseErrorKind::UnexpectedToken, offset..offset, input)
    })
}

fn parse_state_diagram(input: &mut &str) -> ModalResult<StateDiagram> {
    skip.parse_next(input)?;
    header.parse_next(input)?;
    skip.parse_next(input)?;

    let mut diagram = StateDiagram::new(Direction::TB);
    parse_body(
        input,
        &mut diagram.states,
        &mut diagram.transitions,
        &mut diagram.notes,
        &mut diagram.class_defs,
        &mut diagram.style_stmts,
        Some(&mut diagram.direction),
    )?;
    Ok(diagram)
}

fn header(input: &mut &str) -> ModalResult<()> {
    alt((
        "stateDiagram-v2".void(),
        "stateDiagram".void(),
    ))
    .parse_next(input)
}

/// Parse statements in the current scope (top-level or inside a composite state).
fn parse_body(
    input: &mut &str,
    states: &mut Vec<StateNode>,
    transitions: &mut Vec<StateTransition>,
    notes: &mut Vec<StateNote>,
    class_defs: &mut Vec<ClassDef>,
    style_stmts: &mut Vec<StateStyleStmt>,
    mut direction: Option<&mut Direction>,
) -> ModalResult<()> {
    loop {
        skip.parse_next(input)?;
        if input.is_empty() || input.starts_with('}') {
            break;
        }
        if !try_parse_statement(input, states, transitions, notes, class_defs, style_stmts, direction.as_deref_mut())? {
            // Skip unrecognized character
            if !input.is_empty() {
                *input = &input[1..];
            }
        }
    }
    Ok(())
}

/// Try to parse a single statement. Returns true if something was parsed.
fn try_parse_statement(
    input: &mut &str,
    states: &mut Vec<StateNode>,
    transitions: &mut Vec<StateTransition>,
    notes: &mut Vec<StateNote>,
    class_defs: &mut Vec<ClassDef>,
    style_stmts: &mut Vec<StateStyleStmt>,
    direction: Option<&mut Direction>,
) -> ModalResult<bool> {
    // Try classDef
    if input.starts_with("classDef") {
        let checkpoint = *input;
        *input = &input[8..];
        if let Ok(cd) = class_def_body.parse_next(input) {
            class_defs.push(cd);
            return Ok(true);
        }
        *input = checkpoint;
    }

    // Try style statement
    if input.starts_with("style ") {
        let checkpoint = *input;
        *input = &input[5..];
        if let Ok(ss) = style_stmt_body.parse_next(input) {
            style_stmts.push(StateStyleStmt {
                ids: ss.ids,
                styles: ss.styles,
            });
            return Ok(true);
        }
        *input = checkpoint;
    }

    // Try class apply
    if input.starts_with("class ") {
        let checkpoint = *input;
        *input = &input[5..];
        if let Ok(ca) = class_apply_body.parse_next(input) {
            for id in &ca.ids {
                if let Some(s) = states.iter_mut().find(|s| s.id == *id) {
                    s.classes.push(ca.class_name.clone());
                }
            }
            return Ok(true);
        }
        *input = checkpoint;
    }

    // Try direction statement
    if let Some(dir) = opt(parse_direction_stmt).parse_next(input)? {
        if let Some(d) = direction {
            *d = dir;
        }
        return Ok(true);
    }

    // Try note
    if let Some(note) = opt(parse_note).parse_next(input)? {
        notes.push(note);
        return Ok(true);
    }

    // Try composite state: `state "Label" as Name {` or `state Name {`
    {
        let checkpoint = *input;
        if let Ok(node) = parse_composite_state(input, class_defs, style_stmts) {
            upsert_state_full(states, node);
            return Ok(true);
        }
        *input = checkpoint;
    }

    // Try state declaration with stereotype: `state Name <<fork>>`
    if let Some(node) = opt(parse_state_decl).parse_next(input)? {
        upsert_state_full(states, node);
        return Ok(true);
    }

    // Try transition: `A --> B` or `A --> B : label`
    if let Some(trans) = opt(parse_transition).parse_next(input)? {
        ensure_state(states, &trans.src);
        ensure_state(states, &trans.dst);
        transitions.push(trans);
        return Ok(true);
    }

    // Try state with label: `Name : description`
    if let Some(node) = opt(parse_state_label).parse_next(input)? {
        upsert_state(states, node);
        return Ok(true);
    }

    Ok(false)
}

/// Ensure a state exists in the list (auto-create from transitions).
fn ensure_state(states: &mut Vec<StateNode>, id: &str) {
    if id == "[*]" {
        return; // pseudo-states don't need explicit nodes
    }
    if !states.iter().any(|s| s.id == id) {
        states.push(StateNode::new(id, StateKind::Normal));
    }
}

/// Replace an existing placeholder state with a fully-defined one (composite, stereotype),
/// or insert if new. This upgrades auto-created Normal states from transitions.
fn upsert_state_full(states: &mut Vec<StateNode>, node: StateNode) {
    if let Some(existing) = states.iter_mut().find(|s| s.id == node.id) {
        *existing = node;
    } else {
        states.push(node);
    }
}

/// Insert or update a state (e.g., when a label is assigned after first reference).
fn upsert_state(states: &mut Vec<StateNode>, node: StateNode) {
    if let Some(existing) = states.iter_mut().find(|s| s.id == node.id) {
        if node.label.is_some() {
            existing.label = node.label;
        }
    } else {
        states.push(node);
    }
}

/// Parse a state ID — either `[*]` or an identifier.
fn state_id<'i>(input: &mut &'i str) -> ModalResult<&'i str> {
    alt(("[*]", identifier)).parse_next(input)
}

/// Parse `direction LR` / `direction TB` etc.
fn parse_direction_stmt(input: &mut &str) -> ModalResult<Direction> {
    ("direction", ws, parse_direction_keyword)
        .map(|(_, _, d)| d)
        .parse_next(input)
}

fn parse_direction_keyword(input: &mut &str) -> ModalResult<Direction> {
    alt((
        "TB".value(Direction::TB),
        "TD".value(Direction::TB),
        "BT".value(Direction::BT),
        "LR".value(Direction::LR),
        "RL".value(Direction::RL),
    ))
    .parse_next(input)
}

/// Parse `note right of StateId : text` or `note left of StateId : text`.
/// Also handles multi-line notes: `note right of StateId\n  text\nend note`.
fn parse_note(input: &mut &str) -> ModalResult<StateNote> {
    "note".parse_next(input)?;
    ws.parse_next(input)?;
    let position = alt((
        "right".value(NotePosition::Right),
        "left".value(NotePosition::Left),
    ))
    .parse_next(input)?;
    ws.parse_next(input)?;
    "of".parse_next(input)?;
    ws.parse_next(input)?;
    let state_id = state_id.parse_next(input)?;
    ws.parse_next(input)?;

    // Inline note: `: text`
    // Multi-line note: newline ... end note
    let text = if opt(":").parse_next(input)?.is_some() {
        ws.parse_next(input)?;
        let t = take_while(1.., |c: char| c != '\n' && c != '\r').parse_next(input)?;
        t.trim().to_string()
    } else {
        // Multi-line: consume until "end note"
        parse_multiline_note_body(input)?
    };

    Ok(StateNote {
        position,
        state_id: state_id.to_string(),
        text,
    })
}

fn parse_multiline_note_body(input: &mut &str) -> ModalResult<String> {
    let mut lines = Vec::new();
    loop {
        // Skip to next line
        take_while(0.., |c: char| c == '\n' || c == '\r' || c == ' ' || c == '\t')
            .parse_next(input)?;
        if input.starts_with("end note") {
            "end note".parse_next(input)?;
            break;
        }
        if input.is_empty() {
            break;
        }
        let line = take_while(1.., |c: char| c != '\n' && c != '\r').parse_next(input)?;
        lines.push(line.trim());
    }
    Ok(lines.join("\n"))
}

/// Parse `state "Label" as Name {` or `state Name {`.
fn parse_composite_state(
    input: &mut &str,
    class_defs: &mut Vec<ClassDef>,
    style_stmts: &mut Vec<StateStyleStmt>,
) -> ModalResult<StateNode> {
    "state".parse_next(input)?;
    ws.parse_next(input)?;

    // Optional quoted label + "as"
    let (label, id) = if input.starts_with('"') {
        '"'.parse_next(input)?;
        let label = take_while(1.., |c: char| c != '"').parse_next(input)?;
        '"'.parse_next(input)?;
        ws.parse_next(input)?;
        "as".parse_next(input)?;
        ws.parse_next(input)?;
        let id = identifier.parse_next(input)?;
        (Some(unescape_unicode(label)), id.to_string())
    } else {
        let id = identifier.parse_next(input)?;
        (None, id.to_string())
    };

    ws.parse_next(input)?;
    '{'.parse_next(input)?;

    let mut children = Vec::new();
    let mut trans = Vec::new();
    let mut notes = Vec::new();
    let mut direction = None;
    let mut regions: Vec<ConcurrentRegion> = Vec::new();
    let mut region_children: Vec<StateNode> = Vec::new();
    let mut region_trans: Vec<StateTransition> = Vec::new();
    let mut has_divider = false;

    // Parse body
    loop {
        skip.parse_next(input)?;
        if input.is_empty() {
            break;
        }
        if opt('}').parse_next(input)?.is_some() {
            break;
        }

        // Check for concurrency divider `--`
        if input.starts_with("--") && !input.starts_with("-->") {
            "--".parse_next(input)?;
            take_while(0.., |c: char| c == '-').parse_next(input)?;
            // Flush current region
            regions.push(ConcurrentRegion {
                children: std::mem::take(&mut region_children),
                transitions: std::mem::take(&mut region_trans),
            });
            has_divider = true;
            continue;
        }

        // Check for direction
        if let Some(dir) = opt(parse_direction_stmt).parse_next(input)? {
            direction = Some(dir);
            continue;
        }

        if !try_parse_statement(input, &mut region_children, &mut region_trans, &mut notes, class_defs, style_stmts, None)? {
            if !input.is_empty() {
                *input = &input[1..];
            }
        }
    }

    // Flush remaining content
    if has_divider {
        regions.push(ConcurrentRegion {
            children: std::mem::take(&mut region_children),
            transitions: std::mem::take(&mut region_trans),
        });
        // Flatten into children/trans for backward compat
        for r in &regions {
            children.extend(r.children.clone());
            trans.extend(r.transitions.clone());
        }
    } else {
        children = region_children;
        trans = region_trans;
    }

    let mut node = StateNode::new(id, StateKind::Composite {
        direction,
        children,
        transitions: trans,
        notes,
        regions,
    });
    node.label = label;
    Ok(node)
}

/// Parse `state Name <<fork>>`, `state Name <<join>>`, `state Name <<choice>>`.
fn parse_state_decl(input: &mut &str) -> ModalResult<StateNode> {
    "state".parse_next(input)?;
    ws.parse_next(input)?;

    // Optional quoted label + "as"
    let (label, id) = if input.starts_with('"') {
        '"'.parse_next(input)?;
        let label = take_while(1.., |c: char| c != '"').parse_next(input)?;
        '"'.parse_next(input)?;
        ws.parse_next(input)?;
        "as".parse_next(input)?;
        ws.parse_next(input)?;
        let id = identifier.parse_next(input)?;
        (Some(unescape_unicode(label)), id.to_string())
    } else {
        let id = identifier.parse_next(input)?;
        (None, id.to_string())
    };

    ws.parse_next(input)?;

    let kind = alt((
        "<<fork>>".value(StateKind::Fork),
        "<<join>>".value(StateKind::Join),
        "<<choice>>".value(StateKind::Choice),
        "<<history>>".value(StateKind::History),
    ))
    .parse_next(input)?;

    let mut node = StateNode::new(id, kind);
    node.label = label;
    Ok(node)
}

/// Parse `Name : description`.
fn parse_state_label(input: &mut &str) -> ModalResult<StateNode> {
    let id = identifier.parse_next(input)?;
    ws.parse_next(input)?;
    ':'.parse_next(input)?;
    ws.parse_next(input)?;
    let label = take_while(1.., |c: char| c != '\n' && c != '\r').parse_next(input)?;
    Ok(StateNode::new(id, StateKind::Normal).with_label(&unescape_unicode(label.trim())))
}

/// Parse `A --> B` or `A --> B : label`.
fn parse_transition(input: &mut &str) -> ModalResult<StateTransition> {
    let src = state_id.parse_next(input)?;
    ws.parse_next(input)?;
    "-->".parse_next(input)?;
    ws.parse_next(input)?;
    let dst = state_id.parse_next(input)?;

    let label = opt(parse_transition_label).parse_next(input)?;

    Ok(StateTransition {
        src: src.to_string(),
        dst: dst.to_string(),
        label: label.map(|s| unescape_unicode(s)),
    })
}

fn parse_transition_label<'i>(input: &mut &'i str) -> ModalResult<&'i str> {
    ws.parse_next(input)?;
    ':'.parse_next(input)?;
    ws.parse_next(input)?;
    let label = take_while(1.., |c: char| c != '\n' && c != '\r').parse_next(input)?;
    Ok(label.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_transitions() {
        let input = "stateDiagram-v2\n    [*] --> Still\n    Still --> Moving\n    Moving --> Crash\n    Crash --> [*]";
        let d = parse(input).unwrap();
        assert_eq!(d.transitions.len(), 4);
        assert_eq!(d.transitions[0].src, "[*]");
        assert_eq!(d.transitions[0].dst, "Still");
        assert_eq!(d.transitions[3].dst, "[*]");
    }

    #[test]
    fn parse_state_with_label() {
        let input = "stateDiagram-v2\n    s1 : Idle\n    s2 : Processing\n    s1 --> s2";
        let d = parse(input).unwrap();
        let s1 = d.state("s1").unwrap();
        assert_eq!(s1.label.as_deref(), Some("Idle"));
    }

    #[test]
    fn parse_transition_with_label() {
        let input = "stateDiagram-v2\n    A --> B : click event";
        let d = parse(input).unwrap();
        assert_eq!(d.transitions[0].label.as_deref(), Some("click event"));
    }

    #[test]
    fn parse_fork_join() {
        let input = "stateDiagram-v2\n    state fork1 <<fork>>\n    state join1 <<join>>\n    [*] --> fork1\n    fork1 --> A\n    fork1 --> B\n    A --> join1\n    B --> join1";
        let d = parse(input).unwrap();
        let fork = d.state("fork1").unwrap();
        let join = d.state("join1").unwrap();
        assert!(matches!(fork.kind, StateKind::Fork));
        assert!(matches!(join.kind, StateKind::Join));
    }

    #[test]
    fn parse_choice() {
        let input = "stateDiagram-v2\n    state check <<choice>>\n    [*] --> check\n    check --> A : yes\n    check --> B : no";
        let d = parse(input).unwrap();
        let c = d.state("check").unwrap();
        assert!(matches!(c.kind, StateKind::Choice));
        assert_eq!(d.transitions.len(), 3);
    }

    #[test]
    fn parse_composite_state() {
        let input = "stateDiagram-v2\n    state Active {\n        Idle --> Running\n        Running --> Idle\n    }\n    [*] --> Active";
        let d = parse(input).unwrap();
        let active = d.state("Active").unwrap();
        assert!(active.is_composite());
        if let StateKind::Composite { children, transitions, .. } = &active.kind {
            assert_eq!(children.len(), 2);
            assert_eq!(transitions.len(), 2);
        }
    }

    #[test]
    fn parse_composite_with_label() {
        let input = "stateDiagram-v2\n    state \"Active Mode\" as Active {\n        A --> B\n    }";
        let d = parse(input).unwrap();
        let active = d.state("Active").unwrap();
        assert_eq!(active.label.as_deref(), Some("Active Mode"));
        assert!(active.is_composite());
    }

    #[test]
    fn parse_inline_note() {
        let input = "stateDiagram-v2\n    [*] --> Still\n    note right of Still : idle state";
        let d = parse(input).unwrap();
        assert_eq!(d.notes.len(), 1);
        assert_eq!(d.notes[0].text, "idle state");
        assert_eq!(d.notes[0].state_id, "Still");
        assert_eq!(d.notes[0].position, NotePosition::Right);
    }

    #[test]
    fn parse_multiline_note() {
        let input = "stateDiagram-v2\n    [*] --> Still\n    note right of Still\n        line one\n        line two\n    end note";
        let d = parse(input).unwrap();
        assert_eq!(d.notes.len(), 1);
        assert_eq!(d.notes[0].text, "line one\nline two");
    }

    #[test]
    fn parse_v1_header() {
        let input = "stateDiagram\n    A --> B";
        let d = parse(input).unwrap();
        assert_eq!(d.transitions.len(), 1);
    }

    #[test]
    fn parse_concurrency() {
        let input = "stateDiagram-v2\n    state Active {\n        A --> B\n        --\n        C --> D\n    }";
        let d = parse(input).unwrap();
        let active = d.state("Active").unwrap();
        if let StateKind::Composite { regions, children, .. } = &active.kind {
            assert_eq!(regions.len(), 2);
            assert_eq!(regions[0].children.len(), 2); // A, B
            assert_eq!(regions[0].transitions.len(), 1); // A --> B
            assert_eq!(regions[1].children.len(), 2); // C, D
            assert_eq!(regions[1].transitions.len(), 1); // C --> D
            // children should contain flattened list
            assert_eq!(children.len(), 4);
        } else {
            panic!("expected composite");
        }
    }

    #[test]
    fn parse_top_level_direction() {
        let input = "stateDiagram-v2\n    direction LR\n    A --> B";
        let d = parse(input).unwrap();
        assert_eq!(d.direction, Direction::LR);
    }

    #[test]
    fn parse_direction_defaults_to_tb() {
        let input = "stateDiagram-v2\n    A --> B";
        let d = parse(input).unwrap();
        assert_eq!(d.direction, Direction::TB);
    }

    #[test]
    fn parse_direction_in_composite() {
        let input = "stateDiagram-v2\n    state Active {\n        direction LR\n        A --> B\n    }";
        let d = parse(input).unwrap();
        let active = d.state("Active").unwrap();
        if let StateKind::Composite { direction, .. } = &active.kind {
            assert_eq!(*direction, Some(Direction::LR));
        } else {
            panic!("expected composite");
        }
    }

    #[test]
    fn parse_state_decl_with_label() {
        let input = "stateDiagram-v2\n    state \"Check Inventory\" as check <<choice>>\n    [*] --> check";
        let d = parse(input).unwrap();
        let c = d.state("check").unwrap();
        assert!(matches!(c.kind, StateKind::Choice));
        assert_eq!(c.label.as_deref(), Some("Check Inventory"));
    }

    #[test]
    fn auto_creates_states_from_transitions() {
        let input = "stateDiagram-v2\n    A --> B\n    B --> C";
        let d = parse(input).unwrap();
        assert!(d.state("A").is_some());
        assert!(d.state("B").is_some());
        assert!(d.state("C").is_some());
    }

    #[test]
    fn parse_classdef() {
        let input = "stateDiagram-v2\n    A --> B\n    classDef highlight fill:#f9f,stroke:#333";
        let d = parse(input).unwrap();
        assert_eq!(d.class_defs.len(), 1);
        assert_eq!(d.class_defs[0].name, "highlight");
        assert_eq!(d.class_defs[0].styles.len(), 2);
    }

    #[test]
    fn parse_style_stmt() {
        let input = "stateDiagram-v2\n    A --> B\n    style A fill:#f00";
        let d = parse(input).unwrap();
        assert_eq!(d.style_stmts.len(), 1);
        assert_eq!(d.style_stmts[0].ids, vec!["A"]);
    }

    #[test]
    fn parse_history_state() {
        let input = "stateDiagram-v2\n    state hist1 <<history>>\n    [*] --> hist1\n    hist1 --> A";
        let d = parse(input).unwrap();
        let h = d.state("hist1").unwrap();
        assert!(matches!(h.kind, StateKind::History));
    }

    #[test]
    fn parse_class_apply() {
        let input = "stateDiagram-v2\n    A --> B\n    classDef active fill:#0f0\n    class A active";
        let d = parse(input).unwrap();
        let a = d.state("A").unwrap();
        assert_eq!(a.classes, vec!["active"]);
    }

    // ── Negative / error-path tests ──────────────────────────────────

    #[test]
    fn reject_empty_input() {
        assert!(parse("").is_err(), "empty input must fail (no header)");
    }

    #[test]
    fn reject_whitespace_only() {
        assert!(
            parse("   \n\n  ").is_err(),
            "whitespace-only input must fail"
        );
    }

    #[test]
    fn reject_no_header() {
        assert!(
            parse("A --> B").is_err(),
            "input without stateDiagram header must fail"
        );
    }

    #[test]
    fn reject_wrong_diagram_type() {
        assert!(
            parse("flowchart TD\n    A --> B").is_err(),
            "flowchart header must be rejected by state parser"
        );
    }

    #[test]
    fn malformed_transition_no_source() {
        // `--> B` on its own line. The body parser is lenient: it tries each
        // statement type, none match, and it skips the unrecognized chars.
        let d = parse("stateDiagram-v2\n    --> B").unwrap();
        // The `-->` is not a valid state ID, so no transition is created.
        // The lenient parser skips chars until it can parse something.
        assert_eq!(
            d.transitions.len(),
            0,
            "malformed transition with no source is silently skipped"
        );
    }

    #[test]
    fn malformed_transition_no_destination() {
        // `A -->` with no destination. The transition parser fails,
        // and the body falls through to parse_state_label or skip.
        let d = parse("stateDiagram-v2\n    A -->").unwrap();
        // `A` gets created as a state (via state_label or bare reference),
        // but no transition is produced.
        assert_eq!(
            d.transitions.len(),
            0,
            "malformed transition with no destination is silently skipped"
        );
    }

    #[test]
    fn unclosed_composite_state_at_eof() {
        // Composite without closing `}`. The parse_composite_state loop runs
        // until EOF (input.is_empty() → break), so it tolerates this.
        let d = parse("stateDiagram-v2\n    state Active {\n        A --> B").unwrap();
        let active = d.state("Active").unwrap();
        assert!(
            active.is_composite(),
            "unclosed composite is tolerated (EOF closes scope)"
        );
        if let StateKind::Composite { children, transitions, .. } = &active.kind {
            assert_eq!(children.len(), 2);
            assert_eq!(transitions.len(), 1);
        }
    }

    #[test]
    fn invalid_state_id_special_chars_skipped() {
        // `@State --> B`: the `@` is not valid for identifier, so all statement
        // parsers fail. The lenient body parser skips `@` (one char at a time),
        // then on the next iteration `State --> B` is a valid transition.
        let d = parse("stateDiagram-v2\n    @State --> B").unwrap();
        assert_eq!(
            d.transitions.len(),
            1,
            "after skipping '@', 'State --> B' parses as valid transition"
        );
        assert_eq!(d.transitions[0].src, "State");
        assert_eq!(d.transitions[0].dst, "B");
    }

    #[test]
    fn invalid_state_id_starts_with_digit() {
        // `123` starts with a digit — identifier requires alpha or underscore first.
        // Transition parser fails, falls through to skip.
        let d = parse("stateDiagram-v2\n    123 --> B").unwrap();
        assert_eq!(
            d.transitions.len(),
            0,
            "state ID starting with digit is skipped"
        );
    }

    #[test]
    fn multiline_note_missing_end_note() {
        // Multi-line note without `end note`. The parse_multiline_note_body
        // loop runs until EOF (input.is_empty() → break), collecting lines.
        let d = parse(
            "stateDiagram-v2\n    [*] --> A\n    note right of A\n        line one\n        line two",
        )
        .unwrap();
        assert_eq!(d.notes.len(), 1);
        assert_eq!(
            d.notes[0].text, "line one\nline two",
            "missing 'end note' is tolerated (EOF closes note)"
        );
    }

    #[test]
    fn note_with_invalid_position() {
        // `note above of A : text` — "above" is not a valid position.
        // parse_note fails, body parser skips the chars.
        let d = parse("stateDiagram-v2\n    A --> B\n    note above of A : text").unwrap();
        assert_eq!(
            d.notes.len(),
            0,
            "invalid note position causes note to be skipped"
        );
    }

    #[test]
    fn note_missing_of_keyword() {
        // `note right A : text` — missing "of" keyword.
        let d = parse("stateDiagram-v2\n    A --> B\n    note right A : text").unwrap();
        assert_eq!(
            d.notes.len(),
            0,
            "missing 'of' keyword causes note to be skipped"
        );
    }

    #[test]
    fn pseudo_state_in_transition() {
        // `[*]` as both source and destination is valid syntax.
        let d = parse("stateDiagram-v2\n    [*] --> [*]").unwrap();
        assert_eq!(d.transitions.len(), 1);
        assert_eq!(d.transitions[0].src, "[*]");
        assert_eq!(d.transitions[0].dst, "[*]");
    }

    #[test]
    fn only_header_no_body() {
        // Just the header with nothing else — should parse to empty diagram.
        let d = parse("stateDiagram-v2").unwrap();
        assert!(d.states.is_empty());
        assert!(d.transitions.is_empty());
    }

    #[test]
    fn invalid_stereotype() {
        // `<<invalid>>` is not a recognized stereotype.
        // parse_state_decl fails, falls through to other parsers.
        let d = parse("stateDiagram-v2\n    state Foo <<invalid>>").unwrap();
        // `Foo` may be created as a bare state (via state_label fallback or skip),
        // but it should NOT have a stereotype kind.
        let foo = d.state("Foo");
        if let Some(f) = foo {
            assert!(
                matches!(f.kind, StateKind::Normal),
                "unrecognized stereotype falls back to Normal"
            );
        }
    }

    #[test]
    fn garbage_lines_between_valid_statements() {
        // The lenient parser should skip garbage and still parse valid lines.
        let d = parse(
            "stateDiagram-v2\n    A --> B\n    $$$ garbage @@@\n    B --> C",
        )
        .unwrap();
        assert_eq!(d.transitions.len(), 2, "valid transitions survive garbage lines");
        assert_eq!(d.transitions[0].src, "A");
        assert_eq!(d.transitions[1].src, "B");
    }

    #[test]
    fn deeply_nested_unclosed_composites() {
        // Multiple unclosed nested composites — each one's body runs to EOF.
        let d = parse(
            "stateDiagram-v2\n    state Outer {\n        state Inner {\n            A --> B",
        )
        .unwrap();
        let outer = d.state("Outer").unwrap();
        assert!(outer.is_composite());
        if let StateKind::Composite { children, .. } = &outer.kind {
            let inner = children.iter().find(|c| c.id == "Inner").unwrap();
            assert!(inner.is_composite());
        }
    }

    #[test]
    fn transition_arrow_without_dashes() {
        // `A -> B` is not valid state syntax (needs `-->`).
        // Parser skips it.
        let d = parse("stateDiagram-v2\n    A -> B").unwrap();
        assert_eq!(
            d.transitions.len(),
            0,
            "single-dash arrow is not valid state transition syntax"
        );
    }
}
