use std::collections::BTreeMap;

use rusty_mermaid_core::{TextMeasure, TextStyle};

use crate::sequence::ir::*;

// Layout constants (swimlanes-inspired).
pub(super) const ACTOR_MARGIN: f64 = 60.0;
pub(super) const MESSAGE_MARGIN: f64 = 40.0;
pub(super) const ACTOR_PADDING_X: f64 = 16.0;
pub(super) const ACTOR_PADDING_Y: f64 = 10.0;
pub(super) const NOTE_PADDING: f64 = 10.0;
pub(super) const NOTE_MARGIN: f64 = 10.0;
pub(super) const ACTIVATION_WIDTH: f64 = 10.0;
pub(super) const FRAGMENT_PADDING: f64 = 12.0;
pub(super) const SELF_MSG_WIDTH: f64 = 40.0;
pub(super) const SELF_MSG_HEIGHT: f64 = 30.0;
pub(super) const DIAGRAM_MARGIN: f64 = 20.0;
const ACTOR_BOTTOM_MARGIN: f64 = 20.0;
const MIN_ACTOR_WIDTH: f64 = 50.0;
pub(super) const FRAGMENT_LABEL_HEIGHT: f64 = 24.0;

// Stick figure dimensions for ParticipantKind::Actor.
const STICK_HEAD_R: f64 = 8.0;
const STICK_BODY_H: f64 = 16.0;
const STICK_LEG_H: f64 = 12.0;
const STICK_ARM_SPAN: f64 = 24.0;
const STICK_FIGURE_H: f64 = STICK_HEAD_R * 2.0 + STICK_BODY_H + STICK_LEG_H;
const STICK_TEXT_GAP: f64 = 10.0;

/// Positioned actor box.
#[derive(Debug, Clone)]
pub struct ActorLayout {
    pub id: String,
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub kind: ParticipantKind,
}

/// Vertical lifeline for an actor.
#[derive(Debug, Clone)]
pub struct LifelineLayout {
    pub actor_id: String,
    pub x: f64,
    pub top_y: f64,
    pub bottom_y: f64,
}

/// A positioned message arrow.
#[derive(Debug, Clone)]
pub struct MessageLayout {
    pub from_x: f64,
    pub to_x: f64,
    pub y: f64,
    pub label: Option<String>,
    pub arrow: ArrowStyle,
    pub is_self: bool,
    pub number: Option<u32>,
}

/// An activation box on a lifeline.
#[derive(Debug, Clone)]
pub struct ActivationLayout {
    pub actor_id: String,
    pub x: f64,
    pub top_y: f64,
    pub bottom_y: f64,
}

/// A positioned note.
#[derive(Debug, Clone)]
pub struct NoteLayout {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub text: String,
}

/// A positioned fragment (loop, alt, etc.).
#[derive(Debug, Clone)]
pub struct FragmentLayout {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub kind: FragmentKind,
    pub label: Option<String>,
    pub sections: Vec<FragmentSectionLayout>,
}

/// A section divider within a fragment.
#[derive(Debug, Clone)]
pub struct FragmentSectionLayout {
    pub y: f64,
    pub label: Option<String>,
}

/// Complete positioned layout for a sequence diagram.
#[derive(Debug, Clone)]
pub struct SequenceLayout {
    pub width: f64,
    pub height: f64,
    pub title: Option<String>,
    pub title_y: f64,
    pub actors: Vec<ActorLayout>,
    pub bottom_actors: Vec<ActorLayout>,
    pub lifelines: Vec<LifelineLayout>,
    pub messages: Vec<MessageLayout>,
    pub activations: Vec<ActivationLayout>,
    pub notes: Vec<NoteLayout>,
    pub fragments: Vec<FragmentLayout>,
}

