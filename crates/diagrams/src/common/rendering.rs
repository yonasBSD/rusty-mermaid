use rusty_mermaid_core::{
    BBox, Color, MarkerType, PathSegment, Point, Primitive, Scene, Style, TextAnchor, TextStyle,
    Theme,
};

use super::styling::StyleProperty;

/// Default node style from theme.
pub fn node_style(theme: &Theme) -> Style {
    Style {
        fill: Some(theme.node_fill),
        stroke: Some(theme.node_stroke),
        stroke_width: Some(theme.default_stroke_width),
        ..Default::default()
    }
}

/// Default node label style from theme.
pub fn label_style(theme: &Theme) -> TextStyle {
    TextStyle {
        fill: Some(theme.node_text),
        ..Default::default()
    }
}

/// Default edge label text style from theme.
pub fn edge_label_style(theme: &Theme) -> TextStyle {
    TextStyle {
        font_size: theme.font_size_edge_label,
        fill: Some(theme.edge_label_text),
        ..Default::default()
    }
}

/// Default edge label background style from theme.
pub fn edge_label_bg_style(theme: &Theme) -> Style {
    Style {
        fill: Some(theme.edge_label_bg),
        stroke: Some(theme.edge_label_bg),
        stroke_width: Some(0.5),
        ..Default::default()
    }
}

/// Overlay a custom style onto a base style, field by field.
pub fn overlay_style(base: &mut Style, custom: &Style) {
    if custom.fill.is_some() { base.fill = custom.fill; }
    if custom.stroke.is_some() { base.stroke = custom.stroke; }
    if custom.stroke_width.is_some() { base.stroke_width = custom.stroke_width; }
    if custom.stroke_dasharray.is_some() { base.stroke_dasharray = custom.stroke_dasharray.clone(); }
    if custom.opacity.is_some() { base.opacity = custom.opacity; }
}

/// Merge a node's custom style onto the theme default.
pub fn merge_custom_style(custom: Option<&Style>, theme: &Theme) -> Style {
    let mut style = node_style(theme);
    if let Some(custom) = custom {
        overlay_style(&mut style, custom);
    }
    style
}

/// Apply parsed CSS-like style properties onto a Style.
pub fn apply_style_properties(style: &mut Style, props: &[StyleProperty]) {
    for prop in props {
        match prop.key.as_str() {
            "fill" => { style.fill = Color::from_css(&prop.value); }
            "stroke" => { style.stroke = Color::from_css(&prop.value); }
            "stroke-width" => {
                let v = prop.value.trim_end_matches("px");
                if let Ok(w) = v.parse::<f64>() {
                    style.stroke_width = Some(w);
                }
            }
            "stroke-dasharray" => {
                let vals: Vec<f64> = prop.value
                    .split_whitespace()
                    .flat_map(|s| s.split(','))
                    .filter_map(|s| s.trim().parse().ok())
                    .collect();
                if !vals.is_empty() {
                    style.stroke_dasharray = Some(vals);
                }
            }
            "opacity" => {
                if let Ok(o) = prop.value.parse::<f64>() {
                    style.opacity = Some(o);
                }
            }
            _ => {}
        }
    }
}

/// Resolve classDef + class + style statements into per-entity Style maps.
///
/// Generic over entity type — works for flowchart vertices, state nodes, etc.
/// `entities` yields `(id, classes)` pairs for each entity.
/// `style_stmts` contains inline `style nodeId fill:...` statements.
pub fn resolve_entity_styles<'a>(
    entities: impl Iterator<Item = (&'a str, &'a [String])>,
    class_defs: &[super::styling::ClassDef],
    style_stmts: &[super::styling::StyleStmt],
) -> std::collections::BTreeMap<&'a str, Style> {
    let class_map: std::collections::BTreeMap<&str, &[StyleProperty]> = class_defs
        .iter()
        .map(|cd| (cd.name.as_str(), cd.styles.as_slice()))
        .collect();

    let mut result = std::collections::BTreeMap::new();

    for (id, classes) in entities {
        let mut style = Style::default();
        let mut has_custom = false;

        if let Some(props) = class_map.get("default") {
            apply_style_properties(&mut style, props);
            has_custom = true;
        }
        for class_name in classes {
            if let Some(props) = class_map.get(class_name.as_str()) {
                apply_style_properties(&mut style, props);
                has_custom = true;
            }
        }
        for stmt in style_stmts {
            if stmt.ids.iter().any(|sid| sid == id) {
                apply_style_properties(&mut style, &stmt.styles);
                has_custom = true;
            }
        }

        if has_custom {
            result.insert(id, style);
        }
    }

    result
}

