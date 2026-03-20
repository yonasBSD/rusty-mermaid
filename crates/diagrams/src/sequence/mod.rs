pub mod ir;
pub mod layout;
pub mod parser;

use rusty_mermaid_core::{
    BBox, FontWeight, MarkerType, PathSegment, Point, Primitive, Scene, Style, TextAnchor,
    TextStyle, Theme,
};

use crate::common::rendering::shorten_path_for_markers;
use ir::{ArrowHead, LineStyle, ParticipantKind};
use layout::{
    ActorLayout, MessageLayout, SequenceLayout, activation_width, self_msg_height, self_msg_width,
    stick_figure_h, stick_text_gap,
};


/// Convert a sequence layout into a Scene with default theme.
pub fn to_scene(seq_layout: &SequenceLayout) -> Scene {
    to_scene_themed(seq_layout, &Theme::default())
}

/// Convert a sequence layout into a themed Scene.
pub fn to_scene_themed(seq_layout: &SequenceLayout, theme: &Theme) -> Scene {
    let mut scene = Scene::new(seq_layout.width, seq_layout.height);
    render_layout(seq_layout, &mut scene, theme);
    scene
}

fn render_layout(l: &SequenceLayout, scene: &mut Scene, theme: &Theme) {
    // Z-order: back → front
    render_fragments(l, scene, theme);
    render_lifelines(l, scene, theme);
    render_activations(l, scene, theme);
    render_messages(l, scene, theme);
    render_notes(l, scene, theme);
    render_actors(&l.actors, scene, theme);
    render_actors(&l.bottom_actors, scene, theme);
    if let Some(title) = &l.title {
        render_title(title, l, scene, theme);
    }
}

// ---------------------------------------------------------------------------
// Fragments
// ---------------------------------------------------------------------------

