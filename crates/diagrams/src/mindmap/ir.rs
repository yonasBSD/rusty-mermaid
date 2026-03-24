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
}