/// Render a background rect behind an edge label for readability.
pub fn render_edge_label_bg(
    scene: &mut Scene,
    center: Point,
    label_size: (f64, f64),
    theme: &Theme,
) {
    const EDGE_LABEL_PAD: f64 = 4.0;
    const EDGE_LABEL_RX: f64 = 2.0;
    scene.push(Primitive::Rect {
        bbox: BBox::new(center.x, center.y, label_size.0 + EDGE_LABEL_PAD * 2.0, label_size.1 + EDGE_LABEL_PAD * 2.0),
        rx: EDGE_LABEL_RX,
        ry: EDGE_LABEL_RX,
        style: edge_label_bg_style(theme),
    });
}

/// Pick a contrasting text color based on the node fill luminance.
pub fn contrasting_label_style(node_fill: Option<Color>, theme: &Theme) -> TextStyle {
    let mut lstyle = label_style(theme);
    const DARK_FILL_THRESHOLD: f64 = 0.4;
    const LIGHT_FILL_THRESHOLD: f64 = 0.9;
    if let Some(fill) = node_fill {
        let lum = fill.luminance();
        if lum < DARK_FILL_THRESHOLD {
            lstyle.fill = Some(Color::WHITE);
        } else if lum > LIGHT_FILL_THRESHOLD {
            lstyle.fill = Some(Color::BLACK);
        }
    }
    lstyle
}

/// Render an edge label with optional background rect.
pub fn render_edge_label(
    scene: &mut Scene,
    mid: Point,
    label: &str,
    label_size: Option<(f64, f64)>,
    theme: &Theme,
) {
    if let Some(size) = label_size {
        render_edge_label_bg(scene, mid, size, theme);
    }
    scene.push(Primitive::Text {
        position: mid,
        content: label.to_string(),
        anchor: TextAnchor::Middle,
        style: edge_label_style(theme),
    });
}

// ---------------------------------------------------------------------------
// Marker-aware path shortening
// ---------------------------------------------------------------------------

/// Viewbox-unit inset applied uniformly to all markers.
/// Every marker body is wide enough at d=2 from its leading edge
/// to hide the 1.25-viewBox-unit stroke (half-width 0.625).
pub const MARKER_INSET_VB: f64 = 2.0;

/// Convert marker inset from viewBox units to user-space pixels.
/// scale = markerWidth × stroke_width / viewBoxWidth
pub fn marker_inset_px(marker: MarkerType, stroke_width: f64) -> f64 {
    let (marker_w, vb_w) = match marker {
        MarkerType::Aggregation | MarkerType::Composition => (8.0, 12.0),
        MarkerType::Dependency => (6.0, 10.0),
        _ => (8.0, 10.0),
    };
    MARKER_INSET_VB * marker_w / vb_w * stroke_width
}

/// Shorten path endpoints so the stroke butt-cap hides behind marker bodies.
pub fn shorten_path_for_markers(
    segments: &mut Vec<PathSegment>,
    marker_start: Option<MarkerType>,
    marker_end: Option<MarkerType>,
    stroke_width: f64,
) {
    if let Some(m) = marker_end {
        let inset = marker_inset_px(m, stroke_width);
        if inset > 0.0 {
            shorten_path_end(segments, inset);
        }
    }
    if let Some(m) = marker_start {
        let inset = marker_inset_px(m, stroke_width);
        if inset > 0.0 {
            shorten_path_start(segments, inset);
        }
    }
}

