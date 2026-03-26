use super::*;

#[test]
fn diagram_construction() {
    let mut d = SequenceDiagram::new();
    d.participants.push(Participant::new("Alice", "Alice"));
    d.participants.push(Participant::new("Bob", "Bob"));
    d.items.push(SequenceItem::Message(
        Message::new("Alice", "Bob", ArrowStyle::SOLID_FILLED).with_label("Hello"),
    ));

    assert_eq!(d.participants.len(), 2);
    assert_eq!(d.items.len(), 1);
    assert!(d.participant("Alice").is_some());
    assert!(d.participant("Charlie").is_none());
}

#[test]
fn participant_kinds() {
    let box_p = Participant::new("A", "Alice");
    assert_eq!(box_p.kind, ParticipantKind::Box);

    let actor_p = Participant::actor("B", "Bob");
    assert_eq!(actor_p.kind, ParticipantKind::Actor);
}

#[test]
fn message_with_label() {
    let m = Message::new("A", "B", ArrowStyle::DOTTED_OPEN).with_label("response");
    assert_eq!(m.label.as_deref(), Some("response"));
    assert_eq!(m.arrow.line, LineStyle::Dotted);
    assert_eq!(m.arrow.head, ArrowHead::Open);
}

#[test]
fn message_activation() {
    let mut m = Message::new("A", "B", ArrowStyle::SOLID_FILLED);
    m.activate = true;
    assert!(m.activate);
    assert!(!m.deactivate);
}

#[test]
fn arrow_style_constants() {
    assert_eq!(ArrowStyle::SOLID_FILLED.line, LineStyle::Solid);
    assert_eq!(ArrowStyle::SOLID_FILLED.head, ArrowHead::Filled);
    assert_eq!(ArrowStyle::DOTTED_CROSS.line, LineStyle::Dotted);
    assert_eq!(ArrowStyle::DOTTED_CROSS.head, ArrowHead::Cross);
}

#[test]
fn note_positions() {
    let left = Note {
        position: NotePosition::LeftOf("A".into()),
        text: "hi".into(),
    };
    let right = Note {
        position: NotePosition::RightOf("B".into()),
        text: "hi".into(),
    };
    let over = Note {
        position: NotePosition::Over(vec!["A".into(), "B".into()]),
        text: "spans both".into(),
    };
    assert!(matches!(left.position, NotePosition::LeftOf(_)));
    assert!(matches!(right.position, NotePosition::RightOf(_)));
    if let NotePosition::Over(ids) = &over.position {
        assert_eq!(ids.len(), 2);
    }
}

#[test]
fn fragment_construction() {
    let mut frag = Fragment::new(FragmentKind::Alt).with_label("is valid?");
    let mut then_section = FragmentSection::new().with_label("yes");
    then_section.items.push(SequenceItem::Message(
        Message::new("A", "B", ArrowStyle::SOLID_FILLED).with_label("proceed"),
    ));
    let else_section = FragmentSection::new().with_label("no");
    frag.sections.push(then_section);
    frag.sections.push(else_section);

    assert_eq!(frag.kind, FragmentKind::Alt);
    assert_eq!(frag.label.as_deref(), Some("is valid?"));
    assert_eq!(frag.sections.len(), 2);
    assert_eq!(frag.sections[0].items.len(), 1);
}

#[test]
fn fragment_kind_display() {
    assert_eq!(FragmentKind::Loop.to_string(), "loop");
    assert_eq!(FragmentKind::Alt.to_string(), "alt");
    assert_eq!(FragmentKind::Par.to_string(), "par");
    assert_eq!(FragmentKind::Critical.to_string(), "critical");
}

#[test]
fn autonumber_default() {
    let an = AutoNumber::default();
    assert_eq!(an.start, 1);
    assert_eq!(an.step, 1);
}

#[test]
fn activation_item() {
    let act = Activation {
        actor: "Bob".into(),
        active: true,
    };
    let deact = Activation {
        actor: "Bob".into(),
        active: false,
    };
    assert!(act.active);
    assert!(!deact.active);
}

#[test]
fn default_diagram() {
    let d = SequenceDiagram::default();
    assert!(d.participants.is_empty());
    assert!(d.items.is_empty());
    assert!(d.title.is_none());
    assert!(d.autonumber.is_none());
}
