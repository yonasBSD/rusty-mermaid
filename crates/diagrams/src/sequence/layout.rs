use std::collections::BTreeMap;

use rusty_mermaid_core::{TextMeasure, TextStyle};

use crate::sequence::ir::*;

// Layout constants (swimlanes-inspired).
const ACTOR_MARGIN: f64 = 60.0;
const MESSAGE_MARGIN: f64 = 40.0;
const ACTOR_PADDING_X: f64 = 16.0;
const ACTOR_PADDING_Y: f64 = 10.0;
const NOTE_PADDING: f64 = 10.0;
const NOTE_MARGIN: f64 = 10.0;
const ACTIVATION_WIDTH: f64 = 10.0;
const FRAGMENT_PADDING: f64 = 12.0;
const SELF_MSG_WIDTH: f64 = 40.0;
const SELF_MSG_HEIGHT: f64 = 30.0;
const DIAGRAM_MARGIN: f64 = 20.0;
const ACTOR_BOTTOM_MARGIN: f64 = 20.0;
const MIN_ACTOR_WIDTH: f64 = 50.0;
const NOTE_MAX_WIDTH: f64 = 200.0;
const FRAGMENT_LABEL_HEIGHT: f64 = 24.0;

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

    // 1. Measure actor boxes.
    let actor_dims: Vec<(f64, f64)> = diagram
        .participants
        .iter()
        .map(|p| measure_actor(p, text, &style))
        .collect();
    let actor_height = actor_dims.iter().map(|(_, h)| *h).fold(0.0_f64, f64::max);

    // 2. Compute minimum gaps between adjacent actors.
    let mut gaps = vec![ACTOR_MARGIN; n.saturating_sub(1)];
    widen_gaps_for_labels(&diagram.items, &diagram.participants, &mut gaps, text, &style);

    // 3. Position actors L→R.
    let actor_centers = place_actors_x(&actor_dims, &gaps);

    // 4. Title.
    let title_y = DIAGRAM_MARGIN;
    let title_height = diagram
        .title
        .as_deref()
        .map(|t| text.measure(t, &style).1 + 10.0)
        .unwrap_or(0.0);
    let actor_top_y = DIAGRAM_MARGIN + title_height;

    // 5. Build actor layouts.
    let mut actors: Vec<ActorLayout> = diagram
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
        .collect();

    // 6. Walk items top-down.
    let mut pass = LayoutPass {
        actors: &actors,
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

    // Extract results from pass, releasing the borrow on actors.
    let cursor_y = pass.cursor_y + ACTOR_BOTTOM_MARGIN;
    let mut messages = pass.messages;
    let mut activations = pass.activations;
    let mut notes = pass.notes;
    let mut fragments = pass.fragments;
    let max_self_label_right = pass.max_self_label_right;

    // 7. Expand bounds to include notes and self-message labels beyond actor edges.
    let actor_right = if n > 0 {
        actors[n - 1].x + actors[n - 1].width / 2.0
    } else {
        0.0
    };
    let mut min_x = 0.0_f64;
    let mut max_right = actor_right;
    for note in &notes {
        min_x = min_x.min(note.x);
        max_right = max_right.max(note.x + note.width);
    }
    max_right = max_right.max(max_self_label_right);
    let shift = if min_x < 0.0 { -min_x } else { 0.0 };
    if shift > 0.0 {
        for a in &mut actors {
            a.x += shift;
        }
        for m in &mut messages {
            m.from_x += shift;
            m.to_x += shift;
        }
        for act in &mut activations {
            act.x += shift;
        }
        for note in &mut notes {
            note.x += shift;
        }
        for frag in &mut fragments {
            frag.x += shift;
        }
        max_right += shift;
    }

    // 8. Bottom actors (mirrored).
    let bottom_y = cursor_y;
    let bottom_actors: Vec<ActorLayout> = actors
        .iter()
        .map(|a| ActorLayout {
            y: bottom_y,
            ..a.clone()
        })
        .collect();

    // 9. Lifelines.
    let lifeline_top = actor_top_y + actor_height;
    let lifeline_bottom = bottom_y;
    let lifelines = actors
        .iter()
        .map(|a| LifelineLayout {
            actor_id: a.id.clone(),
            x: a.x,
            top_y: lifeline_top,
            bottom_y: lifeline_bottom,
        })
        .collect();

    // 10. Total dimensions.
    let total_width = if n > 0 {
        max_right + DIAGRAM_MARGIN
    } else {
        2.0 * DIAGRAM_MARGIN
    };
    let total_height = bottom_y + actor_height + DIAGRAM_MARGIN;

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

// ---------------------------------------------------------------------------
// Actor measurement & placement
// ---------------------------------------------------------------------------

fn measure_actor(p: &Participant, text: &impl TextMeasure, style: &TextStyle) -> (f64, f64) {
    let (tw, th) = text.measure(&p.label, style);
    let w = (tw + 2.0 * ACTOR_PADDING_X)
        .max(MIN_ACTOR_WIDTH)
        .max(STICK_ARM_SPAN + 2.0 * ACTOR_PADDING_X);
    match p.kind {
        ParticipantKind::Box => {
            let h = th + 2.0 * ACTOR_PADDING_Y;
            (w, h)
        }
        ParticipantKind::Actor => {
            let h = STICK_FIGURE_H + STICK_TEXT_GAP + th;
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
                    let (lw, _) = text.measure(label, style);
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

fn actor_center_x(id: &str, actors: &[ActorLayout]) -> f64 {
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
                let (lw, _) = self.text.measure(label, &self.style);
                let right = from_x + SELF_MSG_WIDTH + 6.0 + lw;
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
        let (tw, th) = self.text.measure(&note.text, &self.style);
        let note_w = (tw + 2.0 * NOTE_PADDING).min(NOTE_MAX_WIDTH);
        let note_h = th + 2.0 * NOTE_PADDING;

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
        let xs: Vec<f64> = ids.iter().map(|id| actor_center_x(id, self.actors)).collect();
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
        self.fragments.push(FragmentLayout {
            x: frag_left,
            y: frag_start_y,
            width: frag_right - frag_left,
            height: self.cursor_y - frag_start_y,
            kind: frag.kind,
            label: frag.label.clone(),
            sections: section_layouts,
        });
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
            while self.activation_stack.get(&id).is_some_and(|s| !s.is_empty()) {
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
mod tests {
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
        d.items
            .push(SequenceItem::Activation(Activation { actor: "A".into(), active: true }));
        d.items.push(SequenceItem::Message(
            Message::new("A", "A", ArrowStyle::SOLID_FILLED).with_label("work"),
        ));
        d.items
            .push(SequenceItem::Activation(Activation { actor: "A".into(), active: false }));
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
}
