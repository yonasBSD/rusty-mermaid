use skrifa::MetadataProvider;

#[test]
fn measure_real_advance() {
    let bytes = include_bytes!("../../raster/fonts/IntelOneMono-Regular.ttf");
    let font = skrifa::FontRef::new(bytes).unwrap();
    let gm = font.glyph_metrics(skrifa::instance::Size::new(14.0), skrifa::instance::LocationRef::default());
    let cm = font.charmap();

    for ch in ['A', 'a', 'W', 'i', ' ', '0'] {
        let gid = cm.map(ch).unwrap();
        let advance = gm.advance_width(gid).unwrap();
        eprintln!("  '{ch}' advance at 14px: {advance:.4}  (SimpleTextMeasure uses 8.4)");
    }

    // Check 100 chars
    let gid_a = cm.map('A').unwrap();
    let advance = gm.advance_width(gid_a).unwrap();
    let error_100 = (advance - 8.4) * 100.0;
    eprintln!("\n  Error over 100 chars: {error_100:.1}px");
    eprintln!("  Actual advance: {advance:.4}");
}
