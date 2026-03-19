use winnow::combinator::{alt, opt};
use winnow::prelude::*;
use winnow::token::take_while;

use crate::common::error::{ParseError, ParseErrorKind};
use crate::common::tokens::{identifier, skip, ws};

use super::ir::*;

/// Parse a mermaid sequence diagram string into IR.
pub fn parse(input: &str) -> Result<SequenceDiagram, ParseError> {
    let mut rest = input;
    parse_sequence_diagram(&mut rest).map_err(|_| {
        let offset = input.len() - rest.len();
        ParseError::new(ParseErrorKind::UnexpectedToken, offset..offset, input)
    })
}

/// Shared state accumulated during parsing (participants, title, autonumber).
/// Items are returned from `parse_items` rather than accumulated here,
/// because different scopes (top-level vs fragment sections) own their items.
#[derive(Default)]
struct ParseContext {
    participants: Vec<Participant>,
    title: Option<String>,
    autonumber: Option<AutoNumber>,
}

fn parse_sequence_diagram(input: &mut &str) -> ModalResult<SequenceDiagram> {
    skip.parse_next(input)?;
    "sequenceDiagram".parse_next(input)?;

    let mut ctx = ParseContext::default();
    let items = parse_items(input, &mut ctx, &[])?;

    Ok(SequenceDiagram {
        title: ctx.title,
        participants: ctx.participants,
        items,
        autonumber: ctx.autonumber,
    })
}

// ── Body / item parsing ────────────────────────────────────

/// Parse items until EOF or a stop keyword is reached.
/// Stop keywords are NOT consumed — the caller handles them.
fn parse_items(
    input: &mut &str,
    ctx: &mut ParseContext,
    stop_keywords: &[&str],
) -> ModalResult<Vec<SequenceItem>> {
    let mut items = Vec::new();
    loop {
        skip.parse_next(input)?;
        if input.is_empty() || at_keyword_boundary(input, stop_keywords) {
            return Ok(items);
        }
        if !try_parse_statement(input, ctx, &mut items)? {
            // Skip unrecognized character
            if !input.is_empty() {
                *input = &input[1..];
            }
        }
    }
}

/// Check if input starts with one of the given keywords at a word boundary.
fn at_keyword_boundary(input: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|kw| {
        input.starts_with(kw) && {
            let rest = &input[kw.len()..];
            rest.is_empty()
                || rest.starts_with(|c: char| c.is_ascii_whitespace())
                || rest.starts_with("%%")
        }
    })
}

/// Check if input starts with a keyword followed by whitespace/EOL/comment/colon.
fn starts_with_keyword(input: &str, keyword: &str) -> bool {
    input.starts_with(keyword) && {
        let rest = &input[keyword.len()..];
        rest.is_empty()
            || rest.starts_with(|c: char| c.is_ascii_whitespace())
            || rest.starts_with("%%")
            || rest.starts_with(':')
    }
}

/// Case-insensitive keyword check (for "Note"/"note").
fn starts_with_keyword_ci(input: &str, keyword: &str) -> bool {
    input.len() >= keyword.len()
        && input[..keyword.len()].eq_ignore_ascii_case(keyword)
        && {
            let rest = &input[keyword.len()..];
            rest.is_empty()
                || rest.starts_with(|c: char| c.is_ascii_whitespace())
                || rest.starts_with("%%")
        }
}

fn try_parse_statement(
    input: &mut &str,
    ctx: &mut ParseContext,
    items: &mut Vec<SequenceItem>,
) -> ModalResult<bool> {
    // Title
    if starts_with_keyword(input, "title") {
        let checkpoint = *input;
        if let Ok(t) = parse_title(input) {
            ctx.title = Some(t);
            return Ok(true);
        }
        *input = checkpoint;
    }

    // Autonumber
    if starts_with_keyword(input, "autonumber") {
        let checkpoint = *input;
        if let Ok(an) = parse_autonumber(input) {
            ctx.autonumber = an;
            return Ok(true);
        }
        *input = checkpoint;
    }

    // Participant / Actor
    if starts_with_keyword(input, "participant") || starts_with_keyword(input, "actor") {
        let checkpoint = *input;
        if let Ok(p) = parse_participant_decl(input) {
            if !ctx.participants.iter().any(|existing| existing.id == p.id) {
                ctx.participants.push(p);
            }
            return Ok(true);
        }
        *input = checkpoint;
    }

    // Activate / Deactivate
    if starts_with_keyword(input, "activate") {
        let checkpoint = *input;
        *input = &input[8..];
        ws.parse_next(input)?;
        if let Ok(actor) = identifier.parse_next(input) {
            items.push(SequenceItem::Activation(Activation {
                actor: actor.to_string(),
                active: true,
            }));
            return Ok(true);
        }
        *input = checkpoint;
    }
    if starts_with_keyword(input, "deactivate") {
        let checkpoint = *input;
        *input = &input[10..];
        ws.parse_next(input)?;
        if let Ok(actor) = identifier.parse_next(input) {
            items.push(SequenceItem::Activation(Activation {
                actor: actor.to_string(),
                active: false,
            }));
            return Ok(true);
        }
        *input = checkpoint;
    }

    // Note
    if starts_with_keyword_ci(input, "note") {
        let checkpoint = *input;
        if let Ok(note) = parse_note(input) {
            items.push(SequenceItem::Note(note));
            return Ok(true);
        }
        *input = checkpoint;
    }

    // Fragment (loop/alt/opt/par/critical/break)
    {
        let checkpoint = *input;
        if let Ok(frag) = parse_fragment(input, ctx) {
            items.push(SequenceItem::Fragment(frag));
            return Ok(true);
        }
        *input = checkpoint;
    }

    // Message (last — actor identifiers are permissive)
    {
        let checkpoint = *input;
        if let Ok(msg) = parse_message(input, &mut ctx.participants) {
            items.push(SequenceItem::Message(msg));
            return Ok(true);
        }
        *input = checkpoint;
    }

    Ok(false)
}

