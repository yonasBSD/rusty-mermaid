use crate::common::styling::{ClassDef, StyleStmt};
use rusty_mermaid_core::Direction;

/// A parsed class diagram.
#[derive(Debug, Clone)]
pub struct ClassDiagram {
    pub direction: Direction,
    pub classes: Vec<ClassNode>,
    pub relationships: Vec<ClassRelation>,
    pub notes: Vec<ClassNote>,
    pub namespaces: Vec<Namespace>,
    pub class_defs: Vec<ClassDef>,
    pub style_stmts: Vec<StyleStmt>,
}

impl ClassDiagram {
    pub fn new(direction: Direction) -> Self {
        Self {
            direction,
            classes: Vec::new(),
            relationships: Vec::new(),
            notes: Vec::new(),
            namespaces: Vec::new(),
            class_defs: Vec::new(),
            style_stmts: Vec::new(),
        }
    }

    /// Find a class by ID.
    pub fn class(&self, id: &str) -> Option<&ClassNode> {
        self.classes.iter().find(|c| c.id == id)
    }
}

/// A class node with members, methods, and annotations.
#[derive(Debug, Clone)]
pub struct ClassNode {
    pub id: String,
    pub label: Option<String>,
    pub generic_type: Option<String>,
    pub members: Vec<ClassMember>,
    pub methods: Vec<ClassMember>,
    pub annotations: Vec<String>,
    pub namespace: Option<String>,
    pub css_classes: Vec<String>,
}

impl ClassNode {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: None,
            generic_type: None,
            members: Vec::new(),
            methods: Vec::new(),
            annotations: Vec::new(),
            namespace: None,
            css_classes: Vec::new(),
        }
    }
}

/// A class member (attribute or method).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassMember {
    pub name: String,
    pub visibility: Visibility,
    pub classifier: Classifier,
    pub return_type: Option<String>,
    pub parameters: Option<String>,
}

impl ClassMember {
    pub fn attribute(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            visibility: Visibility::None,
            classifier: Classifier::None,
            return_type: None,
            parameters: None,
        }
    }

    pub fn method(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            visibility: Visibility::None,
            classifier: Classifier::None,
            return_type: None,
            parameters: Some(String::new()),
        }
    }

    pub fn is_method(&self) -> bool {
        self.parameters.is_some()
    }

    /// Format for display: `+name(params) : ReturnType`
    pub fn display_text(&self) -> String {
        let mut s = String::new();
        s.push_str(self.visibility.prefix());
        s.push_str(&self.name);
        if let Some(params) = &self.parameters {
            s.push('(');
            s.push_str(params);
            s.push(')');
        }
        if let Some(rt) = &self.return_type {
            s.push_str(" : ");
            s.push_str(rt);
        }
        s.push_str(self.classifier.suffix());
        s
    }
}

/// Member visibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Visibility {
    Public,
    Private,
    Protected,
    Package,
    #[default]
    None,
}

impl Visibility {
    pub fn prefix(self) -> &'static str {
        match self {
            Self::Public => "+",
            Self::Private => "-",
            Self::Protected => "#",
            Self::Package => "~",
            Self::None => "",
        }
    }

    pub fn from_char(c: char) -> Option<Self> {
        match c {
            '+' => Some(Self::Public),
            '-' => Some(Self::Private),
            '#' => Some(Self::Protected),
            '~' => Some(Self::Package),
            _ => None,
        }
    }
}

/// Member classifier (static or abstract).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Classifier {
    #[default]
    None,
    Static,
    Abstract,
}

impl Classifier {
    pub fn suffix(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Static => "$",
            Self::Abstract => "*",
        }
    }

    pub fn from_char(c: char) -> Option<Self> {
        match c {
            '$' => Some(Self::Static),
            '*' => Some(Self::Abstract),
            _ => None,
        }
    }
}

/// A relationship between two classes.
#[derive(Debug, Clone)]
pub struct ClassRelation {
    pub from_id: String,
    pub to_id: String,
    /// Marker type at the from (left) end.
    pub from_type: Option<RelationType>,
    /// Marker type at the to (right) end.
    pub to_type: Option<RelationType>,
    pub line_type: LineType,
    pub label: Option<String>,
    pub cardinality_from: Option<String>,
    pub cardinality_to: Option<String>,
}

/// Relationship type (marker shape).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationType {
    Extension,
    Composition,
    Aggregation,
    Dependency,
    Lollipop,
    /// Plain line, no markers.
    Association,
}

/// Line style for relationships.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineType {
    #[default]
    Solid,
    Dotted,
}

/// A note attached to a class.
#[derive(Debug, Clone)]
pub struct ClassNote {
    pub text: String,
    pub class_id: Option<String>,
}

/// A namespace grouping classes.
#[derive(Debug, Clone)]
pub struct Namespace {
    pub id: String,
    pub parent: Option<String>,
    pub class_ids: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_node_default() {
        let c = ClassNode::new("Foo");
        assert_eq!(c.id, "Foo");
        assert!(c.members.is_empty());
        assert!(c.methods.is_empty());
        assert!(c.annotations.is_empty());
    }

    #[test]
    fn member_display_full() {
        let m = ClassMember {
            name: "getName".into(),
            visibility: Visibility::Public,
            classifier: Classifier::None,
            return_type: Some("String".into()),
            parameters: Some("id int".into()),
        };
        assert_eq!(m.display_text(), "+getName(id int) : String");
    }

    #[test]
    fn member_display_attribute() {
        let m = ClassMember {
            name: "count".into(),
            visibility: Visibility::Private,
            classifier: Classifier::Static,
            return_type: Some("int".into()),
            parameters: None,
        };
        assert_eq!(m.display_text(), "-count : int$");
    }

    #[test]
    fn member_is_method() {
        assert!(ClassMember::method("foo").is_method());
        assert!(!ClassMember::attribute("bar").is_method());
    }

    #[test]
    fn visibility_from_char() {
        assert_eq!(Visibility::from_char('+'), Some(Visibility::Public));
        assert_eq!(Visibility::from_char('-'), Some(Visibility::Private));
        assert_eq!(Visibility::from_char('#'), Some(Visibility::Protected));
        assert_eq!(Visibility::from_char('~'), Some(Visibility::Package));
        assert_eq!(Visibility::from_char('x'), None);
    }

    #[test]
    fn classifier_from_char() {
        assert_eq!(Classifier::from_char('$'), Some(Classifier::Static));
        assert_eq!(Classifier::from_char('*'), Some(Classifier::Abstract));
        assert_eq!(Classifier::from_char('x'), None);
    }

    #[test]
    fn class_diagram_find() {
        let mut d = ClassDiagram::new(Direction::TB);
        d.classes.push(ClassNode::new("A"));
        d.classes.push(ClassNode::new("B"));
        assert_eq!(d.class("A").unwrap().id, "A");
        assert!(d.class("C").is_none());
    }

    #[test]
    fn relation_types() {
        let r = ClassRelation {
            from_id: "A".into(),
            to_id: "B".into(),
            from_type: Some(RelationType::Extension),
            to_type: None,
            line_type: LineType::Solid,
            label: Some("extends".into()),
            cardinality_from: None,
            cardinality_to: None,
        };
        assert_eq!(r.from_type, Some(RelationType::Extension));
        assert_eq!(r.line_type, LineType::Solid);
    }

    #[test]
    fn namespace_basic() {
        let ns = Namespace {
            id: "com.example".into(),
            parent: None,
            class_ids: vec!["User".into(), "Order".into()],
        };
        assert_eq!(ns.class_ids.len(), 2);
    }
}
