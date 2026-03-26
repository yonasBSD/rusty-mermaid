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

    if scene.width <= 0.0 || scene.height <= 0.0 {
        return false;
    }

    for elem in scene.elements() {
        match &elem.primitive {
            Primitive::Rect { bbox, .. } => {
                if !bbox.x.is_finite() || !bbox.y.is_finite() {
                    return false;
                }
                if !bbox.width.is_finite() || !bbox.height.is_finite() {
                    return false;
                }
            }
            Primitive::Text { position, .. } => {
                if !position.x.is_finite() || !position.y.is_finite() {
                    return false;
                }
            }
            Primitive::Circle { center, radius, .. } => {
                if !center.x.is_finite() || !center.y.is_finite() || !radius.is_finite() {
                    return false;
                }
            }
            Primitive::Path { segments, .. } => {
                for seg in segments {
                    let pts: Vec<&rusty_mermaid_core::Point> = match seg {
                        rusty_mermaid_core::PathSegment::MoveTo(p) => vec![p],
                        rusty_mermaid_core::PathSegment::LineTo(p) => vec![p],
                        rusty_mermaid_core::PathSegment::CubicTo { cp1, cp2, to } => {
                            vec![cp1, cp2, to]
                        }
                        rusty_mermaid_core::PathSegment::QuadTo { cp, to } => vec![cp, to],
                        _ => vec![],
                    };
                    for p in pts {
                        if !p.x.is_finite() || !p.y.is_finite() {
                            return false;
                        }
                    }
                }
            }
            Primitive::Polygon { points, .. } => {
                for p in points {
                    if !p.x.is_finite() || !p.y.is_finite() {
                        return false;
                    }
                }
            }
            _ => {}
        }
    }
    true
}

/// Generate a random label (1-20 alphanumeric chars, may include spaces)
fn arb_label() -> impl Strategy<Value = String> {
    "[A-Za-z][A-Za-z0-9 ]{0,15}".prop_map(|s| s.trim().to_string())
}

