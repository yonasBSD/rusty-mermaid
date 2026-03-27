#!/usr/bin/env -S cargo +nightly -Zscript
//! Scan docs/*.md for ```mermaid blocks, render each through rusty-mermaid,
//! save as docs/images/{doc}_{n}.svg, and replace the code block with an
//! image tag + collapsible source.
//!
//! Usage: cargo run --manifest-path docs/render-docs-crate/Cargo.toml

use std::fs;
use std::path::Path;

fn main() {
    let docs_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let images_dir = docs_dir.join("images");
    fs::create_dir_all(&images_dir).unwrap();

    let md_files: Vec<_> = fs::read_dir(&docs_dir)
        .unwrap()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "md"))
        .map(|e| e.path())
        .collect();

    let mut total = 0;
    for md_path in &md_files {
        let content = fs::read_to_string(md_path).unwrap();
        let doc_name = md_path.file_stem().unwrap().to_str().unwrap();
        let (new_content, count) = process_markdown(&content, doc_name, &images_dir);
        if count > 0 {
            fs::write(md_path, new_content).unwrap();
            eprintln!("{}: rendered {} diagrams", doc_name, count);
            total += count;
        }
    }
    eprintln!("Total: {} SVGs in docs/images/", total);
}

fn process_markdown(content: &str, doc_name: &str, images_dir: &Path) -> (String, usize) {
    let mut result = String::with_capacity(content.len());
    let mut count = 0;
    let mut lines = content.lines().peekable();
    let mut details_depth = 0;

    while let Some(line) = lines.next() {
        if line.trim_start().starts_with("<details") {
            details_depth += 1;
        }
        if line.trim_start().starts_with("</details") {
            details_depth -= 1;
        }
        // Only process top-level mermaid blocks (not inside <details>)
        if details_depth == 0 && line.trim_start().starts_with("```mermaid") {
            // Collect mermaid source
            let mut mermaid_src = String::new();
            for inner in lines.by_ref() {
                if inner.trim_start().starts_with("```") {
                    break;
                }
                mermaid_src.push_str(inner);
                mermaid_src.push('\n');
            }

            count += 1;
            let svg_name = format!("{}_{}.svg", doc_name, count);
            let svg_path = images_dir.join(&svg_name);

            // Render through our crate
            match rusty_mermaid_diagrams::render_to_scene(&mermaid_src, &rusty_mermaid_core::Theme::default()) {
                Ok(scene) => {
                    use rusty_mermaid_core::Renderer;
                    let svg = rusty_mermaid_svg::SvgRenderer::new().render(&scene);
                    fs::write(&svg_path, &svg).unwrap();

                    // Image tag
                    result.push_str(&format!("![{}](images/{})\n", doc_name, svg_name));

                    // Collapsible source (plain ``` to prevent GitHub rendering)
                    result.push_str("\n<details>\n<summary>Mermaid source</summary>\n\n");
                    result.push_str("```\n");
                    result.push_str(&mermaid_src);
                    result.push_str("```\n\n</details>\n");
                }
                Err(e) => {
                    eprintln!("  WARN: {}_{}  failed: {}", doc_name, count, e);
                    // Keep original code block
                    result.push_str("```mermaid\n");
                    result.push_str(&mermaid_src);
                    result.push_str("```\n");
                }
            }
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }

    (result, count)
}