/// Mutable state accumulated during the top-down layout pass.
struct LayoutPass<'a, T: TextMeasure> {
    actors: &'a [ActorLayout],
    text: &'a T,
    style: TextStyle,
    cursor_y: f64,
    messages: Vec<MessageLayout>,
    notes: Vec<NoteLayout>,
    fragments: Vec<FragmentLayout>,
    activation_stack: BTreeMap<String, Vec<f64>>,
    activations: Vec<ActivationLayout>,
    /// Tracks the rightmost edge of any self-message label for bounds expansion.
    max_self_label_right: f64,
    /// Autonumber counter: (current_value, step). None if autonumber is off.
    msg_counter: Option<(u32, u32)>,
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub fn layout(diagram: &SequenceDiagram, text: &impl TextMeasure) -> SequenceLayout {
    let style = TextStyle::default();
    let n = diagram.participants.len();

    let (actor_dims, actor_height) = measure_actors(diagram, text, &style);
    let actor_centers = compute_actor_positions(diagram, &actor_dims, text, &style);

    let title_y = DIAGRAM_MARGIN;
    let title_height = diagram
        .title
        .as_deref()
        .map(|t| text.measure(t, &style).height + 10.0)
        .unwrap_or(0.0);
    let actor_top_y = DIAGRAM_MARGIN + title_height;

    let mut actors = build_actor_layouts(
        diagram,
        &actor_centers,
        &actor_dims,
        actor_height,
        actor_top_y,
    );

    let (cursor_y, mut messages, mut activations, mut notes, mut fragments, max_self_label_right) =
        run_layout_pass(
            &actors,
            diagram,
            text,
            style.clone(),
            actor_top_y,
            actor_height,
        );

    let max_right = apply_bounds_shift(
        n,
        &mut actors,
        &mut messages,
        &mut activations,
        &mut notes,
        &mut fragments,
        max_self_label_right,
    );

    let (bottom_actors, lifelines) =
        build_bottom_actors_and_lifelines(&actors, cursor_y, actor_top_y, actor_height);

    let (total_width, total_height) = compute_dimensions(n, max_right, cursor_y, actor_height);

    SequenceLayout {
        width: total_width,
        height: total_height,
        title: diagram.title.clone(),
        title_y,
        actors,
        bottom_actors,
        lifelines,
        messages,
        activations,
        notes,
        fragments,
    }
}

fn measure_actors(
    diagram: &SequenceDiagram,
    text: &impl TextMeasure,
    style: &TextStyle,
) -> (Vec<(f64, f64)>, f64) {
    let actor_dims: Vec<(f64, f64)> = diagram
        .participants
        .iter()
        .map(|p| measure_actor(p, text, style))
        .collect();
    let actor_height = actor_dims.iter().map(|(_, h)| *h).fold(0.0_f64, f64::max);
    (actor_dims, actor_height)
}

fn compute_actor_positions(
    diagram: &SequenceDiagram,
    actor_dims: &[(f64, f64)],
    text: &impl TextMeasure,
    style: &TextStyle,
) -> Vec<f64> {
    let n = diagram.participants.len();
    let mut gaps = vec![ACTOR_MARGIN; n.saturating_sub(1)];
    widen_gaps_for_labels(
        &diagram.items,
        &diagram.participants,
        &mut gaps,
        text,
        style,
    );
    place_actors_x(actor_dims, &gaps)
}

fn build_actor_layouts(
    diagram: &SequenceDiagram,
    actor_centers: &[f64],
    actor_dims: &[(f64, f64)],
    actor_height: f64,
    actor_top_y: f64,
) -> Vec<ActorLayout> {
    diagram
        .participants
        .iter()
        .enumerate()
        .map(|(i, p)| ActorLayout {
            id: p.id.clone(),
            label: p.label.clone(),
            x: actor_centers[i],
            y: actor_top_y,
            width: actor_dims[i].0,
            height: actor_height,
            kind: p.kind,
        })
        .collect()
}

fn run_layout_pass(
    actors: &[ActorLayout],
    diagram: &SequenceDiagram,
    text: &impl TextMeasure,
    style: TextStyle,
    actor_top_y: f64,
    actor_height: f64,
) -> (
    f64,
    Vec<MessageLayout>,
    Vec<ActivationLayout>,
    Vec<NoteLayout>,
    Vec<FragmentLayout>,
    f64,
) {
    let mut pass = LayoutPass {
        actors,
        text,
        style,
        cursor_y: actor_top_y + actor_height,
        messages: Vec::new(),
        notes: Vec::new(),
        fragments: Vec::new(),
        activation_stack: BTreeMap::new(),
        activations: Vec::new(),
        max_self_label_right: 0.0,
        msg_counter: diagram.autonumber.map(|a| (a.start, a.step)),
    };
    pass.layout_items(&diagram.items);
    pass.close_remaining_activations();

    let cursor_y = pass.cursor_y + ACTOR_BOTTOM_MARGIN;
    (
        cursor_y,
        pass.messages,
        pass.activations,
        pass.notes,
        pass.fragments,
        pass.max_self_label_right,
    )
}

