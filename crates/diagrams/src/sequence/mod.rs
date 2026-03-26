pub mod ir;
pub mod layout;
mod layout_pass;
pub mod parser;

use rusty_mermaid_core::{
    BBox, FontWeight, MarkerType, PathSegment, Point, Primitive, Scene, Style, TextAnchor,
    TextStyle, Theme,
};

use crate::common::palette::DOTTED_PATTERN;
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
                stroke_dasharray: Some(DOTTED_PATTERN.to_vec()),
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
                    stroke_dasharray: Some(DOTTED_PATTERN.to_vec()),
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
        style.stroke_dasharray = Some(DOTTED_PATTERN.to_vec());
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
            position: Point::new(mid_x, msg.y - 12.0),
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
        style.stroke_dasharray = Some(DOTTED_PATTERN.to_vec());
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
            position: Point::new(x + w + 1.0, msg.y + h / 2.0),
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
mod render_tests;
