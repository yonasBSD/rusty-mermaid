use super::*;
use rusty_mermaid_core::SimpleTextMeasure;

fn tm() -> SimpleTextMeasure {
    SimpleTextMeasure::default()
}

fn two_actor_diagram() -> SequenceDiagram {
    let mut d = SequenceDiagram::new();
    d.participants.push(Participant::new("Alice", "Alice"));
    d.participants.push(Participant::new("Bob", "Bob"));
    d
}

#[test]
fn two_actors_positioned_lr() {
    let d = two_actor_diagram();
    let l = layout(&d, &tm());

    assert_eq!(l.actors.len(), 2);
    assert!(l.actors[0].x < l.actors[1].x, "Alice should be left of Bob");
    assert!(l.actors[0].width >= MIN_ACTOR_WIDTH);
    assert!(l.actors[0].height > 0.0);
    assert_eq!(l.lifelines.len(), 2);
    assert_eq!(l.bottom_actors.len(), 2);
}

#[test]
fn single_actor() {
    let mut d = SequenceDiagram::new();
    d.participants.push(Participant::new("A", "Alice"));
    let l = layout(&d, &tm());

    assert_eq!(l.actors.len(), 1);
    assert_eq!(l.lifelines.len(), 1);
    assert!(l.width > 0.0);
    assert!(l.height > 0.0);
}

#[test]
fn empty_diagram() {
    let d = SequenceDiagram::new();
    let l = layout(&d, &tm());

    assert!(l.actors.is_empty());
    assert!(l.lifelines.is_empty());
    assert!(l.messages.is_empty());
    assert!((l.width - 2.0 * DIAGRAM_MARGIN).abs() < f64::EPSILON);
}

#[test]
fn message_between_actors() {
    let mut d = two_actor_diagram();
    d.items.push(SequenceItem::Message(
        Message::new("Alice", "Bob", ArrowStyle::SOLID_FILLED).with_label("Hello"),
    ));
    let l = layout(&d, &tm());

    assert_eq!(l.messages.len(), 1);
    let msg = &l.messages[0];
    assert!(!msg.is_self);
    assert!(msg.from_x < msg.to_x);
    assert!(msg.y > l.actors[0].y + l.actors[0].height);
}

#[test]
fn self_message_extra_height() {
    let mut d = SequenceDiagram::new();
    d.participants.push(Participant::new("A", "Alice"));
    d.items.push(SequenceItem::Message(
        Message::new("A", "A", ArrowStyle::SOLID_FILLED).with_label("think"),
    ));
    d.items.push(SequenceItem::Message(
        Message::new("A", "A", ArrowStyle::SOLID_FILLED).with_label("again"),
    ));
    let l = layout(&d, &tm());

    assert_eq!(l.messages.len(), 2);
    assert!(l.messages[0].is_self);
    // Second self-message should be further down by at least SELF_MSG_HEIGHT + MESSAGE_MARGIN.
    let gap = l.messages[1].y - l.messages[0].y;
    assert!(
        gap >= SELF_MSG_HEIGHT + MESSAGE_MARGIN - f64::EPSILON,
        "gap={gap} expected >= {}",
        SELF_MSG_HEIGHT + MESSAGE_MARGIN
    );
}

#[test]
fn note_left_of() {
    let mut d = two_actor_diagram();
    d.items.push(SequenceItem::Note(Note {
        position: NotePosition::LeftOf("Alice".into()),
        text: "note".into(),
    }));
    let l = layout(&d, &tm());

    assert_eq!(l.notes.len(), 1);
    let note = &l.notes[0];
    assert!(
        note.x + note.width < l.actors[0].x,
        "note right edge should be left of Alice center"
    );
}

