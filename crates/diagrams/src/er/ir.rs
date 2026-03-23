use crate::common::styling::{ClassDef, StyleStmt};
use rusty_mermaid_core::Direction;

/// A parsed ER diagram.
#[derive(Debug, Clone)]
pub struct ErDiagram {
    pub direction: Direction,
    pub entities: Vec<Entity>,
    pub relationships: Vec<ErRelation>,
    pub class_defs: Vec<ClassDef>,
    pub style_stmts: Vec<StyleStmt>,
}

impl ErDiagram {
    pub fn new(direction: Direction) -> Self {
        Self {
            direction,
            entities: Vec::new(),
            relationships: Vec::new(),
            class_defs: Vec::new(),
            style_stmts: Vec::new(),
        }
    }

    pub fn entity(&self, id: &str) -> Option<&Entity> {
        self.entities.iter().find(|e| e.id == id)
    }
}

/// An entity (table) with attributes.
#[derive(Debug, Clone)]
pub struct Entity {
    pub id: String,
    pub alias: Option<String>,
    pub attributes: Vec<Attribute>,
    pub css_classes: Vec<String>,
}

impl Entity {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            alias: None,
            attributes: Vec::new(),
            css_classes: Vec::new(),
        }
    }

    pub fn display_name(&self) -> &str {
        self.alias.as_deref().unwrap_or(&self.id)
    }
}

/// An entity attribute (column).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    pub attr_type: String,
    pub name: String,
    pub keys: Vec<KeyType>,
    pub comment: Option<String>,
}

/// Key constraint on an attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyType {
    PrimaryKey,
    ForeignKey,
    UniqueKey,
}

impl KeyType {
    pub fn label(self) -> &'static str {
        match self {
            Self::PrimaryKey => "PK",
            Self::ForeignKey => "FK",
            Self::UniqueKey => "UK",
        }
    }
}

/// A relationship between two entities.
#[derive(Debug, Clone)]
pub struct ErRelation {
    pub entity_a: String,
    pub entity_b: String,
    pub cardinality_a: Cardinality,
    pub cardinality_b: Cardinality,
    pub identification: Identification,
    pub label: Option<String>,
}

/// Cardinality at one end of a relationship.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cardinality {
    ExactlyOne,
    ZeroOrOne,
    OneOrMore,
    ZeroOrMore,
}

/// Whether the relationship is identifying (solid) or non-identifying (dashed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Identification {
    #[default]
    Identifying,
    NonIdentifying,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_default() {
        let e = Entity::new("Customer");
        assert_eq!(e.id, "Customer");
        assert!(e.attributes.is_empty());
        assert!(e.alias.is_none());
    }

    #[test]
    fn entity_display_name_uses_alias() {
        let mut e = Entity::new("CUST");
        e.alias = Some("Customer".into());
        assert_eq!(e.display_name(), "Customer");
    }

    #[test]
    fn entity_display_name_falls_back_to_id() {
        let e = Entity::new("Customer");
        assert_eq!(e.display_name(), "Customer");
    }

    #[test]
    fn attribute_with_keys() {
        let a = Attribute {
            attr_type: "int".into(),
            name: "id".into(),
            keys: vec![KeyType::PrimaryKey, KeyType::ForeignKey],
            comment: Some("Unique identifier".into()),
        };
        assert_eq!(a.keys[0].label(), "PK");
        assert_eq!(a.keys[1].label(), "FK");
    }

    #[test]
    fn cardinality_variants() {
        assert_ne!(Cardinality::ExactlyOne, Cardinality::ZeroOrMore);
        assert_eq!(Cardinality::OneOrMore, Cardinality::OneOrMore);
    }

    #[test]
    fn diagram_find_entity() {
        let mut d = ErDiagram::new(Direction::TB);
        d.entities.push(Entity::new("A"));
        d.entities.push(Entity::new("B"));
        assert_eq!(d.entity("A").unwrap().id, "A");
        assert!(d.entity("C").is_none());
    }

    #[test]
    fn relation_structure() {
        let r = ErRelation {
            entity_a: "Customer".into(),
            entity_b: "Order".into(),
            cardinality_a: Cardinality::ExactlyOne,
            cardinality_b: Cardinality::ZeroOrMore,
            identification: Identification::Identifying,
            label: Some("places".into()),
        };
        assert_eq!(r.cardinality_a, Cardinality::ExactlyOne);
        assert_eq!(r.identification, Identification::Identifying);
    }
}