// ── Title ──────────────────────────────────────────────────

fn parse_title(input: &mut &str) -> ModalResult<String> {
    "title".parse_next(input)?;
    ws.parse_next(input)?;
    opt(':').parse_next(input)?;
    ws.parse_next(input)?;
    let text = take_while(1.., |c: char| c != '\n' && c != '\r').parse_next(input)?;
    Ok(text.trim().to_string())
}

// ── Autonumber ─────────────────────────────────────────────

fn parse_autonumber(input: &mut &str) -> ModalResult<Option<AutoNumber>> {
    "autonumber".parse_next(input)?;
    ws.parse_next(input)?;

    // "autonumber off" disables numbering
    if starts_with_keyword(input, "off") {
        *input = &input[3..];
        return Ok(None);
    }

    let start_str: &str =
        take_while(0.., |c: char| c.is_ascii_digit()).parse_next(input)?;
    let start = if start_str.is_empty() {
        1
    } else {
        start_str.parse::<u32>().unwrap_or(1)
    };

    ws.parse_next(input)?;

    let step_str: &str =
        take_while(0.., |c: char| c.is_ascii_digit()).parse_next(input)?;
    let step = if step_str.is_empty() {
        1
    } else {
        step_str.parse::<u32>().unwrap_or(1)
    };

    Ok(Some(AutoNumber { start, step }))
}

// ── Participant / Actor ────────────────────────────────────

fn parse_participant_decl(input: &mut &str) -> ModalResult<Participant> {
    let kind = alt((
        "participant".value(ParticipantKind::Box),
        "actor".value(ParticipantKind::Actor),
    ))
    .parse_next(input)?;

    ws.parse_next(input)?;
    let id = identifier.parse_next(input)?;
    let label = parse_as_label(input)?;
    let label = label.unwrap_or_else(|| id.to_string());

    Ok(Participant {
        id: id.to_string(),
        label,
        kind,
    })
}

/// Parse optional `as Label` or `as "Quoted Label"`.
fn parse_as_label(input: &mut &str) -> ModalResult<Option<String>> {
    ws.parse_next(input)?;
    if !input.starts_with("as") {
        return Ok(None);
    }
    let after = &input[2..];
    if !after.is_empty() && !after.starts_with(|c: char| c.is_ascii_whitespace()) {
        return Ok(None);
    }
    *input = after;
    ws.parse_next(input)?;

    if input.starts_with('"') {
        *input = &input[1..];
        let label = take_while(0.., |c: char| c != '"').parse_next(input)?;
        '"'.parse_next(input)?;
        Ok(Some(label.to_string()))
    } else {
        let label =
            take_while(1.., |c: char| c != '\n' && c != '\r').parse_next(input)?;
        Ok(Some(label.trim().to_string()))
    }
}

// ── Note ───────────────────────────────────────────────────