#[test]
fn note_right_of() {
    let mut d = two_actor_diagram();
    d.items.push(SequenceItem::Note(Note {
        position: NotePosition::RightOf("Bob".into()),
        text: "note".into(),
    }));
    let l = layout(&d, &tm());

    assert_eq!(l.notes.len(), 1);
    assert!(l.notes[0].x > l.actors[1].x, "note should be right of Bob");
}

#[test]
fn note_over_single() {
    let mut d = two_actor_diagram();
    d.items.push(SequenceItem::Note(Note {
        position: NotePosition::Over(vec!["Alice".into()]),
        text: "centered".into(),
    }));
    let l = layout(&d, &tm());

    assert_eq!(l.notes.len(), 1);
    let note = &l.notes[0];
    let note_center = note.x + note.width / 2.0;
    assert!(
        (note_center - l.actors[0].x).abs() < 1.0,
        "note should be centered on Alice"
    );
}

#[test]
fn note_over_span() {
    let mut d = two_actor_diagram();
    d.items.push(SequenceItem::Note(Note {
        position: NotePosition::Over(vec!["Alice".into(), "Bob".into()]),
        text: "spans both".into(),
    }));
    let l = layout(&d, &tm());

    assert_eq!(l.notes.len(), 1);
    let note = &l.notes[0];
    let mid = (l.actors[0].x + l.actors[1].x) / 2.0;
    let note_center = note.x + note.width / 2.0;
    assert!(
        (note_center - mid).abs() < 1.0,
        "note should be centered between Alice and Bob"
    );
}

#[test]
fn fragment_bounds_enclose_content() {
    let mut d = two_actor_diagram();
    let mut section = FragmentSection::new().with_label("condition");
    section.items.push(SequenceItem::Message(
        Message::new("Alice", "Bob", ArrowStyle::SOLID_FILLED).with_label("do"),
    ));
    let mut frag = Fragment::new(FragmentKind::Loop).with_label("repeat");
    frag.sections.push(section);
    d.items.push(SequenceItem::Fragment(frag));
    let l = layout(&d, &tm());

    assert_eq!(l.fragments.len(), 1);
    let f = &l.fragments[0];
    assert!(f.width > 0.0);
    assert!(f.height > FRAGMENT_LABEL_HEIGHT);
    // Fragment should contain the message Y.
    let msg_y = l.messages[0].y;
    assert!(f.y < msg_y && msg_y < f.y + f.height);
}

#[test]
fn alt_fragment_has_section_dividers() {
    let mut d = two_actor_diagram();
    let mut then_sec = FragmentSection::new().with_label("yes");
    then_sec.items.push(SequenceItem::Message(
        Message::new("Alice", "Bob", ArrowStyle::SOLID_FILLED).with_label("ok"),
    ));
    let mut else_sec = FragmentSection::new().with_label("no");
    else_sec.items.push(SequenceItem::Message(
        Message::new("Bob", "Alice", ArrowStyle::DOTTED_FILLED).with_label("err"),
    ));
    let mut frag = Fragment::new(FragmentKind::Alt).with_label("check");
    frag.sections.push(then_sec);
    frag.sections.push(else_sec);
    d.items.push(SequenceItem::Fragment(frag));
    let l = layout(&d, &tm());

    assert_eq!(l.fragments.len(), 1);
    assert_eq!(l.fragments[0].sections.len(), 1); // divider between section 0 and 1
    assert_eq!(l.messages.len(), 2);
}

#[test]
fn activation_tracking() {
    let mut d = two_actor_diagram();
    let mut msg = Message::new("Alice", "Bob", ArrowStyle::SOLID_FILLED);
    msg.activate = true;
    d.items.push(SequenceItem::Message(msg.with_label("req")));
    let mut reply = Message::new("Bob", "Alice", ArrowStyle::DOTTED_FILLED);
    reply.deactivate = true;
    d.items.push(SequenceItem::Message(reply.with_label("res")));
    let l = layout(&d, &tm());

    assert_eq!(l.activations.len(), 1);
    let act = &l.activations[0];
    assert_eq!(act.actor_id, "Bob");
    assert!(act.top_y < act.bottom_y);
}

