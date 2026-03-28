/// Block diagram: grid-based layout with shapes and edges.

#[derive(Debug, Clone, Default)]
pub struct BlockDiagram {
    pub columns: usize,
    pub blocks: Vec<Block>,
    pub edges: Vec<BlockEdge>,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub id: String,
    pub label: String,
    pub shape: BlockShape,
    pub children: Vec<Block>, // nested composite
    pub span: usize,          // column span (default 1)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockShape {
    Rect,
    Round,
    Stadium,
    Diamond,
    Hexagon,
    Circle,
    Cylinder,
    Space, // invisible placeholder
}

#[derive(Debug, Clone)]
pub struct BlockEdge {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub style: EdgeStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeStyle {
    Arrow,
    Dotted,
    Thick,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_empty() {
        let d = BlockDiagram::default();
        assert!(d.blocks.is_empty());
        assert_eq!(d.columns, 0);
    }

    #[test]
    fn default_edges_empty() {
        let d = BlockDiagram::default();
        assert!(d.edges.is_empty());
    }

    #[test]
    fn block_shape_variants() {
        let shapes = [
            BlockShape::Rect,
            BlockShape::Round,
            BlockShape::Stadium,
            BlockShape::Diamond,
            BlockShape::Hexagon,
            BlockShape::Circle,
            BlockShape::Cylinder,
            BlockShape::Space,
        ];
        for (i, a) in shapes.iter().enumerate() {
            for (j, b) in shapes.iter().enumerate() {
                assert_eq!(i == j, *a == *b);
            }
        }
    }

    #[test]
    fn edge_style_variants() {
        assert_ne!(EdgeStyle::Arrow, EdgeStyle::Dotted);
        assert_ne!(EdgeStyle::Dotted, EdgeStyle::Thick);
        assert_eq!(EdgeStyle::Arrow, EdgeStyle::Arrow);
    }

    #[test]
    fn block_with_children() {
        let child = Block {
            id: "c1".into(),
            label: "Child".into(),
            shape: BlockShape::Rect,
            children: vec![],
            span: 1,
        };
        let parent = Block {
            id: "p1".into(),
            label: "Parent".into(),
            shape: BlockShape::Round,
            children: vec![child],
            span: 2,
        };
        assert_eq!(parent.children.len(), 1);
        assert_eq!(parent.span, 2);
        assert_eq!(parent.children[0].id, "c1");
    }
}