fn apply_bounds_shift(
    n: usize,
    actors: &mut [ActorLayout],
    messages: &mut [MessageLayout],
    activations: &mut [ActivationLayout],
    notes: &mut [NoteLayout],
    fragments: &mut [FragmentLayout],
    max_self_label_right: f64,
) -> f64 {
    let actor_right = if n > 0 {
        actors[n - 1].x + actors[n - 1].width / 2.0
    } else {
        0.0
    };
    let mut min_x = 0.0_f64;
    let mut max_right = actor_right;
    for note in notes.iter() {
        min_x = min_x.min(note.x);
        max_right = max_right.max(note.x + note.width);
    }
    max_right = max_right.max(max_self_label_right);
    let shift = if min_x < 0.0 { -min_x } else { 0.0 };
    if shift > 0.0 {
        for a in actors.iter_mut() {
            a.x += shift;
        }
        for m in messages.iter_mut() {
            m.from_x += shift;
            m.to_x += shift;
        }
        for act in activations.iter_mut() {
            act.x += shift;
        }
        for note in notes.iter_mut() {
            note.x += shift;
        }
        for frag in fragments.iter_mut() {
            frag.x += shift;
        }
        max_right += shift;
    }
    max_right
}

fn build_bottom_actors_and_lifelines(
    actors: &[ActorLayout],
    bottom_y: f64,
    actor_top_y: f64,
    actor_height: f64,
) -> (Vec<ActorLayout>, Vec<LifelineLayout>) {
    let bottom_actors: Vec<ActorLayout> = actors
        .iter()
        .map(|a| ActorLayout {
            y: bottom_y,
            ..a.clone()
        })
        .collect();

    let lifeline_top = actor_top_y + actor_height;
    let lifelines = actors
        .iter()
        .map(|a| LifelineLayout {
            actor_id: a.id.clone(),
            x: a.x,
            top_y: lifeline_top,
            bottom_y,
        })
        .collect();

    (bottom_actors, lifelines)
}

fn compute_dimensions(n: usize, max_right: f64, bottom_y: f64, actor_height: f64) -> (f64, f64) {
    let total_width = if n > 0 {
        max_right + DIAGRAM_MARGIN
    } else {
        2.0 * DIAGRAM_MARGIN
    };
    let total_height = bottom_y + actor_height + DIAGRAM_MARGIN;
    (total_width, total_height)
}

// ---------------------------------------------------------------------------
// Actor measurement & placement
// ---------------------------------------------------------------------------

fn measure_actor(p: &Participant, text: &impl TextMeasure, style: &TextStyle) -> (f64, f64) {
    let ts = text.measure(&p.label, style);
    let w = (ts.width + 2.0 * ACTOR_PADDING_X)
        .max(MIN_ACTOR_WIDTH)
        .max(STICK_ARM_SPAN + 2.0 * ACTOR_PADDING_X);
    match p.kind {
        ParticipantKind::Box => {
            let h = ts.height + 2.0 * ACTOR_PADDING_Y;
            (w, h)
        }
        ParticipantKind::Actor => {
            let h = STICK_FIGURE_H + STICK_TEXT_GAP + ts.height;
            (w, h)
        }
    }
}

fn place_actors_x(dims: &[(f64, f64)], gaps: &[f64]) -> Vec<f64> {
    let n = dims.len();
    if n == 0 {
        return Vec::new();
    }
    let mut centers = vec![0.0; n];
    centers[0] = DIAGRAM_MARGIN + dims[0].0 / 2.0;
    for i in 1..n {
        centers[i] = centers[i - 1] + dims[i - 1].0 / 2.0 + gaps[i - 1] + dims[i].0 / 2.0;
    }
    centers
}

/// Widen gaps between adjacent actors so that message labels fit.
fn widen_gaps_for_labels(
    items: &[SequenceItem],
    participants: &[Participant],
    gaps: &mut [f64],
    text: &impl TextMeasure,
    style: &TextStyle,
) {
    for item in items {
        match item {
            SequenceItem::Message(msg) if msg.from != msg.to => {
                let Some(fi) = actor_idx(&msg.from, participants) else {
                    continue;
                };
                let Some(ti) = actor_idx(&msg.to, participants) else {
                    continue;
                };
                let (lo, hi) = if fi < ti { (fi, ti) } else { (ti, fi) };
                if let Some(label) = &msg.label {
                    let lw = text.measure(label, style).width;
                    let needed = lw + 20.0;
                    // Distribute needed width across spanned gaps.
                    let span = hi - lo;
                    let per_gap = needed / span as f64;
                    for g in &mut gaps[lo..hi] {
                        *g = g.max(per_gap);
                    }
                }
            }
            SequenceItem::Fragment(frag) => {
                for section in &frag.sections {
                    widen_gaps_for_labels(&section.items, participants, gaps, text, style);
                }
            }
            _ => {}
        }
    }
}

