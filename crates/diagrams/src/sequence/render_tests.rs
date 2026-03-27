use super::*;
use crate::common::test_helpers::test_helpers::*;
use ir::*;
use rusty_mermaid_core::SimpleTextMeasure;

fn make_scene(d: &SequenceDiagram) -> Scene {
    let l = layout::layout(d, &SimpleTextMeasure::default());
    to_scene(&l, &Theme::default())
}

fn two_actor_diagram() -> SequenceDiagram {
    let mut d = SequenceDiagram::new();
    d.participants.push(Participant::new("Alice", "Alice"));
    d.participants.push(Participant::new("Bob", "Bob"));
    d
}

#[test]
fn basic_scene_valid() {
    let mut d = two_actor_diagram();
    d.items.push(SequenceItem::Message(
        Message::new("Alice", "Bob", ArrowStyle::SOLID_FILLED).with_label("Hello"),
    ));
    let scene = make_scene(&d);
    assert_scene_valid(&scene);
}

#[test]
fn actors_produce_rects_and_text() {
    let d = two_actor_diagram();
    let scene = make_scene(&d);
    // 2 top actors + 2 bottom actors = 4 rects minimum.
    assert!(count_rects(&scene) >= 4);
    assert!(has_text(&scene, "Alice"));
    assert!(has_text(&scene, "Bob"));
}

#[test]
fn message_produces_path_and_label() {
    let mut d = two_actor_diagram();
    d.items.push(SequenceItem::Message(
        Message::new("Alice", "Bob", ArrowStyle::SOLID_FILLED).with_label("Hello"),
    ));
    let scene = make_scene(&d);
    assert!(has_path(&scene));
    assert!(has_text(&scene, "Hello"));
}

#[test]
fn lifelines_are_dashed_paths() {
    let d = two_actor_diagram();
    let scene = make_scene(&d);
    assert!(count_paths(&scene) >= 2);
}

#[test]
fn note_produces_rect_and_text() {
    let mut d = two_actor_diagram();
    d.items.push(SequenceItem::Note(Note {
        position: NotePosition::RightOf("Bob".into()),
        text: "Important".into(),
    }));
    let scene = make_scene(&d);
    assert!(has_text(&scene, "Important"));
}

#[test]
fn fragment_renders_kind_label() {
    let mut d = two_actor_diagram();
    let mut section = FragmentSection::new().with_label("cond");
    section.items.push(SequenceItem::Message(
        Message::new("Alice", "Bob", ArrowStyle::SOLID_FILLED).with_label("do"),
    ));
    let mut frag = Fragment::new(FragmentKind::Loop).with_label("repeat");
    frag.sections.push(section);
    d.items.push(SequenceItem::Fragment(frag));
    let scene = make_scene(&d);
    assert!(has_text(&scene, "loop"));
    assert!(has_text(&scene, "repeat"));
}

#[test]
fn themed_scene_uses_dark_edge_stroke() {
    let mut d = two_actor_diagram();
    d.items.push(SequenceItem::Message(
        Message::new("Alice", "Bob", ArrowStyle::SOLID_FILLED).with_label("hi"),
    ));
    let l = layout::layout(&d, &SimpleTextMeasure::default());
    let dark = Theme::dark();
    let scene = to_scene(&l, &dark);
    let has_dark_stroke = scene.elements().iter().any(|e| {
        matches!(&e.primitive, Primitive::Path { style, marker_end: Some(_), .. }
                if style.stroke == Some(dark.edge_stroke))
    });
    assert!(
        has_dark_stroke,
        "dark theme should apply edge_stroke to message paths"
    );
}

#[test]
fn title_appears_in_scene() {
    let mut d = two_actor_diagram();
    d.title = Some("My Diagram".into());
    let scene = make_scene(&d);
    assert!(has_text(&scene, "My Diagram"));
}

