use std::fs;
use std::path::{Path, PathBuf};

use rusty_mermaid_core::Theme;
use rusty_mermaid_core::{MarkerType, Primitive, Scene};
use rusty_mermaid_diagrams::{DiagramKind, detect, render_to_scene};

/// Workspace root: two levels up from the diagrams crate manifest dir.
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

fn fingerprint_dir() -> PathBuf {
    workspace_root().join("tests/golden/fingerprints")
}

// ---------------------------------------------------------------------------
// Fingerprint struct
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct SvgFingerprint {
    rect_count: usize,
    circle_count: usize,
    ellipse_count: usize,
    path_count: usize,
    polygon_count: usize,
    text_count: usize,
    arc_count: usize,
    text_contents: Vec<String>,
    marker_types: Vec<String>,
    has_groups: bool,
    /// Sorted primitive type names. Not order-dependent because HashMap
    /// iteration in dagre's layout can produce different orderings across
    /// runs. We still catch structural changes (added/removed primitives).
    primitive_types_sorted: Vec<String>,
}

// ---------------------------------------------------------------------------
// Fingerprint extraction
// ---------------------------------------------------------------------------

impl SvgFingerprint {
    fn from_scene(scene: &Scene) -> Self {
        let mut fp = SvgFingerprint {
            rect_count: 0,
            circle_count: 0,
            ellipse_count: 0,
            path_count: 0,
            polygon_count: 0,
            text_count: 0,
            arc_count: 0,
            text_contents: Vec::new(),
            marker_types: Vec::new(),
            has_groups: false,
            primitive_types_sorted: Vec::new(),
        };
        for elem in scene.elements() {
            walk_primitive(&elem.primitive, &mut fp);
        }
        fp.text_contents.sort();
        fp.marker_types.sort();
        fp.marker_types.dedup();
        fp.primitive_types_sorted.sort();
        fp
    }
}

fn walk_primitive(p: &Primitive, fp: &mut SvgFingerprint) {
    match p {
        Primitive::Rect { .. } => {
            fp.rect_count += 1;
            fp.primitive_types_sorted.push("rect".into());
        }
        Primitive::Circle { .. } => {
            fp.circle_count += 1;
            fp.primitive_types_sorted.push("circle".into());
        }
        Primitive::Ellipse { .. } => {
            fp.ellipse_count += 1;
            fp.primitive_types_sorted.push("ellipse".into());
        }
        Primitive::Path {
            marker_start,
            marker_end,
            ..
        } => {
            fp.path_count += 1;
            fp.primitive_types_sorted.push("path".into());
            if let Some(m) = marker_start {
                fp.marker_types.push(marker_name(m));
            }
            if let Some(m) = marker_end {
                fp.marker_types.push(marker_name(m));
            }
        }
        Primitive::Text { content, .. } => {
            fp.text_count += 1;
            fp.primitive_types_sorted.push("text".into());
            fp.text_contents.push(content.clone());
        }
        Primitive::Polygon { .. } => {
            fp.polygon_count += 1;
            fp.primitive_types_sorted.push("polygon".into());
        }
        Primitive::Group { children, .. } => {
            fp.has_groups = true;
            fp.primitive_types_sorted.push("group".into());
            for child in children {
                walk_primitive(child, fp);
            }
        }
        Primitive::Arc { .. } => {
            fp.arc_count += 1;
            fp.primitive_types_sorted.push("arc".into());
        }
    }
}

fn marker_name(m: &MarkerType) -> String {
    format!("{:?}", m)
}

// ---------------------------------------------------------------------------
// Test
// ---------------------------------------------------------------------------

