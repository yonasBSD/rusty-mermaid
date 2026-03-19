use rusty_mermaid_core::MarkerType;

/// Return the SVG `<marker>` definition for a marker type.
/// Each marker has a unique ID used in `marker-start`/`marker-end` references.
pub fn marker_id(marker: MarkerType) -> &'static str {
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

/// Generate all marker definitions for the given marker types.
/// `color` is used for fills and strokes (e.g. "#333333" for light, "#6c7086" for dark).
pub fn marker_defs(markers: &[MarkerType], color: &str) -> String {
    let mut defs = String::new();
    let mut seen = Vec::new();
    for &m in markers {
        if seen.contains(&m) {
            continue;
        }
        seen.push(m);
        defs.push_str(&marker_def(m, color));
    }
    defs
}

fn marker_def(marker: MarkerType, color: &str) -> String {
    let id = marker_id(marker);
    match marker {
        MarkerType::ArrowPoint => format!(
            r##"<marker id="{id}" viewBox="0 0 10 10" refX="10" refY="5" markerWidth="8" markerHeight="8" orient="auto-start-reverse">
  <path d="M10 5 L0 10 L4 5 L0 0 Z" fill="{color}" />
</marker>
"##
        ),
        MarkerType::ArrowBarb | MarkerType::ArrowOpen => format!(
            r##"<marker id="{id}" viewBox="0 0 10 10" refX="10" refY="5" markerWidth="8" markerHeight="8" orient="auto-start-reverse">
  <path d="M10 5 L0 10 L4 5 L0 0 Z" fill="white" stroke="{color}" stroke-width="1" stroke-linejoin="round" />
</marker>
"##
        ),
        MarkerType::Circle => format!(
            r##"<marker id="{id}" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="5" markerHeight="5" orient="auto-start-reverse">
  <circle cx="5" cy="5" r="4" fill="{color}" />
</marker>
"##
        ),
        MarkerType::Cross => format!(
            r##"<marker id="{id}" viewBox="0 0 10 10" refX="5" refY="5" markerWidth="8" markerHeight="8" orient="auto-start-reverse">
  <path d="M2 2 Q5 4.5 8 8 M8 2 Q5 5.5 2 8" fill="none" stroke="{color}" stroke-width="1.5" stroke-linecap="round" />
</marker>
"##
        ),
        MarkerType::Aggregation => format!(
            r##"<marker id="{id}" viewBox="0 0 12 12" refX="12" refY="6" markerWidth="8" markerHeight="8" orient="auto-start-reverse">
  <path d="M0 6 L6 0 L12 6 L6 12 Z" fill="white" stroke="{color}" stroke-width="1" />
</marker>
"##
        ),
        MarkerType::Composition => format!(
            r##"<marker id="{id}" viewBox="0 0 12 12" refX="12" refY="6" markerWidth="8" markerHeight="8" orient="auto-start-reverse">
  <path d="M0 6 L6 0 L12 6 L6 12 Z" fill="{color}" />
</marker>
"##
        ),
        MarkerType::Dependency => format!(
            r##"<marker id="{id}" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
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
    fn arrow_point_id() {
        assert_eq!(marker_id(MarkerType::ArrowPoint), "arrow-point");
    }

    #[test]
    fn arrow_point_def() {
        let def = marker_def(MarkerType::ArrowPoint, "#333333");
        assert!(def.contains(r#"id="arrow-point""#));
        assert!(def.contains("M10 5"));
        assert!(def.contains(r##"fill="#333333""##));
    }

    #[test]
    fn circle_marker_def() {
        let def = marker_def(MarkerType::Circle, "#333333");
        assert!(def.contains(r#"id="marker-circle""#));
        assert!(def.contains("<circle"));
    }

    #[test]
    fn marker_defs_deduplicates() {
        let defs = marker_defs(&[MarkerType::ArrowPoint, MarkerType::ArrowPoint, MarkerType::Circle], "#333");
        let count = defs.matches("arrow-point").count();
        assert_eq!(count, 1);
    }

    #[test]
    fn marker_defs_empty() {
        let defs = marker_defs(&[], "#333");
        assert!(defs.is_empty());
    }

    #[test]
    fn marker_def_uses_custom_color() {
        let def = marker_def(MarkerType::ArrowPoint, "#6c7086");
        assert!(def.contains(r##"fill="#6c7086""##));
    }
}