fn parse_note(input: &mut &str) -> ModalResult<Note> {
    // Accept "Note" (mermaid.js canonical) and "note"
    alt(("Note", "note")).parse_next(input)?;
    ws.parse_next(input)?;

    let position = if input.starts_with("left of") {
        *input = &input[7..];
        ws.parse_next(input)?;
        let actor = identifier.parse_next(input)?;
        NotePosition::LeftOf(actor.to_string())
    } else if input.starts_with("right of") {
        *input = &input[8..];
        ws.parse_next(input)?;
        let actor = identifier.parse_next(input)?;
        NotePosition::RightOf(actor.to_string())
    } else if input.starts_with("over") {
        *input = &input[4..];
        ws.parse_next(input)?;
        let mut actors = vec![identifier.parse_next(input)?.to_string()];
        loop {
            ws.parse_next(input)?;
            if !input.starts_with(',') {
                break;
            }
            *input = &input[1..];
            ws.parse_next(input)?;
            actors.push(identifier.parse_next(input)?.to_string());
        }
        NotePosition::Over(actors)
    } else {
        return Err(winnow::error::ErrMode::Backtrack(
            winnow::error::ContextError::new(),
        ));
    };

    ws.parse_next(input)?;
    ':'.parse_next(input)?;
    ws.parse_next(input)?;
    let text =
        take_while(1.., |c: char| c != '\n' && c != '\r').parse_next(input)?;

    Ok(Note {
        position,
        text: text.trim().to_string(),
    })
}

// ── Fragment ───────────────────────────────────────────────

fn parse_fragment(
    input: &mut &str,
    ctx: &mut ParseContext,
) -> ModalResult<Fragment> {
    let kind = alt((
        "loop".value(FragmentKind::Loop),
        "critical".value(FragmentKind::Critical),
        "break".value(FragmentKind::Break),
        "alt".value(FragmentKind::Alt),
        "opt".value(FragmentKind::Opt),
        "par".value(FragmentKind::Par),
    ))
    .parse_next(input)?;

    // Verify word boundary
    if !input.is_empty()
        && !input.starts_with(|c: char| c.is_ascii_whitespace())
        && !input.starts_with("%%")
    {
        return Err(winnow::error::ErrMode::Backtrack(
            winnow::error::ContextError::new(),
        ));
    }

    // Fragment label (rest of line)
    ws.parse_next(input)?;
    let label_text: &str =
        take_while(0.., |c: char| c != '\n' && c != '\r').parse_next(input)?;
    let label = if label_text.trim().is_empty() {
        None
    } else {
        Some(label_text.trim().to_string())
    };

    // Determine section divider keyword
    let divider = match kind {
        FragmentKind::Alt => Some("else"),
        FragmentKind::Par => Some("and"),
        FragmentKind::Critical => Some("option"),
        _ => None,
    };

    // Stop keywords for inner sections
    let inner_stop: Vec<&str> = match divider {
        Some(d) => vec!["end", d],
        None => vec!["end"],
    };

    let mut frag = Fragment::new(kind);
    frag.label = label.clone();

    // First section
    let section_items = parse_items(input, ctx, &inner_stop)?;
    let mut first_section = FragmentSection::new();
    first_section.label = label;
    first_section.items = section_items;
    frag.sections.push(first_section);

    // Additional sections (else/and/option)
    if let Some(d) = divider {
        while at_keyword_boundary(input, &[d]) {
            *input = &input[d.len()..];
            ws.parse_next(input)?;
            let section_label: &str =
                take_while(0.., |c: char| c != '\n' && c != '\r')
                    .parse_next(input)?;
            let section_label = if section_label.trim().is_empty() {
                None
            } else {
                Some(section_label.trim().to_string())
            };

            let section_items = parse_items(input, ctx, &inner_stop)?;
            let mut section = FragmentSection::new();
            section.label = section_label;
            section.items = section_items;
            frag.sections.push(section);
        }
    }

    // Consume "end"
    skip.parse_next(input)?;
    if at_keyword_boundary(input, &["end"]) {
        *input = &input[3..];
    }

    Ok(frag)
}

// ── Message ────────────────────────────────────────────────

fn parse_message(
    input: &mut &str,
    participants: &mut Vec<Participant>,
) -> ModalResult<Message> {
    let from = identifier.parse_next(input)?;
    let from_str = from.to_string();

    ws.parse_next(input)?;
    let arrow = parse_arrow(input)?;

    // Optional activation suffix: + activates target, - deactivates source
    let (activate, deactivate) = match input.chars().next() {
        Some('+') => {
            *input = &input[1..];
            (true, false)
        }
        Some('-') => {
            *input = &input[1..];
            (false, true)
        }
        _ => (false, false),
    };

    ws.parse_next(input)?;
    let to = identifier.parse_next(input)?;
    let to_str = to.to_string();

    ensure_participant(participants, &from_str);
    ensure_participant(participants, &to_str);

    // Optional `: label`
    ws.parse_next(input)?;
    let label = if input.starts_with(':') {
        *input = &input[1..];
        ws.parse_next(input)?;
        let text =
            take_while(0.., |c: char| c != '\n' && c != '\r').parse_next(input)?;
        let text = text.trim();
        if text.is_empty() { None } else { Some(text.to_string()) }
    } else {
        None
    };

    Ok(Message {
        from: from_str,
        to: to_str,
        label,
        arrow,
        activate,
        deactivate,
    })
}

