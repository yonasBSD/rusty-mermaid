//! The Excalidraw element model — a typed, serialize-focused mirror of the
//! `.excalidraw` JSON format. Only what a generated diagram needs; Excalidraw's
//! import-time `restoreElements` backfills the rest.

use serde::Serialize;

/// A reference from a shape to a bound element (an arrow on the shape, or a
/// contained text). The mirror of an arrow's `startBinding`/`endBinding`.
#[derive(Debug, Clone, Serialize)]
pub struct BoundElement {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
}

/// An arrow endpoint bound to a shape.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Binding {
    pub element_id: String,
    pub focus: f64,
    pub gap: f64,
}

/// Corner rounding. Excalidraw serializes `{ "type": <int> }` or omits it.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Roundness {
    #[serde(rename = "type")]
    pub kind: u8,
}

/// One Excalidraw element. Common fields are shared; the `kind` carries the
/// `type` tag and type-specific fields (flattened onto the same object).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExElement {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub angle: f64,
    pub stroke_color: String,
    pub background_color: String,
    pub fill_style: String,
    pub stroke_width: f64,
    pub stroke_style: String,
    pub roughness: u8,
    pub opacity: u8,
    pub group_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roundness: Option<Roundness>,
    pub seed: u32,
    pub version: u32,
    pub version_nonce: u32,
    pub is_deleted: bool,
    pub bound_elements: Vec<BoundElement>,
    pub updated: u64,
    pub locked: bool,
    #[serde(flatten)]
    pub kind: ElementKind,
}

/// Type-specific element data, tagged by Excalidraw's `type` field.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ElementKind {
    Rectangle,
    Ellipse,
    Diamond,
    #[serde(rename_all = "camelCase")]
    Text {
        text: String,
        font_size: f64,
        /// Excalidraw font family: 1 hand-drawn, 2 normal, 3 code.
        font_family: u8,
        text_align: String,
        vertical_align: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        container_id: Option<String>,
        line_height: f64,
        original_text: String,
    },
    /// A connector. `line` for plain polylines (closed polygons too), `arrow`
    /// when an endpoint has an arrowhead — both share this shape.
    #[serde(rename_all = "camelCase")]
    Arrow {
        points: Vec<[f64; 2]>,
        #[serde(skip_serializing_if = "Option::is_none")]
        last_committed_point: Option<[f64; 2]>,
        #[serde(skip_serializing_if = "Option::is_none")]
        start_binding: Option<Binding>,
        #[serde(skip_serializing_if = "Option::is_none")]
        end_binding: Option<Binding>,
        #[serde(skip_serializing_if = "Option::is_none")]
        start_arrowhead: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        end_arrowhead: Option<String>,
    },
    /// A plain polyline / closed polygon (no arrowheads).
    #[serde(rename_all = "camelCase")]
    Line {
        points: Vec<[f64; 2]>,
        #[serde(skip_serializing_if = "Option::is_none")]
        last_committed_point: Option<[f64; 2]>,
    },
}

impl ElementKind {
    /// The Excalidraw `type` string for this kind.
    pub fn type_str(&self) -> &'static str {
        match self {
            Self::Rectangle => "rectangle",
            Self::Ellipse => "ellipse",
            Self::Diamond => "diamond",
            Self::Text { .. } => "text",
            Self::Arrow { .. } => "arrow",
            Self::Line { .. } => "line",
        }
    }
}

/// The top-level `.excalidraw` document envelope.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExScene {
    #[serde(rename = "type")]
    pub doc_type: &'static str,
    pub version: u32,
    pub source: &'static str,
    pub elements: Vec<ExElement>,
    pub app_state: AppState,
    pub files: serde_json::Value,
}

/// The `appState` block — only the fields a generated diagram sets.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    pub view_background_color: String,
}

impl ExScene {
    /// Wrap a converted element list in a valid `.excalidraw` envelope.
    pub fn new(elements: Vec<ExElement>, view_background: String) -> Self {
        Self {
            doc_type: "excalidraw",
            version: 2,
            source: "rusty-mermaid",
            elements,
            app_state: AppState {
                view_background_color: view_background,
            },
            files: serde_json::json!({}),
        }
    }
}
