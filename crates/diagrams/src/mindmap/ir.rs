/// A parsed mindmap diagram.
#[derive(Debug, Clone)]
pub struct MindmapDiagram {
    pub root: MindmapNode,
}

/// A node in the mindmap tree.
#[derive(Debug, Clone)]
pub struct MindmapNode {
    pub text: String,
    pub shape: MindmapShape,
    pub children: Vec<MindmapNode>,
    pub icon: Option<String>,
    pub css_classes: Vec<String>,
}

impl MindmapNode {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            shape: MindmapShape::Default,
            children: Vec::new(),
            icon: None,
            css_classes: Vec::new(),
        }
    }

    /// Count total nodes in this subtree (including self).
    pub fn count(&self) -> usize {
        1 + self.children.iter().map(|c| c.count()).sum::<usize>()
    }
}

/// Node shape, determined by delimiter syntax.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MindmapShape {
    #[default]
    Default,
    Rect,
    RoundedRect,
    Circle,
    Cloud,
    Bang,
    Hexagon,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_count() {
        let mut root = MindmapNode::new("Root");
        root.children.push(MindmapNode::new("A"));
        root.children.push(MindmapNode::new("B"));
        root.children[0].children.push(MindmapNode::new("A1"));
        assert_eq!(root.count(), 4);
    }

    #[test]
    fn shape_default() {
        assert_eq!(MindmapShape::default(), MindmapShape::Default);
    }

    #[test]
    fn node_new_defaults() {
        let n = MindmapNode::new("Test");
        assert_eq!(n.text, "Test");
        assert_eq!(n.shape, MindmapShape::Default);
        assert!(n.children.is_empty());
        assert!(n.icon.is_none());
        assert!(n.css_classes.is_empty());
    }

    #[test]
    fn count_single_node() {
        let n = MindmapNode::new("solo");
        assert_eq!(n.count(), 1);
    }

    #[test]
    fn shape_variants_distinct() {
        let shapes = [
            MindmapShape::Default, MindmapShape::Rect, MindmapShape::RoundedRect,
            MindmapShape::Circle, MindmapShape::Cloud, MindmapShape::Bang,
            MindmapShape::Hexagon,
        ];
        for (i, a) in shapes.iter().enumerate() {
            for (j, b) in shapes.iter().enumerate() {
                assert_eq!(i == j, *a == *b);
            }
        }
    }

    #[test]
    fn diagram_construction() {
        let root = MindmapNode::new("Central Idea");
        let d = MindmapDiagram { root };
        assert_eq!(d.root.text, "Central Idea");
        assert_eq!(d.root.count(), 1);
    }
}
