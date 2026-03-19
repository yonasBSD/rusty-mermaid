use std::fs;
use std::path::{Path, PathBuf};

use rusty_mermaid_core::Renderer;
use rusty_mermaid_diagrams::{detect, render_to_scene, DiagramKind};
use rusty_mermaid_svg::SvgRenderer;

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

fn golden_svg_dir() -> PathBuf {
    workspace_root().join("tests/golden/svg")
}

fn renderable_entries() -> Vec<(String, PathBuf)> {
    let mmd_dir = golden_mmd_dir();
    let mut entries: Vec<(String, PathBuf)> = Vec::new();
    for entry in fs::read_dir(&mmd_dir).expect("read golden/mmd dir") {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("mmd") {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap();
        match detect(&text) {
            Some(DiagramKind::Flowchart) | Some(DiagramKind::State) => {}
            _ => continue,
        }
        if render_to_scene(&text).is_err() {
            continue;
        }
        let stem = path.file_stem().unwrap().to_str().unwrap().to_string();
        entries.push((stem, path));
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    entries
}

#[test]
fn svg_golden_regression() {
    let svg_dir = golden_svg_dir();
    let entries = renderable_entries();
    assert!(!entries.is_empty(), "no renderable .mmd files found");

    fs::create_dir_all(&svg_dir).expect("create golden/svg dir");

    let update_mode = std::env::var("UPDATE_GOLDEN_SVG").is_ok();

    let mut generated = 0usize;
    let mut updated = 0usize;
    let mut verified = 0usize;
    let mut failures: Vec<String> = Vec::new();

    for (stem, path) in &entries {
        let text = fs::read_to_string(path).unwrap();
        let scene = render_to_scene(&text).unwrap();
        let actual_svg = SvgRenderer.render(&scene);
        let svg_path = svg_dir.join(format!("{stem}.svg"));

        if svg_path.exists() && !update_mode {
            let expected_svg = fs::read_to_string(&svg_path).unwrap();
            if actual_svg != expected_svg {
                let actual_lines: Vec<&str> = actual_svg.lines().collect();
                let expected_lines: Vec<&str> = expected_svg.lines().collect();
                let mut diff_line = None;
                for (i, (a, e)) in actual_lines.iter().zip(expected_lines.iter()).enumerate() {
                    if a != e {
                        diff_line = Some((i + 1, (*a).to_string(), (*e).to_string()));
                        break;
                    }
                }
                if diff_line.is_none() && actual_lines.len() != expected_lines.len() {
                    diff_line = Some((
                        actual_lines.len().min(expected_lines.len()) + 1,
                        format!("<{} lines>", actual_lines.len()),
                        format!("<{} lines>", expected_lines.len()),
                    ));
                }
                let detail = match diff_line {
                    Some((line, actual, expected)) => {
                        format!("  first diff at line {line}:\n    actual:   {actual}\n    expected: {expected}")
                    }
                    None => "  (unknown diff)".to_string(),
                };
                failures.push(format!("{stem}: SVG mismatch\n{detail}"));
            } else {
                verified += 1;
            }
        } else {
            let is_update = svg_path.exists();
            fs::write(&svg_path, &actual_svg).unwrap();
            if is_update {
                updated += 1;
            } else {
                generated += 1;
            }
        }
    }

    if generated > 0 {
        eprintln!(
            "Generated {generated} golden SVG file(s) in {}. Re-run to verify.",
            svg_dir.display()
        );
    }
    if updated > 0 {
        eprintln!("Updated {updated} golden SVG file(s).");
    }
    if verified > 0 {
        eprintln!("Verified {verified} golden SVG(s) — byte-exact match.");
    }
    if !failures.is_empty() {
        panic!(
            "{} SVG regression failure(s):\n\
             Hint: run with UPDATE_GOLDEN_SVG=1 to regenerate golden files.\n\n{}",
            failures.len(),
            failures.join("\n\n")
        );
    }
}

/// Regenerate all golden SVG files unconditionally.
/// Run with: cargo test --test svg_golden update_golden_svg -- --ignored
#[test]
#[ignore]
fn update_golden_svg() {
    let svg_dir = golden_svg_dir();
    fs::create_dir_all(&svg_dir).expect("create golden/svg dir");

    let entries = renderable_entries();
    let mut count = 0usize;
    for (stem, path) in &entries {
        let text = fs::read_to_string(path).unwrap();
        let scene = render_to_scene(&text).unwrap();
        let svg = SvgRenderer.render(&scene);
        fs::write(svg_dir.join(format!("{stem}.svg")), &svg).unwrap();
        count += 1;
    }
    eprintln!("Regenerated {count} golden SVG file(s).");
}
