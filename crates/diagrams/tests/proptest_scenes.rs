//! Property tests: random diagram inputs → Scene invariants hold.
//!
//! For each diagram type, generates random valid-ish inputs and checks
//! that render_to_scene produces a valid Scene (finite coords, positive dims).

use proptest::prelude::*;
use rusty_mermaid_core::Primitive;
use rusty_mermaid_diagrams::render_to_scene;

fn scene_is_valid(input: &str) -> bool {
    let scene = match render_to_scene(input) {
        Ok(s) => s,
        Err(_) => return true, // parse errors are fine for random input
    };

    if scene.width <= 0.0 || scene.height <= 0.0 { return false; }

    for elem in scene.elements() {
        match &elem.primitive {
            Primitive::Rect { bbox, .. } => {
                if !bbox.x.is_finite() || !bbox.y.is_finite() { return false; }
                if !bbox.width.is_finite() || !bbox.height.is_finite() { return false; }
            }
            Primitive::Text { position, .. } => {
                if !position.x.is_finite() || !position.y.is_finite() { return false; }
            }
            Primitive::Circle { center, radius, .. } => {
                if !center.x.is_finite() || !center.y.is_finite() || !radius.is_finite() { return false; }
            }
            Primitive::Path { segments, .. } => {
                for seg in segments {
                    let pts: Vec<&rusty_mermaid_core::Point> = match seg {
                        rusty_mermaid_core::PathSegment::MoveTo(p) => vec![p],
                        rusty_mermaid_core::PathSegment::LineTo(p) => vec![p],
                        rusty_mermaid_core::PathSegment::CubicTo { cp1, cp2, to } => vec![cp1, cp2, to],
                        rusty_mermaid_core::PathSegment::QuadTo { cp, to } => vec![cp, to],
                        _ => vec![],
                    };
                    for p in pts {
                        if !p.x.is_finite() || !p.y.is_finite() { return false; }
                    }
                }
            }
            Primitive::Polygon { points, .. } => {
                for p in points {
                    if !p.x.is_finite() || !p.y.is_finite() { return false; }
                }
            }
            _ => {}
        }
    }
    true
}

/// Generate a random label (1-20 alphanumeric chars)
fn arb_label() -> impl Strategy<Value = String> {
    "[A-Za-z][A-Za-z0-9 ]{0,15}".prop_map(|s| s.trim().to_string())
}

/// Generate a random positive number
fn arb_value() -> impl Strategy<Value = f64> {
    (1.0f64..1000.0)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))]

    #[test]
    fn pie_random(
        title in arb_label(),
        labels in prop::collection::vec(arb_label(), 2..6),
        values in prop::collection::vec(arb_value(), 2..6),
    ) {
        let n = labels.len().min(values.len());
        let mut input = format!("pie\n    title {}\n", title);
        for i in 0..n {
            input.push_str(&format!("    \"{}\" : {:.0}\n", labels[i], values[i]));
        }
        prop_assert!(scene_is_valid(&input), "invalid scene for pie: {}", input);
    }

    #[test]
    fn sankey_random(
        sources in prop::collection::vec(arb_label(), 2..5),
        targets in prop::collection::vec(arb_label(), 2..5),
        values in prop::collection::vec(arb_value(), 4..10),
    ) {
        let mut input = "sankey-beta\n".to_string();
        for (i, v) in values.iter().enumerate() {
            let s = &sources[i % sources.len()];
            let t = &targets[i % targets.len()];
            if s != t {
                input.push_str(&format!("{},{},{:.0}\n", s, t, v));
            }
        }
        prop_assert!(scene_is_valid(&input), "invalid scene for sankey");
    }

    #[test]
    fn packet_random(
        n_fields in 2usize..8,
        widths in prop::collection::vec(1usize..16, 2..8),
    ) {
        let mut input = "packet-beta\n".to_string();
        let mut bit = 0;
        for i in 0..n_fields.min(widths.len()) {
            let w = widths[i];
            input.push_str(&format!("+{}: \"Field {}\"\n", w, i));
            bit += w;
        }
        let _ = bit;
        prop_assert!(scene_is_valid(&input), "invalid scene for packet");
    }

    #[test]
    fn quadrant_random(
        points in prop::collection::vec((arb_label(), 0.0f64..1.0, 0.0f64..1.0), 1..8),
    ) {
        let mut input = "quadrantChart\n".to_string();
        for (label, x, y) in &points {
            input.push_str(&format!("  {}: [{:.2}, {:.2}]\n", label, x, y));
        }
        prop_assert!(scene_is_valid(&input), "invalid scene for quadrant");
    }

    #[test]
    fn radar_random(
        axes in prop::collection::vec(arb_label(), 3..7),
        curve_vals in prop::collection::vec(1.0f64..10.0, 3..7),
    ) {
        let n = axes.len().min(curve_vals.len());
        let axis_str = axes[..n].join(",");
        let vals: Vec<String> = curve_vals[..n].iter().map(|v| format!("{:.0}", v)).collect();
        let input = format!("radar-beta\naxis {}\ncurve data{{{}}}\n", axis_str, vals.join(","));
        prop_assert!(scene_is_valid(&input), "invalid scene for radar");
    }

    #[test]
    fn mindmap_random(
        root in arb_label(),
        children in prop::collection::vec(arb_label(), 1..6),
    ) {
        let mut input = format!("mindmap\n    {}\n", root);
        for child in &children {
            input.push_str(&format!("        {}\n", child));
        }
        prop_assert!(scene_is_valid(&input), "invalid scene for mindmap");
    }

    #[test]
    fn venn_random(
        sets in prop::collection::vec((arb_label(), arb_value()), 2..4),
    ) {
        let mut input = "venn-beta\n".to_string();
        for (label, size) in &sets {
            input.push_str(&format!("  set {}:{:.0}\n", label, size));
        }
        prop_assert!(scene_is_valid(&input), "invalid scene for venn");
    }

    #[test]
    fn journey_random(
        section in arb_label(),
        tasks in prop::collection::vec((arb_label(), 1u8..6), 1..5),
    ) {
        let mut input = format!("journey\n  section {}\n", section);
        for (name, score) in &tasks {
            input.push_str(&format!("    {}: {}\n", name, score));
        }
        prop_assert!(scene_is_valid(&input), "invalid scene for journey");
    }

    #[test]
    fn treeview_random(
        root in arb_label(),
        children in prop::collection::vec(arb_label(), 1..6),
    ) {
        let mut input = format!("treeView-beta\n    {}\n", root);
        for child in &children {
            input.push_str(&format!("        {}\n", child));
        }
        prop_assert!(scene_is_valid(&input), "invalid scene for treeview");
    }

    #[test]
    fn treemap_random(
        items in prop::collection::vec((arb_label(), arb_value()), 2..8),
    ) {
        let mut input = "treemap\n".to_string();
        for (name, val) in &items {
            input.push_str(&format!("    {}: {:.0}\n", name, val));
        }
        prop_assert!(scene_is_valid(&input), "invalid scene for treemap");
    }

    #[test]
    fn block_random(
        n_blocks in 2usize..6,
        labels in prop::collection::vec(arb_label(), 2..6),
    ) {
        let mut input = "block-beta\n  columns 3\n".to_string();
        for i in 0..n_blocks.min(labels.len()) {
            input.push_str(&format!("  b{}[\"{}\"]\n", i, labels[i]));
        }
        prop_assert!(scene_is_valid(&input), "invalid scene for block");
    }
}