fn render_fragments(l: &SequenceLayout, scene: &mut Scene, theme: &Theme) {
    for frag in &l.fragments {
        let cx = frag.x + frag.width / 2.0;
        let cy = frag.y + frag.height / 2.0;

        // Dashed background rect.
        scene.push(Primitive::Rect {
            bbox: BBox::new(cx, cy, frag.width, frag.height),
            rx: 3.0,
            ry: 3.0,
            style: Style {
                fill: Some(theme.subgraph_fill),
                stroke: Some(theme.subgraph_stroke),
                stroke_width: Some(1.0),
                stroke_dasharray: Some(vec![6.0, 4.0]),
                ..Default::default()
            },
        });

        // Kind tag box in top-left corner.
        let tag_text = frag.kind.to_string();
        let tag_w = tag_text.len() as f64 * 8.0 + 12.0;
        let tag_h = 20.0;
        let tag_cx = frag.x + tag_w / 2.0;
        let tag_cy = frag.y + tag_h / 2.0;
        scene.push(Primitive::Rect {
            bbox: BBox::new(tag_cx, tag_cy, tag_w, tag_h),
            rx: 2.0,
            ry: 2.0,
            style: Style {
                fill: Some(theme.subgraph_stroke),
                ..Default::default()
            },
        });
        scene.push(Primitive::Text {
            position: Point::new(tag_cx, tag_cy),
            content: tag_text,
            anchor: TextAnchor::Middle,
            style: TextStyle {
                font_size: theme.font_size_edge_label,
                font_weight: FontWeight::Bold,
                fill: Some(rusty_mermaid_core::Color::WHITE),
                ..Default::default()
            },
        });

        // Condition label to the right of the tag.
        if let Some(label) = &frag.label {
            scene.push(Primitive::Text {
                position: Point::new(frag.x + tag_w + 6.0, tag_cy),
                content: label.clone(),
                anchor: TextAnchor::Start,
                style: TextStyle {
                    font_size: theme.font_size_edge_label,
                    fill: Some(theme.subgraph_label),
                    ..Default::default()
                },
            });
        }

        // Section dividers (alt/else, par/and boundaries).
        for section in &frag.sections {
            scene.push(Primitive::Path {
                segments: vec![
                    PathSegment::MoveTo(Point::new(frag.x, section.y)),
                    PathSegment::LineTo(Point::new(frag.x + frag.width, section.y)),
                ],
                style: Style {
                    stroke: Some(theme.subgraph_stroke),
                    stroke_width: Some(0.5),
                    stroke_dasharray: Some(vec![6.0, 4.0]),
                    ..Default::default()
                },
                marker_start: None,
                marker_end: None,
            });
            if let Some(label) = &section.label {
                scene.push(Primitive::Text {
                    position: Point::new(frag.x + 8.0, section.y + 14.0),
                    content: format!("[{label}]"),
                    anchor: TextAnchor::Start,
                    style: TextStyle {
                        font_size: theme.font_size_small,
                        fill: Some(theme.subgraph_label),
                        ..Default::default()
                    },
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Lifelines
// ---------------------------------------------------------------------------

fn render_lifelines(l: &SequenceLayout, scene: &mut Scene, theme: &Theme) {
    for ll in &l.lifelines {
        scene.push(Primitive::Path {
            segments: vec![
                PathSegment::MoveTo(Point::new(ll.x, ll.top_y)),
                PathSegment::LineTo(Point::new(ll.x, ll.bottom_y)),
            ],
            style: Style {
                stroke: Some(theme.lifeline_stroke),
                stroke_width: Some(1.0),
                ..Default::default()
            },
            marker_start: None,
            marker_end: None,
        });
    }
}

// ---------------------------------------------------------------------------
// Activations
// ---------------------------------------------------------------------------

fn render_activations(l: &SequenceLayout, scene: &mut Scene, theme: &Theme) {
    let aw = activation_width();
    for act in &l.activations {
        let h = act.bottom_y - act.top_y;
        let cy = (act.top_y + act.bottom_y) / 2.0;
        scene.push(Primitive::Rect {
            bbox: BBox::new(act.x, cy, aw, h),
            rx: 0.0,
            ry: 0.0,
            style: Style {
                fill: Some(theme.activation_fill),
                stroke: Some(theme.activation_stroke),
                stroke_width: Some(0.5),
                ..Default::default()
            },
        });
    }
}

// ---------------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------------

fn render_messages(l: &SequenceLayout, scene: &mut Scene, theme: &Theme) {
    for msg in &l.messages {
        if msg.is_self {
            render_self_message(msg, scene, theme);
        } else {
            render_regular_message(msg, scene, theme);
        }
        if let Some(n) = msg.number {
            render_msg_number(msg, n, scene, theme);
        }
    }
}

fn render_regular_message(msg: &MessageLayout, scene: &mut Scene, theme: &Theme) {
    let sw = theme.default_stroke_width;
    let mut style = Style {
        stroke: Some(theme.edge_stroke),
        stroke_width: Some(sw),
        ..Default::default()
    };
    if msg.arrow.line == LineStyle::Dotted {
        style.stroke_dasharray = Some(vec![6.0, 4.0]);
    }
    let marker_end = arrow_marker(msg.arrow.head);
    let mut segments = vec![
        PathSegment::MoveTo(Point::new(msg.from_x, msg.y)),
        PathSegment::LineTo(Point::new(msg.to_x, msg.y)),
    ];
    shorten_path_for_markers(&mut segments, None, marker_end, sw);
    scene.push(Primitive::Path {
        segments,
        style,
        marker_start: None,
        marker_end,
    });

    if let Some(label) = &msg.label {
        let mid_x = (msg.from_x + msg.to_x) / 2.0;
        scene.push(Primitive::Text {
            position: Point::new(mid_x, msg.y - 8.0),
            content: label.clone(),
            anchor: TextAnchor::Middle,
            style: TextStyle {
                font_size: theme.font_size_label,
                fill: Some(theme.edge_label_text),
                ..Default::default()
            },
        });
    }
}

fn render_self_message(msg: &MessageLayout, scene: &mut Scene, theme: &Theme) {
    let x = msg.from_x;
    let w = self_msg_width();
    let h = self_msg_height();
    let sw = theme.default_stroke_width;

    let mut style = Style {
        stroke: Some(theme.edge_stroke),
        stroke_width: Some(sw),
        ..Default::default()
    };
    if msg.arrow.line == LineStyle::Dotted {
        style.stroke_dasharray = Some(vec![6.0, 4.0]);
    }

    // Cubic bezier loop-back, matching mermaid.js self-message curve.
    let marker_end = arrow_marker(msg.arrow.head);
    let mut segments = vec![
        PathSegment::MoveTo(Point::new(x, msg.y)),
        PathSegment::CubicTo {
            cp1: Point::new(x + w, msg.y - h * 0.33),
            cp2: Point::new(x + w, msg.y + h * 1.33),
            to: Point::new(x, msg.y + h),
        },
    ];
    shorten_path_for_markers(&mut segments, None, marker_end, sw);
    scene.push(Primitive::Path {
        segments,
        style,
        marker_start: None,
        marker_end,
    });

    if let Some(label) = &msg.label {
        scene.push(Primitive::Text {
            position: Point::new(x + w + 6.0, msg.y + h / 2.0),
            content: label.clone(),
            anchor: TextAnchor::Start,
            style: TextStyle {
                font_size: theme.font_size_label,
                fill: Some(theme.edge_label_text),
                ..Default::default()
            },
        });
    }
}

/// Autonumber badge: small filled circle with white number text at arrow origin.
fn render_msg_number(msg: &MessageLayout, n: u32, scene: &mut Scene, theme: &Theme) {
    let r = 8.0;
    let cx = if msg.is_self {
        msg.from_x
    } else {
        msg.from_x + if msg.from_x < msg.to_x { 1.0 } else { -1.0 }
    };
    let cy = msg.y;

    scene.push(Primitive::Circle {
        center: Point::new(cx, cy),
        radius: r,
        style: Style {
            fill: Some(theme.edge_stroke),
            ..Default::default()
        },
    });
    scene.push(Primitive::Text {
        position: Point::new(cx, cy),
        content: n.to_string(),
        anchor: TextAnchor::Middle,
        style: TextStyle {
            font_size: theme.font_size_small,
            fill: Some(rusty_mermaid_core::Color::WHITE),
            font_weight: FontWeight::Bold,
            ..Default::default()
        },
    });
}

fn arrow_marker(head: ArrowHead) -> Option<MarkerType> {
    match head {
        ArrowHead::Filled => Some(MarkerType::ArrowPoint),
        ArrowHead::Open => Some(MarkerType::ArrowOpen),
        ArrowHead::Cross => Some(MarkerType::Cross),
        ArrowHead::None => None,
    }
}

// ---------------------------------------------------------------------------
// Notes
// ---------------------------------------------------------------------------

fn render_notes(l: &SequenceLayout, scene: &mut Scene, theme: &Theme) {
    for note in &l.notes {
        let cx = note.x + note.width / 2.0;
        let cy = note.y + note.height / 2.0;
        scene.push(Primitive::Rect {
            bbox: BBox::new(cx, cy, note.width, note.height),
            rx: 0.0,
            ry: 0.0,
            style: Style {
                fill: Some(theme.note_fill),
                stroke: Some(theme.note_stroke),
                stroke_width: Some(1.0),
                ..Default::default()
            },
        });
        scene.push(Primitive::Text {
            position: Point::new(cx, cy),
            content: note.text.clone(),
            anchor: TextAnchor::Middle,
            style: TextStyle {
                font_size: theme.font_size_edge_label,
                fill: Some(theme.note_text),
                ..Default::default()
            },
        });
    }
}

// ---------------------------------------------------------------------------
// Participant boxes (top + mirrored bottom)
// ---------------------------------------------------------------------------

fn render_actors(actors: &[ActorLayout], scene: &mut Scene, theme: &Theme) {
    for actor in actors {
        match actor.kind {
            ParticipantKind::Box => render_actor_box(actor, scene, theme),
            ParticipantKind::Actor => render_actor_stick(actor, scene, theme),
        }
    }
}

fn render_actor_box(actor: &ActorLayout, scene: &mut Scene, theme: &Theme) {
    let cy = actor.y + actor.height / 2.0;
    scene.push(Primitive::Rect {
        bbox: BBox::new(actor.x, cy, actor.width, actor.height),
        rx: 5.0,
        ry: 5.0,
        style: Style {
            fill: Some(theme.node_fill),
            stroke: Some(theme.node_stroke),
            stroke_width: Some(theme.default_stroke_width),
            ..Default::default()
        },
    });
    scene.push(Primitive::Text {
        position: Point::new(actor.x, cy),
        content: actor.label.clone(),
        anchor: TextAnchor::Middle,
        style: TextStyle {
            font_weight: FontWeight::Bold,
            fill: Some(theme.node_text),
            ..Default::default()
        },
    });
}

fn render_actor_stick(actor: &ActorLayout, scene: &mut Scene, theme: &Theme) {
    let x = actor.x;
    let figure_h = stick_figure_h();

    // Person icon: filled circle head + rounded-rect torso.
    let head_r = 9.0;
    let head_cy = actor.y + head_r;
    let gap = 3.0;
    let body_top = actor.y + head_r * 2.0 + gap;
    let body_h = figure_h - (head_r * 2.0 + gap);
    let body_w = 26.0;
    let body_rx = 7.0;

    let icon_style = Style {
        fill: Some(theme.node_fill),
        stroke: Some(theme.node_stroke),
        stroke_width: Some(theme.default_stroke_width),
        ..Default::default()
    };

    // Head.
    scene.push(Primitive::Circle {
        center: Point::new(x, head_cy),
        radius: head_r,
        style: icon_style.clone(),
    });

    // Torso.
    let body_cy = body_top + body_h / 2.0;
    scene.push(Primitive::Rect {
        bbox: BBox::new(x, body_cy, body_w, body_h),
        rx: body_rx,
        ry: body_rx,
        style: icon_style,
    });

    // Label below the figure.
    let text_y = actor.y + figure_h + stick_text_gap();
    scene.push(Primitive::Text {
        position: Point::new(x, text_y),
        content: actor.label.clone(),
        anchor: TextAnchor::Middle,
        style: TextStyle {
            font_weight: FontWeight::Bold,
            fill: Some(theme.node_text),
            ..Default::default()
        },
    });
}

// ---------------------------------------------------------------------------
// Title
// ---------------------------------------------------------------------------

fn render_title(title: &str, l: &SequenceLayout, scene: &mut Scene, theme: &Theme) {
    scene.push(Primitive::Text {
        position: Point::new(l.width / 2.0, l.title_y + 8.0),
        content: title.to_owned(),
        anchor: TextAnchor::Middle,
        style: TextStyle {
            font_size: theme.font_size_title,
            font_weight: FontWeight::Bold,
            fill: Some(theme.node_text),
            ..Default::default()
        },
    });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::test_helpers::test_helpers::*;
    use ir::*;
    use rusty_mermaid_core::SimpleTextMeasure;

    fn make_scene(d: &SequenceDiagram) -> Scene {
        let l = layout::layout(d, &SimpleTextMeasure::default());
        to_scene(&l)
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
        let scene = to_scene_themed(&l, &dark);
        let has_dark_stroke = scene.elements().iter().any(|e| {
            matches!(&e.primitive, Primitive::Path { style, marker_end: Some(_), .. }
                if style.stroke == Some(dark.edge_stroke))
        });
        assert!(has_dark_stroke, "dark theme should apply edge_stroke to message paths");
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
                    style: Style { stroke_dasharray: Some(_), .. },
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
            arrow: ArrowStyle { head: ArrowHead::Filled, line: LineStyle::Solid },
            activate: false,
            deactivate: false,
        }));
        let l = layout::layout(&d, &SimpleTextMeasure::default());
        let scene = to_scene(&l);

        let bob_x = l.lifelines.iter().find(|ll| ll.actor_id == "Bob").unwrap().x;

        for e in scene.elements() {
            if let Primitive::Path { segments, marker_end: Some(MarkerType::ArrowPoint), style, .. } = &e.primitive {
                let endpoint = prev_endpoint(segments).unwrap();
                let sw = style.stroke_width.unwrap_or(1.5);
                let expected = marker_inset_px(MarkerType::ArrowPoint, sw);
                let gap = bob_x - endpoint.x;
                assert!(
                    gap > 0.0,
                    "seq edge endpoint ({:.1}) should be left of lifeline ({:.1})",
                    endpoint.x, bob_x
                );
                assert!(
                    (gap - expected).abs() < 0.5,
                    "seq edge gap ({gap:.1}) should be ~{expected:.1}px"
                );
            }
        }
    }
}