#[test]
fn explicit_activation() {
    let mut d = SequenceDiagram::new();
    d.participants.push(Participant::new("A", "Alice"));
    d.items.push(SequenceItem::Activation(Activation {
        actor: "A".into(),
        active: true,
    }));
    d.items.push(SequenceItem::Message(
        Message::new("A", "A", ArrowStyle::SOLID_FILLED).with_label("work"),
    ));
    d.items.push(SequenceItem::Activation(Activation {
        actor: "A".into(),
        active: false,
    }));
    let l = layout(&d, &tm());

    assert_eq!(l.activations.len(), 1);
    assert_eq!(l.activations[0].actor_id, "A");
}

#[test]
fn unclosed_activation_auto_closed() {
    let mut d = two_actor_diagram();
    let mut msg = Message::new("Alice", "Bob", ArrowStyle::SOLID_FILLED);
    msg.activate = true;
    d.items.push(SequenceItem::Message(msg.with_label("req")));
    // No deactivate — should auto-close at end.
    let l = layout(&d, &tm());

    assert_eq!(l.activations.len(), 1);
}

#[test]
fn long_label_widens_gap() {
    let mut d = two_actor_diagram();
    d.items.push(SequenceItem::Message(
        Message::new("Alice", "Bob", ArrowStyle::SOLID_FILLED)
            .with_label("this is a very long message label that should widen the gap"),
    ));
    let l_wide = layout(&d, &tm());

    let mut d2 = two_actor_diagram();
    d2.items.push(SequenceItem::Message(
        Message::new("Alice", "Bob", ArrowStyle::SOLID_FILLED).with_label("hi"),
    ));
    let l_narrow = layout(&d2, &tm());

    assert!(
        l_wide.actors[1].x - l_wide.actors[0].x > l_narrow.actors[1].x - l_narrow.actors[0].x,
        "long label should produce wider spacing"
    );
}

#[test]
fn lifelines_span_actors_to_bottom() {
    let d = two_actor_diagram();
    let l = layout(&d, &tm());

    for ll in &l.lifelines {
        assert!(ll.top_y < ll.bottom_y);
        assert!((ll.top_y - (l.actors[0].y + l.actors[0].height)).abs() < f64::EPSILON);
        assert!((ll.bottom_y - l.bottom_actors[0].y).abs() < f64::EPSILON);
    }
}

#[test]
fn bottom_actors_mirror_top() {
    let d = two_actor_diagram();
    let l = layout(&d, &tm());

    for (top, bot) in l.actors.iter().zip(l.bottom_actors.iter()) {
        assert_eq!(top.id, bot.id);
        assert!((top.x - bot.x).abs() < f64::EPSILON);
        assert!((top.width - bot.width).abs() < f64::EPSILON);
        assert!(bot.y > top.y);
    }
}

#[test]
fn title_shifts_actors_down() {
    let mut d = two_actor_diagram();
    d.title = Some("My Diagram".into());
    let l_titled = layout(&d, &tm());

    let d2 = two_actor_diagram();
    let l_no_title = layout(&d2, &tm());

    assert!(
        l_titled.actors[0].y > l_no_title.actors[0].y,
        "title should push actors down"
    );
}

#[test]
fn messages_advance_y_monotonically() {
    let mut d = two_actor_diagram();
    for label in &["first", "second", "third"] {
        d.items.push(SequenceItem::Message(
            Message::new("Alice", "Bob", ArrowStyle::SOLID_FILLED).with_label(*label),
        ));
    }
    let l = layout(&d, &tm());

    for w in l.messages.windows(2) {
        assert!(
            w[1].y > w[0].y,
            "messages must advance downward: {} vs {}",
            w[0].y,
            w[1].y
        );
    }
}

// -- Autonumber tests --