#[test]
fn self_message_has_path_and_label() {
    let mut d = SequenceDiagram::new();
    d.participants.push(Participant::new("A", "Alice"));
    d.items.push(SequenceItem::Message(
        Message::new("A", "A", ArrowStyle::SOLID_FILLED).with_label("think"),
    ));
    let scene = make_scene(&d);
    assert!(has_text(&scene, "think"));
    assert!(has_path(&scene));
}

#[test]
fn activation_produces_rect() {
    let mut d = two_actor_diagram();
    let mut msg = Message::new("Alice", "Bob", ArrowStyle::SOLID_FILLED);
    msg.activate = true;
    d.items.push(SequenceItem::Message(msg.with_label("req")));
    let mut reply = Message::new("Bob", "Alice", ArrowStyle::DOTTED_FILLED);
    reply.deactivate = true;
    d.items.push(SequenceItem::Message(reply.with_label("res")));
    let scene = make_scene(&d);
    // Activation produces a rect (beyond the actor box rects).
    assert!(count_rects(&scene) >= 5);
}

#[test]
fn actor_stick_figure_produces_circle_and_paths() {
    let mut d = SequenceDiagram::new();
    d.participants.push(Participant::actor("U", "User"));
    d.participants.push(Participant::new("S", "Server"));
    d.items.push(SequenceItem::Message(
        Message::new("U", "S", ArrowStyle::SOLID_FILLED).with_label("request"),
    ));
    let scene = make_scene(&d);
    assert!(has_circle(&scene));
    assert!(has_text(&scene, "User"));
    assert!(has_text(&scene, "Server"));
}

#[test]
fn autonumber_renders_circles_and_numbers() {
    let mut d = two_actor_diagram();
    d.autonumber = Some(AutoNumber { start: 1, step: 1 });
    d.items.push(SequenceItem::Message(
        Message::new("Alice", "Bob", ArrowStyle::SOLID_FILLED).with_label("first"),
    ));
    d.items.push(SequenceItem::Message(
        Message::new("Bob", "Alice", ArrowStyle::DOTTED_FILLED).with_label("second"),
    ));
    let scene = make_scene(&d);
    // Each numbered message gets a circle badge + number text.
    assert!(has_text(&scene, "1"));
    assert!(has_text(&scene, "2"));
    // At least 2 circles for numbers (plus any actor circles).
    let circles = scene
        .elements()
        .iter()
        .filter(|e| matches!(e.primitive, Primitive::Circle { .. }))
        .count();
    assert!(circles >= 2, "expected ≥2 number circles, got {circles}");
}

#[test]
fn no_autonumber_no_number_circles() {
    let mut d = two_actor_diagram();
    d.items.push(SequenceItem::Message(
        Message::new("Alice", "Bob", ArrowStyle::SOLID_FILLED).with_label("msg"),
    ));
    let scene = make_scene(&d);
    // Without autonumber, no number badge circles.
    let circles = scene
        .elements()
        .iter()
        .filter(|e| matches!(e.primitive, Primitive::Circle { .. }))
        .count();
    assert_eq!(circles, 0, "no circles expected without autonumber");
}

#[test]
fn self_message_uses_cubic_bezier() {
    let mut d = SequenceDiagram::new();
    d.participants.push(Participant::new("A", "Alice"));
    d.items.push(SequenceItem::Message(
        Message::new("A", "A", ArrowStyle::SOLID_FILLED).with_label("think"),
    ));
    let scene = make_scene(&d);
    let has_cubic = scene.elements().iter().any(|e| {
        if let Primitive::Path { segments, .. } = &e.primitive {
            segments
                .iter()
                .any(|s| matches!(s, PathSegment::CubicTo { .. }))
        } else {
            false
        }
    });
    assert!(has_cubic, "self-message should use cubic bezier");
}

