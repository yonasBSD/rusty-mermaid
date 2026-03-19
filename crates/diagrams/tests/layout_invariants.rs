use std::fs;
use std::path::{Path, PathBuf};

use rusty_mermaid_diagrams::{detect, DiagramKind};

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
    layout: rusty_mermaid_diagrams::flowchart::bridge::LayoutResult,
}

struct StateResult {
    stem: String,
    layout: rusty_mermaid_diagrams::state::bridge::LayoutResult,
}

fn load_flowcharts() -> Vec<FlowResult> {
    let mmd_dir = golden_mmd_dir();
    let mut results = Vec::new();
    for entry in fs::read_dir(&mmd_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("mmd") {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap();
        if detect(&text) != Some(DiagramKind::Flowchart) {
            continue;
        }
        let diagram = match rusty_mermaid_diagrams::flowchart::parser::parse(&text) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let layout = rusty_mermaid_diagrams::flowchart::bridge::layout(&diagram);
        let stem = path.file_stem().unwrap().to_str().unwrap().to_string();
        results.push(FlowResult { stem, layout });
    }
    results.sort_by(|a, b| a.stem.cmp(&b.stem));
    results
}

fn load_state_diagrams() -> Vec<StateResult> {
    let mmd_dir = golden_mmd_dir();
    let mut results = Vec::new();
    for entry in fs::read_dir(&mmd_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("mmd") {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap();
        if detect(&text) != Some(DiagramKind::State) {
            continue;
        }
        let diagram = match rusty_mermaid_diagrams::state::parser::parse(&text) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let layout = rusty_mermaid_diagrams::state::bridge::layout(&diagram);
        let stem = path.file_stem().unwrap().to_str().unwrap().to_string();
        results.push(StateResult { stem, layout });
    }
    results.sort_by(|a, b| a.stem.cmp(&b.stem));
    results
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn node_bbox(
    n: &rusty_mermaid_diagrams::common::layout::NodeLayout,
) -> (f64, f64, f64, f64) {
    let left = n.x - n.width / 2.0;
    let top = n.y - n.height / 2.0;
    let right = n.x + n.width / 2.0;
    let bottom = n.y + n.height / 2.0;
    (left, top, right, bottom)
}

fn rects_overlap(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> bool {
    // Two rects overlap if they intersect in both x and y
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

// ---------------------------------------------------------------------------
// Invariant: positive dimensions
// ---------------------------------------------------------------------------

#[test]
fn flowchart_positive_dimensions() {
    let mut failures = Vec::new();
    for fr in load_flowcharts() {
        if fr.layout.width <= 0.0 || fr.layout.height <= 0.0 {
            failures.push(format!(
                "{}: layout {}x{}", fr.stem, fr.layout.width, fr.layout.height
            ));
        }
        for n in &fr.layout.nodes {
            if n.width <= 0.0 || n.height <= 0.0 {
                failures.push(format!(
                    "{}: node {} has {}x{}", fr.stem, n.id, n.width, n.height
                ));
            }
        }
    }
    assert!(failures.is_empty(), "non-positive dimensions:\n{}", failures.join("\n"));
}

#[test]
fn state_positive_dimensions() {
    let mut failures = Vec::new();
    for sr in load_state_diagrams() {
        if sr.layout.width <= 0.0 || sr.layout.height <= 0.0 {
            failures.push(format!(
                "{}: layout {}x{}", sr.stem, sr.layout.width, sr.layout.height
            ));
        }
        for n in &sr.layout.nodes {
            if n.width <= 0.0 || n.height <= 0.0 {
                failures.push(format!(
                    "{}: node {} has {}x{}", sr.stem, n.id, n.width, n.height
                ));
            }
        }
    }
    assert!(failures.is_empty(), "non-positive dimensions:\n{}", failures.join("\n"));
}

// ---------------------------------------------------------------------------
// Invariant: all nodes within layout bounds
// ---------------------------------------------------------------------------

#[test]
fn flowchart_nodes_within_bounds() {
    let mut failures = Vec::new();
    for fr in load_flowcharts() {
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
    assert!(failures.is_empty(), "nodes outside bounds:\n{}", failures.join("\n"));
}

#[test]
fn state_nodes_within_bounds() {
    let mut failures = Vec::new();
    for sr in load_state_diagrams() {
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
    assert!(failures.is_empty(), "nodes outside bounds:\n{}", failures.join("\n"));
}

// ---------------------------------------------------------------------------
// Invariant: no leaf-node overlaps
// ---------------------------------------------------------------------------

#[test]
fn flowchart_no_leaf_node_overlaps() {
    let mut failures = Vec::new();
    for fr in load_flowcharts() {
        let leaves: Vec<_> = fr.layout.nodes.iter().filter(|n| !n.is_compound).collect();
        for i in 0..leaves.len() {
            for j in (i + 1)..leaves.len() {
                let a = node_bbox(leaves[i]);
                let b = node_bbox(leaves[j]);
                if rects_overlap(a, b) {
                    failures.push(format!(
                        "{}: nodes {} and {} overlap", fr.stem, leaves[i].id, leaves[j].id
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "overlapping nodes:\n{}", failures.join("\n"));
}

#[test]
fn state_no_leaf_node_overlaps() {
    let mut failures = Vec::new();
    for sr in load_state_diagrams() {
        let leaves: Vec<_> = sr.layout.nodes.iter()
            .filter(|n| !n.is_compound)
            // Pseudo-states ([*]_start/end) are tiny circles that dagre may
            // place touching regular nodes — exclude from overlap check.
            .filter(|n| !matches!(n.shape, rusty_mermaid_core::Shape::StateStart | rusty_mermaid_core::Shape::StateEnd))
            .collect();
        for i in 0..leaves.len() {
            for j in (i + 1)..leaves.len() {
                let a = node_bbox(leaves[i]);
                let b = node_bbox(leaves[j]);
                if rects_overlap(a, b) {
                    failures.push(format!(
                        "{}: nodes {} and {} overlap", sr.stem, leaves[i].id, leaves[j].id
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "overlapping nodes:\n{}", failures.join("\n"));
}

// ---------------------------------------------------------------------------
// Invariant: edges have at least 2 points
// ---------------------------------------------------------------------------

#[test]
fn flowchart_edges_have_points() {
    let mut failures = Vec::new();
    for fr in load_flowcharts() {
        for (i, e) in fr.layout.edges.iter().enumerate() {
            if e.points.len() < 2 {
                failures.push(format!(
                    "{}: edge {} ({}->{}) has {} points",
                    fr.stem, i, e.src, e.dst, e.points.len()
                ));
            }
        }
    }
    assert!(failures.is_empty(), "edges with <2 points:\n{}", failures.join("\n"));
}

#[test]
fn state_edges_have_points() {
    let mut failures = Vec::new();
    for sr in load_state_diagrams() {
        for (i, e) in sr.layout.edges.iter().enumerate() {
            if e.points.len() < 2 {
                failures.push(format!(
                    "{}: edge {} ({}->{}) has {} points",
                    sr.stem, i, e.src, e.dst, e.points.len()
                ));
            }
        }
    }
    assert!(failures.is_empty(), "edges with <2 points:\n{}", failures.join("\n"));
}

// ---------------------------------------------------------------------------
// Invariant: edge endpoints near their source/destination nodes
// ---------------------------------------------------------------------------

#[test]
fn flowchart_edge_endpoints_near_nodes() {
    let tol = 25.0; // allow some slack for curve interpolation and markers
    let mut failures = Vec::new();
    for fr in load_flowcharts() {
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
    assert!(failures.is_empty(), "edge endpoints far from nodes:\n{}", failures.join("\n"));
}

// ---------------------------------------------------------------------------
// Invariant: edge endpoints near their source/destination nodes (state)
// ---------------------------------------------------------------------------

#[test]
fn state_edge_endpoints_near_nodes() {
    let tol = 25.0;
    let mut failures = Vec::new();
    for sr in load_state_diagrams() {
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
    assert!(failures.is_empty(), "edge endpoints far from nodes:\n{}", failures.join("\n"));
}

// ---------------------------------------------------------------------------
// Invariant: edge labels within layout bounds
// ---------------------------------------------------------------------------

#[test]
fn flowchart_edge_labels_within_bounds() {
    let mut failures = Vec::new();
    for fr in load_flowcharts() {
        let (lw, lh) = (fr.layout.width, fr.layout.height);
        for e in &fr.layout.edges {
            let Some((label_w, label_h)) = e.label_size else { continue };
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
    assert!(failures.is_empty(), "edge labels outside bounds:\n{}", failures.join("\n"));
}

#[test]
fn state_edge_labels_within_bounds() {
    let mut failures = Vec::new();
    for sr in load_state_diagrams() {
        let (lw, lh) = (sr.layout.width, sr.layout.height);
        for e in &sr.layout.edges {
            let Some((label_w, label_h)) = e.label_size else { continue };
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
    assert!(failures.is_empty(), "edge labels outside bounds:\n{}", failures.join("\n"));
}

// ---------------------------------------------------------------------------
// Invariant: subgraphs contain their children
// ---------------------------------------------------------------------------

#[test]
fn flowchart_subgraphs_contain_children() {
    let tol = 2.0;
    let mut failures = Vec::new();
    for fr in load_flowcharts() {
        // Compound nodes represent subgraphs
        let compounds: Vec<_> = fr.layout.nodes.iter().filter(|n| n.is_compound).collect();
        if compounds.is_empty() {
            continue;
        }
        // Match subgraph layouts to compound nodes by id
        for sg in &fr.layout.subgraphs {
            let sg_bbox = (
                sg.x - sg.width / 2.0,
                sg.y - sg.height / 2.0,
                sg.x + sg.width / 2.0,
                sg.y + sg.height / 2.0,
            );
            // Find leaf nodes that should be inside this subgraph.
            // We check if any leaf node overlaps but isn't contained.
            for n in &fr.layout.nodes {
                if n.is_compound || n.id == sg.id {
                    continue;
                }
                let nb = node_bbox(n);
                // If the node center is inside the subgraph, its bbox should be contained
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

// ---------------------------------------------------------------------------
// Invariant: no NaN or Inf in coordinates
// ---------------------------------------------------------------------------

#[test]
fn flowchart_no_nan_inf() {
    let mut failures = Vec::new();
    for fr in load_flowcharts() {
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
                        "{}: edge {}->{} has non-finite point", fr.stem, e.src, e.dst
                    ));
                    break;
                }
            }
        }
    }
    assert!(failures.is_empty(), "NaN/Inf detected:\n{}", failures.join("\n"));
}

#[test]
fn state_no_nan_inf() {
    let mut failures = Vec::new();
    for sr in load_state_diagrams() {
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
                        "{}: edge {}->{} has non-finite point", sr.stem, e.src, e.dst
                    ));
                    break;
                }
            }
        }
    }
    assert!(failures.is_empty(), "NaN/Inf detected:\n{}", failures.join("\n"));
}

// ---------------------------------------------------------------------------
// Invariant: edge points within layout bounds (with tolerance for curves)
// ---------------------------------------------------------------------------

#[test]
fn flowchart_edge_points_within_bounds() {
    let tol = 1.0;
    let mut failures = Vec::new();
    for fr in load_flowcharts() {
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
    assert!(failures.is_empty(), "edge points outside bounds:\n{}", failures.join("\n"));
}

#[test]
fn state_edge_points_within_bounds() {
    let tol = 1.0;
    let mut failures = Vec::new();
    for sr in load_state_diagrams() {
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
    assert!(failures.is_empty(), "edge points outside bounds:\n{}", failures.join("\n"));
}

#[test]
fn state_compounds_contain_children() {
    let tol = 2.0;
    let mut failures = Vec::new();
    for sr in load_state_diagrams() {
        let compounds: Vec<_> = sr.layout.nodes.iter().filter(|n| n.is_compound).collect();
        for compound in &compounds {
            let outer = node_bbox(compound);
            for n in &sr.layout.nodes {
                if n.is_compound || n.id == compound.id {
                    continue;
                }
                // If the node center is inside the compound, its bbox should be contained
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
