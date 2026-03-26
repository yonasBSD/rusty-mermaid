//! Universal scene invariant tests for ALL diagram types.
//!
//! For every .mmd golden file, renders to Scene and checks:
//! 1. Scene has positive dimensions
//! 2. Scene is non-empty (has elements)
//! 3. All coordinates are finite (no NaN/Inf)
//! 4. All dimensions are non-negative
//! 5. All text positions are within scene bounds (with tolerance)

use std::fs;
use std::path::{Path, PathBuf};

use rusty_mermaid_core::{Primitive, Scene};
use rusty_mermaid_diagrams::render_to_scene;

fn golden_mmd_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/golden/mmd")
}

/// Collect all .mmd files across all diagram type directories.
fn all_golden_files() -> Vec<(String, PathBuf)> {
    let mmd_dir = golden_mmd_dir();
    let mut files = Vec::new();
    for type_entry in fs::read_dir(&mmd_dir).unwrap() {
        let type_path = type_entry.unwrap().path();
        if !type_path.is_dir() {
            continue;
        }
        let type_name = type_path.file_name().unwrap().to_str().unwrap().to_string();
        for entry in fs::read_dir(&type_path).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) == Some("mmd") {
                let stem = path.file_stem().unwrap().to_str().unwrap().to_string();
                let label = format!("{}/{}", type_name, stem);
                files.push((label, path));
            }
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

fn check_scene_invariants(scene: &Scene, label: &str) -> Vec<String> {
    let mut failures = Vec::new();

    // 1. Positive dimensions
    if scene.width <= 0.0 || scene.height <= 0.0 {
        failures.push(format!(
            "{}: scene dimensions {}x{}",
            label, scene.width, scene.height
        ));
    }

    // 2. Non-empty
    if scene.is_empty() {
        failures.push(format!("{}: scene is empty", label));
    }

    // 3-5. Check all elements
    let tol = 100.0; // tolerance for elements near edges (labels can extend beyond)
    for elem in scene.elements() {
        match &elem.primitive {
            Primitive::Rect { bbox, .. } => {
                if !bbox.x.is_finite() || !bbox.y.is_finite() {
                    failures.push(format!(
                        "{}: rect at NaN/Inf ({}, {})",
                        label, bbox.x, bbox.y
                    ));
                }
                if bbox.width < 0.0 || bbox.height < 0.0 {
                    failures.push(format!(
                        "{}: rect negative size {}x{}",
                        label, bbox.width, bbox.height
                    ));
                }
            }
            Primitive::Text { position, .. } => {
                if !position.x.is_finite() || !position.y.is_finite() {
                    failures.push(format!(
                        "{}: text at NaN/Inf ({}, {})",
                        label, position.x, position.y
                    ));
                }
                if position.x < -tol
                    || position.y < -tol
                    || position.x > scene.width + tol
                    || position.y > scene.height + tol
                {
                    failures.push(format!(
                        "{}: text at ({:.0}, {:.0}) outside scene {}x{}",
                        label, position.x, position.y, scene.width, scene.height
                    ));
                }
            }
            Primitive::Circle { center, radius, .. } => {
                if !center.x.is_finite() || !center.y.is_finite() || !radius.is_finite() {
                    failures.push(format!("{}: circle at NaN/Inf", label));
                }
            }
            Primitive::Path { segments, .. } => {
                for seg in segments {
                    let points = match seg {
                        rusty_mermaid_core::PathSegment::MoveTo(p) => vec![p],
                        rusty_mermaid_core::PathSegment::LineTo(p) => vec![p],
                        rusty_mermaid_core::PathSegment::CubicTo { cp1, cp2, to } => {
                            vec![cp1, cp2, to]
                        }
                        rusty_mermaid_core::PathSegment::QuadTo { cp, to } => vec![cp, to],
                        _ => vec![],
                    };
                    for p in points {
                        if !p.x.is_finite() || !p.y.is_finite() {
                            failures
                                .push(format!("{}: path point NaN/Inf ({}, {})", label, p.x, p.y));
                        }
                    }
                }
            }
            Primitive::Polygon { points, .. } => {
                for p in points {
                    if !p.x.is_finite() || !p.y.is_finite() {
                        failures.push(format!(
                            "{}: polygon point NaN/Inf ({}, {})",
                            label, p.x, p.y
                        ));
                    }
                }
            }
            Primitive::Ellipse { center, .. } => {
                if !center.x.is_finite() || !center.y.is_finite() {
                    failures.push(format!("{}: ellipse at NaN/Inf", label));
                }
            }
            Primitive::Group { children, .. } => {
                for child in children {
                    if let Primitive::Text { position, .. } = child {
                        if !position.x.is_finite() || !position.y.is_finite() {
                            failures.push(format!("{}: group text at NaN/Inf", label));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    failures
}

#[test]
fn all_golden_scenes_have_positive_dimensions() {
    let files = all_golden_files();
    let mut failures = Vec::new();
    for (label, path) in &files {
        let text = fs::read_to_string(path).unwrap();
        let scene = match render_to_scene(&text) {
            Ok(s) => s,
            Err(_) => continue, // parse errors caught by parse_golden tests
        };
        if scene.width <= 0.0 || scene.height <= 0.0 {
            failures.push(format!("{}: {}x{}", label, scene.width, scene.height));
        }
    }
    assert!(
        failures.is_empty(),
        "Scenes with non-positive dimensions:\n{}",
        failures.join("\n")
    );
}

#[test]
fn all_golden_scenes_are_non_empty() {
    let files = all_golden_files();
    let mut failures = Vec::new();
    for (label, path) in &files {
        let text = fs::read_to_string(path).unwrap();
        let scene = match render_to_scene(&text) {
            Ok(s) => s,
            Err(_) => continue,
        };
        if scene.is_empty() {
            failures.push(label.clone());
        }
    }
    assert!(
        failures.is_empty(),
        "Empty scenes:\n{}",
        failures.join("\n")
    );
}

#[test]
fn all_golden_scenes_have_finite_coordinates() {
    let files = all_golden_files();
    let mut all_failures = Vec::new();
    for (label, path) in &files {
        let text = fs::read_to_string(path).unwrap();
        let scene = match render_to_scene(&text) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let failures = check_scene_invariants(&scene, label);
        all_failures.extend(failures);
    }
    assert!(
        all_failures.is_empty(),
        "Invariant violations:\n{}",
        all_failures.join("\n")
    );
}

#[test]
fn all_golden_scenes_have_reasonable_size() {
    let files = all_golden_files();
    let mut failures = Vec::new();
    let max_dim = 6000.0; // no scene should be larger than 6000px
    for (label, path) in &files {
        let text = fs::read_to_string(path).unwrap();
        let scene = match render_to_scene(&text) {
            Ok(s) => s,
            Err(_) => continue,
        };
        if scene.width > max_dim || scene.height > max_dim {
            failures.push(format!("{}: {}x{}", label, scene.width, scene.height));
        }
    }
    assert!(
        failures.is_empty(),
        "Oversized scenes:\n{}",
        failures.join("\n")
    );
}
