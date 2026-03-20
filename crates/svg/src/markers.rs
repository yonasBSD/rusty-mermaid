use rusty_mermaid_core::MarkerType;

/// Base SVG ID for a marker type (without color suffix).
fn marker_base_id(marker: MarkerType) -> &'static str {
    match marker {
        MarkerType::ArrowPoint => "arrow-point",
        MarkerType::ArrowBarb => "arrow-barb",
        MarkerType::ArrowOpen => "arrow-open",
        MarkerType::Circle => "marker-circle",
        MarkerType::Cross => "marker-cross",
        MarkerType::Aggregation => "marker-aggregation",
        MarkerType::Composition => "marker-composition",
        MarkerType::Dependency => "marker-dependency",
        _ => "arrow-point",
    }
}

/// Strip leading `#` from a CSS color string for use as an ID suffix.
fn color_suffix(color: &str) -> &str {
    color.strip_prefix('#').unwrap_or(color)
}

/// Return the SVG marker reference ID for a (marker, color) pair.
/// E.g. `"arrow-point-333333"` for ArrowPoint with stroke `"#333333"`.
pub fn marker_id(marker: MarkerType, color: &str) -> String {
    format!("{}-{}", marker_base_id(marker), color_suffix(color))
}

/// Generate all `<marker>` definitions for the given (type, color) pairs.
/// Deduplicates identical pairs.
pub fn marker_defs(markers: &[(MarkerType, String)]) -> String {
    let mut defs = String::new();
    let mut seen: Vec<(MarkerType, &str)> = Vec::new();
    for (m, color) in markers {
        let pair = (*m, color.as_str());
        if seen.contains(&pair) {
            continue;
        }
        seen.push(pair);
        defs.push_str(&marker_def(*m, color));
    }
    defs
}

fn marker_def(marker: MarkerType, color: &str) -> String {
    let id = marker_id(marker, color);
    match marker {
        MarkerType::ArrowPoint => format!(
            r##"<marker id="{id}" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="8" markerHeight="8" orient="auto-start-reverse">
  <path d="M10 5 L0 10 L4 5 L0 0 Z" fill="{color}" />
</marker>
"##
        ),
        MarkerType::ArrowBarb | MarkerType::ArrowOpen => format!(
            r##"<marker id="{id}" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="8" markerHeight="8" orient="auto-start-reverse">
  <path d="M10 5 L0 10 L4 5 L0 0 Z" fill="white" stroke="{color}" stroke-width="1" stroke-linejoin="round" />
</marker>
"##
        ),
        MarkerType::Circle => format!(
            r##"<marker id="{id}" viewBox="0 0 10 10" refX="7" refY="5" markerWidth="8" markerHeight="8" orient="auto-start-reverse">
  <circle cx="5" cy="5" r="4" fill="{color}" />
</marker>
"##
        ),
        MarkerType::Cross => format!(
            r##"<marker id="{id}" viewBox="0 0 10 10" refX="6" refY="5" markerWidth="8" markerHeight="8" orient="auto-start-reverse">
  <path d="M2 2 Q5 4.5 8 8 M8 2 Q5 5.5 2 8" fill="none" stroke="{color}" stroke-width="1.5" stroke-linecap="round" />
</marker>
"##
        ),
        MarkerType::Aggregation => format!(
            r##"<marker id="{id}" viewBox="0 0 12 12" refX="10" refY="6" markerWidth="8" markerHeight="8" orient="auto-start-reverse">
  <path d="M0 6 L6 0 L12 6 L6 12 Z" fill="white" stroke="{color}" stroke-width="1" />
</marker>
"##
        ),
        MarkerType::Composition => format!(
            r##"<marker id="{id}" viewBox="0 0 12 12" refX="10" refY="6" markerWidth="8" markerHeight="8" orient="auto-start-reverse">
  <path d="M0 6 L6 0 L12 6 L6 12 Z" fill="{color}" />
</marker>
"##
        ),
        MarkerType::Dependency => format!(
            r##"<marker id="{id}" viewBox="0 0 10 10" refX="7" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
  <path d="M0 0 L10 5 L0 10" fill="none" stroke="{color}" stroke-width="1.5" />
</marker>
"##
        ),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_id_includes_color() {
        let id = marker_id(MarkerType::ArrowPoint, "#333333");
        assert_eq!(id, "arrow-point-333333");
    }

    #[test]
    fn marker_id_strips_hash() {
        let id = marker_id(MarkerType::Circle, "#ff0000");
        assert_eq!(id, "marker-circle-ff0000");
    }

    #[test]
    fn arrow_point_def() {
        let defs = marker_defs(&[(MarkerType::ArrowPoint, "#333333".into())]);
        assert!(defs.contains(r#"id="arrow-point-333333""#));
        assert!(defs.contains(r##"fill="#333333""##));
    }

    #[test]
    fn multiple_colors_generate_separate_defs() {
        let defs = marker_defs(&[
            (MarkerType::ArrowPoint, "#333333".into()),
            (MarkerType::ArrowPoint, "#ff0000".into()),
        ]);
        assert!(defs.contains(r#"id="arrow-point-333333""#));
        assert!(defs.contains(r#"id="arrow-point-ff0000""#));
    }

    #[test]
    fn deduplicates_same_marker_color() {
        let defs = marker_defs(&[
            (MarkerType::ArrowPoint, "#333".into()),
            (MarkerType::Circle, "#333".into()),
            (MarkerType::ArrowPoint, "#333".into()),
        ]);
        let arrow_count = defs.matches("arrow-point").count();
        assert_eq!(arrow_count, 1, "should deduplicate identical (type, color)");
    }

    #[test]
    fn marker_defs_empty() {
        let defs = marker_defs(&[]);
        assert!(defs.is_empty());
    }
}
