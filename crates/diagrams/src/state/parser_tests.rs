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
