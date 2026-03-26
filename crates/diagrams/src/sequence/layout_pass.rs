use std::collections::BTreeMap;

use rusty_mermaid_core::{TextMeasure, TextStyle};

use crate::sequence::ir::*;

use super::layout::{
    actor_center_x, ActorLayout, ActivationLayout, FragmentLayout, FragmentSectionLayout,
    MessageLayout, NoteLayout, ACTIVATION_WIDTH, DIAGRAM_MARGIN, FRAGMENT_LABEL_HEIGHT,
    FRAGMENT_PADDING, MESSAGE_MARGIN, NOTE_MARGIN, NOTE_MAX_WIDTH, NOTE_PADDING, SELF_MSG_HEIGHT,
    SELF_MSG_WIDTH,
};

/// Mutable state accumulated during the top-down layout pass.
pub(super) struct LayoutPass<'a, T: TextMeasure> {
    pub(super) actors: &'a [ActorLayout],
    pub(super) text: &'a T,
    pub(super) style: TextStyle,
    pub(super) cursor_y: f64,
    pub(super) messages: Vec<MessageLayout>,
    pub(super) notes: Vec<NoteLayout>,
    pub(super) fragments: Vec<FragmentLayout>,
    pub(super) activation_stack: BTreeMap<String, Vec<f64>>,
    pub(super) activations: Vec<ActivationLayout>,
    pub(super) max_self_label_right: f64,
    pub(super) msg_counter: Option<(u32, u32)>,
}

impl<'a, T: TextMeasure> LayoutPass<'a, T> {
    pub(super) fn layout_items(&mut self, items: &[SequenceItem]) {
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
        let note_w = (ts.width + 2.0 * NOTE_PADDING).min(NOTE_MAX_WIDTH);
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
        self.fragments.insert(slot, FragmentLayout {
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

    pub(super) fn close_activation(&mut self, actor_id: &str) {
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

    pub(super) fn close_remaining_activations(&mut self) {
        let ids: Vec<String> = self.activation_stack.keys().cloned().collect();
        for id in ids {
            while self.activation_stack.get(&id).is_some_and(|s| !s.is_empty()) {
                self.close_activation(&id);
            }
        }
    }
}
