use std::fs;
use std::path::Path;

use rusty_mermaid_core::Renderer;
use rusty_mermaid_diagrams::{detect, render_to_scene, DiagramKind};
use rusty_mermaid_svg::SvgRenderer;

fn golden_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/golden/mmd")
}

fn gallery_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/visual/gallery")
}

/// Render all golden .mmd files to SVG and write an HTML gallery index.
#[test]
fn generate_svg_gallery() {
    let gdir = golden_dir();
    let outdir = gallery_dir();
    fs::create_dir_all(&outdir).unwrap();

    let mut entries: Vec<(String, String)> = Vec::new();

    for entry in fs::read_dir(&gdir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "mmd") {
            continue;
        }
        let stem = path.file_stem().unwrap().to_str().unwrap().to_string();
        let text = fs::read_to_string(&path).unwrap();

        // Only render diagram types we support
        let kind = detect(&text);
        if !matches!(kind, Some(DiagramKind::Flowchart) | Some(DiagramKind::State)) {
            continue;
        }

        let scene = match render_to_scene(&text) {
            Ok(s) => s,
            Err(_) => continue, // skip files the parser can't handle yet
        };
        let svg = SvgRenderer.render(&scene);

        let svg_path = outdir.join(format!("{stem}.svg"));
        fs::write(&svg_path, &svg).unwrap();
        entries.push((stem, text));
    }

    entries.sort_by(|a, b| a.0.cmp(&b.0));

    // Generate HTML index
    let mut html = String::from(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>rusty-mermaid SVG Gallery</title>
<style>
  body { font-family: system-ui; max-width: 1200px; margin: 0 auto; padding: 20px; background: #f5f5f5; }
  h1 { color: #333; }
  .card { background: white; border-radius: 8px; padding: 16px; margin: 16px 0; box-shadow: 0 1px 3px rgba(0,0,0,0.1); }
  .card h2 { margin-top: 0; color: #555; font-size: 18px; }
  .card pre { background: #f8f8f8; padding: 8px; border-radius: 4px; font-size: 12px; overflow-x: auto; }
  .card img { max-width: 100%; border: 1px solid #eee; border-radius: 4px; }
</style>
</head>
<body>
<h1>rusty-mermaid SVG Gallery</h1>
"#,
    );

    for (stem, mmd) in &entries {
        html.push_str(&format!(
            r#"<div class="card">
<h2>{stem}</h2>
<img src="{stem}.svg" alt="{stem}">
<details><summary>Source</summary><pre>{mmd}</pre></details>
</div>
"#,
            mmd = mmd.replace('<', "&lt;").replace('>', "&gt;"),
        ));
    }

    html.push_str("</body>\n</html>\n");
    fs::write(outdir.join("index.html"), &html).unwrap();

    // Verify we generated some files
    assert!(!entries.is_empty(), "should have rendered at least one SVG");
}
