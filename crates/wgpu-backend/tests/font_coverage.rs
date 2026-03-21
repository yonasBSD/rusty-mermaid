use skrifa::MetadataProvider;

#[test]
fn check_font_coverage() {
    let fonts: Vec<(&str, &[u8])> = vec![
        ("IntelOneMono", include_bytes!("../../raster/fonts/IntelOneMono-Regular.ttf")),
        ("NotoSans", include_bytes!("../../raster/fonts/NotoSans-Regular.ttf")),
        ("NotoSansMono", include_bytes!("../../raster/fonts/NotoSansMono-Regular.ttf")),
        ("NotoSansSymbols2", include_bytes!("../../raster/fonts/NotoSansSymbols2-Regular.ttf")),
    ];
    let chars = ['→', '←', '↑', '↓', '☕', '✔', '✘', '★', '☆', 'α', 'β', 'П', 'م', '你'];
    for (name, bytes) in &fonts {
        let font = skrifa::FontRef::new(bytes).unwrap();
        let cm = skrifa::MetadataProvider::charmap(&font);
        let mut results = Vec::new();
        for &ch in &chars {
            let gid = cm.map(ch).map(|g| g.to_u32()).unwrap_or(0);
            results.push(format!("{}:{}", ch, if gid != 0 { "Y" } else { "N" }));
        }
        eprintln!("{name:20} {}", results.join("  "));
    }
}
