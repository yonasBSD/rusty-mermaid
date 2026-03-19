use std::collections::BTreeMap;

use rusty_mermaid_core::Point;
use rusty_mermaid_graph::NodeId;

/// Position of an edge label relative to the edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LabelPos {
    Left,
    Center,
    #[default]
    Right,
}

/// Dummy node kind inserted during layout phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DummyKind {
    /// Long-edge dummy (normalize)
    Edge,
    /// Dummy at the label rank of a split edge (normalize)
    EdgeLabel,
    /// Left/right border of a compound node (add_border_segments)
    Border,
    /// Dummy node for a self-edge
    SelfEdge,
}

/// Border type for border dummy nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderType {
    Left,
    Right,
}

/// Data stored on edge-dummy nodes to reconstruct original edges during denormalize.
#[derive(Debug, Clone)]
pub struct EdgeDummyData {
    pub(crate) edge_label: EdgeLabel,
    pub(crate) edge_src: NodeId,
    pub(crate) edge_dst: NodeId,
}

/// A self-edge temporarily removed during layout.
#[derive(Debug, Clone)]
pub(crate) struct SelfEdge {
    pub(crate) src: NodeId,
    pub(crate) dst: NodeId,
    pub(crate) label: EdgeLabel,
}

/// Node data for dagre layout.
///
/// User provides `width` and `height`. The layout algorithm sets
/// `x`, `y`, `rank`, and `order`.
#[derive(Debug, Clone)]
pub struct NodeLabel {
    pub width: f64,
    pub height: f64,
    pub x: f64,
    pub y: f64,
    pub rank: i32,
    pub order: usize,
    pub(crate) dummy: Option<DummyKind>,

    // --- normalize: stored on edge-dummy nodes ---
    pub(crate) edge_data: Option<EdgeDummyData>,

    // --- compound node fields (nesting / border_segments) ---
    pub(crate) border_top: Option<NodeId>,
    pub(crate) border_bottom: Option<NodeId>,
    pub(crate) border_left: BTreeMap<i32, NodeId>,
    pub(crate) border_right: BTreeMap<i32, NodeId>,
    pub(crate) min_rank: Option<i32>,
    pub(crate) max_rank: Option<i32>,

    // --- border dummy node ---
    pub(crate) border_type: Option<BorderType>,

    // --- self-edge storage (removed before layout, reinserted after) ---
    pub(crate) self_edges: Vec<SelfEdge>,

    // --- edge label positioning (set on dummy nodes by BK) ---
    pub(crate) label_pos: Option<LabelPos>,

    // --- self-edge dummy node data ---
    pub(crate) self_edge_data: Option<crate::self_edges::SelfEdgeData>,

    // --- edge label proxy: stores the edge this proxy represents ---
    pub(crate) proxy_edge: Option<rusty_mermaid_graph::EdgeId>,
}

impl NodeLabel {
    pub fn new(width: f64, height: f64) -> Self {
        Self {
            width,
            height,
            x: 0.0,
            y: 0.0,
            rank: 0,
            order: 0,
            dummy: None,
            edge_data: None,
            border_top: None,
            border_bottom: None,
            border_left: BTreeMap::new(),
            border_right: BTreeMap::new(),
            min_rank: None,
            max_rank: None,
            border_type: None,
            self_edges: Vec::new(),
            label_pos: None,
            self_edge_data: None,
            proxy_edge: None,
        }
    }
}

/// Edge data for dagre layout.
///
/// User provides `minlen`, `weight`, and optional label dimensions.
/// The layout algorithm sets `x`, `y`, and `points`.
#[derive(Debug, Clone)]
pub struct EdgeLabel {
    pub minlen: i32,
    pub weight: f64,
    pub width: f64,
    pub height: f64,
    pub labelpos: LabelPos,
    pub labeloffset: f64,
    pub x: f64,
    pub y: f64,
    pub points: Vec<Point>,
    pub(crate) reversed: bool,
    pub(crate) nesting_edge: bool,
    pub(crate) label_rank: Option<i32>,
}

impl Default for EdgeLabel {
    fn default() -> Self {
        Self {
            minlen: 1,
            weight: 1.0,
            width: 0.0,
            height: 0.0,
            labelpos: LabelPos::default(),
            labeloffset: 10.0,
            x: 0.0,
            y: 0.0,
            points: Vec::new(),
            reversed: false,
            nesting_edge: false,
            label_rank: None,
        }
    }
}

impl EdgeLabel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_minlen(mut self, minlen: i32) -> Self {
        self.minlen = minlen;
        self
    }

    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }

    pub(crate) fn with_nesting(mut self) -> Self {
        self.nesting_edge = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_label_new() {
        let n = NodeLabel::new(100.0, 50.0);
        assert!((n.width - 100.0).abs() < f64::EPSILON);
        assert!((n.height - 50.0).abs() < f64::EPSILON);
        assert_eq!(n.rank, 0);
        assert!(n.dummy.is_none());
    }

    #[test]
    fn edge_label_defaults() {
        let e = EdgeLabel::default();
        assert_eq!(e.minlen, 1);
        assert!((e.weight - 1.0).abs() < f64::EPSILON);
        assert!(!e.reversed);
    }

    #[test]
    fn edge_label_builder() {
        let e = EdgeLabel::new().with_minlen(3).with_weight(2.0);
        assert_eq!(e.minlen, 3);
        assert!((e.weight - 2.0).abs() < f64::EPSILON);
    }
}
