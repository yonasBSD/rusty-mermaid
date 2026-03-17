use rusty_mermaid_core::{Direction, Shape};

use crate::common::styling::{ClassDef, StyleProperty};

/// A parsed flowchart diagram.
#[derive(Debug, Clone)]
pub struct FlowDiagram {
    pub direction: Direction,
    pub vertices: Vec<FlowVertex>,
    pub edges: Vec<FlowEdge>,
    pub subgraphs: Vec<FlowSubGraph>,
    pub class_defs: Vec<ClassDef>,
    pub style_stmts: Vec<FlowStyleStmt>,
    pub link_styles: Vec<FlowLinkStyle>,
}

/// A flowchart node.
#[derive(Debug, Clone)]
pub struct FlowVertex {
    pub id: String,
    /// Raw label text (may contain HTML tags like `<b>`, `<br/>`).
    pub label: String,
    pub shape: Shape,
    pub classes: Vec<String>,
}

/// Edge stroke type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StrokeType {
    #[default]
    Normal,
    Thick,
    Dotted,
}

/// Arrow endpoint marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArrowEnd {
    #[default]
    Arrow,
    Circle,
    Cross,
    None,
}

/// A flowchart edge.
#[derive(Debug, Clone)]
pub struct FlowEdge {
    pub src: String,
    pub dst: String,
    pub label: Option<String>,
    pub stroke: StrokeType,
    pub start_arrow: ArrowEnd,
    pub end_arrow: ArrowEnd,
    /// Minimum edge length in ranks. `-->` = 1, `--->` = 2, etc.
    pub minlen: i32,
}

/// A subgraph container.
#[derive(Debug, Clone)]
pub struct FlowSubGraph {
    pub id: String,
    pub label: Option<String>,
    pub direction: Option<Direction>,
    /// Node IDs directly contained (not nested subgraph contents).
    pub node_ids: Vec<String>,
    /// IDs of child subgraphs.
    pub subgraph_ids: Vec<String>,
}

/// Direct style applied to a node by ID.
#[derive(Debug, Clone)]
pub struct FlowStyleStmt {
    pub ids: Vec<String>,
    pub styles: Vec<StyleProperty>,
}

/// Edge style applied by declaration-order index.
/// `linkStyle 0,1 stroke:#f00,stroke-width:3px` or `linkStyle default stroke:green`.
#[derive(Debug, Clone)]
pub struct FlowLinkStyle {
    /// Edge indices (0-based declaration order), or empty for `default`.
    pub indices: Vec<usize>,
    pub is_default: bool,
    pub styles: Vec<StyleProperty>,
}

impl FlowDiagram {
    pub fn new(direction: Direction) -> Self {
        Self {
            direction,
            vertices: Vec::new(),
            edges: Vec::new(),
            subgraphs: Vec::new(),
            class_defs: Vec::new(),
            style_stmts: Vec::new(),
            link_styles: Vec::new(),
        }
    }

    /// Find a vertex by ID.
    pub fn vertex(&self, id: &str) -> Option<&FlowVertex> {
        self.vertices.iter().find(|v| v.id == id)
    }

    /// Find the subgraph containing a given node ID (if any).
    pub fn parent_subgraph(&self, node_id: &str) -> Option<&FlowSubGraph> {
        self.subgraphs.iter().find(|sg| sg.node_ids.contains(&node_id.to_string()))
    }
}

impl FlowVertex {
    pub fn new(id: impl Into<String>, label: impl Into<String>, shape: Shape) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            shape,
            classes: Vec::new(),
        }
    }
}

impl FlowEdge {
    pub fn new(src: impl Into<String>, dst: impl Into<String>) -> Self {
        Self {
            src: src.into(),
            dst: dst.into(),
            label: None,
            stroke: StrokeType::Normal,
            start_arrow: ArrowEnd::None,
            end_arrow: ArrowEnd::Arrow,
            minlen: 1,
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
    fn flow_diagram_construction() {
        let mut d = FlowDiagram::new(Direction::TB);
        d.vertices.push(FlowVertex::new("A", "Start", Shape::Rect));
        d.vertices.push(FlowVertex::new("B", "End", Shape::Rect));
        d.edges.push(FlowEdge::new("A", "B"));

        assert_eq!(d.vertices.len(), 2);
        assert_eq!(d.edges.len(), 1);
        assert!(d.vertex("A").is_some());
        assert!(d.vertex("Z").is_none());
    }

    #[test]
    fn edge_with_label() {
        let e = FlowEdge::new("A", "B").with_label("yes");
        assert_eq!(e.label.as_deref(), Some("yes"));
        assert_eq!(e.stroke, StrokeType::Normal);
        assert_eq!(e.end_arrow, ArrowEnd::Arrow);
    }

    #[test]
    fn vertex_shape_mapping() {
        let v = FlowVertex::new("X", "Decision?", Shape::Diamond);
        assert_eq!(v.shape, Shape::Diamond);
        assert_eq!(v.id, "X");
    }

    #[test]
    fn subgraph_membership() {
        let mut d = FlowDiagram::new(Direction::TB);
        d.vertices.push(FlowVertex::new("A", "Node A", Shape::Rect));
        d.vertices.push(FlowVertex::new("B", "Node B", Shape::Rect));
        d.subgraphs.push(FlowSubGraph {
            id: "sg1".into(),
            label: Some("Group".into()),
            direction: None,
            node_ids: vec!["A".into()],
            subgraph_ids: Vec::new(),
        });

        assert!(d.parent_subgraph("A").is_some());
        assert!(d.parent_subgraph("B").is_none());
    }
}
