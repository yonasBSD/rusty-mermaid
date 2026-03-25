/// Block diagram: grid-based layout with shapes and edges.

#[derive(Debug, Clone)]
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

impl Default for BlockDiagram {
    fn default() -> Self {
        Self { columns: 0, blocks: Vec::new(), edges: Vec::new() }
    }
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
}
