use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;

fn main() {
    let mmd_dir = Path::new("../../tests/golden/mmd");
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir).join("diagrams.rs");
    let mut out = fs::File::create(&out_path).unwrap();

    let mut entries: Vec<(String, String)> = Vec::new();

    // Collect all .mmd files from subdirectories
    if let Ok(dirs) = fs::read_dir(mmd_dir) {
        for dir_entry in dirs.flatten() {
            let dir_path = dir_entry.path();
            if dir_path.is_dir() {
                let type_name = dir_path.file_name().unwrap().to_str().unwrap().to_string();
                if let Ok(files) = fs::read_dir(&dir_path) {
                    for file_entry in files.flatten() {
                        let file_path = file_entry.path();
                        if file_path.extension().is_some_and(|e| e == "mmd") {
                            let stem = file_path.file_stem().unwrap().to_str().unwrap().to_string();
                            let content = fs::read_to_string(&file_path).unwrap();
                            entries.push((format!("{type_name}/{stem}"), content));
                        }
                    }
                }
            }
        }
    }

    entries.sort_by(|a, b| a.0.cmp(&b.0));

    writeln!(out, "pub static DIAGRAMS: &[(&str, &str)] = &[").unwrap();
    for (name, content) in &entries {
        let escaped = content
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n");
        writeln!(out, "    (\"{name}\", \"{escaped}\"),").unwrap();
    }
    writeln!(out, "];").unwrap();

    // Tell cargo to re-run if mmd files change
    println!("cargo:rerun-if-changed=../../tests/golden/mmd");
}
