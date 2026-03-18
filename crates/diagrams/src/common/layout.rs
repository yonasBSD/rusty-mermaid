use rusty_mermaid_core::{Point, Shape, Style};

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

#[derive(Debug)]
pub struct NodeLayout {
    pub id: String,
    pub label: String,
    pub shape: Shape,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub is_compound: bool,
    /// Resolved style from classDef, class, style, and :::class.
    pub custom_style: Option<Style>,
    /// Number of concurrent regions (0 = not concurrent).
    pub region_count: usize,
}

#[derive(Debug)]
pub struct EdgeLayout {
    pub src: String,
    pub dst: String,
    pub points: Vec<Point>,
    pub label: Option<String>,
    /// Measured label dimensions (width, height) for background rect.
    pub label_size: Option<(f64, f64)>,
    pub stroke: StrokeType,
    pub start_arrow: ArrowEnd,
    pub end_arrow: ArrowEnd,
    /// Resolved style from linkStyle statements.
    pub custom_style: Option<Style>,
}