#[test]
fn nested_fragment_renders_inner_on_top() {
    let mut d = two_actor_diagram();
    let inner_section = FragmentSection {
        label: None,
        items: vec![SequenceItem::Message(
            Message::new("Alice", "Bob", ArrowStyle::SOLID_FILLED).with_label("data"),
        )],
    };
    let inner = Fragment {
        kind: FragmentKind::Alt,
        label: Some("available".into()),
        sections: vec![inner_section],
    };
    let outer_section = FragmentSection {
        label: None,
        items: vec![SequenceItem::Fragment(inner)],
    };
    let outer = Fragment {
        kind: FragmentKind::Loop,
        label: Some("retry".into()),
        sections: vec![outer_section],
    };
    d.items.push(SequenceItem::Fragment(outer));
    let scene = make_scene(&d);

    // Collect fragment background rects (dashed stroke = fragment).
    let frag_rects: Vec<&BBox> = scene
        .elements()
        .iter()
        .filter_map(|e| match &e.primitive {
            Primitive::Rect {
                bbox,
                style:
                    Style {
                        stroke_dasharray: Some(_),
                        ..
                    },
                ..
            } => Some(bbox),
            _ => None,
        })
        .collect();

    assert!(frag_rects.len() >= 2, "need at least 2 fragment rects");
    let outer_bbox = frag_rects[0];
    let inner_bbox = frag_rects[1];
    // Outer must be larger and appear first (renders behind).
    assert!(
        outer_bbox.width >= inner_bbox.width && outer_bbox.height >= inner_bbox.height,
        "first fragment rect should be the outer (larger) one"
    );
}

#[test]
fn fragment_section_divider_renders() {
    let mut d = two_actor_diagram();
    d.items.push(SequenceItem::Fragment(Fragment {
        kind: FragmentKind::Alt,
        label: Some("check".into()),
        sections: vec![
            FragmentSection {
                label: None,
                items: vec![SequenceItem::Message(
                    Message::new("Alice", "Bob", ArrowStyle::SOLID_FILLED).with_label("ok"),
                )],
            },
            FragmentSection {
                label: Some("else".into()),
                items: vec![SequenceItem::Message(
                    Message::new("Alice", "Bob", ArrowStyle::SOLID_FILLED).with_label("err"),
                )],
            },
        ],
    }));
    let scene = make_scene(&d);
    // Fragment renders: kind tag text, condition text, divider label.
    assert!(has_text(&scene, "alt"));
    assert!(has_text(&scene, "[else]"));
}

#[test]
fn edge_path_shortened_for_arrow_marker() {
    use crate::common::rendering::{marker_inset_px, prev_endpoint};
    let mut d = two_actor_diagram();
    d.items.push(SequenceItem::Message(Message {
        from: "Alice".into(),
        to: "Bob".into(),
        label: Some("test".into()),
        arrow: ArrowStyle {
            head: ArrowHead::Filled,
            line: LineStyle::Solid,
        },
        activate: false,
        deactivate: false,
    }));
    let l = layout::layout(&d, &SimpleTextMeasure::default());
    let scene = to_scene(&l, &Theme::default());

    let bob_x = l
        .lifelines
        .iter()
        .find(|ll| ll.actor_id == "Bob")
        .unwrap()
        .x;

    for e in scene.elements() {
        if let Primitive::Path {
            segments,
            marker_end: Some(MarkerType::ArrowPoint),
            style,
            ..
        } = &e.primitive
        {
            let endpoint = prev_endpoint(segments).unwrap();
            let sw = style.stroke_width.unwrap_or(1.5);
            let expected = marker_inset_px(MarkerType::ArrowPoint, sw);
            let gap = bob_x - endpoint.x;
            assert!(
                gap > 0.0,
                "seq edge endpoint ({:.1}) should be left of lifeline ({:.1})",
                endpoint.x,
                bob_x
            );
            assert!(
                (gap - expected).abs() < 0.5,
                "seq edge gap ({gap:.1}) should be ~{expected:.1}px"
            );
        }
    }
}
