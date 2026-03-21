use std::fs;
use std::path::Path;

use rusty_mermaid_core::Theme;
use rusty_mermaid_diagrams::render_to_scene;
use rusty_mermaid_wgpu::GpuRenderer;

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
        .join("tests/visual/gallery_gpu")
}

#[test]
fn generate_gpu_gallery() {
    let gdir = golden_dir();
    let outdir = gallery_dir();
    fs::create_dir_all(&outdir).unwrap();

    let theme = Theme::light();

    // Single GPU device for all renders
    let mut gpu = GpuRenderer::new();

    let mut entries: Vec<(String, String)> = Vec::new();

    for type_entry in fs::read_dir(&gdir).unwrap() {
        let type_path = type_entry.unwrap().path();
        if !type_path.is_dir() { continue; }
        let type_name = type_path.file_name().unwrap().to_str().unwrap().to_string();
        let type_outdir = outdir.join(&type_name);
        fs::create_dir_all(&type_outdir).unwrap();

        let mut mmd_files: Vec<_> = fs::read_dir(&type_path)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|x| x == "mmd"))
            .collect();
        mmd_files.sort_by_key(|e| e.file_name());

        for entry in mmd_files {
            let path = entry.path();
            let stem = path.file_stem().unwrap().to_str().unwrap().to_string();
            let input = fs::read_to_string(&path).unwrap();
            let Ok(scene) = render_to_scene(&input) else { continue };

            let png = gpu.render_scene_to_png(&scene, &theme, 2.0);
            let out_path = type_outdir.join(format!("{stem}.png"));
            fs::write(&out_path, &png).unwrap();
            entries.push((format!("{type_name}/{stem}"), format!("{type_name}/{stem}.png")));
            eprintln!("  rendered {type_name}/{stem}");
        }
    }

    entries.sort();

    // Generate HTML index
    let mut html = String::from(r#"<!DOCTYPE html>
<html><head><title>rusty-mermaid GPU Gallery (vello/wgpu)</title>
<style>
  body { font-family: system-ui; max-width: 1200px; margin: 0 auto; padding: 20px; background: #f5f5f5; }
  h1 { color: #333; }
  .card { background: white; border-radius: 8px; padding: 16px; margin: 16px 0; box-shadow: 0 1px 3px rgba(0,0,0,0.1); }
  .card h2 { margin-top: 0; color: #555; font-size: 18px; }
  .card img { max-width: 100%; border: 1px solid #eee; border-radius: 4px; }
</style></head><body>
<h1>rusty-mermaid GPU Gallery (vello/wgpu)</h1>
<p>Rendered via vello compute shaders on GPU (Metal/Vulkan/DX12). Single device, batch rendering.</p>
"#);

    for (name, img_path) in &entries {
        html.push_str(&format!(
            "<div class=\"card\"><h2>{name}</h2><img src=\"{img_path}\" /></div>\n"
        ));
    }
    html.push_str("</body></html>");
    fs::write(outdir.join("index.html"), html).unwrap();

    eprintln!("GPU gallery: {} diagrams → {}", entries.len(), outdir.display());
}
