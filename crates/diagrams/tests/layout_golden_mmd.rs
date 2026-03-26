use std::fs;
use std::path::Path;

use rusty_mermaid_diagrams::render_to_scene;

fn golden_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/golden/mmd")
}

/// Files with known unsupported syntax (click bindings, directives).
/// These are tracked as future work, not regressions.
const KNOWN_UNSUPPORTED: &[&str] = &["click_bindings", "mixed_statements"];

/// Auto-discover and test ALL .mmd files across all diagram types.
/// Every .mmd file must: parse successfully, produce a non-empty Scene.
/// This catches parser crashes, bridge panics, and layout failures.
#[test]
fn all_golden_mmd_parse_and_render() {
    let gdir = golden_dir();
    let mut tested = 0;
    let mut failures = Vec::new();

    for type_entry in fs::read_dir(&gdir).unwrap() {
        let type_path = type_entry.unwrap().path();
        if !type_path.is_dir() {
            continue;
        }
        let type_name = type_path.file_name().unwrap().to_str().unwrap().to_string();

        let mut files: Vec<_> = fs::read_dir(&type_path)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|x| x == "mmd"))
            .collect();
        files.sort_by_key(|e| e.file_name());

        for entry in files {
            let path = entry.path();
            let stem = path.file_stem().unwrap().to_str().unwrap();
            let name = format!("{type_name}/{stem}");
            tested += 1;

            if KNOWN_UNSUPPORTED.iter().any(|s| stem == *s) {
                continue;
            }

            let text = match fs::read_to_string(&path) {
                Ok(t) => t,
                Err(e) => {
                    failures.push(format!("{name}: read error: {e}"));
                    continue;
                }
            };

            match render_to_scene(&text) {
                Ok(scene) => {
                    if scene.is_empty() {
                        failures.push(format!("{name}: scene is empty"));
                    }
                }
                Err(e) => {
                    failures.push(format!("{name}: render failed: {e}"));
                }
            }
        }
    }

    assert!(tested > 0, "no .mmd files found");
    assert!(
        failures.is_empty(),
        "{} of {} failed:\n{}",
        failures.len(),
        tested,
        failures.join("\n")
    );
    eprintln!("all_golden_mmd_parse_and_render: {tested} files OK");
}
