/// A parsed sequence diagram.
#[derive(Debug, Clone)]
pub struct SequenceDiagram {
    pub title: Option<String>,
    pub participants: Vec<Participant>,
    pub items: Vec<SequenceItem>,
    pub autonumber: Option<AutoNumber>,
}

/// A participant (actor or box) in the sequence diagram.
#[derive(Debug, Clone)]
pub struct Participant {
    pub id: String,
    pub label: String,
    pub kind: ParticipantKind,
}

/// How a participant is rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ParticipantKind {
    #[default]
    Box,
    Actor,
}

/// A sequential item in the diagram body.
#[derive(Debug, Clone)]
pub enum SequenceItem {
    Message(Message),
    Note(Note),
    Activation(Activation),
    Fragment(Fragment),
}

/// A message arrow between two participants.
#[derive(Debug, Clone)]
pub struct Message {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub arrow: ArrowStyle,
    /// Activate the target participant on delivery.
    pub activate: bool,
    /// Deactivate the source participant on delivery.
    pub deactivate: bool,
}

/// Arrow line and head style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArrowStyle {
    pub line: LineStyle,
    pub head: ArrowHead,
}

/// Line rendering for message arrows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineStyle {
    #[default]
    Solid,
    Dotted,
}

/// Arrowhead rendering at the target end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArrowHead {
    #[default]
    Filled,
    Open,
    Cross,
    None,
}

/// A note placed near one or more participants.
#[derive(Debug, Clone)]
pub struct Note {
    pub position: NotePosition,
    pub text: String,
}

/// Where a note is placed relative to participants.
#[derive(Debug, Clone)]
pub enum NotePosition {
    LeftOf(String),
    RightOf(String),
    Over(Vec<String>),
}

/// Explicit activation or deactivation of a participant's lifeline.
#[derive(Debug, Clone)]
pub struct Activation {
    pub actor: String,
    pub active: bool,
}

/// An interaction fragment (loop, alt, opt, par, critical, break).
#[derive(Debug, Clone)]
pub struct Fragment {
    pub kind: FragmentKind,
    pub label: Option<String>,
    /// Sections within the fragment. A simple loop has one section.
    /// alt/else has multiple sections (first is the alt, rest are else branches).
    /// par/and has multiple sections (first is the par, rest are and branches).
    pub sections: Vec<FragmentSection>,
}

/// A section within a fragment, with its own label and items.
#[derive(Debug, Clone)]
pub struct FragmentSection {
    pub label: Option<String>,
    pub items: Vec<SequenceItem>,
}

/// Fragment type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FragmentKind {
    Loop,
    Alt,
    Opt,
    Par,
    Critical,
    Break,
}

/// Autonumbering configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AutoNumber {
    pub start: u32,
    pub step: u32,
}

// --- Constructors ---

impl SequenceDiagram {
    pub fn new() -> Self {
        Self {
            title: None,
            participants: Vec::new(),
            items: Vec::new(),
            autonumber: None,
        }
    }

    /// Find a participant by ID.
    pub fn participant(&self, id: &str) -> Option<&Participant> {
        self.participants.iter().find(|p| p.id == id)
    }
}

impl Default for SequenceDiagram {
    fn default() -> Self {
        Self::new()
    }
}

impl Participant {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            kind: ParticipantKind::Box,
        }
    }

    pub fn actor(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            kind: ParticipantKind::Actor,
        }
    }
}

impl Message {
    pub fn new(from: impl Into<String>, to: impl Into<String>, arrow: ArrowStyle) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            label: None,
            arrow,
            activate: false,
            deactivate: false,
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

impl ArrowStyle {
    pub const SOLID_FILLED: Self = Self { line: LineStyle::Solid, head: ArrowHead::Filled };
    pub const SOLID_OPEN: Self = Self { line: LineStyle::Solid, head: ArrowHead::Open };
    pub const DOTTED_FILLED: Self = Self { line: LineStyle::Dotted, head: ArrowHead::Filled };
    pub const DOTTED_OPEN: Self = Self { line: LineStyle::Dotted, head: ArrowHead::Open };
    pub const SOLID_CROSS: Self = Self { line: LineStyle::Solid, head: ArrowHead::Cross };
    pub const DOTTED_CROSS: Self = Self { line: LineStyle::Dotted, head: ArrowHead::Cross };
}

impl Default for ArrowStyle {
    fn default() -> Self {
        Self::SOLID_FILLED
    }
}

impl Default for AutoNumber {
    fn default() -> Self {
        Self { start: 1, step: 1 }
    }
}

impl Fragment {
    pub fn new(kind: FragmentKind) -> Self {
        Self {
            kind,
            label: None,
            sections: Vec::new(),
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

impl FragmentSection {
    pub fn new() -> Self {
        Self {
            label: None,
            items: Vec::new(),
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

impl Default for FragmentSection {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for FragmentKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Loop => write!(f, "loop"),
            Self::Alt => write!(f, "alt"),
            Self::Opt => write!(f, "opt"),
            Self::Par => write!(f, "par"),
            Self::Critical => write!(f, "critical"),
            Self::Break => write!(f, "break"),
        }
    }
}

#[cfg(test)]
mod tests {
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
        let left = Note { position: NotePosition::LeftOf("A".into()), text: "hi".into() };
        let right = Note { position: NotePosition::RightOf("B".into()), text: "hi".into() };
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
        let act = Activation { actor: "Bob".into(), active: true };
        let deact = Activation { actor: "Bob".into(), active: false };
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
}