/// Generate a random identifier (no spaces — for state names, class IDs, etc.)
fn arb_id() -> impl Strategy<Value = String> {
    "[A-Za-z][A-Za-z0-9]{0,10}"
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

    #[test]
    fn flowchart_random(
        nodes in prop::collection::vec(arb_label(), 2..6),
    ) {
        let mut input = "flowchart TD\n".to_string();
        for (i, label) in nodes.iter().enumerate() {
            input.push_str(&format!("  n{}[\"{}\"]\n", i, label));
        }
        for i in 0..nodes.len().saturating_sub(1) {
            input.push_str(&format!("  n{} --> n{}\n", i, i + 1));
        }
        prop_assert!(scene_is_valid(&input), "invalid scene for flowchart");
    }

    #[test]
    fn state_random(
        states in prop::collection::vec(arb_id(), 2..5),
    ) {
        let mut input = "stateDiagram-v2\n".to_string();
        for s in &states {
            input.push_str(&format!("  {}\n", s));
        }
        for i in 0..states.len().saturating_sub(1) {
            input.push_str(&format!("  {} --> {}\n", states[i], states[i + 1]));
        }
        prop_assert!(scene_is_valid(&input), "invalid scene for state");
    }

    #[test]
    fn sequence_random(
        actors in prop::collection::vec(arb_id(), 2..5),
        n_messages in 1usize..5,
    ) {
        let mut input = "sequenceDiagram\n".to_string();
        for a in &actors {
            input.push_str(&format!("  participant {}\n", a));
        }
        for i in 0..n_messages.min(actors.len().saturating_sub(1)) {
            input.push_str(&format!("  {} ->> {}: msg{}\n", actors[i], actors[i + 1], i));
        }
        prop_assert!(scene_is_valid(&input), "invalid scene for sequence");
    }

    #[test]
    fn class_random(
        classes in prop::collection::vec(arb_id(), 2..5),
    ) {
        let mut input = "classDiagram\n".to_string();
        for c in &classes {
            input.push_str(&format!("  class {}\n", c));
        }
        if classes.len() >= 2 {
            input.push_str(&format!("  {} --> {}\n", classes[0], classes[1]));
        }
        prop_assert!(scene_is_valid(&input), "invalid scene for class");
    }

    #[test]
    fn er_random(
        entities in prop::collection::vec(arb_id(), 2..5),
    ) {
        let mut input = "erDiagram\n".to_string();
        if entities.len() >= 2 {
            input.push_str(&format!("  {} ||--o{{ {} : has\n", entities[0], entities[1]));
        }
        prop_assert!(scene_is_valid(&input), "invalid scene for er");
    }

    #[test]
    fn requirement_random(
        reqs in prop::collection::vec(arb_id(), 1..4),
    ) {
        let mut input = "requirementDiagram\n".to_string();
        for r in &reqs {
            input.push_str(&format!("  requirement {} {{\n    id: {}\n    text: test\n  }}\n", r, r));
        }
        if reqs.len() >= 2 {
            input.push_str(&format!("  {} - contains -> {}\n", reqs[0], reqs[1]));
        }
        prop_assert!(scene_is_valid(&input), "invalid scene for requirement");
    }

    #[test]
    fn timeline_random(
        sections in prop::collection::vec(arb_label(), 1..4),
        events in prop::collection::vec(arb_label(), 1..4),
    ) {
        let mut input = "timeline\n".to_string();
        for (si, section) in sections.iter().enumerate() {
            input.push_str(&format!("  section {}\n", section));
            let ei = si % events.len();
            input.push_str(&format!("    {} : {}\n", events[ei], section));
        }
        prop_assert!(scene_is_valid(&input), "invalid scene for timeline");
    }

    #[test]
    fn gantt_random(
        tasks in prop::collection::vec(arb_label(), 1..5),
    ) {
        let mut input = "gantt\n  dateFormat YYYY-MM-DD\n  section Work\n".to_string();
        for (i, task) in tasks.iter().enumerate() {
            input.push_str(&format!("    {} : t{}, 2024-01-{:02}, 3d\n", task, i, (i % 28) + 1));
        }
        prop_assert!(scene_is_valid(&input), "invalid scene for gantt");
    }

    #[test]
    fn gitgraph_random(n_commits in 2usize..8) {
        let mut input = "gitGraph\n".to_string();
        for _ in 0..n_commits {
            input.push_str("  commit\n");
        }
        prop_assert!(scene_is_valid(&input), "invalid scene for gitgraph");
    }

    #[test]
    fn ishikawa_random(
        effect in arb_label(),
        causes in prop::collection::vec(arb_label(), 2..5),
        subcauses in prop::collection::vec(arb_label(), 1..3),
    ) {
        let mut input = format!("ishikawa-beta\n    {}\n", effect);
        for cause in &causes {
            input.push_str(&format!("    {}\n", cause));
            for sc in &subcauses {
                input.push_str(&format!("        {}\n", sc));
            }
        }
        prop_assert!(scene_is_valid(&input), "invalid scene for ishikawa");
    }

    #[test]
    fn kanban_random(
        columns in prop::collection::vec(arb_label(), 1..4),
        cards in prop::collection::vec(arb_label(), 1..4),
    ) {
        let mut input = "kanban\n".to_string();
        for (ci, col) in columns.iter().enumerate() {
            input.push_str(&format!("    {}\n", col));
            let card_idx = ci % cards.len();
            input.push_str(&format!("        id{}[{}]\n", ci, cards[card_idx]));
        }
        prop_assert!(scene_is_valid(&input), "invalid scene for kanban");
    }

    #[test]
    fn architecture_random(
        services in prop::collection::vec(arb_id(), 2..5),
    ) {
        let mut input = "architecture-beta\n".to_string();
        for s in &services {
            input.push_str(&format!("  service {}(server)[{}]\n", s, s));
        }
        if services.len() >= 2 {
            input.push_str(&format!("  {}:R -- L:{}\n", services[0], services[1]));
        }
        prop_assert!(scene_is_valid(&input), "invalid scene for architecture");
    }

    #[test]
    fn c4_random(
        elements in prop::collection::vec(arb_id(), 1..4),
    ) {
        let mut input = "C4Context\n".to_string();
        for e in &elements {
            input.push_str(&format!("  System({e}, \"{e}\", \"desc\")\n"));
        }
        if elements.len() >= 2 {
            input.push_str(&format!("  Rel({}, {}, \"uses\")\n", elements[0], elements[1]));
        }
        prop_assert!(scene_is_valid(&input), "invalid scene for c4");
    }

    #[test]
    fn xychart_random(
        values in prop::collection::vec(1.0f64..100.0, 3..8),
    ) {
        let labels: Vec<String> = (0..values.len()).map(|i| format!("L{}", i)).collect();
        let val_str: Vec<String> = values.iter().map(|v| format!("{:.0}", v)).collect();
        let input = format!(
            "xychart-beta\n  x-axis [{}]\n  y-axis 0 --> 100\n  bar [{}]\n",
            labels.join(", "), val_str.join(", ")
        );
        prop_assert!(scene_is_valid(&input), "invalid scene for xychart");
    }
}