fn actor_idx(id: &str, participants: &[Participant]) -> Option<usize> {
    participants.iter().position(|p| p.id == id)
}

pub(super) fn actor_center_x(id: &str, actors: &[ActorLayout]) -> f64 {
    actors
        .iter()
        .find(|a| a.id == id)
        .map(|a| a.x)
        .unwrap_or(0.0)
}

// ---------------------------------------------------------------------------
// Top-down item layout
// ---------------------------------------------------------------------------

impl<'a, T: TextMeasure> LayoutPass<'a, T> {
    fn layout_items(&mut self, items: &[SequenceItem]) {
        for item in items {
            match item {
                SequenceItem::Message(msg) => self.layout_message(msg),
                SequenceItem::Note(note) => self.layout_note(note),
                SequenceItem::Activation(act) => self.layout_activation(act),
                SequenceItem::Fragment(frag) => self.layout_fragment(frag),
            }
        }
    }

    fn layout_message(&mut self, msg: &Message) {
        self.cursor_y += MESSAGE_MARGIN;
        let mut from_x = actor_center_x(&msg.from, self.actors);
        let mut to_x = actor_center_x(&msg.to, self.actors);
        let is_self = msg.from == msg.to;

        // Adjust endpoints to activation box edges when activations are active.
        let half_aw = ACTIVATION_WIDTH / 2.0;
        if is_self {
            let active = self
                .activation_stack
                .get(&msg.from)
                .is_some_and(|s| !s.is_empty());
            if active {
                from_x += half_aw;
                to_x += half_aw;
            }
        } else {
            let going_right = from_x < to_x;

            let from_active = self
                .activation_stack
                .get(&msg.from)
                .is_some_and(|s| !s.is_empty());
            if from_active {
                from_x += if going_right { half_aw } else { -half_aw };
            }

            let to_active = self
                .activation_stack
                .get(&msg.to)
                .is_some_and(|s| !s.is_empty())
                || msg.activate;
            if to_active {
                to_x += if going_right { -half_aw } else { half_aw };
            }
        }

        let number = self.msg_counter.map(|(val, step)| {
            self.msg_counter = Some((val + step, step));
            val
        });

        self.messages.push(MessageLayout {
            from_x,
            to_x,
            y: self.cursor_y,
            label: msg.label.clone(),
            arrow: msg.arrow,
            is_self,
            number,
        });

        if is_self {
            self.cursor_y += SELF_MSG_HEIGHT;
            if let Some(label) = &msg.label {
                let lw = self.text.measure(label, &self.style).width;
                let right = from_x + SELF_MSG_WIDTH + 1.0 + lw;
                self.max_self_label_right = self.max_self_label_right.max(right);
            }
        }

        if msg.activate {
            self.activation_stack
                .entry(msg.to.clone())
                .or_default()
                .push(self.cursor_y);
        }
        if msg.deactivate {
            self.close_activation(&msg.from);
        }
    }

    fn layout_note(&mut self, note: &Note) {
        self.cursor_y += MESSAGE_MARGIN;
        let ts = self.text.measure(&note.text, &self.style);
        let note_w = ts.width + 2.0 * NOTE_PADDING;
        let note_h = ts.height + 2.0 * NOTE_PADDING;

        let note_x = match &note.position {
            NotePosition::LeftOf(id) => {
                let ax = actor_center_x(id, self.actors);
                ax - note_w - NOTE_MARGIN
            }
            NotePosition::RightOf(id) => {
                let ax = actor_center_x(id, self.actors);
                ax + NOTE_MARGIN
            }
            NotePosition::Over(ids) => self.note_over_x(ids, note_w),
        };

        self.notes.push(NoteLayout {
            x: note_x,
            y: self.cursor_y,
            width: note_w,
            height: note_h,
            text: note.text.clone(),
        });
        self.cursor_y += note_h;
    }

