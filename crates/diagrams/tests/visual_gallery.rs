use std::fs;
use std::path::Path;

use rusty_mermaid_core::Renderer;
use rusty_mermaid_diagrams::render_to_scene;
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

    let mut entries: Vec<(String, String, String)> = Vec::new();

    for type_entry in fs::read_dir(&gdir).unwrap() {
        let type_path = type_entry.unwrap().path();
        if !type_path.is_dir() {
            continue;
        }
        let type_name = type_path.file_name().unwrap().to_str().unwrap().to_string();

        let type_outdir = outdir.join(&type_name);
        fs::create_dir_all(&type_outdir).unwrap();

        for entry in fs::read_dir(&type_path).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_none_or(|e| e != "mmd") {
                continue;
            }
            let stem = path.file_stem().unwrap().to_str().unwrap().to_string();
            let text = fs::read_to_string(&path).unwrap();

            let scene = match render_to_scene(&text) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let svg = SvgRenderer.render(&scene);

            let svg_path = type_outdir.join(format!("{stem}.svg"));
            fs::write(&svg_path, &svg).unwrap();
            entries.push((type_name.clone(), stem, text));
        }
    }

    entries.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));

    let gallery_css = r#"  body { font-family: system-ui; max-width: 1200px; margin: 0 auto; padding: 20px; background: #f5f5f5; }
  h1 { color: #333; }
  .card { background: white; border-radius: 8px; padding: 16px; margin: 16px 0; box-shadow: 0 1px 3px rgba(0,0,0,0.1); }
  .card h2 { margin-top: 0; color: #555; font-size: 18px; }
  .card pre { background: #f8f8f8; padding: 8px; border-radius: 4px; font-size: 12px; overflow-x: auto; }
  .card img { max-width: 100%; border: 1px solid #eee; border-radius: 4px; }
  a { color: #9370db; }"#;

    // Collect all type subdirectories (including empty ones like sequence/)
    let mut types: Vec<String> = Vec::new();
    for type_entry in fs::read_dir(&gdir).unwrap() {
        let type_path = type_entry.unwrap().path();
        if type_path.is_dir() {
            types.push(type_path.file_name().unwrap().to_str().unwrap().to_string());
        }
    }
    types.sort();

    // Per-type gallery: each type gets its own index.html
    for type_name in &types {
        let type_outdir = outdir.join(type_name);
        fs::create_dir_all(&type_outdir).unwrap();
        let type_entries: Vec<_> = entries.iter().filter(|(t, _, _)| t == type_name).collect();
        let mut html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>{type_name} — rusty-mermaid Gallery</title>
<style>
{gallery_css}
</style>
</head>
<body>
<p><a href="../index.html">&larr; all diagrams</a></p>
<h1>{type_name}</h1>
"#,
        );

        for (_, stem, mmd) in &type_entries {
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

        if type_entries.is_empty() {
            html.push_str("<p>No diagrams yet.</p>\n");
        }

        html.push_str("</body>\n</html>\n");
        fs::write(type_outdir.join("index.html"), &html).unwrap();
    }

    // Top-level index: links to per-type galleries
    let mut html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>rusty-mermaid SVG Gallery</title>
<style>
{gallery_css}
</style>
</head>
<body>
<h1>rusty-mermaid SVG Gallery</h1>
<ul>
"#,
    );

    for type_name in &types {
        let count = entries.iter().filter(|(t, _, _)| t == type_name).count();
        html.push_str(&format!(
            "<li><a href=\"{type_name}/index.html\">{type_name}</a> ({count} diagrams)</li>\n"
        ));
    }

    html.push_str("</ul>\n</body>\n</html>\n");
    fs::write(outdir.join("index.html"), &html).unwrap();

    // Verify we generated some files
    assert!(!entries.is_empty(), "should have rendered at least one SVG");
}
