use crate::common::styling::{ClassDef, StyleStmt};
use rusty_mermaid_core::Direction;

/// A parsed requirement diagram.
#[derive(Debug, Clone)]
pub struct RequirementDiagram {
    pub direction: Direction,
    pub requirements: Vec<Requirement>,
    pub elements: Vec<DesignElement>,
    pub relationships: Vec<ReqRelation>,
    pub class_defs: Vec<ClassDef>,
    pub style_stmts: Vec<StyleStmt>,
}

impl RequirementDiagram {
    pub fn new(direction: Direction) -> Self {
        Self {
            direction,
            requirements: Vec::new(),
            elements: Vec::new(),
            relationships: Vec::new(),
            class_defs: Vec::new(),
            style_stmts: Vec::new(),
        }
    }
}

/// A requirement node.
#[derive(Debug, Clone)]
pub struct Requirement {
    pub name: String,
    pub req_type: RequirementType,
    pub id: Option<String>,
    pub text: Option<String>,
    pub risk: Option<RiskLevel>,
    pub verify_method: Option<VerifyMethod>,
    pub css_classes: Vec<String>,
}

impl Requirement {
    pub fn new(name: impl Into<String>, req_type: RequirementType) -> Self {
        Self {
            name: name.into(),
            req_type,
            id: None,
            text: None,
            risk: None,
            verify_method: None,
            css_classes: Vec::new(),
        }
    }

    pub fn display_type(&self) -> &'static str {
        self.req_type.label()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequirementType {
    Requirement,
    FunctionalRequirement,
    InterfaceRequirement,
    PerformanceRequirement,
    PhysicalRequirement,
    DesignConstraint,
}

impl RequirementType {
    pub fn label(self) -> &'static str {
        match self {
            Self::Requirement => "Requirement",
            Self::FunctionalRequirement => "Functional Requirement",
            Self::InterfaceRequirement => "Interface Requirement",
            Self::PerformanceRequirement => "Performance Requirement",
            Self::PhysicalRequirement => "Physical Requirement",
            Self::DesignConstraint => "Design Constraint",
        }
    }
}

/// A design element node.
#[derive(Debug, Clone)]
pub struct DesignElement {
    pub name: String,
    pub elem_type: Option<String>,
    pub docref: Option<String>,
    pub css_classes: Vec<String>,
}

impl DesignElement {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            elem_type: None,
            docref: None,
            css_classes: Vec::new(),
        }
    }
}

/// A relationship between requirements/elements.
#[derive(Debug, Clone)]
pub struct ReqRelation {
    pub src: String,
    pub dst: String,
    pub rel_type: RelationshipType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationshipType {
    Contains,
    Copies,
    Derives,
    Satisfies,
    Verifies,
    Refines,
    Traces,
}

impl RelationshipType {
    pub fn label(self) -> &'static str {
        match self {
            Self::Contains => "contains",
            Self::Copies => "copies",
            Self::Derives => "derives",
            Self::Satisfies => "satisfies",
            Self::Verifies => "verifies",
            Self::Refines => "refines",
            Self::Traces => "traces",
        }
    }

    pub fn is_dashed(self) -> bool {
        self != Self::Contains
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyMethod {
    Analysis,
    Demonstration,
    Inspection,
    Test,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requirement_type_labels() {
        assert_eq!(RequirementType::Requirement.label(), "Requirement");
        assert_eq!(RequirementType::DesignConstraint.label(), "Design Constraint");
    }

    #[test]
    fn relationship_dashing() {
        assert!(!RelationshipType::Contains.is_dashed());
        assert!(RelationshipType::Satisfies.is_dashed());
        assert!(RelationshipType::Traces.is_dashed());
    }

    #[test]
    fn requirement_default() {
        let r = Requirement::new("REQ_01", RequirementType::Requirement);
        assert_eq!(r.name, "REQ_01");
        assert!(r.id.is_none());
        assert!(r.text.is_none());
    }

    #[test]
    fn element_default() {
        let e = DesignElement::new("Component");
        assert_eq!(e.name, "Component");
        assert!(e.elem_type.is_none());
    }
}