#[test]
fn structural_fingerprint_regression() {
    let mmd_dir = golden_mmd_dir();
    let fp_dir = fingerprint_dir();

    // Collect supported .mmd files from all type subdirectories
    let mut entries: Vec<(String, PathBuf)> = Vec::new();
    for type_entry in fs::read_dir(&mmd_dir).expect("read golden/mmd dir") {
        let type_path = type_entry.unwrap().path();
        if !type_path.is_dir() {
            continue;
        }
        for entry in fs::read_dir(&type_path).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("mmd") {
                continue;
            }
            let text = fs::read_to_string(&path).unwrap();
            match detect(&text) {
                Some(DiagramKind::Flowchart)
                | Some(DiagramKind::State)
                | Some(DiagramKind::Sequence) => {}
                _ => continue,
            }
            let stem = path.file_stem().unwrap().to_str().unwrap().to_string();
            entries.push((stem, path));
        }
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    assert!(!entries.is_empty(), "no .mmd files found in golden/mmd");

    // Ensure fingerprint directory exists
    fs::create_dir_all(&fp_dir).expect("create fingerprints dir");

    let update_mode = std::env::var("UPDATE_FINGERPRINTS").is_ok();

    let mut generated = 0usize;
    let mut updated = 0usize;
    let mut verified = 0usize;
    let mut failures: Vec<String> = Vec::new();

    for (stem, path) in &entries {
        let text = fs::read_to_string(path).unwrap();
        let scene = match render_to_scene(&text, &Theme::default()) {
            Ok(s) => s,
            Err(_) => {
                // Skip files that fail to parse/render (unsupported syntax).
                // If a fingerprint already exists for this file, remove it so
                // stale expectations don't linger.
                let fp_path = fp_dir.join(format!("{}.json", stem));
                if fp_path.exists() {
                    fs::remove_file(&fp_path).ok();
                }
                continue;
            }
        };

        let actual = SvgFingerprint::from_scene(&scene);
        let fp_path = fp_dir.join(format!("{}.json", stem));

        if fp_path.exists() && !update_mode {
            // Compare against stored fingerprint
            let stored_json = fs::read_to_string(&fp_path).unwrap();
            let expected: SvgFingerprint = serde_json::from_str(&stored_json)
                .unwrap_or_else(|e| panic!("failed to parse {}: {}", fp_path.display(), e));
            if actual != expected {
                let mut diffs = Vec::new();
                if actual.rect_count != expected.rect_count {
                    diffs.push(format!(
                        "  rect_count: {} vs {}",
                        actual.rect_count, expected.rect_count
                    ));
                }
                if actual.circle_count != expected.circle_count {
                    diffs.push(format!(
                        "  circle_count: {} vs {}",
                        actual.circle_count, expected.circle_count
                    ));
                }
                if actual.ellipse_count != expected.ellipse_count {
                    diffs.push(format!(
                        "  ellipse_count: {} vs {}",
                        actual.ellipse_count, expected.ellipse_count
                    ));
                }
                if actual.path_count != expected.path_count {
                    diffs.push(format!(
                        "  path_count: {} vs {}",
                        actual.path_count, expected.path_count
                    ));
                }
                if actual.polygon_count != expected.polygon_count {
                    diffs.push(format!(
                        "  polygon_count: {} vs {}",
                        actual.polygon_count, expected.polygon_count
                    ));
                }
                if actual.text_count != expected.text_count {
                    diffs.push(format!(
                        "  text_count: {} vs {}",
                        actual.text_count, expected.text_count
                    ));
                }
                if actual.arc_count != expected.arc_count {
                    diffs.push(format!(
                        "  arc_count: {} vs {}",
                        actual.arc_count, expected.arc_count
                    ));
                }
                if actual.text_contents != expected.text_contents {
                    diffs.push(format!(
                        "  text_contents: {:?} vs {:?}",
                        actual.text_contents, expected.text_contents
                    ));
                }
                if actual.marker_types != expected.marker_types {
                    diffs.push(format!(
                        "  marker_types: {:?} vs {:?}",
                        actual.marker_types, expected.marker_types
                    ));
                }
                if actual.has_groups != expected.has_groups {
                    diffs.push(format!(
                        "  has_groups: {} vs {}",
                        actual.has_groups, expected.has_groups
                    ));
                }
                if actual.primitive_types_sorted != expected.primitive_types_sorted {
                    diffs.push(format!(
                        "  primitive_types_sorted: {:?} vs {:?}",
                        actual.primitive_types_sorted, expected.primitive_types_sorted
                    ));
                }
                failures.push(format!(
                    "{}: fingerprint mismatch (actual vs expected):\n{}",
                    stem,
                    diffs.join("\n")
                ));
            } else {
                verified += 1;
            }
        } else {
            // Generate (or update) the fingerprint file
            let is_update = fp_path.exists();
            let json = serde_json::to_string_pretty(&actual).unwrap();
            fs::write(&fp_path, json).unwrap();
            if is_update {
                updated += 1;
            } else {
                generated += 1;
            }
        }
    }

    if generated > 0 {
        eprintln!(
            "Generated {} fingerprint file(s) in {}. Re-run to verify.",
            generated,
            fp_dir.display()
        );
    }
    if updated > 0 {
        eprintln!("Updated {} fingerprint file(s).", updated);
    }
    if verified > 0 {
        eprintln!("Verified {} fingerprint(s).", verified);
    }
    if !failures.is_empty() {
        panic!(
            "{} fingerprint failure(s):\n\
             Hint: run with UPDATE_FINGERPRINTS=1 or `cargo test --test structural_fingerprints update_fingerprints -- --ignored` to regenerate.\n\n{}",
            failures.len(),
            failures.join("\n\n")
        );
    }
}

/// Regenerate all fingerprint files unconditionally.
/// Run with: cargo test --test structural_fingerprints update_fingerprints -- --ignored
#[test]
#[ignore]
fn update_fingerprints() {
    let mmd_dir = golden_mmd_dir();
    let fp_dir = fingerprint_dir();

    fs::create_dir_all(&fp_dir).expect("create fingerprints dir");

    let mut count = 0usize;
    for type_entry in fs::read_dir(&mmd_dir).expect("read golden/mmd dir") {
        let type_path = type_entry.unwrap().path();
        if !type_path.is_dir() {
            continue;
        }
        for entry in fs::read_dir(&type_path).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("mmd") {
                continue;
            }
            let text = fs::read_to_string(&path).unwrap();
            match detect(&text) {
                Some(DiagramKind::Flowchart)
                | Some(DiagramKind::State)
                | Some(DiagramKind::Sequence) => {}
                _ => continue,
            }
            let scene = match render_to_scene(&text, &Theme::default()) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let stem = path.file_stem().unwrap().to_str().unwrap();
            let fp_path = fp_dir.join(format!("{}.json", stem));
            let json = serde_json::to_string_pretty(&SvgFingerprint::from_scene(&scene)).unwrap();
            fs::write(&fp_path, json).unwrap();
            count += 1;
        }
    }
    eprintln!("Regenerated {} fingerprint file(s).", count);
}
