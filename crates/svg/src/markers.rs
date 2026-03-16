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
pub fn marker_defs(markers: &[MarkerType]) -> String {
    let mut defs = String::new();
    let mut seen = Vec::new();
    for &m in markers {
        if seen.contains(&m) {
            continue;
        }
        seen.push(m);
        defs.push_str(&marker_def(m));
    }
    defs
}

fn marker_def(marker: MarkerType) -> String {
    let id = marker_id(marker);
    match marker {
        MarkerType::ArrowPoint => format!(
            r##"<marker id="{id}" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
  <path d="M0 0 L10 5 L0 10 Z" fill="#333" />
</marker>
"##
        ),
        MarkerType::ArrowBarb | MarkerType::ArrowOpen => format!(
            r##"<marker id="{id}" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
  <path d="M0 0 L10 5 L0 10" fill="none" stroke="#333" stroke-width="1.5" />
</marker>
"##
        ),
        MarkerType::Circle => format!(
            r##"<marker id="{id}" viewBox="0 0 10 10" refX="5" refY="5" markerWidth="5" markerHeight="5">
  <circle cx="5" cy="5" r="4" fill="#333" />
</marker>
"##
        ),
        MarkerType::Cross => format!(
            r##"<marker id="{id}" viewBox="0 0 10 10" refX="5" refY="5" markerWidth="5" markerHeight="5">
  <path d="M2 2 L8 8 M8 2 L2 8" stroke="#333" stroke-width="1.5" />
</marker>
"##
        ),
        MarkerType::Aggregation => format!(
            r##"<marker id="{id}" viewBox="0 0 12 12" refX="12" refY="6" markerWidth="8" markerHeight="8" orient="auto-start-reverse">
  <path d="M0 6 L6 0 L12 6 L6 12 Z" fill="white" stroke="#333" stroke-width="1" />
</marker>
"##
        ),
        MarkerType::Composition => format!(
            r##"<marker id="{id}" viewBox="0 0 12 12" refX="12" refY="6" markerWidth="8" markerHeight="8" orient="auto-start-reverse">
  <path d="M0 6 L6 0 L12 6 L6 12 Z" fill="#333" />
</marker>
"##
        ),
        MarkerType::Dependency => format!(
            r##"<marker id="{id}" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
  <path d="M0 0 L10 5 L0 10" fill="none" stroke="#333" stroke-width="1.5" />
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
        let def = marker_def(MarkerType::ArrowPoint);
        assert!(def.contains(r#"id="arrow-point""#));
        assert!(def.contains("L10 5"));
    }

    #[test]
    fn circle_marker_def() {
        let def = marker_def(MarkerType::Circle);
        assert!(def.contains(r#"id="marker-circle""#));
        assert!(def.contains("<circle"));
    }

    #[test]
    fn marker_defs_deduplicates() {
        let defs = marker_defs(&[MarkerType::ArrowPoint, MarkerType::ArrowPoint, MarkerType::Circle]);
        let count = defs.matches("arrow-point").count();
        assert_eq!(count, 1);
    }

    #[test]
    fn marker_defs_empty() {
        let defs = marker_defs(&[]);
        assert!(defs.is_empty());
    }
}
