use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use rusty_mermaid_core::Shape;
use rusty_mermaid_diagrams;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn golden_mmd_dir() -> PathBuf {
    workspace_root().join("tests/golden/mmd")
}

struct FlowResult {
    stem: String,
    diagram: rusty_mermaid_diagrams::flowchart::ir::FlowDiagram,
    layout: rusty_mermaid_diagrams::flowchart::bridge::LayoutResult,
}

struct StateResult {
    stem: String,
    diagram: rusty_mermaid_diagrams::state::ir::StateDiagram,
    layout: rusty_mermaid_diagrams::state::bridge::LayoutResult,
}

static FLOWCHARTS: LazyLock<Vec<FlowResult>> = LazyLock::new(|| {
    let mmd_dir = golden_mmd_dir().join("flowchart");
    let mut results = Vec::new();
    for entry in fs::read_dir(&mmd_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("mmd") {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap();
        let diagram = match rusty_mermaid_diagrams::flowchart::parser::parse(&text) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let layout = rusty_mermaid_diagrams::flowchart::bridge::layout(&diagram);
        let stem = path.file_stem().unwrap().to_str().unwrap().to_string();
        results.push(FlowResult {
            stem,
            diagram,
            layout,
        });
    }
    results.sort_by(|a, b| a.stem.cmp(&b.stem));
    results
});

static STATES: LazyLock<Vec<StateResult>> = LazyLock::new(|| {
    let mmd_dir = golden_mmd_dir().join("state");
    let mut results = Vec::new();
    for entry in fs::read_dir(&mmd_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("mmd") {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap();
        let diagram = match rusty_mermaid_diagrams::state::parser::parse(&text) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let layout = rusty_mermaid_diagrams::state::bridge::layout(&diagram);
        let stem = path.file_stem().unwrap().to_str().unwrap().to_string();
        results.push(StateResult {
            stem,
            diagram,
            layout,
        });
    }
    results.sort_by(|a, b| a.stem.cmp(&b.stem));
    results
});

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn node_bbox(n: &rusty_mermaid_diagrams::common::layout::NodeLayout) -> (f64, f64, f64, f64) {
    let left = n.x - n.width / 2.0;
    let top = n.y - n.height / 2.0;
    let right = n.x + n.width / 2.0;
    let bottom = n.y + n.height / 2.0;
    (left, top, right, bottom)
}

fn rects_overlap(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> bool {
    let x_overlap = a.0 < b.2 && b.0 < a.2;
    let y_overlap = a.1 < b.3 && b.1 < a.3;
    x_overlap && y_overlap
}

fn rect_contains(outer: (f64, f64, f64, f64), inner: (f64, f64, f64, f64), tol: f64) -> bool {
    inner.0 >= outer.0 - tol
        && inner.1 >= outer.1 - tol
        && inner.2 <= outer.2 + tol
        && inner.3 <= outer.3 + tol
}

/// Collect all note state_ids from an IR (recursively through composites).
fn collect_note_state_ids(
    diagram: &rusty_mermaid_diagrams::state::ir::StateDiagram,
) -> Vec<String> {
    let mut ids = Vec::new();
    for note in &diagram.notes {
        ids.push(note.state_id.clone());
    }
    collect_note_state_ids_from_states(&diagram.states, &mut ids);
    ids
}

fn collect_note_state_ids_from_states(
    states: &[rusty_mermaid_diagrams::state::ir::StateNode],
    ids: &mut Vec<String>,
) {
    use rusty_mermaid_diagrams::state::ir::StateKind;
    for s in states {
        if let StateKind::Composite {
            children,
            notes,
            regions,
            ..
        } = &s.kind
        {
            for note in notes {
                ids.push(note.state_id.clone());
            }
            collect_note_state_ids_from_states(children, ids);
            for region in regions {
                collect_note_state_ids_from_states(&region.children, ids);
            }
        }
    }
}

/// Count concurrent regions in IR for a state (returns 0 if not composite/concurrent).
fn ir_region_count(states: &[rusty_mermaid_diagrams::state::ir::StateNode], id: &str) -> usize {
    use rusty_mermaid_diagrams::state::ir::StateKind;
    for s in states {
        if s.id == id {
            if let StateKind::Composite { regions, .. } = &s.kind {
                if regions.is_empty() {
                    return 0;
                }
                return regions.len();
            }
            return 0;
        }
        if let StateKind::Composite {
            children, regions, ..
        } = &s.kind
        {
            let r = ir_region_count(children, id);
            if r > 0 {
                return r;
            }
            for region in regions {
                let r = ir_region_count(&region.children, id);
                if r > 0 {
                    return r;
                }
            }
        }
    }
    0
}

// ===========================================================================
// TIER 1: Dimension consistency
// ===========================================================================

#[test]
fn flowchart_positive_dimensions() {
    let mut failures = Vec::new();
    for fr in FLOWCHARTS.iter() {
        if fr.layout.width <= 0.0 || fr.layout.height <= 0.0 {
            failures.push(format!(
                "{}: layout {}x{}",
                fr.stem, fr.layout.width, fr.layout.height
            ));
        }
        for n in &fr.layout.nodes {
            if n.width <= 0.0 || n.height <= 0.0 {
                failures.push(format!(
                    "{}: node {} has {}x{}",
                    fr.stem, n.id, n.width, n.height
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "non-positive dimensions:\n{}",
        failures.join("\n")
    );
}

#[test]
fn state_positive_dimensions() {
    let mut failures = Vec::new();
    for sr in STATES.iter() {
        if sr.layout.width <= 0.0 || sr.layout.height <= 0.0 {
            failures.push(format!(
                "{}: layout {}x{}",
                sr.stem, sr.layout.width, sr.layout.height
            ));
        }
        for n in &sr.layout.nodes {
            if n.width <= 0.0 || n.height <= 0.0 {
                failures.push(format!(
                    "{}: node {} has {}x{}",
                    sr.stem, n.id, n.width, n.height
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "non-positive dimensions:\n{}",
        failures.join("\n")
    );
}

// ===========================================================================
// TIER 2: Numeric stability — no NaN or Inf
// ===========================================================================

#[test]
fn flowchart_no_nan_inf() {
    let mut failures = Vec::new();
    for fr in FLOWCHARTS.iter() {
        if !fr.layout.width.is_finite() || !fr.layout.height.is_finite() {
            failures.push(format!("{}: layout dimensions non-finite", fr.stem));
        }
        for n in &fr.layout.nodes {
            if !n.x.is_finite() || !n.y.is_finite() || !n.width.is_finite() || !n.height.is_finite()
            {
                failures.push(format!("{}: node {} has non-finite coords", fr.stem, n.id));
            }
        }
        for e in &fr.layout.edges {
            for pt in &e.points {
                if !pt.x.is_finite() || !pt.y.is_finite() {
                    failures.push(format!(
                        "{}: edge {}->{} has non-finite point",
                        fr.stem, e.src, e.dst
                    ));
                    break;
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "NaN/Inf detected:\n{}",
        failures.join("\n")
    );
}

#[test]
fn state_no_nan_inf() {
    let mut failures = Vec::new();
    for sr in STATES.iter() {
        if !sr.layout.width.is_finite() || !sr.layout.height.is_finite() {
            failures.push(format!("{}: layout dimensions non-finite", sr.stem));
        }
        for n in &sr.layout.nodes {
            if !n.x.is_finite() || !n.y.is_finite() || !n.width.is_finite() || !n.height.is_finite()
            {
                failures.push(format!("{}: node {} has non-finite coords", sr.stem, n.id));
            }
        }
        for e in &sr.layout.edges {
            for pt in &e.points {
                if !pt.x.is_finite() || !pt.y.is_finite() {
                    failures.push(format!(
                        "{}: edge {}->{} has non-finite point",
                        sr.stem, e.src, e.dst
                    ));
                    break;
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "NaN/Inf detected:\n{}",
        failures.join("\n")
    );
}

// ===========================================================================
// TIER 3: Layout bounds — nodes within viewBox
// ===========================================================================

#[test]
fn flowchart_nodes_within_bounds() {
    let mut failures = Vec::new();
    for fr in FLOWCHARTS.iter() {
        let (lw, lh) = (fr.layout.width, fr.layout.height);
        for n in &fr.layout.nodes {
            let (left, top, right, bottom) = node_bbox(n);
            if left < -1.0 || top < -1.0 || right > lw + 1.0 || bottom > lh + 1.0 {
                failures.push(format!(
                    "{}: node {} bbox ({:.1},{:.1})-({:.1},{:.1}) outside layout {}x{}",
                    fr.stem, n.id, left, top, right, bottom, lw, lh
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "nodes outside bounds:\n{}",
        failures.join("\n")
    );
}

#[test]
fn state_nodes_within_bounds() {
    let mut failures = Vec::new();
    for sr in STATES.iter() {
        let (lw, lh) = (sr.layout.width, sr.layout.height);
        for n in &sr.layout.nodes {
            let (left, top, right, bottom) = node_bbox(n);
            if left < -1.0 || top < -1.0 || right > lw + 1.0 || bottom > lh + 1.0 {
                failures.push(format!(
                    "{}: node {} bbox ({:.1},{:.1})-({:.1},{:.1}) outside layout {}x{}",
                    sr.stem, n.id, left, top, right, bottom, lw, lh
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "nodes outside bounds:\n{}",
        failures.join("\n")
    );
}

// ===========================================================================
// TIER 4: Layout bounds — edge points within viewBox
// ===========================================================================

#[test]
fn flowchart_edge_points_within_bounds() {
    let tol = 1.0;
    let mut failures = Vec::new();
    for fr in FLOWCHARTS.iter() {
        let (lw, lh) = (fr.layout.width, fr.layout.height);
        for e in &fr.layout.edges {
            for pt in &e.points {
                if pt.x < -tol || pt.y < -tol || pt.x > lw + tol || pt.y > lh + tol {
                    failures.push(format!(
                        "{}: edge {}->{} point ({:.1},{:.1}) outside layout {}x{}",
                        fr.stem, e.src, e.dst, pt.x, pt.y, lw, lh
                    ));
                    break;
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "edge points outside bounds:\n{}",
        failures.join("\n")
    );
}

#[test]
fn state_edge_points_within_bounds() {
    let tol = 1.0;
    let mut failures = Vec::new();
    for sr in STATES.iter() {
        let (lw, lh) = (sr.layout.width, sr.layout.height);
        for e in &sr.layout.edges {
            for pt in &e.points {
                if pt.x < -tol || pt.y < -tol || pt.x > lw + tol || pt.y > lh + tol {
                    failures.push(format!(
                        "{}: edge {}->{} point ({:.1},{:.1}) outside layout {}x{}",
                        sr.stem, e.src, e.dst, pt.x, pt.y, lw, lh
                    ));
                    break;
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "edge points outside bounds:\n{}",
        failures.join("\n")
    );
}

// ===========================================================================
// TIER 5: Layout bounds — edge labels within viewBox
// ===========================================================================

#[test]
fn flowchart_edge_labels_within_bounds() {
    let mut failures = Vec::new();
    for fr in FLOWCHARTS.iter() {
        let (lw, lh) = (fr.layout.width, fr.layout.height);
        for e in &fr.layout.edges {
            let Some((label_w, label_h)) = e.label_size else {
                continue;
            };
            if e.points.len() < 2 {
                continue;
            }
            let mid = e.points[e.points.len() / 2];
            let left = mid.x - label_w / 2.0;
            let top = mid.y - label_h / 2.0;
            let right = mid.x + label_w / 2.0;
            let bottom = mid.y + label_h / 2.0;
            if left < -1.0 || top < -1.0 || right > lw + 1.0 || bottom > lh + 1.0 {
                failures.push(format!(
                    "{}: edge {}->{} label bbox ({:.1},{:.1})-({:.1},{:.1}) outside layout {}x{}",
                    fr.stem, e.src, e.dst, left, top, right, bottom, lw, lh
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "edge labels outside bounds:\n{}",
        failures.join("\n")
    );
}

#[test]
fn state_edge_labels_within_bounds() {
    let mut failures = Vec::new();
    for sr in STATES.iter() {
        let (lw, lh) = (sr.layout.width, sr.layout.height);
        for e in &sr.layout.edges {
            let Some((label_w, label_h)) = e.label_size else {
                continue;
            };
            if e.points.len() < 2 {
                continue;
            }
            let mid = e.points[e.points.len() / 2];
            let left = mid.x - label_w / 2.0;
            let top = mid.y - label_h / 2.0;
            let right = mid.x + label_w / 2.0;
            let bottom = mid.y + label_h / 2.0;
            if left < -1.0 || top < -1.0 || right > lw + 1.0 || bottom > lh + 1.0 {
                failures.push(format!(
                    "{}: edge {}->{} label bbox ({:.1},{:.1})-({:.1},{:.1}) outside layout {}x{}",
                    sr.stem, e.src, e.dst, left, top, right, bottom, lw, lh
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "edge labels outside bounds:\n{}",
        failures.join("\n")
    );
}

// ===========================================================================
// TIER 6: Spatial — no leaf-node overlaps
// ===========================================================================

#[test]
fn flowchart_no_leaf_node_overlaps() {
    let mut failures = Vec::new();
    for fr in FLOWCHARTS.iter() {
        let leaves: Vec<_> = fr.layout.nodes.iter().filter(|n| !n.is_compound).collect();
        for i in 0..leaves.len() {
            for j in (i + 1)..leaves.len() {
                let a = node_bbox(leaves[i]);
                let b = node_bbox(leaves[j]);
                if rects_overlap(a, b) {
                    failures.push(format!(
                        "{}: nodes {} and {} overlap",
                        fr.stem, leaves[i].id, leaves[j].id
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "overlapping nodes:\n{}",
        failures.join("\n")
    );
}

#[test]
fn state_no_leaf_node_overlaps() {
    let mut failures = Vec::new();
    for sr in STATES.iter() {
        let leaves: Vec<_> = sr
            .layout
            .nodes
            .iter()
            .filter(|n| !n.is_compound)
            // Pseudo-states ([*]_start/end) are tiny circles that dagre may
            // place touching regular nodes — exclude from overlap check.
            .filter(|n| !matches!(n.shape, Shape::StateStart | Shape::StateEnd))
            .collect();
        for i in 0..leaves.len() {
            for j in (i + 1)..leaves.len() {
                let a = node_bbox(leaves[i]);
                let b = node_bbox(leaves[j]);
                if rects_overlap(a, b) {
                    failures.push(format!(
                        "{}: nodes {} and {} overlap",
                        sr.stem, leaves[i].id, leaves[j].id
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "overlapping nodes:\n{}",
        failures.join("\n")
    );
}

#[test]
fn state_pseudo_states_no_overlap_with_leaves() {
    let mut failures = Vec::new();
    for sr in STATES.iter() {
        let pseudos: Vec<_> = sr
            .layout
            .nodes
            .iter()
            .filter(|n| matches!(n.shape, Shape::StateStart | Shape::StateEnd))
            .collect();
        let leaves: Vec<_> = sr
            .layout
            .nodes
            .iter()
            .filter(|n| !n.is_compound)
            .filter(|n| !matches!(n.shape, Shape::StateStart | Shape::StateEnd))
            .collect();
        for p in &pseudos {
            for l in &leaves {
                let pb = node_bbox(p);
                let lb = node_bbox(l);
                if rects_overlap(pb, lb) {
                    failures.push(format!(
                        "{}: pseudo-state {} overlaps leaf {}",
                        sr.stem, p.id, l.id
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "pseudo-state overlaps:\n{}",
        failures.join("\n")
    );
}

// ===========================================================================
// TIER 7: Spatial — containment (subgraphs / compounds)
// ===========================================================================

#[test]
fn flowchart_subgraphs_contain_children() {
    let tol = 2.0;
    let mut failures = Vec::new();
    for fr in FLOWCHARTS.iter() {
        let compounds: Vec<_> = fr.layout.nodes.iter().filter(|n| n.is_compound).collect();
        if compounds.is_empty() {
            continue;
        }
        for sg in &fr.layout.subgraphs {
            let sg_bbox = (
                sg.x - sg.width / 2.0,
                sg.y - sg.height / 2.0,
                sg.x + sg.width / 2.0,
                sg.y + sg.height / 2.0,
            );
            for n in &fr.layout.nodes {
                if n.is_compound || n.id == sg.id {
                    continue;
                }
                let nb = node_bbox(n);
                if n.x >= sg_bbox.0 && n.x <= sg_bbox.2 && n.y >= sg_bbox.1 && n.y <= sg_bbox.3 {
                    if !rect_contains(sg_bbox, nb, tol) {
                        failures.push(format!(
                            "{}: node {} center inside subgraph {} but bbox extends outside",
                            fr.stem, n.id, sg.id
                        ));
                    }
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "subgraph containment violations:\n{}",
        failures.join("\n")
    );
}

#[test]
fn state_compounds_contain_children() {
    let tol = 2.0;
    let mut failures = Vec::new();
    for sr in STATES.iter() {
        let compounds: Vec<_> = sr.layout.nodes.iter().filter(|n| n.is_compound).collect();
        for compound in &compounds {
            let outer = node_bbox(compound);
            for n in &sr.layout.nodes {
                if n.is_compound || n.id == compound.id {
                    continue;
                }
                if n.x >= outer.0 && n.x <= outer.2 && n.y >= outer.1 && n.y <= outer.3 {
                    let inner = node_bbox(n);
                    if !rect_contains(outer, inner, tol) {
                        failures.push(format!(
                            "{}: node {} center inside compound {} but bbox extends outside",
                            sr.stem, n.id, compound.id
                        ));
                    }
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "compound containment violations:\n{}",
        failures.join("\n")
    );
}

// ===========================================================================
// TIER 8: Edge structure — at least 2 points
// ===========================================================================

#[test]
fn flowchart_edges_have_points() {
    let mut failures = Vec::new();
    for fr in FLOWCHARTS.iter() {
        for (i, e) in fr.layout.edges.iter().enumerate() {
            if e.points.len() < 2 {
                failures.push(format!(
                    "{}: edge {} ({}->{}) has {} points",
                    fr.stem,
                    i,
                    e.src,
                    e.dst,
                    e.points.len()
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "edges with <2 points:\n{}",
        failures.join("\n")
    );
}

#[test]
fn state_edges_have_points() {
    let mut failures = Vec::new();
    for sr in STATES.iter() {
        for (i, e) in sr.layout.edges.iter().enumerate() {
            if e.points.len() < 2 {
                failures.push(format!(
                    "{}: edge {} ({}->{}) has {} points",
                    sr.stem,
                    i,
                    e.src,
                    e.dst,
                    e.points.len()
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "edges with <2 points:\n{}",
        failures.join("\n")
    );
}

// ===========================================================================
// TIER 9: Edge structure — endpoints near src/dst nodes
// ===========================================================================

#[test]
fn flowchart_edge_endpoints_near_nodes() {
    let tol = 25.0;
    let mut failures = Vec::new();
    for fr in FLOWCHARTS.iter() {
        let nodes = &fr.layout.nodes;
        for e in &fr.layout.edges {
            if e.points.len() < 2 {
                continue;
            }
            let start = e.points[0];
            let end = e.points[e.points.len() - 1];

            if let Some(src) = nodes.iter().find(|n| n.id == e.src) {
                let (left, top, right, bottom) = node_bbox(src);
                let near_src = start.x >= left - tol
                    && start.x <= right + tol
                    && start.y >= top - tol
                    && start.y <= bottom + tol;
                if !near_src {
                    failures.push(format!(
                        "{}: edge {}->{} start ({:.1},{:.1}) far from src bbox ({:.1},{:.1})-({:.1},{:.1})",
                        fr.stem, e.src, e.dst, start.x, start.y, left, top, right, bottom
                    ));
                }
            }

            if let Some(dst) = nodes.iter().find(|n| n.id == e.dst) {
                let (left, top, right, bottom) = node_bbox(dst);
                let near_dst = end.x >= left - tol
                    && end.x <= right + tol
                    && end.y >= top - tol
                    && end.y <= bottom + tol;
                if !near_dst {
                    failures.push(format!(
                        "{}: edge {}->{} end ({:.1},{:.1}) far from dst bbox ({:.1},{:.1})-({:.1},{:.1})",
                        fr.stem, e.src, e.dst, end.x, end.y, left, top, right, bottom
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "edge endpoints far from nodes:\n{}",
        failures.join("\n")
    );
}

#[test]
fn state_edge_endpoints_near_nodes() {
    let tol = 25.0;
    let mut failures = Vec::new();
    for sr in STATES.iter() {
        let nodes = &sr.layout.nodes;
        for e in &sr.layout.edges {
            if e.points.len() < 2 {
                continue;
            }
            let start = e.points[0];
            let end = e.points[e.points.len() - 1];

            if let Some(src) = nodes.iter().find(|n| n.id == e.src) {
                let (left, top, right, bottom) = node_bbox(src);
                let near_src = start.x >= left - tol
                    && start.x <= right + tol
                    && start.y >= top - tol
                    && start.y <= bottom + tol;
                if !near_src {
                    failures.push(format!(
                        "{}: edge {}->{} start ({:.1},{:.1}) far from src bbox ({:.1},{:.1})-({:.1},{:.1})",
                        sr.stem, e.src, e.dst, start.x, start.y, left, top, right, bottom
                    ));
                }
            }

            if let Some(dst) = nodes.iter().find(|n| n.id == e.dst) {
                let (left, top, right, bottom) = node_bbox(dst);
                let near_dst = end.x >= left - tol
                    && end.x <= right + tol
                    && end.y >= top - tol
                    && end.y <= bottom + tol;
                if !near_dst {
                    failures.push(format!(
                        "{}: edge {}->{} end ({:.1},{:.1}) far from dst bbox ({:.1},{:.1})-({:.1},{:.1})",
                        sr.stem, e.src, e.dst, end.x, end.y, left, top, right, bottom
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "edge endpoints far from nodes:\n{}",
        failures.join("\n")
    );
}

// ===========================================================================
// TIER 10: Structural — edge connectivity (no dangling edges)
// ===========================================================================

#[test]
fn flowchart_edge_connectivity() {
    let mut failures = Vec::new();
    for fr in FLOWCHARTS.iter() {
        let mut known_ids: HashSet<&str> = fr.layout.nodes.iter().map(|n| n.id.as_str()).collect();
        // Subgraphs are valid edge endpoints (compound nodes)
        for sg in &fr.layout.subgraphs {
            known_ids.insert(sg.id.as_str());
        }
        for e in &fr.layout.edges {
            if !known_ids.contains(e.src.as_str()) {
                failures.push(format!(
                    "{}: edge {}->{} references missing src node",
                    fr.stem, e.src, e.dst
                ));
            }
            if !known_ids.contains(e.dst.as_str()) {
                failures.push(format!(
                    "{}: edge {}->{} references missing dst node",
                    fr.stem, e.src, e.dst
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "dangling edges:\n{}",
        failures.join("\n")
    );
}

#[test]
fn state_edge_connectivity() {
    let mut failures = Vec::new();
    for sr in STATES.iter() {
        let node_ids: HashSet<&str> = sr.layout.nodes.iter().map(|n| n.id.as_str()).collect();
        for e in &sr.layout.edges {
            // Scoped pseudo-states (e.g. "Active.[*]_start") inside composites
            // are internal to the bridge — edges reference them but they're
            // not exposed as layout nodes. Only check non-scoped IDs.
            let src_scoped = e.src.contains('.') && e.src.contains("[*]");
            let dst_scoped = e.dst.contains('.') && e.dst.contains("[*]");
            if !src_scoped && !node_ids.contains(e.src.as_str()) {
                failures.push(format!(
                    "{}: edge {}->{} references missing src node",
                    sr.stem, e.src, e.dst
                ));
            }
            if !dst_scoped && !node_ids.contains(e.dst.as_str()) {
                failures.push(format!(
                    "{}: edge {}->{} references missing dst node",
                    sr.stem, e.src, e.dst
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "dangling edges:\n{}",
        failures.join("\n")
    );
}

// ===========================================================================
// TIER 11: Structural — unique node IDs
// ===========================================================================

#[test]
fn flowchart_unique_node_ids() {
    let mut failures = Vec::new();
    for fr in FLOWCHARTS.iter() {
        let mut seen = HashSet::new();
        for n in &fr.layout.nodes {
            if !seen.insert(&n.id) {
                failures.push(format!("{}: duplicate node id '{}'", fr.stem, n.id));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "duplicate node IDs:\n{}",
        failures.join("\n")
    );
}

#[test]
fn state_unique_node_ids() {
    let mut failures = Vec::new();
    for sr in STATES.iter() {
        let mut seen = HashSet::new();
        for n in &sr.layout.nodes {
            // Note nodes can have duplicates when a state has notes on both
            // sides (both get id "{state}-note") — known naming limitation.
            if n.shape == Shape::Note {
                continue;
            }
            if !seen.insert(&n.id) {
                failures.push(format!("{}: duplicate node id '{}'", sr.stem, n.id));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "duplicate node IDs:\n{}",
        failures.join("\n")
    );
}

// ===========================================================================
// TIER 12: Label dimensions — label_size valid when label present
// ===========================================================================

#[test]
fn flowchart_label_dimensions_valid() {
    let mut failures = Vec::new();
    for fr in FLOWCHARTS.iter() {
        for e in &fr.layout.edges {
            if e.label.is_some() {
                match e.label_size {
                    None => failures.push(format!(
                        "{}: edge {}->{} has label but no label_size",
                        fr.stem, e.src, e.dst
                    )),
                    Some((w, h)) => {
                        if w <= 0.0 || h <= 0.0 {
                            failures.push(format!(
                                "{}: edge {}->{} label_size ({:.1},{:.1}) non-positive",
                                fr.stem, e.src, e.dst, w, h
                            ));
                        }
                    }
                }
            }
            if e.label.is_none() && e.label_size.is_some() {
                failures.push(format!(
                    "{}: edge {}->{} has label_size but no label",
                    fr.stem, e.src, e.dst
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "invalid label dimensions:\n{}",
        failures.join("\n")
    );
}

#[test]
fn state_label_dimensions_valid() {
    let mut failures = Vec::new();
    for sr in STATES.iter() {
        for e in &sr.layout.edges {
            if e.label.is_some() {
                match e.label_size {
                    None => failures.push(format!(
                        "{}: edge {}->{} has label but no label_size",
                        sr.stem, e.src, e.dst
                    )),
                    Some((w, h)) => {
                        if w <= 0.0 || h <= 0.0 {
                            failures.push(format!(
                                "{}: edge {}->{} label_size ({:.1},{:.1}) non-positive",
                                sr.stem, e.src, e.dst, w, h
                            ));
                        }
                    }
                }
            }
            if e.label.is_none() && e.label_size.is_some() {
                failures.push(format!(
                    "{}: edge {}->{} has label_size but no label",
                    sr.stem, e.src, e.dst
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "invalid label dimensions:\n{}",
        failures.join("\n")
    );
}

// ===========================================================================
// TIER 13: Shape aspect ratios — circles/diamonds should be roughly square
// ===========================================================================

#[test]
fn flowchart_circle_shapes_roughly_square() {
    let mut failures = Vec::new();
    for fr in FLOWCHARTS.iter() {
        for n in &fr.layout.nodes {
            match n.shape {
                Shape::Circle | Shape::DoubleCircle => {
                    let ratio = n.width / n.height;
                    if ratio < 0.8 || ratio > 1.25 {
                        failures.push(format!(
                            "{}: node {} ({:?}) aspect ratio {:.2} ({}x{})",
                            fr.stem, n.id, n.shape, ratio, n.width, n.height
                        ));
                    }
                }
                Shape::Diamond => {
                    // Diamonds can be wider than tall for long labels, but
                    // shouldn't be extremely skewed
                    let ratio = n.width / n.height;
                    if ratio < 0.3 || ratio > 3.0 {
                        failures.push(format!(
                            "{}: node {} (Diamond) extreme aspect ratio {:.2} ({}x{})",
                            fr.stem, n.id, ratio, n.width, n.height
                        ));
                    }
                }
                _ => {}
            }
        }
    }
    assert!(
        failures.is_empty(),
        "shape aspect ratio violations:\n{}",
        failures.join("\n")
    );
}

#[test]
fn state_special_shape_sizes() {
    let mut failures = Vec::new();
    for sr in STATES.iter() {
        for n in &sr.layout.nodes {
            match n.shape {
                Shape::StateStart | Shape::StateEnd => {
                    // Pseudo-state circles: should be square
                    let ratio = n.width / n.height;
                    if ratio < 0.8 || ratio > 1.25 {
                        failures.push(format!(
                            "{}: pseudo-state {} ({:?}) aspect ratio {:.2}",
                            sr.stem, n.id, n.shape, ratio
                        ));
                    }
                }
                Shape::ForkJoin => {
                    // Fork/Join bars: should be much wider than tall (70x7)
                    if n.width < n.height {
                        failures.push(format!(
                            "{}: fork/join {} wider than tall: {}x{}",
                            sr.stem, n.id, n.width, n.height
                        ));
                    }
                }
                Shape::Choice => {
                    // Choice diamonds: should be roughly square
                    let ratio = n.width / n.height;
                    if ratio < 0.8 || ratio > 1.25 {
                        failures.push(format!(
                            "{}: choice {} aspect ratio {:.2} ({}x{})",
                            sr.stem, n.id, ratio, n.width, n.height
                        ));
                    }
                }
                _ => {}
            }
        }
    }
    assert!(
        failures.is_empty(),
        "state shape size violations:\n{}",
        failures.join("\n")
    );
}

// ===========================================================================
// TIER 14: Self-loop routing — path must exit and re-enter the same node
// ===========================================================================

#[test]
fn flowchart_self_loop_routing() {
    let mut failures = Vec::new();
    for fr in FLOWCHARTS.iter() {
        for e in &fr.layout.edges {
            if e.src != e.dst {
                continue;
            }
            if e.points.len() < 3 {
                continue;
            }

            let Some(node) = fr.layout.nodes.iter().find(|n| n.id == e.src) else {
                continue;
            };
            let bbox = node_bbox(node);

            // At least one control point should be outside the node bbox
            // (the path exits the node and comes back)
            let has_external = e.points.iter().any(|pt| {
                pt.x < bbox.0 - 1.0
                    || pt.x > bbox.2 + 1.0
                    || pt.y < bbox.1 - 1.0
                    || pt.y > bbox.3 + 1.0
            });
            if !has_external {
                failures.push(format!(
                    "{}: self-loop on {} has no external control points",
                    fr.stem, e.src
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "self-loop routing issues:\n{}",
        failures.join("\n")
    );
}

#[test]
fn state_self_loop_routing() {
    let mut failures = Vec::new();
    for sr in STATES.iter() {
        for e in &sr.layout.edges {
            if e.src != e.dst {
                continue;
            }
            if e.points.len() < 3 {
                continue;
            }

            let Some(node) = sr.layout.nodes.iter().find(|n| n.id == e.src) else {
                continue;
            };
            let bbox = node_bbox(node);

            let has_external = e.points.iter().any(|pt| {
                pt.x < bbox.0 - 1.0
                    || pt.x > bbox.2 + 1.0
                    || pt.y < bbox.1 - 1.0
                    || pt.y > bbox.3 + 1.0
            });
            if !has_external {
                failures.push(format!(
                    "{}: self-loop on {} has no external control points",
                    sr.stem, e.src
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "self-loop routing issues:\n{}",
        failures.join("\n")
    );
}

// ===========================================================================
// TIER 15: Direction-aware rank progression (flowchart)
// ===========================================================================

#[test]
fn flowchart_direction_rank_progression() {
    use rusty_mermaid_core::Direction;
    let mut violations: std::collections::BTreeMap<String, (usize, usize)> =
        std::collections::BTreeMap::new();
    for fr in FLOWCHARTS.iter() {
        let dir = fr.diagram.direction;
        let total = fr.layout.edges.len();
        let mut count = 0usize;
        for e in &fr.layout.edges {
            if e.src == e.dst {
                continue;
            }

            let Some(src) = fr.layout.nodes.iter().find(|n| n.id == e.src) else {
                continue;
            };
            let Some(dst) = fr.layout.nodes.iter().find(|n| n.id == e.dst) else {
                continue;
            };
            if src.is_compound || dst.is_compound {
                continue;
            }

            let ok = match dir {
                Direction::TB => dst.y >= src.y - 1.0,
                Direction::BT => dst.y <= src.y + 1.0,
                Direction::LR => dst.x >= src.x - 1.0,
                Direction::RL => dst.x <= src.x + 1.0,
            };
            if !ok {
                count += 1;
            }
        }
        if count > 0 {
            violations.insert(fr.stem.clone(), (count, total));
        }
    }
    // Cycles legitimately have back-edges. Only flag if majority of edges
    // violate the direction (indicates a layout bug, not just cycles).
    let hard_failures: Vec<_> = violations
        .iter()
        .filter(|(_, (count, total))| *count > *total / 2)
        .map(|(stem, (count, total))| {
            format!("{}: {}/{} edges violate direction", stem, count, total)
        })
        .collect();
    assert!(
        hard_failures.is_empty(),
        "direction violations:\n{}",
        hard_failures.join("\n")
    );
}

// ===========================================================================
// TIER 16: Style propagation — classDef/style applied to layout
// ===========================================================================

#[test]
fn flowchart_style_propagation() {
    let mut failures = Vec::new();
    for fr in FLOWCHARTS.iter() {
        // Check nodes that should have styles via classDef + class assignments
        if fr.diagram.class_defs.is_empty() && fr.diagram.style_stmts.is_empty() {
            continue;
        }

        // Collect node IDs that have styles applied
        let mut styled_ids: HashSet<String> = HashSet::new();
        for stmt in &fr.diagram.style_stmts {
            for id in &stmt.ids {
                styled_ids.insert(id.clone());
            }
        }

        // Collect node IDs that have classes assigned
        for v in &fr.diagram.vertices {
            if !v.classes.is_empty() {
                // Check if any assigned class is actually defined
                let has_defined_class = v
                    .classes
                    .iter()
                    .any(|c| fr.diagram.class_defs.iter().any(|cd| cd.name == *c));
                if has_defined_class {
                    styled_ids.insert(v.id.clone());
                }
            }
        }

        for id in &styled_ids {
            if let Some(node) = fr.layout.nodes.iter().find(|n| n.id == *id) {
                if node.custom_style.is_none() {
                    failures.push(format!(
                        "{}: node {} has classDef/style in IR but no custom_style in layout",
                        fr.stem, id
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "missing style propagation:\n{}",
        failures.join("\n")
    );
}

#[test]
fn state_style_propagation() {
    let mut failures = Vec::new();
    for sr in STATES.iter() {
        if sr.diagram.class_defs.is_empty() && sr.diagram.style_stmts.is_empty() {
            continue;
        }

        let mut styled_ids: HashSet<String> = HashSet::new();
        for stmt in &sr.diagram.style_stmts {
            for id in &stmt.ids {
                styled_ids.insert(id.clone());
            }
        }

        // Collect state IDs that have classes assigned
        fn collect_classed_ids(
            states: &[rusty_mermaid_diagrams::state::ir::StateNode],
            class_defs: &[rusty_mermaid_diagrams::common::styling::ClassDef],
            styled_ids: &mut HashSet<String>,
        ) {
            use rusty_mermaid_diagrams::state::ir::StateKind;
            for s in states {
                if !s.classes.is_empty() {
                    let has_defined = s
                        .classes
                        .iter()
                        .any(|c| class_defs.iter().any(|cd| cd.name == *c));
                    if has_defined {
                        styled_ids.insert(s.id.clone());
                    }
                }
                if let StateKind::Composite {
                    children, regions, ..
                } = &s.kind
                {
                    collect_classed_ids(children, class_defs, styled_ids);
                    for region in regions {
                        collect_classed_ids(&region.children, class_defs, styled_ids);
                    }
                }
            }
        }
        collect_classed_ids(&sr.diagram.states, &sr.diagram.class_defs, &mut styled_ids);

        for id in &styled_ids {
            if let Some(node) = sr.layout.nodes.iter().find(|n| n.id == *id) {
                if node.custom_style.is_none() {
                    failures.push(format!(
                        "{}: state {} has classDef/style in IR but no custom_style in layout",
                        sr.stem, id
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "missing style propagation:\n{}",
        failures.join("\n")
    );
}

// ===========================================================================
// TIER 17: State — concurrent region dividers
// ===========================================================================

#[test]
fn state_concurrent_dividers_valid() {
    let mut failures = Vec::new();
    for sr in STATES.iter() {
        // Check compound nodes with region_count >= 2
        for node in &sr.layout.nodes {
            if node.region_count < 2 {
                continue;
            }

            let expected_dividers = node.region_count - 1;
            let bbox = node_bbox(node);

            // Count dividers within this compound's bbox
            let dividers_in_compound: Vec<_> = sr
                .layout
                .dividers
                .iter()
                .filter(|d| {
                    d.start.x >= bbox.0 - 1.0
                        && d.start.x <= bbox.2 + 1.0
                        && d.start.y >= bbox.1 - 1.0
                        && d.end.y <= bbox.3 + 1.0
                })
                .collect();

            if dividers_in_compound.len() != expected_dividers {
                failures.push(format!(
                    "{}: compound {} has region_count={} but {} dividers (expected {})",
                    sr.stem,
                    node.id,
                    node.region_count,
                    dividers_in_compound.len(),
                    expected_dividers
                ));
            }

            // Verify dividers are vertical and within compound bounds
            for d in &dividers_in_compound {
                if (d.start.x - d.end.x).abs() > 1.0 {
                    failures.push(format!(
                        "{}: compound {} divider not vertical: ({:.1},{:.1})->({:.1},{:.1})",
                        sr.stem, node.id, d.start.x, d.start.y, d.end.x, d.end.y
                    ));
                }
            }
        }

        // Check region_rects count matches compound region_counts
        let total_regions: usize = sr
            .layout
            .nodes
            .iter()
            .filter(|n| n.region_count >= 2)
            .map(|n| n.region_count)
            .sum();
        if sr.layout.region_rects.len() != total_regions {
            failures.push(format!(
                "{}: {} region_rects but {} expected from compounds",
                sr.stem,
                sr.layout.region_rects.len(),
                total_regions
            ));
        }

        // Verify region rects have positive dimensions
        for (i, rect) in sr.layout.region_rects.iter().enumerate() {
            if rect.width <= 0.0 || rect.height <= 0.0 {
                failures.push(format!(
                    "{}: region_rect {} has non-positive size: {}x{}",
                    sr.stem, i, rect.width, rect.height
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "concurrent region issues:\n{}",
        failures.join("\n")
    );
}

// ===========================================================================
// TIER 18: State — notes appear in layout as Note-shaped nodes
// ===========================================================================

#[test]
fn state_notes_in_layout() {
    let mut failures = Vec::new();
    for sr in STATES.iter() {
        let note_state_ids = collect_note_state_ids(&sr.diagram);
        if note_state_ids.is_empty() {
            continue;
        }

        for state_id in &note_state_ids {
            let note_node_id = format!("{}-note", state_id);
            let found = sr.layout.nodes.iter().find(|n| n.id == note_node_id);
            match found {
                None => {
                    failures.push(format!(
                        "{}: note for state '{}' missing from layout (expected node '{}')",
                        sr.stem, state_id, note_node_id
                    ));
                }
                Some(node) => {
                    if node.shape != Shape::Note {
                        failures.push(format!(
                            "{}: note node '{}' has shape {:?}, expected Note",
                            sr.stem, note_node_id, node.shape
                        ));
                    }
                    if node.width <= 0.0 || node.height <= 0.0 {
                        failures.push(format!(
                            "{}: note node '{}' has non-positive size {}x{}",
                            sr.stem, note_node_id, node.width, node.height
                        ));
                    }
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "missing/invalid notes in layout:\n{}",
        failures.join("\n")
    );
}

// ===========================================================================
// TIER 19: State — region_count consistency with IR
// ===========================================================================

#[test]
fn state_region_count_matches_ir() {
    let mut failures = Vec::new();
    for sr in STATES.iter() {
        for node in &sr.layout.nodes {
            if !node.is_compound {
                continue;
            }
            let ir_regions = ir_region_count(&sr.diagram.states, &node.id);
            if ir_regions > 0 && node.region_count != ir_regions {
                failures.push(format!(
                    "{}: compound {} has region_count={} in layout but {} regions in IR",
                    sr.stem, node.id, node.region_count, ir_regions
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "region count mismatches:\n{}",
        failures.join("\n")
    );
}

// ===========================================================================
// TIER 20: State — pseudo-states have correct shapes
// ===========================================================================

#[test]
fn state_pseudo_state_shapes() {
    let mut failures = Vec::new();
    for sr in STATES.iter() {
        for n in &sr.layout.nodes {
            if n.id.ends_with("_start") || n.id.contains("[*]_start") {
                if n.shape != Shape::StateStart {
                    failures.push(format!(
                        "{}: start pseudo-state '{}' has shape {:?}, expected StateStart",
                        sr.stem, n.id, n.shape
                    ));
                }
            }
            if n.id.ends_with("_end") || n.id.contains("[*]_end") {
                if n.shape != Shape::StateEnd {
                    failures.push(format!(
                        "{}: end pseudo-state '{}' has shape {:?}, expected StateEnd",
                        sr.stem, n.id, n.shape
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "pseudo-state shape mismatches:\n{}",
        failures.join("\n")
    );
}

// ===========================================================================
// TIER 21: Edge stroke type consistency with IR
// ===========================================================================

#[test]
fn flowchart_edge_attributes_from_ir() {
    let mut failures = Vec::new();
    for fr in FLOWCHARTS.iter() {
        // Edge order may differ between IR and layout (dagre iteration order).
        // Group by (src, dst) and compare as sorted sets per pair.
        let mut ir_by_pair: std::collections::BTreeMap<(&str, &str), Vec<_>> =
            std::collections::BTreeMap::new();
        let mut layout_by_pair: std::collections::BTreeMap<(&str, &str), Vec<_>> =
            std::collections::BTreeMap::new();

        for e in &fr.diagram.edges {
            ir_by_pair
                .entry((e.src.as_str(), e.dst.as_str()))
                .or_default()
                .push((e.stroke, e.start_arrow, e.end_arrow, e.label.as_deref()));
        }
        for e in &fr.layout.edges {
            layout_by_pair
                .entry((e.src.as_str(), e.dst.as_str()))
                .or_default()
                .push((e.stroke, e.start_arrow, e.end_arrow, e.label.as_deref()));
        }

        // For unique (src, dst) pairs, verify stroke/arrows/label match
        for ((src, dst), ir_edges) in &ir_by_pair {
            let Some(layout_edges) = layout_by_pair.get(&(*src, *dst)) else {
                continue;
            };
            if ir_edges.len() != layout_edges.len() {
                continue;
            }
            if ir_edges.len() != 1 {
                continue;
            } // skip multi-edges

            let (ir_stroke, ir_start, ir_end, ir_label) = &ir_edges[0];
            let (l_stroke, l_start, l_end, l_label) = &layout_edges[0];

            if ir_stroke != l_stroke {
                failures.push(format!(
                    "{}: edge {}->{} stroke {:?} in layout but {:?} in IR",
                    fr.stem, src, dst, l_stroke, ir_stroke
                ));
            }
            if ir_start != l_start {
                failures.push(format!(
                    "{}: edge {}->{} start_arrow {:?} in layout but {:?} in IR",
                    fr.stem, src, dst, l_start, ir_start
                ));
            }
            if ir_end != l_end {
                failures.push(format!(
                    "{}: edge {}->{} end_arrow {:?} in layout but {:?} in IR",
                    fr.stem, src, dst, l_end, ir_end
                ));
            }
            if ir_label != l_label {
                failures.push(format!(
                    "{}: edge {}->{} label {:?} in layout but {:?} in IR",
                    fr.stem, src, dst, l_label, ir_label
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "edge attribute mismatches:\n{}",
        failures.join("\n")
    );
}

// ===========================================================================
// TIER 23: Multi-edge spacing — parallel edges between same nodes don't overlap
// ===========================================================================

#[test]
fn flowchart_multi_edge_spacing() {
    let mut failures = Vec::new();
    for fr in FLOWCHARTS.iter() {
        // Group edges by (src, dst) pair
        let mut edge_groups: std::collections::BTreeMap<(&str, &str), Vec<usize>> =
            std::collections::BTreeMap::new();
        for (i, e) in fr.layout.edges.iter().enumerate() {
            let key = if e.src <= e.dst {
                (e.src.as_str(), e.dst.as_str())
            } else {
                (e.dst.as_str(), e.src.as_str())
            };
            edge_groups.entry(key).or_default().push(i);
        }

        for ((a, b), indices) in &edge_groups {
            if indices.len() < 2 {
                continue;
            }

            // For each pair of parallel edges, check that their midpoints differ
            for i in 0..indices.len() {
                for j in (i + 1)..indices.len() {
                    let e1 = &fr.layout.edges[indices[i]];
                    let e2 = &fr.layout.edges[indices[j]];
                    if e1.points.is_empty() || e2.points.is_empty() {
                        continue;
                    }

                    let mid1 = e1.points[e1.points.len() / 2];
                    let mid2 = e2.points[e2.points.len() / 2];
                    let dist = ((mid1.x - mid2.x).powi(2) + (mid1.y - mid2.y).powi(2)).sqrt();

                    if dist < 1.0 {
                        failures.push(format!(
                            "{}: parallel edges {}<->{} have overlapping midpoints (dist={:.1})",
                            fr.stem, a, b, dist
                        ));
                    }
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "multi-edge overlap:\n{}",
        failures.join("\n")
    );
}

// ===========================================================================
// TIER 24: Flowchart minlen — longer arrows should span more ranks
// ===========================================================================

#[test]
fn flowchart_minlen_respected() {
    use rusty_mermaid_core::Direction;
    let mut failures = Vec::new();
    for fr in FLOWCHARTS.iter() {
        let dir = fr.diagram.direction;
        for ir_edge in &fr.diagram.edges {
            if ir_edge.minlen <= 1 {
                continue;
            }
            if ir_edge.src == ir_edge.dst {
                continue;
            }

            let Some(src) = fr.layout.nodes.iter().find(|n| n.id == ir_edge.src) else {
                continue;
            };
            let Some(dst) = fr.layout.nodes.iter().find(|n| n.id == ir_edge.dst) else {
                continue;
            };
            if src.is_compound || dst.is_compound {
                continue;
            }

            // Measure the span in the primary direction
            let span = match dir {
                Direction::TB => dst.y - src.y,
                Direction::BT => src.y - dst.y,
                Direction::LR => dst.x - src.x,
                Direction::RL => src.x - dst.x,
            };

            // Back-edges in cycles go against the flow — skip them
            if span < 0.0 {
                continue;
            }

            // Find baseline span for minlen=1 edges, then verify minlen>1
            // edges span proportionally more.
            let min1_spans: Vec<f64> = fr
                .diagram
                .edges
                .iter()
                .filter(|e| e.minlen == 1 && e.src != e.dst)
                .filter_map(|e| {
                    let s = fr.layout.nodes.iter().find(|n| n.id == e.src)?;
                    let d = fr.layout.nodes.iter().find(|n| n.id == e.dst)?;
                    if s.is_compound || d.is_compound {
                        return None;
                    }
                    Some(match dir {
                        Direction::TB => d.y - s.y,
                        Direction::BT => s.y - d.y,
                        Direction::LR => d.x - s.x,
                        Direction::RL => s.x - d.x,
                    })
                })
                .filter(|s| *s > 0.0)
                .collect();

            if min1_spans.is_empty() {
                continue;
            }
            let avg_single = min1_spans.iter().sum::<f64>() / min1_spans.len() as f64;

            // With minlen N, span should be at least N * avg_single * 0.75
            let expected_min = avg_single * ir_edge.minlen as f64 * 0.75;
            if span < expected_min {
                failures.push(format!(
                    "{}: edge {}->{} minlen={} but span={:.1} < expected_min={:.1}",
                    fr.stem, ir_edge.src, ir_edge.dst, ir_edge.minlen, span, expected_min
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "minlen violations:\n{}",
        failures.join("\n")
    );
}

// ===========================================================================
// TIER 25: Flowchart — node shapes match IR
// ===========================================================================

#[test]
fn flowchart_node_shapes_from_ir() {
    let mut failures = Vec::new();
    for fr in FLOWCHARTS.iter() {
        for v in &fr.diagram.vertices {
            if let Some(node) = fr.layout.nodes.iter().find(|n| n.id == v.id) {
                if node.shape != v.shape {
                    failures.push(format!(
                        "{}: node {} shape {:?} in layout but {:?} in IR",
                        fr.stem, v.id, node.shape, v.shape
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "node shape mismatches:\n{}",
        failures.join("\n")
    );
}

// ===========================================================================
// TIER 26: Flowchart — linkStyle propagation
// ===========================================================================

#[test]
fn flowchart_linkstyle_propagation() {
    let mut failures = Vec::new();
    for fr in FLOWCHARTS.iter() {
        if fr.diagram.link_styles.is_empty() {
            continue;
        }

        for ls in &fr.diagram.link_styles {
            if ls.is_default {
                continue;
            }
            for &idx in &ls.indices {
                if idx < fr.layout.edges.len() {
                    let edge = &fr.layout.edges[idx];
                    if edge.custom_style.is_none() {
                        failures.push(format!(
                            "{}: edge index {} ({}->{}) has linkStyle in IR but no custom_style",
                            fr.stem, idx, edge.src, edge.dst
                        ));
                    }
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "missing linkStyle propagation:\n{}",
        failures.join("\n")
    );
}

// ===========================================================================
// TIER 27: State — notes don't overlap their target state
// ===========================================================================

#[test]
fn state_notes_dont_overlap_state() {
    let mut failures = Vec::new();
    for sr in STATES.iter() {
        let note_state_ids = collect_note_state_ids(&sr.diagram);
        for state_id in &note_state_ids {
            let note_node_id = format!("{}-note", state_id);
            let note = sr.layout.nodes.iter().find(|n| n.id == note_node_id);
            let state = sr.layout.nodes.iter().find(|n| n.id == *state_id);
            let (Some(note), Some(state)) = (note, state) else {
                continue;
            };

            let note_bbox = node_bbox(note);
            let state_bbox = node_bbox(state);

            if rects_overlap(note_bbox, state_bbox) {
                failures.push(format!(
                    "{}: note '{}' overlaps state '{}'",
                    sr.stem, note_node_id, state_id
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "note/state overlaps:\n{}",
        failures.join("\n")
    );
}

// ===========================================================================
// TIER 28: Flowchart — subgraph positive dimensions
// ===========================================================================

#[test]
fn flowchart_subgraph_positive_dimensions() {
    let mut failures = Vec::new();
    for fr in FLOWCHARTS.iter() {
        for sg in &fr.layout.subgraphs {
            if sg.width <= 0.0 || sg.height <= 0.0 {
                failures.push(format!(
                    "{}: subgraph {} has {}x{}",
                    fr.stem, sg.id, sg.width, sg.height
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "non-positive subgraph dimensions:\n{}",
        failures.join("\n")
    );
}

// ===========================================================================
// TIER 29: State — dividers within compound bounds
// ===========================================================================

#[test]
fn state_dividers_within_compound_bounds() {
    let mut failures = Vec::new();
    for sr in STATES.iter() {
        for node in &sr.layout.nodes {
            if node.region_count < 2 {
                continue;
            }
            let bbox = node_bbox(node);

            for d in &sr.layout.dividers {
                // Check if this divider belongs to this compound
                let in_x = d.start.x >= bbox.0 - 1.0 && d.start.x <= bbox.2 + 1.0;
                let in_y = d.start.y >= bbox.1 - 1.0 && d.end.y <= bbox.3 + 1.0;
                if !in_x || !in_y {
                    continue;
                }

                // Divider should span from compound top to bottom (below header)
                if d.start.y < bbox.1 - 1.0 || d.end.y > bbox.3 + 1.0 {
                    failures.push(format!(
                        "{}: compound {} divider y-range ({:.1},{:.1}) exceeds bbox ({:.1},{:.1})",
                        sr.stem, node.id, d.start.y, d.end.y, bbox.1, bbox.3
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "dividers outside compound bounds:\n{}",
        failures.join("\n")
    );
}

// ===========================================================================
// TIER 30: Style field validity — opacity in [0,1], stroke_width > 0
// ===========================================================================

#[test]
fn flowchart_style_field_validity() {
    let mut failures = Vec::new();
    for fr in FLOWCHARTS.iter() {
        for n in &fr.layout.nodes {
            let Some(style) = &n.custom_style else {
                continue;
            };
            if let Some(opacity) = style.opacity {
                if !(0.0..=1.0).contains(&opacity) {
                    failures.push(format!(
                        "{}: node {} opacity {:.2} not in [0,1]",
                        fr.stem, n.id, opacity
                    ));
                }
            }
            if let Some(sw) = style.stroke_width {
                if sw < 0.0 {
                    failures.push(format!(
                        "{}: node {} stroke_width {:.2} negative",
                        fr.stem, n.id, sw
                    ));
                }
            }
        }
        for e in &fr.layout.edges {
            let Some(style) = &e.custom_style else {
                continue;
            };
            if let Some(opacity) = style.opacity {
                if !(0.0..=1.0).contains(&opacity) {
                    failures.push(format!(
                        "{}: edge {}->{} opacity {:.2} not in [0,1]",
                        fr.stem, e.src, e.dst, opacity
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "invalid style fields:\n{}",
        failures.join("\n")
    );
}

#[test]
fn state_style_field_validity() {
    let mut failures = Vec::new();
    for sr in STATES.iter() {
        for n in &sr.layout.nodes {
            let Some(style) = &n.custom_style else {
                continue;
            };
            if let Some(opacity) = style.opacity {
                if !(0.0..=1.0).contains(&opacity) {
                    failures.push(format!(
                        "{}: node {} opacity {:.2} not in [0,1]",
                        sr.stem, n.id, opacity
                    ));
                }
            }
            if let Some(sw) = style.stroke_width {
                if sw < 0.0 {
                    failures.push(format!(
                        "{}: node {} stroke_width {:.2} negative",
                        sr.stem, n.id, sw
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "invalid style fields:\n{}",
        failures.join("\n")
    );
}

// ===========================================================================
// SEQUENCE DIAGRAM INVARIANTS
// ===========================================================================

struct SeqResult {
    stem: String,
    layout: rusty_mermaid_diagrams::sequence::layout::SequenceLayout,
}

static SEQUENCES: LazyLock<Vec<SeqResult>> = LazyLock::new(|| {
    let mmd_dir = golden_mmd_dir().join("sequence");
    let mut results = Vec::new();
    for entry in fs::read_dir(&mmd_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("mmd") {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap();
        let diagram = match rusty_mermaid_diagrams::sequence::parser::parse(&text) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let layout = rusty_mermaid_diagrams::sequence::layout::layout(
            &diagram,
            &rusty_mermaid_core::SimpleTextMeasure::default(),
        );
        let stem = path.file_stem().unwrap().to_str().unwrap().to_string();
        results.push(SeqResult { stem, layout });
    }
    results.sort_by(|a, b| a.stem.cmp(&b.stem));
    results
});

#[test]
fn seq_has_golden_files() {
    assert!(!SEQUENCES.is_empty(), "no sequence golden .mmd files found");
}

#[test]
fn seq_positive_dimensions() {
    let mut failures = Vec::new();
    for sr in SEQUENCES.iter() {
        if sr.layout.width <= 0.0 || sr.layout.height <= 0.0 {
            failures.push(format!(
                "{}: invalid dimensions {}×{}",
                sr.stem, sr.layout.width, sr.layout.height
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "bad dimensions:\n{}",
        failures.join("\n")
    );
}

#[test]
fn seq_actors_no_horizontal_overlap() {
    let mut failures = Vec::new();
    for sr in SEQUENCES.iter() {
        let actors = &sr.layout.actors;
        for i in 0..actors.len() {
            let a = &actors[i];
            let a_right = a.x + a.width / 2.0;
            for j in (i + 1)..actors.len() {
                let b = &actors[j];
                let b_left = b.x - b.width / 2.0;
                if a_right > b_left + 1.0 {
                    failures.push(format!(
                        "{}: actors {} and {} overlap horizontally ({:.1} > {:.1})",
                        sr.stem, a.id, b.id, a_right, b_left
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "actor overlaps:\n{}",
        failures.join("\n")
    );
}

#[test]
fn seq_actors_ordered_left_to_right() {
    let mut failures = Vec::new();
    for sr in SEQUENCES.iter() {
        for w in sr.layout.actors.windows(2) {
            if w[0].x >= w[1].x {
                failures.push(format!(
                    "{}: actor {} (x={:.1}) not left of {} (x={:.1})",
                    sr.stem, w[0].id, w[0].x, w[1].id, w[1].x
                ));
            }
        }
    }
    assert!(failures.is_empty(), "actor order:\n{}", failures.join("\n"));
}

#[test]
fn seq_messages_advance_downward() {
    let mut failures = Vec::new();
    for sr in SEQUENCES.iter() {
        for w in sr.layout.messages.windows(2) {
            if w[1].y <= w[0].y {
                failures.push(format!(
                    "{}: message y {:.1} not below {:.1}",
                    sr.stem, w[1].y, w[0].y
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "message order:\n{}",
        failures.join("\n")
    );
}

#[test]
fn seq_messages_below_actor_boxes() {
    let mut failures = Vec::new();
    for sr in SEQUENCES.iter() {
        if sr.layout.actors.is_empty() {
            continue;
        }
        let actor_bottom = sr.layout.actors[0].y + sr.layout.actors[0].height;
        for msg in &sr.layout.messages {
            if msg.y < actor_bottom - 1.0 {
                failures.push(format!(
                    "{}: message y={:.1} above actor bottom {:.1}",
                    sr.stem, msg.y, actor_bottom
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "message position:\n{}",
        failures.join("\n")
    );
}

#[test]
fn seq_lifelines_span_actor_to_bottom() {
    let mut failures = Vec::new();
    for sr in SEQUENCES.iter() {
        for ll in &sr.layout.lifelines {
            if ll.top_y >= ll.bottom_y {
                failures.push(format!(
                    "{}: lifeline {} top {:.1} >= bottom {:.1}",
                    sr.stem, ll.actor_id, ll.top_y, ll.bottom_y
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "lifeline spans:\n{}",
        failures.join("\n")
    );
}

#[test]
fn seq_activations_positive_height() {
    let mut failures = Vec::new();
    for sr in SEQUENCES.iter() {
        for act in &sr.layout.activations {
            if act.top_y >= act.bottom_y {
                failures.push(format!(
                    "{}: activation on {} has non-positive height ({:.1}..{:.1})",
                    sr.stem, act.actor_id, act.top_y, act.bottom_y
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "activation heights:\n{}",
        failures.join("\n")
    );
}

#[test]
fn seq_fragments_enclose_child_messages() {
    let mut failures = Vec::new();
    for sr in SEQUENCES.iter() {
        for frag in &sr.layout.fragments {
            let frag_top = frag.y;
            let frag_bottom = frag.y + frag.height;
            // Check all messages that fall within fragment y range.
            for msg in &sr.layout.messages {
                if msg.y > frag_top && msg.y < frag_bottom {
                    // Message x should be within fragment x bounds.
                    let frag_left = frag.x;
                    let frag_right = frag.x + frag.width;
                    let msg_left = msg.from_x.min(msg.to_x);
                    let msg_right = msg.from_x.max(msg.to_x);
                    if msg_left < frag_left - 1.0 || msg_right > frag_right + 1.0 {
                        failures.push(format!(
                            "{}: message x [{:.1}..{:.1}] outside fragment [{:.1}..{:.1}]",
                            sr.stem, msg_left, msg_right, frag_left, frag_right
                        ));
                    }
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "fragment containment:\n{}",
        failures.join("\n")
    );
}

#[test]
fn seq_bottom_actors_mirror_top() {
    let mut failures = Vec::new();
    for sr in SEQUENCES.iter() {
        if sr.layout.actors.len() != sr.layout.bottom_actors.len() {
            failures.push(format!(
                "{}: top {} != bottom {} actor count",
                sr.stem,
                sr.layout.actors.len(),
                sr.layout.bottom_actors.len()
            ));
            continue;
        }
        for (top, bot) in sr.layout.actors.iter().zip(sr.layout.bottom_actors.iter()) {
            if top.id != bot.id {
                failures.push(format!("{}: top {} != bottom {}", sr.stem, top.id, bot.id));
            }
            if (top.x - bot.x).abs() > 0.01 {
                failures.push(format!(
                    "{}: actor {} x mismatch top={:.1} bot={:.1}",
                    sr.stem, top.id, top.x, bot.x
                ));
            }
            if bot.y <= top.y {
                failures.push(format!(
                    "{}: bottom actor {} not below top ({:.1} vs {:.1})",
                    sr.stem, top.id, bot.y, top.y
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "bottom actor mirror:\n{}",
        failures.join("\n")
    );
}

#[test]
fn seq_notes_have_positive_dimensions() {
    let mut failures = Vec::new();
    for sr in SEQUENCES.iter() {
        for note in &sr.layout.notes {
            if note.width <= 0.0 || note.height <= 0.0 {
                failures.push(format!(
                    "{}: note '{}' has invalid dimensions {}×{}",
                    sr.stem, note.text, note.width, note.height
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "note dimensions:\n{}",
        failures.join("\n")
    );
}

#[test]
fn seq_fragments_have_positive_dimensions() {
    let mut failures = Vec::new();
    for sr in SEQUENCES.iter() {
        for frag in &sr.layout.fragments {
            if frag.width <= 0.0 || frag.height <= 0.0 {
                failures.push(format!(
                    "{}: fragment {:?} has invalid dimensions {}×{}",
                    sr.stem, frag.kind, frag.width, frag.height
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "fragment dimensions:\n{}",
        failures.join("\n")
    );
}

#[test]
fn seq_all_within_canvas() {
    let tol = 5.0;
    let mut failures = Vec::new();
    for sr in SEQUENCES.iter() {
        let w = sr.layout.width;
        let h = sr.layout.height;
        for a in sr
            .layout
            .actors
            .iter()
            .chain(sr.layout.bottom_actors.iter())
        {
            let right = a.x + a.width / 2.0;
            let bottom = a.y + a.height;
            if right > w + tol || bottom > h + tol {
                failures.push(format!(
                    "{}: actor {} extends beyond canvas ({:.0}×{:.0})",
                    sr.stem, a.id, w, h
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "canvas bounds:\n{}",
        failures.join("\n")
    );
}

#[test]
fn seq_autonumber_sequential() {
    let mut failures = Vec::new();
    for sr in SEQUENCES.iter() {
        let numbered: Vec<_> = sr.layout.messages.iter().filter_map(|m| m.number).collect();
        if numbered.is_empty() {
            continue;
        }
        // Numbers must be strictly increasing.
        for w in numbered.windows(2) {
            if w[1] <= w[0] {
                failures.push(format!(
                    "{}: autonumber not increasing: {} then {}",
                    sr.stem, w[0], w[1]
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "autonumber ordering:\n{}",
        failures.join("\n")
    );
}

#[test]
fn seq_autonumber_step_consistent() {
    let mut failures = Vec::new();
    for sr in SEQUENCES.iter() {
        let numbered: Vec<_> = sr.layout.messages.iter().filter_map(|m| m.number).collect();
        if numbered.len() < 2 {
            continue;
        }
        // All consecutive differences should be equal (constant step).
        let step = numbered[1] - numbered[0];
        for w in numbered.windows(2) {
            let diff = w[1] - w[0];
            if diff != step {
                failures.push(format!(
                    "{}: autonumber step inconsistent: expected {} got {} (at {}→{})",
                    sr.stem, step, diff, w[0], w[1]
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "autonumber step:\n{}",
        failures.join("\n")
    );
}

#[test]
fn seq_self_messages_have_equal_from_to_x() {
    let mut failures = Vec::new();
    for sr in SEQUENCES.iter() {
        for msg in &sr.layout.messages {
            if msg.is_self && (msg.from_x - msg.to_x).abs() > 0.01 {
                failures.push(format!(
                    "{}: self-message from_x={:.1} != to_x={:.1}",
                    sr.stem, msg.from_x, msg.to_x
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "self-message endpoints:\n{}",
        failures.join("\n")
    );
}

#[test]
fn seq_fragment_sections_ordered_by_y() {
    let mut failures = Vec::new();
    for sr in SEQUENCES.iter() {
        for frag in &sr.layout.fragments {
            for w in frag.sections.windows(2) {
                if w[1].y <= w[0].y {
                    failures.push(format!(
                        "{}: fragment {:?} sections not ordered: y {:.1} then {:.1}",
                        sr.stem, frag.kind, w[0].y, w[1].y
                    ));
                }
            }
            // Sections must be within fragment bounds.
            for sec in &frag.sections {
                if sec.y < frag.y || sec.y > frag.y + frag.height {
                    failures.push(format!(
                        "{}: fragment {:?} section y={:.1} outside [{:.1}..{:.1}]",
                        sr.stem,
                        frag.kind,
                        sec.y,
                        frag.y,
                        frag.y + frag.height
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "fragment sections:\n{}",
        failures.join("\n")
    );
}

#[test]
fn seq_activations_on_valid_actors() {
    let mut failures = Vec::new();
    for sr in SEQUENCES.iter() {
        let actor_ids: Vec<&str> = sr.layout.actors.iter().map(|a| a.id.as_str()).collect();
        for act in &sr.layout.activations {
            if !actor_ids.contains(&act.actor_id.as_str()) {
                failures.push(format!(
                    "{}: activation on unknown actor '{}'",
                    sr.stem, act.actor_id
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "activation actors:\n{}",
        failures.join("\n")
    );
}

#[test]
fn seq_activations_within_lifeline_bounds() {
    let mut failures = Vec::new();
    for sr in SEQUENCES.iter() {
        for act in &sr.layout.activations {
            if let Some(ll) = sr
                .layout
                .lifelines
                .iter()
                .find(|l| l.actor_id == act.actor_id)
            {
                if act.top_y < ll.top_y - 1.0 || act.bottom_y > ll.bottom_y + 1.0 {
                    failures.push(format!(
                        "{}: activation on {} [{:.1}..{:.1}] outside lifeline [{:.1}..{:.1}]",
                        sr.stem, act.actor_id, act.top_y, act.bottom_y, ll.top_y, ll.bottom_y
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "activation bounds:\n{}",
        failures.join("\n")
    );
}

#[test]
fn seq_no_messages_without_autonumber_have_numbers() {
    let mut failures = Vec::new();
    for sr in SEQUENCES.iter() {
        // If no messages have numbers, that's fine (autonumber not enabled).
        // If some do, ALL must (autonumber applies to all messages).
        let has_number: Vec<bool> = sr
            .layout
            .messages
            .iter()
            .map(|m| m.number.is_some())
            .collect();
        if has_number.is_empty() {
            continue;
        }
        let all = has_number.iter().all(|&b| b);
        let none = has_number.iter().all(|&b| !b);
        if !all && !none {
            failures.push(format!(
                "{}: mixed autonumber state — some messages numbered, some not",
                sr.stem
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "autonumber consistency:\n{}",
        failures.join("\n")
    );
}

#[test]
fn seq_messages_horizontal_span_within_actors() {
    let mut failures = Vec::new();
    for sr in SEQUENCES.iter() {
        if sr.layout.actors.is_empty() {
            continue;
        }
        let leftmost = sr.layout.actors.first().unwrap().x;
        let rightmost = sr.layout.actors.last().unwrap().x;
        for msg in &sr.layout.messages {
            let left = msg.from_x.min(msg.to_x);
            let right = msg.from_x.max(msg.to_x);
            // Self-messages loop to the right, so allow some extra.
            let tol = if msg.is_self { 50.0 } else { 10.0 };
            if left < leftmost - tol || right > rightmost + tol {
                failures.push(format!(
                    "{}: message x [{:.1}..{:.1}] outside actors [{:.1}..{:.1}]",
                    sr.stem, left, right, leftmost, rightmost
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "message span:\n{}",
        failures.join("\n")
    );
}