    fn note_over_x(&self, ids: &[String], note_w: f64) -> f64 {
        if ids.len() == 1 {
            let ax = actor_center_x(&ids[0], self.actors);
            return ax - note_w / 2.0;
        }
        let xs: Vec<f64> = ids
            .iter()
            .map(|id| actor_center_x(id, self.actors))
            .collect();
        let min_x = xs.iter().copied().fold(f64::INFINITY, f64::min);
        let max_x = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let span_center = (min_x + max_x) / 2.0;
        span_center - note_w / 2.0
    }

    fn layout_activation(&mut self, act: &Activation) {
        if act.active {
            self.activation_stack
                .entry(act.actor.clone())
                .or_default()
                .push(self.cursor_y);
        } else {
            self.close_activation(&act.actor);
        }
    }

    fn layout_fragment(&mut self, frag: &Fragment) {
        self.cursor_y += MESSAGE_MARGIN / 2.0;
        let frag_start_y = self.cursor_y;
        self.cursor_y += FRAGMENT_LABEL_HEIGHT;

        // Reserve slot so outer fragment renders before (behind) nested children.
        let slot = self.fragments.len();

        let mut section_layouts = Vec::new();
        for (i, section) in frag.sections.iter().enumerate() {
            if i > 0 {
                section_layouts.push(FragmentSectionLayout {
                    y: self.cursor_y,
                    label: section.label.clone(),
                });
                self.cursor_y += FRAGMENT_LABEL_HEIGHT / 2.0;
            }
            self.layout_items(&section.items);
        }
        self.cursor_y += FRAGMENT_PADDING;

        let (frag_left, frag_right) = self.fragment_bounds();
        self.fragments.insert(
            slot,
            FragmentLayout {
                x: frag_left,
                y: frag_start_y,
                width: frag_right - frag_left,
                height: self.cursor_y - frag_start_y,
                kind: frag.kind,
                label: frag.label.clone(),
                sections: section_layouts,
            },
        );
    }

    fn fragment_bounds(&self) -> (f64, f64) {
        let left = self
            .actors
            .first()
            .map(|a| a.x - a.width / 2.0 - FRAGMENT_PADDING)
            .unwrap_or(DIAGRAM_MARGIN);
        let right = self
            .actors
            .last()
            .map(|a| a.x + a.width / 2.0 + FRAGMENT_PADDING)
            .unwrap_or(DIAGRAM_MARGIN);
        (left, right)
    }

    fn close_activation(&mut self, actor_id: &str) {
        let Some(stack) = self.activation_stack.get_mut(actor_id) else {
            return;
        };
        let Some(start_y) = stack.pop() else {
            return;
        };
        let x = actor_center_x(actor_id, self.actors);
        self.activations.push(ActivationLayout {
            actor_id: actor_id.to_owned(),
            x,
            top_y: start_y,
            bottom_y: self.cursor_y,
        });
    }

    fn close_remaining_activations(&mut self) {
        let ids: Vec<String> = self.activation_stack.keys().cloned().collect();
        for id in ids {
            while self
                .activation_stack
                .get(&id)
                .is_some_and(|s| !s.is_empty())
            {
                self.close_activation(&id);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Public constants (for Scene rendering in mod.rs)
// ---------------------------------------------------------------------------

pub const fn activation_width() -> f64 {
    ACTIVATION_WIDTH
}
pub const fn diagram_margin() -> f64 {
    DIAGRAM_MARGIN
}
pub const fn note_padding() -> f64 {
    NOTE_PADDING
}
pub const fn fragment_padding() -> f64 {
    FRAGMENT_PADDING
}
pub const fn self_msg_width() -> f64 {
    SELF_MSG_WIDTH
}
pub const fn self_msg_height() -> f64 {
    SELF_MSG_HEIGHT
}
pub const fn stick_head_r() -> f64 {
    STICK_HEAD_R
}
pub const fn stick_body_h() -> f64 {
    STICK_BODY_H
}
pub const fn stick_leg_h() -> f64 {
    STICK_LEG_H
}
pub const fn stick_arm_span() -> f64 {
    STICK_ARM_SPAN
}
pub const fn stick_figure_h() -> f64 {
    STICK_FIGURE_H
}
pub const fn stick_text_gap() -> f64 {
    STICK_TEXT_GAP
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "layout_tests.rs"]
mod layout_tests;