fn ensure_participant(participants: &mut Vec<Participant>, id: &str) {
    if !participants.iter().any(|p| p.id == id) {
        participants.push(Participant::new(id, id));
    }
}

/// Parse a sequence diagram arrow. Longest match first to avoid prefix ambiguity.
fn parse_arrow(input: &mut &str) -> ModalResult<ArrowStyle> {
    alt((
        "-->>".value(ArrowStyle::DOTTED_FILLED),
        "--x".value(ArrowStyle::DOTTED_CROSS),
        "--)".value(ArrowStyle {
            line: LineStyle::Dotted,
            head: ArrowHead::None,
        }),
        "-->".value(ArrowStyle::DOTTED_OPEN),
        "->>".value(ArrowStyle::SOLID_FILLED),
        "-x".value(ArrowStyle::SOLID_CROSS),
        "-)".value(ArrowStyle {
            line: LineStyle::Solid,
            head: ArrowHead::None,
        }),
        "->".value(ArrowStyle::SOLID_OPEN),
    ))
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Header ─────────────────────────────────────────────

    #[test]
    fn parse_empty_diagram() {
        let d = parse("sequenceDiagram").unwrap();
        assert!(d.participants.is_empty());
        assert!(d.items.is_empty());
        assert!(d.title.is_none());
    }

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
    fn skip_leading_comment() {
        let d = parse("%% comment\nsequenceDiagram\n    Alice->>Bob: hi").unwrap();
        assert_eq!(d.participants.len(), 2);
    }

    // ── Participants ───────────────────────────────────────

    #[test]
    fn parse_participant() {
        let d = parse("sequenceDiagram\n    participant Alice").unwrap();
        assert_eq!(d.participants.len(), 1);
        assert_eq!(d.participants[0].id, "Alice");
        assert_eq!(d.participants[0].label, "Alice");
        assert_eq!(d.participants[0].kind, ParticipantKind::Box);
    }

    #[test]
    fn parse_participant_with_alias() {
        let d = parse("sequenceDiagram\n    participant A as Alice").unwrap();
        assert_eq!(d.participants[0].id, "A");
        assert_eq!(d.participants[0].label, "Alice");
    }

    #[test]
    fn parse_participant_with_quoted_alias() {
        let d =
            parse("sequenceDiagram\n    participant A as \"Alice Smith\"").unwrap();
        assert_eq!(d.participants[0].id, "A");
        assert_eq!(d.participants[0].label, "Alice Smith");
    }

    #[test]
    fn parse_actor() {
        let d = parse("sequenceDiagram\n    actor Bob").unwrap();
        assert_eq!(d.participants[0].kind, ParticipantKind::Actor);
        assert_eq!(d.participants[0].id, "Bob");
    }

    #[test]
    fn parse_actor_with_alias() {
        let d = parse("sequenceDiagram\n    actor B as Bob").unwrap();
        assert_eq!(d.participants[0].id, "B");
        assert_eq!(d.participants[0].label, "Bob");
        assert_eq!(d.participants[0].kind, ParticipantKind::Actor);
    }

    #[test]
    fn duplicate_participant_ignored() {
        let d = parse(
            "sequenceDiagram\n    participant A\n    participant A",
        )
        .unwrap();
        assert_eq!(d.participants.len(), 1);
    }

    #[test]
    fn participant_order_preserved() {
        let d = parse(
            "sequenceDiagram\n    participant C\n    participant A\n    participant B",
        )
        .unwrap();
        let ids: Vec<&str> = d.participants.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, vec!["C", "A", "B"]);
    }

    // ── Messages ───────────────────────────────────────────

    #[test]
    fn parse_solid_filled_arrow() {
        let d = parse("sequenceDiagram\n    Alice->>Bob: Hello").unwrap();
        assert_eq!(d.items.len(), 1);
        let SequenceItem::Message(ref m) = d.items[0] else {
            panic!("expected message");
        };
        assert_eq!(m.from, "Alice");
        assert_eq!(m.to, "Bob");
        assert_eq!(m.label.as_deref(), Some("Hello"));
        assert_eq!(m.arrow, ArrowStyle::SOLID_FILLED);
    }

    #[test]
    fn parse_solid_open_arrow() {
        let d = parse("sequenceDiagram\n    A->B: open").unwrap();
        let SequenceItem::Message(ref m) = d.items[0] else {
            panic!("expected message");
        };
        assert_eq!(m.arrow, ArrowStyle::SOLID_OPEN);
    }

    #[test]
    fn parse_dotted_filled_arrow() {
        let d = parse("sequenceDiagram\n    A-->>B: dotted filled").unwrap();
        let SequenceItem::Message(ref m) = d.items[0] else {
            panic!("expected message");
        };
        assert_eq!(m.arrow, ArrowStyle::DOTTED_FILLED);
    }

    #[test]
    fn parse_dotted_open_arrow() {
        let d = parse("sequenceDiagram\n    A-->B: dotted open").unwrap();
        let SequenceItem::Message(ref m) = d.items[0] else {
            panic!("expected message");
        };
        assert_eq!(m.arrow, ArrowStyle::DOTTED_OPEN);
    }

    #[test]
    fn parse_solid_cross_arrow() {
        let d = parse("sequenceDiagram\n    A-xB: cross").unwrap();
        let SequenceItem::Message(ref m) = d.items[0] else {
            panic!("expected message");
        };
        assert_eq!(m.arrow, ArrowStyle::SOLID_CROSS);
    }

    #[test]
    fn parse_dotted_cross_arrow() {
        let d = parse("sequenceDiagram\n    A--xB: dotted cross").unwrap();
        let SequenceItem::Message(ref m) = d.items[0] else {
            panic!("expected message");
        };
        assert_eq!(m.arrow, ArrowStyle::DOTTED_CROSS);
    }

    #[test]
    fn parse_solid_async_arrow() {
        let d = parse("sequenceDiagram\n    A-)B: async").unwrap();
        let SequenceItem::Message(ref m) = d.items[0] else {
            panic!("expected message");
        };
        assert_eq!(m.arrow.line, LineStyle::Solid);
        assert_eq!(m.arrow.head, ArrowHead::None);
    }

    #[test]
    fn parse_dotted_async_arrow() {
        let d = parse("sequenceDiagram\n    A--)B: dotted async").unwrap();
        let SequenceItem::Message(ref m) = d.items[0] else {
            panic!("expected message");
        };
        assert_eq!(m.arrow.line, LineStyle::Dotted);
        assert_eq!(m.arrow.head, ArrowHead::None);
    }

    #[test]
    fn parse_all_eight_arrow_types() {
        let input = "\
sequenceDiagram
    A->>B: solid filled
    A->B: solid open
    A-xB: solid cross
    A-)B: solid async
    A-->>B: dotted filled
    A-->B: dotted open
    A--xB: dotted cross
    A--)B: dotted async";
        let d = parse(input).unwrap();
        assert_eq!(d.items.len(), 8);
    }

    #[test]
    fn message_without_label() {
        let d = parse("sequenceDiagram\n    A->>B").unwrap();
        let SequenceItem::Message(ref m) = d.items[0] else {
            panic!("expected message");
        };
        assert!(m.label.is_none());
    }

    #[test]
    fn message_with_spaces_around_arrow() {
        let d = parse("sequenceDiagram\n    Alice ->> Bob : Hello").unwrap();
        let SequenceItem::Message(ref m) = d.items[0] else {
            panic!("expected message");
        };
        assert_eq!(m.from, "Alice");
        assert_eq!(m.to, "Bob");
        assert_eq!(m.label.as_deref(), Some("Hello"));
    }

    #[test]
    fn auto_create_participants_from_message() {
        let d = parse("sequenceDiagram\n    Alice->>Bob: Hello").unwrap();
        assert_eq!(d.participants.len(), 2);
        assert_eq!(d.participants[0].id, "Alice");
        assert_eq!(d.participants[1].id, "Bob");
    }

    #[test]
    fn declared_participants_before_auto_created() {
        let d = parse(
            "sequenceDiagram\n    participant Bob\n    Alice->>Bob: Hello",
        )
        .unwrap();
        // Bob was declared first, Alice auto-created from message
        assert_eq!(d.participants[0].id, "Bob");
        assert_eq!(d.participants[1].id, "Alice");
    }

    #[test]
    fn multiple_messages() {
        let d = parse(
            "sequenceDiagram\n    Alice->>Bob: Hello\n    Bob-->>Alice: Hi",
        )
        .unwrap();
        assert_eq!(d.items.len(), 2);
    }

    // ── Activation / Deactivation ──────────────────────────

    #[test]
    fn parse_activation_suffix_plus() {
        let d = parse("sequenceDiagram\n    Alice->>+Bob: Hello").unwrap();
        let SequenceItem::Message(ref m) = d.items[0] else {
            panic!("expected message");
        };
        assert!(m.activate);
        assert!(!m.deactivate);
    }

    #[test]
    fn parse_deactivation_suffix_minus() {
        let d = parse("sequenceDiagram\n    Alice->>-Bob: Bye").unwrap();
        let SequenceItem::Message(ref m) = d.items[0] else {
            panic!("expected message");
        };
        assert!(!m.activate);
        assert!(m.deactivate);
    }

    #[test]
    fn parse_explicit_activate() {
        let d = parse(
            "sequenceDiagram\n    Alice->>Bob: req\n    activate Bob",
        )
        .unwrap();
        assert_eq!(d.items.len(), 2);
        let SequenceItem::Activation(ref act) = d.items[1] else {
            panic!("expected activation");
        };
        assert_eq!(act.actor, "Bob");
        assert!(act.active);
    }

    #[test]
    fn parse_explicit_deactivate() {
        let d = parse(
            "sequenceDiagram\n    Alice->>Bob: req\n    activate Bob\n    Bob-->>Alice: resp\n    deactivate Bob",
        )
        .unwrap();
        let SequenceItem::Activation(ref act) = d.items[3] else {
            panic!("expected deactivation");
        };
        assert_eq!(act.actor, "Bob");
        assert!(!act.active);
    }

    // ── Notes ──────────────────────────────────────────────

    #[test]
    fn parse_note_right_of() {
        let d = parse(
            "sequenceDiagram\n    Alice->>Bob: hi\n    Note right of Bob: Thinking",
        )
        .unwrap();
        let SequenceItem::Note(ref n) = d.items[1] else {
            panic!("expected note");
        };
        assert!(matches!(&n.position, NotePosition::RightOf(a) if a == "Bob"));
        assert_eq!(n.text, "Thinking");
    }

    #[test]
    fn parse_note_left_of() {
        let d = parse(
            "sequenceDiagram\n    Alice->>Bob: hi\n    Note left of Alice: Waiting",
        )
        .unwrap();
        let SequenceItem::Note(ref n) = d.items[1] else {
            panic!("expected note");
        };
        assert!(matches!(&n.position, NotePosition::LeftOf(a) if a == "Alice"));
    }

    #[test]
    fn parse_note_over_single() {
        let d = parse(
            "sequenceDiagram\n    Note over Alice: Important",
        )
        .unwrap();
        let SequenceItem::Note(ref n) = d.items[0] else {
            panic!("expected note");
        };
        if let NotePosition::Over(ref actors) = n.position {
            assert_eq!(actors, &["Alice"]);
        } else {
            panic!("expected Over");
        }
    }

    #[test]
    fn parse_note_over_multiple() {
        let d = parse(
            "sequenceDiagram\n    Note over Alice, Bob: Spans both",
        )
        .unwrap();
        let SequenceItem::Note(ref n) = d.items[0] else {
            panic!("expected note");
        };
        if let NotePosition::Over(ref actors) = n.position {
            assert_eq!(actors, &["Alice", "Bob"]);
        } else {
            panic!("expected Over");
        }
    }

    #[test]
    fn parse_note_lowercase() {
        let d = parse(
            "sequenceDiagram\n    note right of Bob: lowercase",
        )
        .unwrap();
        assert_eq!(d.items.len(), 1);
    }

    // ── Fragments ──────────────────────────────────────────

    #[test]
    fn parse_loop() {
        let d = parse(
            "sequenceDiagram\n    loop Every minute\n        Alice->>Bob: ping\n    end",
        )
        .unwrap();
        let SequenceItem::Fragment(ref f) = d.items[0] else {
            panic!("expected fragment");
        };
        assert_eq!(f.kind, FragmentKind::Loop);
        assert_eq!(f.label.as_deref(), Some("Every minute"));
        assert_eq!(f.sections.len(), 1);
        assert_eq!(f.sections[0].items.len(), 1);
    }

    #[test]
    fn parse_alt_else() {
        let d = parse(
            "sequenceDiagram\n    alt is valid\n        Alice->>Bob: ok\n    else not valid\n        Alice->>Bob: error\n    end",
        )
        .unwrap();
        let SequenceItem::Fragment(ref f) = d.items[0] else {
            panic!("expected fragment");
        };
        assert_eq!(f.kind, FragmentKind::Alt);
        assert_eq!(f.label.as_deref(), Some("is valid"));
        assert_eq!(f.sections.len(), 2);
        assert_eq!(f.sections[0].label.as_deref(), Some("is valid"));
        assert_eq!(f.sections[1].label.as_deref(), Some("not valid"));
        assert_eq!(f.sections[0].items.len(), 1);
        assert_eq!(f.sections[1].items.len(), 1);
    }

    #[test]
    fn parse_alt_multiple_else() {
        let d = parse(
            "sequenceDiagram\n    alt a\n        A->>B: 1\n    else b\n        A->>B: 2\n    else c\n        A->>B: 3\n    end",
        )
        .unwrap();
        let SequenceItem::Fragment(ref f) = d.items[0] else {
            panic!("expected fragment");
        };
        assert_eq!(f.sections.len(), 3);
    }

    #[test]
    fn parse_opt() {
        let d = parse(
            "sequenceDiagram\n    opt Extra\n        Alice->>Bob: bonus\n    end",
        )
        .unwrap();
        let SequenceItem::Fragment(ref f) = d.items[0] else {
            panic!("expected fragment");
        };
        assert_eq!(f.kind, FragmentKind::Opt);
        assert_eq!(f.sections.len(), 1);
    }

    #[test]
    fn parse_par_and() {
        let d = parse(
            "sequenceDiagram\n    par Task A\n        Alice->>Bob: a\n    and Task B\n        Alice->>Charlie: b\n    end",
        )
        .unwrap();
        let SequenceItem::Fragment(ref f) = d.items[0] else {
            panic!("expected fragment");
        };
        assert_eq!(f.kind, FragmentKind::Par);
        assert_eq!(f.sections.len(), 2);
        assert_eq!(f.sections[0].label.as_deref(), Some("Task A"));
        assert_eq!(f.sections[1].label.as_deref(), Some("Task B"));
    }

    #[test]
    fn parse_critical_option() {
        let d = parse(
            "sequenceDiagram\n    critical Establish connection\n        A->>B: connect\n    option Timeout\n        A->>B: retry\n    end",
        )
        .unwrap();
        let SequenceItem::Fragment(ref f) = d.items[0] else {
            panic!("expected fragment");
        };
        assert_eq!(f.kind, FragmentKind::Critical);
        assert_eq!(f.sections.len(), 2);
    }

    #[test]
    fn parse_break() {
        let d = parse(
            "sequenceDiagram\n    break When error\n        A->>B: fail\n    end",
        )
        .unwrap();
        let SequenceItem::Fragment(ref f) = d.items[0] else {
            panic!("expected fragment");
        };
        assert_eq!(f.kind, FragmentKind::Break);
    }

    #[test]
    fn parse_nested_fragments() {
        let d = parse(
            "sequenceDiagram\n    alt check\n        loop retry\n            A->>B: try\n        end\n    else fail\n        A->>C: fail\n    end",
        )
        .unwrap();
        let SequenceItem::Fragment(ref outer) = d.items[0] else {
            panic!("expected fragment");
        };
        assert_eq!(outer.kind, FragmentKind::Alt);
        assert_eq!(outer.sections.len(), 2);
        // First section contains the inner loop
        let SequenceItem::Fragment(ref inner) = outer.sections[0].items[0] else {
            panic!("expected inner fragment");
        };
        assert_eq!(inner.kind, FragmentKind::Loop);
    }

    #[test]
    fn unclosed_fragment_at_eof() {
        let d = parse(
            "sequenceDiagram\n    loop forever\n        A->>B: ping",
        )
        .unwrap();
        let SequenceItem::Fragment(ref f) = d.items[0] else {
            panic!("expected fragment");
        };
        assert_eq!(f.kind, FragmentKind::Loop);
        assert_eq!(f.sections[0].items.len(), 1);
    }

    // ── Title ──────────────────────────────────────────────

    #[test]
    fn parse_title_with_colon() {
        let d =
            parse("sequenceDiagram\n    title: My Diagram\n    A->>B: hi").unwrap();
        assert_eq!(d.title.as_deref(), Some("My Diagram"));
    }

    #[test]
    fn parse_title_without_colon() {
        let d =
            parse("sequenceDiagram\n    title My Diagram\n    A->>B: hi").unwrap();
        assert_eq!(d.title.as_deref(), Some("My Diagram"));
    }

    // ── Autonumber ─────────────────────────────────────────

    #[test]
    fn parse_autonumber_default() {
        let d = parse(
            "sequenceDiagram\n    autonumber\n    A->>B: first",
        )
        .unwrap();
        let an = d.autonumber.unwrap();
        assert_eq!(an.start, 1);
        assert_eq!(an.step, 1);
    }

    #[test]
    fn parse_autonumber_with_start() {
        let d = parse(
            "sequenceDiagram\n    autonumber 10\n    A->>B: first",
        )
        .unwrap();
        let an = d.autonumber.unwrap();
        assert_eq!(an.start, 10);
        assert_eq!(an.step, 1);
    }

    #[test]
    fn parse_autonumber_with_start_and_step() {
        let d = parse(
            "sequenceDiagram\n    autonumber 5 2\n    A->>B: first",
        )
        .unwrap();
        let an = d.autonumber.unwrap();
        assert_eq!(an.start, 5);
        assert_eq!(an.step, 2);
    }

    #[test]
    fn parse_autonumber_off() {
        let d = parse(
            "sequenceDiagram\n    autonumber off\n    A->>B: first",
        )
        .unwrap();
        assert!(d.autonumber.is_none());
    }

    // ── Comments ───────────────────────────────────────────

    #[test]
    fn comments_ignored() {
        let d = parse(
            "sequenceDiagram\n    %% This is a comment\n    Alice->>Bob: Hello",
        )
        .unwrap();
        assert_eq!(d.items.len(), 1);
    }

    #[test]
    fn inline_comments_between_statements() {
        let d = parse(
            "sequenceDiagram\n    Alice->>Bob: hi\n    %% middle\n    Bob-->>Alice: bye",
        )
        .unwrap();
        assert_eq!(d.items.len(), 2);
    }

    // ── Realistic diagrams ─────────────────────────────────

    #[test]
    fn parse_realistic_sequence() {
        let input = "\
sequenceDiagram
    participant Client
    participant Server
    participant DB

    Client->>Server: POST /login
    activate Server
    Server->>DB: SELECT user
    activate DB
    DB-->>Server: user record
    deactivate DB
    alt valid credentials
        Server-->>Client: 200 OK
    else invalid
        Server-->>Client: 401 Unauthorized
    end
    deactivate Server";
        let d = parse(input).unwrap();
        assert_eq!(d.participants.len(), 3);
        // 3 messages + 2 activate + 2 deactivate + 1 alt fragment = 8
        assert_eq!(d.items.len(), 8);
    }

    #[test]
    fn parse_complete_diagram_with_all_features() {
        let input = "\
sequenceDiagram
    title: Authentication Flow
    autonumber

    actor User
    participant App as Application
    participant Auth as Auth Service

    User->>App: Login request
    activate App
    App->>+Auth: Validate credentials
    Note right of Auth: Check database

    alt valid
        Auth-->>App: Token
        App-->>User: Welcome
    else invalid
        Auth-->>App: Error
        App-->>User: Please retry
    end

    deactivate App";
        let d = parse(input).unwrap();
        assert_eq!(d.title.as_deref(), Some("Authentication Flow"));
        assert!(d.autonumber.is_some());
        assert_eq!(d.participants.len(), 3);
        assert_eq!(d.participants[0].kind, ParticipantKind::Actor);
        assert_eq!(d.participants[1].label, "Application");
        assert_eq!(d.participants[2].label, "Auth Service");
    }

    // ── Edge cases / negative tests ────────────────────────

    #[test]
    fn garbage_between_statements_skipped() {
        let d = parse(
            "sequenceDiagram\n    Alice->>Bob: hi\n    $$$ garbage\n    Bob-->>Alice: bye",
        )
        .unwrap();
        assert_eq!(d.items.len(), 2);
    }

    #[test]
    fn self_message() {
        let d = parse("sequenceDiagram\n    Alice->>Alice: think").unwrap();
        let SequenceItem::Message(ref m) = d.items[0] else {
            panic!("expected message");
        };
        assert_eq!(m.from, "Alice");
        assert_eq!(m.to, "Alice");
    }

    #[test]
    fn fragment_no_label() {
        let d = parse(
            "sequenceDiagram\n    loop\n        A->>B: ping\n    end",
        )
        .unwrap();
        let SequenceItem::Fragment(ref f) = d.items[0] else {
            panic!("expected fragment");
        };
        assert!(f.label.is_none());
    }

    #[test]
    fn deeply_nested_alt_in_alt() {
        let input = "\
sequenceDiagram
    alt outer
        alt inner
            A->>B: yes
        else
            A->>B: no
        end
    else outer-else
        A->>C: fail
    end";
        let d = parse(input).unwrap();
        let SequenceItem::Fragment(ref outer) = d.items[0] else {
            panic!("expected fragment");
        };
        assert_eq!(outer.sections.len(), 2);
        let SequenceItem::Fragment(ref inner) = outer.sections[0].items[0] else {
            panic!("expected inner fragment");
        };
        assert_eq!(inner.kind, FragmentKind::Alt);
        assert_eq!(inner.sections.len(), 2);
    }

    #[test]
    fn message_after_fragment() {
        let d = parse(
            "sequenceDiagram\n    loop x\n        A->>B: inside\n    end\n    A->>C: outside",
        )
        .unwrap();
        assert_eq!(d.items.len(), 2);
        assert!(matches!(d.items[0], SequenceItem::Fragment(_)));
        assert!(matches!(d.items[1], SequenceItem::Message(_)));
    }

    #[test]
    fn note_invalid_position_skipped() {
        let d = parse(
            "sequenceDiagram\n    Note above Alice: bad\n    Alice->>Bob: hi",
        )
        .unwrap();
        // The invalid note is skipped, message still parses
        let msg_count = d
            .items
            .iter()
            .filter(|i| matches!(i, SequenceItem::Message(_)))
            .count();
        assert_eq!(msg_count, 1);
    }

    #[test]
    fn empty_fragment() {
        let d =
            parse("sequenceDiagram\n    loop empty\n    end").unwrap();
        let SequenceItem::Fragment(ref f) = d.items[0] else {
            panic!("expected fragment");
        };
        assert!(f.sections[0].items.is_empty());
    }
}