#[test]
fn autonumber_default() {
    let mut d = two_actor_diagram();
    d.autonumber = Some(AutoNumber { start: 1, step: 1 });
    d.items.push(SequenceItem::Message(
        Message::new("Alice", "Bob", ArrowStyle::SOLID_FILLED).with_label("a"),
    ));
    d.items.push(SequenceItem::Message(
        Message::new("Bob", "Alice", ArrowStyle::DOTTED_FILLED).with_label("b"),
    ));
    d.items.push(SequenceItem::Message(
        Message::new("Alice", "Bob", ArrowStyle::SOLID_FILLED).with_label("c"),
    ));
    let l = layout(&d, &tm());
    assert_eq!(l.messages[0].number, Some(1));
    assert_eq!(l.messages[1].number, Some(2));
    assert_eq!(l.messages[2].number, Some(3));
}

#[test]
fn autonumber_custom_start_step() {
    let mut d = two_actor_diagram();
    d.autonumber = Some(AutoNumber { start: 10, step: 5 });
    d.items.push(SequenceItem::Message(
        Message::new("Alice", "Bob", ArrowStyle::SOLID_FILLED).with_label("a"),
    ));
    d.items.push(SequenceItem::Message(
        Message::new("Bob", "Alice", ArrowStyle::DOTTED_FILLED).with_label("b"),
    ));
    let l = layout(&d, &tm());
    assert_eq!(l.messages[0].number, Some(10));
    assert_eq!(l.messages[1].number, Some(15));
}

#[test]
fn no_autonumber_means_no_numbers() {
    let mut d = two_actor_diagram();
    d.items.push(SequenceItem::Message(
        Message::new("Alice", "Bob", ArrowStyle::SOLID_FILLED).with_label("a"),
    ));
    let l = layout(&d, &tm());
    assert_eq!(l.messages[0].number, None);
}

#[test]
fn autonumber_counts_self_messages() {
    let mut d = two_actor_diagram();
    d.autonumber = Some(AutoNumber { start: 1, step: 1 });
    d.items.push(SequenceItem::Message(
        Message::new("Alice", "Bob", ArrowStyle::SOLID_FILLED).with_label("a"),
    ));
    d.items.push(SequenceItem::Message(
        Message::new("Bob", "Bob", ArrowStyle::SOLID_FILLED).with_label("self"),
    ));
    d.items.push(SequenceItem::Message(
        Message::new("Bob", "Alice", ArrowStyle::DOTTED_FILLED).with_label("c"),
    ));
    let l = layout(&d, &tm());
    assert_eq!(l.messages[0].number, Some(1));
    assert_eq!(l.messages[1].number, Some(2));
    assert_eq!(l.messages[2].number, Some(3));
}

#[test]
fn autonumber_inside_fragments() {
    let mut d = two_actor_diagram();
    d.autonumber = Some(AutoNumber { start: 1, step: 1 });
    d.items.push(SequenceItem::Message(
        Message::new("Alice", "Bob", ArrowStyle::SOLID_FILLED).with_label("before"),
    ));
    d.items.push(SequenceItem::Fragment(Fragment {
        kind: FragmentKind::Loop,
        label: Some("retry".into()),
        sections: vec![FragmentSection {
            label: None,
            items: vec![SequenceItem::Message(
                Message::new("Bob", "Alice", ArrowStyle::DOTTED_FILLED).with_label("inside"),
            )],
        }],
    }));
    d.items.push(SequenceItem::Message(
        Message::new("Alice", "Bob", ArrowStyle::SOLID_FILLED).with_label("after"),
    ));
    let l = layout(&d, &tm());
    assert_eq!(l.messages[0].number, Some(1));
    assert_eq!(l.messages[1].number, Some(2));
    assert_eq!(l.messages[2].number, Some(3));
}

// -- Par/Critical/Break/Opt fragment tests --

