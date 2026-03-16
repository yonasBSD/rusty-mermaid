use rusty_mermaid_core::Point;

/// Position of an edge label relative to the edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LabelPos {
    Left,
    #[default]
    Center,
    Right,
}

/// Dummy node kind (inserted during normalization, Phase 1b).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DummyKind {
    Edge,
    BorderTop,
    BorderBottom,
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