/// Pull the last point of a path back along its incoming tangent by `dist` pixels.
/// If the terminal segment is shorter than `dist`, removes it and continues into
/// the preceding segment (handles short final segments from Bézier interpolation).
fn shorten_path_end(segments: &mut Vec<PathSegment>, mut remaining: f64) {
    while remaining > 0.0 && segments.len() > 1 {
        let n = segments.len();
        // Absorb any short terminal segment (LineTo, CubicTo, QuadTo)
        let (to, prev) = match segments[n - 1] {
            PathSegment::LineTo(to)
            | PathSegment::CubicTo { to, .. }
            | PathSegment::QuadTo { to, .. } => {
                if let Some(prev) = prev_endpoint(&segments[..n - 1]) {
                    (to, prev)
                } else {
                    break;
                }
            }
            _ => break,
        };
        let seg_len = to.distance_to(prev);
        if seg_len <= remaining {
            segments.pop();
            remaining -= seg_len;
            continue;
        }
        // Segment is long enough — pull back within it
        let n = segments.len();
        match &mut segments[n - 1] {
            PathSegment::LineTo(to) => {
                pull_toward(to, prev, remaining);
            }
            PathSegment::CubicTo { cp2, to, .. } => {
                let dir = *cp2;
                pull_toward(to, dir, remaining);
            }
            PathSegment::QuadTo { cp, to, .. } => {
                let dir = *cp;
                pull_toward(to, dir, remaining);
            }
            _ => {}
        }
        break;
    }
}

/// Pull the first point of a path inward along its outgoing tangent by `dist` pixels.
/// Cascades through short leading segments (analogous to `shorten_path_end`).
fn shorten_path_start(segments: &mut Vec<PathSegment>, mut remaining: f64) {
    while remaining > 0.0 && segments.len() > 1 {
        let start = match segments[0] {
            PathSegment::MoveTo(p) => p,
            _ => return,
        };
        // Absorb short second segment (LineTo, CubicTo, QuadTo)
        let seg_endpoint = match segments[1] {
            PathSegment::LineTo(to)
            | PathSegment::CubicTo { to, .. }
            | PathSegment::QuadTo { to, .. } => Some(to),
            _ => None,
        };
        if let Some(to) = seg_endpoint {
            let seg_len = start.distance_to(to);
            if seg_len <= remaining {
                segments[0] = PathSegment::MoveTo(to);
                segments.remove(1);
                remaining -= seg_len;
                continue;
            }
        }
        let toward = match &segments[1] {
            PathSegment::LineTo(to) => *to,
            PathSegment::CubicTo { cp1, .. } => *cp1,
            PathSegment::QuadTo { cp, .. } => *cp,
            _ => return,
        };
        if let PathSegment::MoveTo(start) = &mut segments[0] {
            pull_toward(start, toward, remaining);
        }
        break;
    }
}

pub fn prev_endpoint(segments: &[PathSegment]) -> Option<Point> {
    segments.iter().rev().find_map(|seg| match seg {
        PathSegment::MoveTo(p)
        | PathSegment::LineTo(p)
        | PathSegment::QuadTo { to: p, .. }
        | PathSegment::CubicTo { to: p, .. }
        | PathSegment::ArcTo { to: p, .. } => Some(*p),
        PathSegment::Close => None,
    })
}

/// Move `pt` toward `toward` by `dist` pixels.
fn pull_toward(pt: &mut Point, toward: Point, dist: f64) {
    let dx = toward.x - pt.x;
    let dy = toward.y - pt.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len > dist.abs() && len > 0.0 {
        pt.x += dx / len * dist;
        pt.y += dy / len * dist;
    }
}

#[cfg(test)]
#[path = "rendering_tests.rs"]
mod rendering_tests;
