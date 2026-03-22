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
mod tests {
    use super::*;

    #[test]
    fn node_style_uses_theme() {
        let theme = Theme::light();
        let s = node_style(&theme);
        assert_eq!(s.fill, Some(theme.node_fill));
        assert_eq!(s.stroke, Some(theme.node_stroke));
        assert_eq!(s.stroke_width, Some(theme.default_stroke_width));
    }

    #[test]
    fn overlay_style_replaces_set_fields() {
        let mut base = Style {
            fill: Some(Color::WHITE),
            stroke: Some(Color::BLACK),
            stroke_width: Some(1.0),
            ..Default::default()
        };
        let custom = Style {
            fill: Some(Color::rgb(255, 0, 0)),
            ..Default::default()
        };
        overlay_style(&mut base, &custom);
        assert_eq!(base.fill, Some(Color::rgb(255, 0, 0)));
        assert_eq!(base.stroke, Some(Color::BLACK)); // unchanged
    }

    #[test]
    fn apply_style_properties_parses_css() {
        let props = vec![
            StyleProperty { key: "fill".into(), value: "#f9f".into() },
            StyleProperty { key: "stroke-width".into(), value: "4px".into() },
            StyleProperty { key: "opacity".into(), value: "0.5".into() },
        ];
        let mut style = Style::default();
        apply_style_properties(&mut style, &props);
        assert!(style.fill.is_some());
        assert_eq!(style.stroke_width, Some(4.0));
        assert_eq!(style.opacity, Some(0.5));
    }

    #[test]
    fn merge_custom_style_with_none() {
        let theme = Theme::light();
        let s = merge_custom_style(None, &theme);
        assert_eq!(s.fill, Some(theme.node_fill));
    }

    #[test]
    fn merge_custom_style_overrides() {
        let theme = Theme::light();
        let custom = Style {
            fill: Some(Color::rgb(255, 0, 0)),
            ..Default::default()
        };
        let s = merge_custom_style(Some(&custom), &theme);
        assert_eq!(s.fill, Some(Color::rgb(255, 0, 0)));
        assert_eq!(s.stroke, Some(theme.node_stroke));
    }

    #[test]
    fn contrasting_label_dark_fill() {
        let theme = Theme::light();
        let lstyle = contrasting_label_style(Some(Color::rgb(20, 20, 20)), &theme);
        assert_eq!(lstyle.fill, Some(Color::WHITE));
    }

    #[test]
    fn contrasting_label_light_fill() {
        let theme = Theme::light();
        let lstyle = contrasting_label_style(Some(Color::rgb(250, 250, 250)), &theme);
        assert_eq!(lstyle.fill, Some(Color::BLACK));
    }

    #[test]
    fn contrasting_label_mid_fill() {
        let theme = Theme::light();
        // rgb(180, 180, 180) has luminance ~0.46 — in the mid range, no override
        let lstyle = contrasting_label_style(Some(Color::rgb(180, 180, 180)), &theme);
        assert_eq!(lstyle.fill, Some(theme.node_text)); // unchanged
    }

    // -- Marker inset & path shortening tests --

    #[test]
    fn marker_inset_all_markers_have_positive_inset() {
        for m in [
            MarkerType::ArrowPoint,
            MarkerType::ArrowBarb,
            MarkerType::ArrowOpen,
            MarkerType::Cross,
            MarkerType::Circle,
            MarkerType::Aggregation,
            MarkerType::Composition,
            MarkerType::Dependency,
        ] {
            assert!(MARKER_INSET_VB > 0.0);
            assert!(marker_inset_px(m, 1.5) > 0.0, "{m:?} normal");
            assert!(marker_inset_px(m, 3.5) > 0.0, "{m:?} thick");
        }
    }

    #[test]
    fn marker_inset_px_scales_with_stroke_width() {
        let normal = marker_inset_px(MarkerType::ArrowPoint, 1.5);
        let thick = marker_inset_px(MarkerType::ArrowPoint, 3.5);
        assert!(thick > normal);
        let ratio = thick / normal;
        assert!((ratio - 3.5 / 1.5).abs() < 0.01);
    }

    #[test]
    fn shorten_path_end_pulls_back_line() {
        let mut segs = vec![
            PathSegment::MoveTo(Point::new(0.0, 0.0)),
            PathSegment::LineTo(Point::new(0.0, 100.0)),
        ];
        shorten_path_end(&mut segs, 10.0);
        if let PathSegment::LineTo(p) = segs[1] {
            assert!((p.y - 90.0).abs() < 0.01);
        } else {
            panic!("expected LineTo");
        }
    }

