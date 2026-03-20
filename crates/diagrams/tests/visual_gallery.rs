use std::fs;
use std::path::Path;

use rusty_mermaid_core::{Renderer, Theme};
use rusty_mermaid_diagrams::render_to_scene_themed;
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

const GALLERY_CSS: &str = r#"
  :root { --bg: #f5f5f5; --card-bg: white; --text: #333; --text2: #555; --pre-bg: #f8f8f8; --border: #eee; --shadow: rgba(0,0,0,0.1); }
  :root.dark { --bg: #1e1e2e; --card-bg: #2d2d44; --text: #cdd6f4; --text2: #bac2de; --pre-bg: #252538; --border: #45475a; --shadow: rgba(0,0,0,0.4); }
  body { font-family: system-ui; max-width: 1200px; margin: 0 auto; padding: 20px; background: var(--bg); color: var(--text); transition: background 0.2s, color 0.2s; }
  h1 { color: var(--text); }
  .card { background: var(--card-bg); border-radius: 8px; padding: 16px; margin: 16px 0; box-shadow: 0 1px 3px var(--shadow); transition: background 0.2s; }
  .card h2 { margin-top: 0; color: var(--text2); font-size: 18px; }
  .card pre { background: var(--pre-bg); padding: 8px; border-radius: 4px; font-size: 12px; overflow-x: auto; color: var(--text); transition: background 0.2s; }
  .card img { max-width: 100%; border: 1px solid var(--border); border-radius: 4px; }
  a { color: #9370db; }
  .header { display: flex; justify-content: space-between; align-items: center; }
  .theme-btn { background: var(--card-bg); border: 1px solid var(--border); border-radius: 6px; padding: 6px 14px; cursor: pointer; font-size: 14px; color: var(--text); transition: background 0.2s; }
  .theme-btn:hover { opacity: 0.8; }
"#;

const THEME_JS: &str = r#"
<script>
(function() {
  var root = document.documentElement;
  var saved = localStorage.getItem('rm-theme') || 'light';
  if (saved === 'dark') root.classList.add('dark');

  function applyTheme(theme) {
    var suffix = theme === 'dark' ? '_dark' : '';
    root.classList.toggle('dark', theme === 'dark');
    document.querySelectorAll('img[data-stem]').forEach(function(img) {
      img.src = img.dataset.stem + suffix + '.svg';
    });
    localStorage.setItem('rm-theme', theme);
    var btn = document.querySelector('.theme-btn');
    if (btn) btn.textContent = theme === 'dark' ? 'Light' : 'Dark';
  }

  applyTheme(saved);

  document.addEventListener('click', function(e) {
    if (e.target.classList.contains('theme-btn')) {
      applyTheme(root.classList.contains('dark') ? 'light' : 'dark');
    }
  });
})();
</script>
"#;

/// Render all golden .mmd files to SVG and write an HTML gallery index.
#[test]
fn generate_svg_gallery() {
    let gdir = golden_dir();
    let outdir = gallery_dir();
    fs::create_dir_all(&outdir).unwrap();

    let light = Theme::light();
    let dark = Theme::dark();
    let renderer = SvgRenderer::new();

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

            // Light theme SVG
            let scene_light = match render_to_scene_themed(&text, &light) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let svg_light = renderer.render(&scene_light);
            fs::write(type_outdir.join(format!("{stem}.svg")), &svg_light).unwrap();

            // Dark theme SVG
            if let Ok(scene_dark) = render_to_scene_themed(&text, &dark) {
                let svg_dark = renderer.render(&scene_dark);
                fs::write(type_outdir.join(format!("{stem}_dark.svg")), &svg_dark).unwrap();
            }

            entries.push((type_name.clone(), stem, text));
        }
    }

    entries.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));

    // Collect all type subdirectories
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
<style>{GALLERY_CSS}</style>
</head>
<body>
<div class="header">
  <div><a href="../index.html">&larr; all diagrams</a></div>
  <button class="theme-btn">Dark</button>
</div>
<h1>{type_name}</h1>
"#,
        );

        for (_, stem, mmd) in &type_entries {
            html.push_str(&format!(
                r#"<div class="card">
<h2>{stem}</h2>
<img src="{stem}.svg" alt="{stem}" data-stem="{stem}">
<details><summary>Source</summary><pre>{mmd}</pre></details>
</div>
"#,
                mmd = mmd.replace('<', "&lt;").replace('>', "&gt;"),
            ));
        }

        if type_entries.is_empty() {
            html.push_str("<p>No diagrams yet.</p>\n");
        }

        html.push_str(THEME_JS);
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
<style>{GALLERY_CSS}</style>
</head>
<body>
<div class="header">
  <h1>rusty-mermaid SVG Gallery</h1>
  <button class="theme-btn">Dark</button>
</div>
<ul>
"#,
    );

    for type_name in &types {
        let count = entries.iter().filter(|(t, _, _)| t == type_name).count();
        html.push_str(&format!(
            "<li><a href=\"{type_name}/index.html\">{type_name}</a> ({count} diagrams)</li>\n"
        ));
    }

    html.push_str("</ul>\n");
    html.push_str(THEME_JS);
    html.push_str("</body>\n</html>\n");
    fs::write(outdir.join("index.html"), &html).unwrap();

    // Verify we generated some files
    assert!(!entries.is_empty(), "should have rendered at least one SVG");
}
