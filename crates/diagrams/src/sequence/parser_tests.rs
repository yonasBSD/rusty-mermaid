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
    let d = parse("sequenceDiagram\n    participant A as \"Alice Smith\"").unwrap();
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
    let d = parse("sequenceDiagram\n    participant A\n    participant A").unwrap();
    assert_eq!(d.participants.len(), 1);
}

#[test]
fn participant_order_preserved() {
    let d =
        parse("sequenceDiagram\n    participant C\n    participant A\n    participant B").unwrap();
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
    let d = parse("sequenceDiagram\n    participant Bob\n    Alice->>Bob: Hello").unwrap();
    // Bob was declared first, Alice auto-created from message
    assert_eq!(d.participants[0].id, "Bob");
    assert_eq!(d.participants[1].id, "Alice");
}

#[test]
fn multiple_messages() {
    let d = parse("sequenceDiagram\n    Alice->>Bob: Hello\n    Bob-->>Alice: Hi").unwrap();
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
    let d = parse("sequenceDiagram\n    Alice->>Bob: req\n    activate Bob").unwrap();
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
    let d = parse("sequenceDiagram\n    Alice->>Bob: hi\n    Note right of Bob: Thinking").unwrap();
    let SequenceItem::Note(ref n) = d.items[1] else {
        panic!("expected note");
    };
    assert!(matches!(&n.position, NotePosition::RightOf(a) if a == "Bob"));
    assert_eq!(n.text, "Thinking");
}

#[test]
fn parse_note_left_of() {
    let d = parse("sequenceDiagram\n    Alice->>Bob: hi\n    Note left of Alice: Waiting").unwrap();
    let SequenceItem::Note(ref n) = d.items[1] else {
        panic!("expected note");
    };
    assert!(matches!(&n.position, NotePosition::LeftOf(a) if a == "Alice"));
}

#[test]
fn parse_note_over_single() {
    let d = parse("sequenceDiagram\n    Note over Alice: Important").unwrap();
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
    let d = parse("sequenceDiagram\n    Note over Alice, Bob: Spans both").unwrap();
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
    let d = parse("sequenceDiagram\n    note right of Bob: lowercase").unwrap();
    assert_eq!(d.items.len(), 1);
}

// ── Fragments ──────────────────────────────────────────

#[test]
fn parse_loop() {
    let d = parse("sequenceDiagram\n    loop Every minute\n        Alice->>Bob: ping\n    end")
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
    let d = parse("sequenceDiagram\n    opt Extra\n        Alice->>Bob: bonus\n    end").unwrap();
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
    let d = parse("sequenceDiagram\n    break When error\n        A->>B: fail\n    end").unwrap();
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
    let d = parse("sequenceDiagram\n    loop forever\n        A->>B: ping").unwrap();
    let SequenceItem::Fragment(ref f) = d.items[0] else {
        panic!("expected fragment");
    };
    assert_eq!(f.kind, FragmentKind::Loop);
    assert_eq!(f.sections[0].items.len(), 1);
}

// ── Title ──────────────────────────────────────────────

#[test]
fn parse_title_with_colon() {
    let d = parse("sequenceDiagram\n    title: My Diagram\n    A->>B: hi").unwrap();
    assert_eq!(d.title.as_deref(), Some("My Diagram"));
}

#[test]
fn parse_title_without_colon() {
    let d = parse("sequenceDiagram\n    title My Diagram\n    A->>B: hi").unwrap();
    assert_eq!(d.title.as_deref(), Some("My Diagram"));
}

// ── Autonumber ─────────────────────────────────────────

#[test]
fn parse_autonumber_default() {
    let d = parse("sequenceDiagram\n    autonumber\n    A->>B: first").unwrap();
    let an = d.autonumber.unwrap();
    assert_eq!(an.start, 1);
    assert_eq!(an.step, 1);
}

#[test]
fn parse_autonumber_with_start() {
    let d = parse("sequenceDiagram\n    autonumber 10\n    A->>B: first").unwrap();
    let an = d.autonumber.unwrap();
    assert_eq!(an.start, 10);
    assert_eq!(an.step, 1);
}

#[test]
fn parse_autonumber_with_start_and_step() {
    let d = parse("sequenceDiagram\n    autonumber 5 2\n    A->>B: first").unwrap();
    let an = d.autonumber.unwrap();
    assert_eq!(an.start, 5);
    assert_eq!(an.step, 2);
}

#[test]
fn parse_autonumber_off() {
    let d = parse("sequenceDiagram\n    autonumber off\n    A->>B: first").unwrap();
    assert!(d.autonumber.is_none());
}

// ── Comments ───────────────────────────────────────────

#[test]
fn comments_ignored() {
    let d = parse("sequenceDiagram\n    %% This is a comment\n    Alice->>Bob: Hello").unwrap();
    assert_eq!(d.items.len(), 1);
}

#[test]
fn inline_comments_between_statements() {
    let d = parse("sequenceDiagram\n    Alice->>Bob: hi\n    %% middle\n    Bob-->>Alice: bye")
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
    let d = parse("sequenceDiagram\n    Alice->>Bob: hi\n    $$$ garbage\n    Bob-->>Alice: bye")
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
    let d = parse("sequenceDiagram\n    loop\n        A->>B: ping\n    end").unwrap();
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
    let d =
        parse("sequenceDiagram\n    loop x\n        A->>B: inside\n    end\n    A->>C: outside")
            .unwrap();
    assert_eq!(d.items.len(), 2);
    assert!(matches!(d.items[0], SequenceItem::Fragment(_)));
    assert!(matches!(d.items[1], SequenceItem::Message(_)));
}

#[test]
fn note_invalid_position_skipped() {
    let d = parse("sequenceDiagram\n    Note above Alice: bad\n    Alice->>Bob: hi").unwrap();
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
    let d = parse("sequenceDiagram\n    loop empty\n    end").unwrap();
    let SequenceItem::Fragment(ref f) = d.items[0] else {
        panic!("expected fragment");
    };
    assert!(f.sections[0].items.is_empty());
}