    #[test]
    fn shorten_path_end_cascades_through_short_segment() {
        let mut segs = vec![
            PathSegment::MoveTo(Point::new(0.0, 0.0)),
            PathSegment::CubicTo {
                cp1: Point::new(0.0, 20.0),
                cp2: Point::new(0.0, 70.0),
                to: Point::new(0.0, 96.0),
            },
            PathSegment::LineTo(Point::new(0.0, 100.0)),
        ];
        shorten_path_end(&mut segs, 6.0);
        assert_eq!(segs.len(), 2);
        if let PathSegment::CubicTo { to, .. } = segs[1] {
            assert!((to.y - 94.0).abs() < 0.1);
        }
    }

    #[test]
    fn shorten_path_start_pulls_forward() {
        let mut segs = vec![
            PathSegment::MoveTo(Point::new(0.0, 0.0)),
            PathSegment::LineTo(Point::new(0.0, 100.0)),
        ];
        shorten_path_start(&mut segs, 10.0);
        if let PathSegment::MoveTo(p) = segs[0] {
            assert!((p.y - 10.0).abs() < 0.01);
        }
    }

    #[test]
    fn shorten_path_start_cascades_through_short_segment() {
        // When the first LineTo is shorter than the requested shortening,
        // absorb it and continue into the next segment.
        let mut segs = vec![
            PathSegment::MoveTo(Point::new(0.0, 0.0)),
            PathSegment::LineTo(Point::new(0.0, 4.0)),
            PathSegment::CubicTo {
                cp1: Point::new(0.0, 20.0),
                cp2: Point::new(0.0, 70.0),
                to: Point::new(0.0, 100.0),
            },
        ];
        shorten_path_start(&mut segs, 6.0);
        // First LineTo (len=4) absorbed, remaining 2.0 pulled into CubicTo
        assert_eq!(segs.len(), 2);
        if let PathSegment::MoveTo(p) = segs[0] {
            // Started at (0,4) after absorbing, pulled 2.0 toward cp1 (0,20)
            assert!((p.y - 6.0).abs() < 0.01);
        } else {
            panic!("expected MoveTo");
        }
    }

    #[test]
    fn shorten_path_end_cascades_through_micro_cubic() {
        // Basis spline interpolation can produce micro CubicTo segments near
        // endpoints. The cascade must absorb these so shortening doesn't silently
        // fail. Regression: resume edge in state_history_in_composite had a
        // 0.07px CubicTo that defeated the entire 3.15px shortening.
        let mut segs = vec![
            PathSegment::MoveTo(Point::new(0.0, 0.0)),
            PathSegment::CubicTo {
                cp1: Point::new(0.0, 30.0),
                cp2: Point::new(0.0, 80.0),
                to: Point::new(0.0, 96.0),
            },
            // Micro cubic: only 0.07px
            PathSegment::CubicTo {
                cp1: Point::new(0.0, 96.03),
                cp2: Point::new(0.0, 96.05),
                to: Point::new(0.0, 96.07),
            },
        ];
        shorten_path_end(&mut segs, 3.15);
        // Micro cubic (0.07px) absorbed, remaining ~3.08px pulled from main cubic
        assert_eq!(segs.len(), 2, "micro cubic should be absorbed");
        let end = prev_endpoint(&segs).unwrap();
        assert!(
            (end.y - (96.0 - 3.08)).abs() < 0.5,
            "endpoint should be pulled back into preceding cubic, got y={:.2}",
            end.y
        );
    }

    #[test]
    fn shorten_path_start_cascades_through_micro_cubic() {
        let mut segs = vec![
            PathSegment::MoveTo(Point::new(0.0, 0.0)),
            // Micro cubic: only 0.05px
            PathSegment::CubicTo {
                cp1: Point::new(0.0, 0.02),
                cp2: Point::new(0.0, 0.04),
                to: Point::new(0.0, 0.05),
            },
            PathSegment::CubicTo {
                cp1: Point::new(0.0, 20.0),
                cp2: Point::new(0.0, 80.0),
                to: Point::new(0.0, 100.0),
            },
        ];
        shorten_path_start(&mut segs, 3.15);
        assert_eq!(segs.len(), 2, "micro cubic should be absorbed");
        if let PathSegment::MoveTo(p) = segs[0] {
            assert!(
                p.y > 3.0,
                "start should be pulled forward past micro cubic, got y={:.2}",
                p.y
            );
        } else {
            panic!("expected MoveTo");
        }
    }

    mod prop_tests {
        use super::*;
        use proptest::prelude::*;

        fn arb_point() -> impl Strategy<Value = Point> {
            (-500.0..500.0_f64, -500.0..500.0_f64)
                .prop_map(|(x, y)| Point::new(x, y))
        }

        fn arb_straight_path(min_len: f64) -> impl Strategy<Value = Vec<PathSegment>> {
            (arb_point(), min_len..500.0_f64).prop_map(|(start, len)| {
                vec![
                    PathSegment::MoveTo(start),
                    PathSegment::LineTo(Point::new(start.x, start.y + len)),
                ]
            })
        }

