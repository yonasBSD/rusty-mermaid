use rusty_mermaid_core::Direction;

use crate::common::styling::{ClassDef, StyleProperty};

/// A parsed state diagram.
#[derive(Debug, Clone)]
pub struct StateDiagram {
    pub direction: Direction,
    pub states: Vec<StateNode>,
    pub transitions: Vec<StateTransition>,
    pub notes: Vec<StateNote>,
    pub class_defs: Vec<ClassDef>,
    pub style_stmts: Vec<StateStyleStmt>,
}

/// Direct style applied to a state by ID.
#[derive(Debug, Clone)]
pub struct StateStyleStmt {
    pub ids: Vec<String>,
    pub styles: Vec<StyleProperty>,
}

/// A state node.
#[derive(Debug, Clone)]
pub struct StateNode {
    pub id: String,
    pub label: Option<String>,
    pub kind: StateKind,
    pub classes: Vec<String>,
}

/// What kind of state this is.
#[derive(Debug, Clone)]
pub enum StateKind {
    Normal,
    /// `[*]` — start pseudo-state.
    Start,
    /// `[*]` — end pseudo-state (determined by usage context in transitions).
    End,
    /// `<<fork>>` — horizontal bar splitting.
    Fork,
    /// `<<join>>` — horizontal bar merging.
    Join,
    /// `<<choice>>` — diamond decision point.
    Choice,
    /// Composite state containing children.
    Composite {
        direction: Option<Direction>,
        children: Vec<StateNode>,
        transitions: Vec<StateTransition>,
        notes: Vec<StateNote>,
        /// Concurrency dividers split a composite into parallel regions.
        concurrent: bool,
    },
}

/// A transition between states.
#[derive(Debug, Clone)]
pub struct StateTransition {
    pub src: String,
    pub dst: String,
    pub label: Option<String>,
}

/// Note placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotePosition {
    Left,
    Right,
}

/// A note attached to a state.
#[derive(Debug, Clone)]
pub struct StateNote {
    pub position: NotePosition,
    pub state_id: String,
    pub text: String,
}

impl StateDiagram {
    pub fn new(direction: Direction) -> Self {
        Self {
            direction,
            states: Vec::new(),
            transitions: Vec::new(),
            notes: Vec::new(),
            class_defs: Vec::new(),
            style_stmts: Vec::new(),
        }
    }

    /// Find a state by ID (top-level only).
    pub fn state(&self, id: &str) -> Option<&StateNode> {
        self.states.iter().find(|s| s.id == id)
    }
}

impl StateNode {
    pub fn new(id: impl Into<String>, kind: StateKind) -> Self {
        Self {
            id: id.into(),
            label: None,
            kind,
            classes: Vec::new(),
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn is_composite(&self) -> bool {
        matches!(self.kind, StateKind::Composite { .. })
    }
}

impl StateTransition {
    pub fn new(src: impl Into<String>, dst: impl Into<String>) -> Self {
        Self {
            src: src.into(),
            dst: dst.into(),
            label: None,
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagram_construction() {
        let mut d = StateDiagram::new(Direction::TB);
        d.states.push(StateNode::new("Still", StateKind::Normal));
        d.states.push(StateNode::new("Moving", StateKind::Normal));
        d.transitions.push(StateTransition::new("Still", "Moving"));

        assert_eq!(d.states.len(), 2);
        assert_eq!(d.transitions.len(), 1);
        assert!(d.state("Still").is_some());
        assert!(d.state("Gone").is_none());
    }

    #[test]
    fn state_with_label() {
        let s = StateNode::new("idle", StateKind::Normal).with_label("Idle State");
        assert_eq!(s.label.as_deref(), Some("Idle State"));
        assert!(matches!(s.kind, StateKind::Normal));
    }

    #[test]
    fn composite_state() {
        let inner = StateNode::new("A", StateKind::Normal);
        let composite = StateNode::new("parent", StateKind::Composite {
            direction: Some(Direction::LR),
            children: vec![inner],
            transitions: vec![],
            notes: vec![],
            concurrent: false,
        });
        assert!(composite.is_composite());
    }

    #[test]
    fn transition_with_label() {
        let t = StateTransition::new("A", "B").with_label("event");
        assert_eq!(t.label.as_deref(), Some("event"));
        assert_eq!(t.src, "A");
        assert_eq!(t.dst, "B");
    }

    #[test]
    fn special_state_kinds() {
        let fork = StateNode::new("f1", StateKind::Fork);
        let join = StateNode::new("j1", StateKind::Join);
        let choice = StateNode::new("c1", StateKind::Choice);
        assert!(matches!(fork.kind, StateKind::Fork));
        assert!(matches!(join.kind, StateKind::Join));
        assert!(matches!(choice.kind, StateKind::Choice));
        assert!(!fork.is_composite());
    }

    #[test]
    fn note_construction() {
        let note = StateNote {
            position: NotePosition::Right,
            state_id: "Still".into(),
            text: "idle state".into(),
        };
        assert_eq!(note.position, NotePosition::Right);
    }
}