#[test]
fn par_fragment_layout() {
    let mut d = SequenceDiagram::new();
    d.participants.push(Participant::new("A", "A"));
    d.participants.push(Participant::new("B", "B"));
    d.participants.push(Participant::new("C", "C"));
    d.items.push(SequenceItem::Fragment(Fragment {
        kind: FragmentKind::Par,
        label: Some("parallel".into()),
        sections: vec![
            FragmentSection {
                label: None,
                items: vec![SequenceItem::Message(
                    Message::new("A", "B", ArrowStyle::SOLID_FILLED).with_label("task1"),
                )],
            },
            FragmentSection {
                label: Some("and".into()),
                items: vec![SequenceItem::Message(
                    Message::new("A", "C", ArrowStyle::SOLID_FILLED).with_label("task2"),
                )],
            },
        ],
    }));
    let l = layout(&d, &tm());
    assert_eq!(l.fragments.len(), 1);
    assert_eq!(l.fragments[0].sections.len(), 1); // first section has no divider
    assert!(l.fragments[0].height > 0.0);
    assert_eq!(l.messages.len(), 2);
}

#[test]
fn opt_fragment_layout() {
    let mut d = two_actor_diagram();
    d.items.push(SequenceItem::Fragment(Fragment {
        kind: FragmentKind::Opt,
        label: Some("optional".into()),
        sections: vec![FragmentSection {
            label: None,
            items: vec![SequenceItem::Message(
                Message::new("Alice", "Bob", ArrowStyle::SOLID_FILLED).with_label("maybe"),
            )],
        }],
    }));
    let l = layout(&d, &tm());
    assert_eq!(l.fragments.len(), 1);
    assert!(l.fragments[0].sections.is_empty()); // single section = no dividers
}

#[test]
fn nested_fragments_outer_before_inner() {
    let mut d = two_actor_diagram();
    let inner = Fragment {
        kind: FragmentKind::Alt,
        label: Some("available".into()),
        sections: vec![FragmentSection {
            label: None,
            items: vec![SequenceItem::Message(
                Message::new("Bob", "Alice", ArrowStyle::SOLID_FILLED).with_label("data"),
            )],
        }],
    };
    let outer = Fragment {
        kind: FragmentKind::Loop,
        label: Some("retry".into()),
        sections: vec![FragmentSection {
            label: None,
            items: vec![SequenceItem::Fragment(inner)],
        }],
    };
    d.items.push(SequenceItem::Fragment(outer));
    let l = layout(&d, &tm());

    assert_eq!(l.fragments.len(), 2);
    // Outer fragment must come first (renders behind).
    assert_eq!(l.fragments[0].kind, FragmentKind::Loop);
    assert_eq!(l.fragments[1].kind, FragmentKind::Alt);
    // Outer must fully enclose inner.
    let outer_f = &l.fragments[0];
    let inner_f = &l.fragments[1];
    assert!(outer_f.y <= inner_f.y);
    assert!(outer_f.x <= inner_f.x);
    assert!(outer_f.y + outer_f.height >= inner_f.y + inner_f.height);
}

#[test]
fn break_fragment_layout() {
    let mut d = two_actor_diagram();
    d.items.push(SequenceItem::Message(
        Message::new("Alice", "Bob", ArrowStyle::SOLID_FILLED).with_label("request"),
    ));
    d.items.push(SequenceItem::Fragment(Fragment {
        kind: FragmentKind::Break,
        label: Some("on error".into()),
        sections: vec![FragmentSection {
            label: None,
            items: vec![SequenceItem::Message(
                Message::new("Bob", "Alice", ArrowStyle::DOTTED_FILLED).with_label("error"),
            )],
        }],
    }));
    let l = layout(&d, &tm());
    assert_eq!(l.fragments.len(), 1);
    assert_eq!(l.messages.len(), 2);
    // Fragment must be below the first message.
    assert!(l.fragments[0].y > l.messages[0].y);
}