        fn arb_cascade_path() -> impl Strategy<Value = (Vec<PathSegment>, f64, f64)> {
            (
                arb_point(),
                50.0..200.0_f64,
                1.0..10.0_f64,
                0.01..0.9_f64,
            )
                .prop_map(|(start, span, short_len, extra_frac)| {
                    let cubic_tangent = span * 0.1;
                    let cubic_end_y = start.y + span;
                    let final_y = cubic_end_y + short_len;
                    let dist = short_len + extra_frac * cubic_tangent;
                    let segs = vec![
                        PathSegment::MoveTo(start),
                        PathSegment::CubicTo {
                            cp1: Point::new(start.x, start.y + span * 0.3),
                            cp2: Point::new(start.x, cubic_end_y - cubic_tangent),
                            to: Point::new(start.x, cubic_end_y),
                        },
                        PathSegment::LineTo(Point::new(start.x, final_y)),
                    ];
                    (segs, dist, final_y)
                })
        }

        proptest! {
            #[test]
            fn shorten_end_distance_exact(
                dist in 1.0..50.0_f64,
                path in arb_straight_path(51.0),
            ) {
                let mut segs = path.clone();
                let orig_end = prev_endpoint(&segs).unwrap();
                shorten_path_end(&mut segs, dist);
                let new_end = prev_endpoint(&segs).unwrap();
                let actual = orig_end.distance_to(new_end);
                prop_assert!((actual - dist).abs() < 0.01);
            }

            #[test]
            fn shorten_end_stays_collinear(
                dist in 1.0..50.0_f64,
                path in arb_straight_path(51.0),
            ) {
                let mut segs = path.clone();
                let start = match segs[0] { PathSegment::MoveTo(p) => p, _ => unreachable!() };
                let orig_end = prev_endpoint(&segs).unwrap();
                shorten_path_end(&mut segs, dist);
                let new_end = prev_endpoint(&segs).unwrap();
                let cross = ((orig_end.x - start.x) * (new_end.y - start.y)
                           - (orig_end.y - start.y) * (new_end.x - start.x)).abs();
                prop_assert!(cross < 0.01);
            }

            #[test]
            fn shorten_end_never_overshoots(
                dist in 1.0..50.0_f64,
                path in arb_straight_path(51.0),
            ) {
                let mut segs = path.clone();
                let start = match segs[0] { PathSegment::MoveTo(p) => p, _ => unreachable!() };
                let orig_end = prev_endpoint(&segs).unwrap();
                shorten_path_end(&mut segs, dist);
                let new_end = prev_endpoint(&segs).unwrap();
                prop_assert!(start.distance_to(new_end) < start.distance_to(orig_end));
                prop_assert!(start.distance_to(new_end) > 0.0);
            }

            #[test]
            fn shorten_start_distance_exact(
                dist in 1.0..50.0_f64,
                path in arb_straight_path(51.0),
            ) {
                let mut segs = path.clone();
                let orig = match segs[0] { PathSegment::MoveTo(p) => p, _ => unreachable!() };
                shorten_path_start(&mut segs, dist);
                let new = match segs[0] { PathSegment::MoveTo(p) => p, _ => unreachable!() };
                prop_assert!((orig.distance_to(new) - dist).abs() < 0.01);
            }

            #[test]
            fn cascade_retraction_correct(
                (mut segs, dist, orig_final_y) in arb_cascade_path(),
            ) {
                let orig_end = Point::new(
                    match segs[0] { PathSegment::MoveTo(p) => p.x, _ => unreachable!() },
                    orig_final_y,
                );
                shorten_path_end(&mut segs, dist);
                let new_end = prev_endpoint(&segs).unwrap();
                let actual = orig_end.distance_to(new_end);
                prop_assert!((actual - dist).abs() < 1.5);
            }

            #[test]
            fn inset_scales_linearly(
                sw1 in 0.5..5.0_f64,
                sw2 in 0.5..5.0_f64,
            ) {
                let r1 = marker_inset_px(MarkerType::ArrowPoint, sw1) / sw1;
                let r2 = marker_inset_px(MarkerType::ArrowPoint, sw2) / sw2;
                prop_assert!((r1 - r2).abs() < 0.001);
            }

            #[test]
            fn bidir_shortening_independent(
                dist in 1.0..20.0_f64,
            ) {
                let mut segs = vec![
                    PathSegment::MoveTo(Point::new(0.0, 0.0)),
                    PathSegment::LineTo(Point::new(0.0, 100.0)),
                ];
                shorten_path_start(&mut segs, dist);
                shorten_path_end(&mut segs, dist);
                let s = match segs[0] { PathSegment::MoveTo(p) => p, _ => unreachable!() };
                let e = prev_endpoint(&segs).unwrap();
                prop_assert!((s.distance_to(e) - (100.0 - 2.0 * dist)).abs() < 0.01);
            }
        }
    }
}
