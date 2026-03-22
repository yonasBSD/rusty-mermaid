use std::sync::{Arc, OnceLock};

use vello::kurbo::{self, Affine, BezPath, Cap, Join, Point as KPoint, Rect, RoundedRect, Stroke, Vec2};
use vello::peniko::{Blob, Color as VelloColor, Fill, FontData};
use vello::Scene as VelloScene;

use rusty_mermaid_core::{
    marker_geometry, parse_inline_markdown, transform_marker_circle, transform_marker_curves,
    transform_marker_points, Color, MarkerShape, MarkerType, PathSegment, Point, Primitive, Style,
    TextAnchor, Theme, Transform,
};
use rusty_mermaid_viewport::ViewportState;

/// Build a vello Scene from a rusty-mermaid Scene.
pub fn build_vello_scene(
    scene: &rusty_mermaid_core::Scene,
    theme: &Theme,
    viewport: &ViewportState,
) -> VelloScene {
    let mut vscene = VelloScene::new();

    let padding = theme.padding;
    let zoom = viewport.zoom;
    let ox = viewport.offset.x + padding;
    let oy = viewport.offset.y + padding;
    let transform = Affine::translate(Vec2::new(ox, oy)) * Affine::scale(zoom);

    for elem in scene.elements() {
        paint_primitive(&mut vscene, &elem.primitive, transform, theme);
    }

    vscene
}

fn paint_primitive(
    scene: &mut VelloScene,
    prim: &Primitive,
    transform: Affine,
    theme: &Theme,
) {
    match prim {
        Primitive::Rect { bbox, rx, ry, style } => {
            let left = bbox.x - bbox.width / 2.0;
            let top = bbox.y - bbox.height / 2.0;
            let rect = Rect::new(left, top, left + bbox.width, top + bbox.height);
            let r = rx.max(*ry);

            if r > 0.0 {
                let rrect = RoundedRect::from_rect(rect, r);
                if let Some(fill) = style.fill {
                    scene.fill(Fill::NonZero, transform, to_vello_color(fill), None, &rrect);
                }
                if let Some((color, width)) = resolve_stroke(style, theme) {
                    let stroke = make_stroke(width, style);
                    scene.stroke(&stroke, transform, to_vello_color(color), None, &rrect);
                }
            } else {
                if let Some(fill) = style.fill {
                    scene.fill(Fill::NonZero, transform, to_vello_color(fill), None, &rect);
                }
                if let Some((color, width)) = resolve_stroke(style, theme) {
                    let stroke = make_stroke(width, style);
                    scene.stroke(&stroke, transform, to_vello_color(color), None, &rect);
                }
            }
        }

        Primitive::Circle { center, radius, style } => {
            let circle = kurbo::Circle::new(KPoint::new(center.x, center.y), *radius);

            if let Some(fill) = style.fill {
                scene.fill(Fill::NonZero, transform, to_vello_color(fill), None, &circle);
            }
            if let Some((color, width)) = resolve_stroke(style, theme) {
                let stroke = make_stroke(width, style);
                scene.stroke(&stroke, transform, to_vello_color(color), None, &circle);
            }
        }

        Primitive::Ellipse { center, rx, ry, style } => {
            let ellipse = kurbo::Ellipse::new(KPoint::new(center.x, center.y), Vec2::new(*rx, *ry), 0.0);

            if let Some(fill) = style.fill {
                scene.fill(Fill::NonZero, transform, to_vello_color(fill), None, &ellipse);
            }
            if let Some((color, width)) = resolve_stroke(style, theme) {
                let stroke = make_stroke(width, style);
                scene.stroke(&stroke, transform, to_vello_color(color), None, &ellipse);
            }
        }

        Primitive::Path { segments, style, marker_start, marker_end } => {
            let path = segments_to_bezpath(segments);

            if let Some(fill) = style.fill {
                scene.fill(Fill::NonZero, transform, to_vello_color(fill), None, &path);
            }

            let (color, width) = stroke_or_default(style, theme);
            let stroke = make_stroke(width, style);
            scene.stroke(&stroke, transform, to_vello_color(color), None, &path);

            if let Some(marker) = marker_start {
                if let Some((tip, angle)) = path_start_tangent(segments) {
                    paint_marker(scene, *marker, tip, angle, color, width, transform);
                }
            }
            if let Some(marker) = marker_end {
                if let Some((tip, angle)) = path_end_tangent(segments) {
                    paint_marker(scene, *marker, tip, angle, color, width, transform);
                }
            }
        }

        Primitive::Text { position, content, anchor, style } => {
            render_text(scene, position, content, *anchor, style, theme, transform);
        }

        Primitive::Polygon { points, style } => {
            if points.len() < 3 { return; }
            let mut path = BezPath::new();
            path.move_to(to_kpoint(&points[0]));
            for p in &points[1..] {
                path.line_to(to_kpoint(p));
            }
            path.close_path();

            if let Some(fill) = style.fill {
                scene.fill(Fill::NonZero, transform, to_vello_color(fill), None, &path);
            }
            if let Some((color, width)) = resolve_stroke(style, theme) {
                let stroke = make_stroke(width, style);
                scene.stroke(&stroke, transform, to_vello_color(color), None, &path);
            }
        }

        Primitive::Group { transform: group_tf, children } => {
            let child_transform = transform * compose_affine(group_tf);
            for child in children {
                paint_primitive(scene, child, child_transform, theme);
            }
        }

        Primitive::Arc { center, inner_r, outer_r, start_angle, end_angle, style } => {
            paint_arc(scene, center, *inner_r, *outer_r, *start_angle, *end_angle, style, theme, transform);
        }
    }
}

// ── Helpers ──

fn to_vello_color(c: Color) -> VelloColor {
    VelloColor::from_rgba8(c.r, c.g, c.b, c.a)
}

fn to_kpoint(p: &Point) -> KPoint {
    KPoint::new(p.x, p.y)
}

fn resolve_stroke(style: &Style, theme: &Theme) -> Option<(Color, f64)> {
    match (style.stroke, style.stroke_width) {
        (Some(c), Some(w)) => Some((c, w)),
        (Some(c), None) => Some((c, theme.default_stroke_width)),
        (None, Some(w)) => Some((theme.edge_stroke, w)),
        (None, None) => None,
    }
}

fn stroke_or_default(style: &Style, theme: &Theme) -> (Color, f64) {
    let color = style.stroke.unwrap_or(theme.edge_stroke);
    let width = style.stroke_width.unwrap_or(theme.default_stroke_width);
    (color, width)
}

fn make_stroke(width: f64, style: &Style) -> Stroke {
    let mut stroke = Stroke::new(width)
        .with_join(Join::Round)
        .with_caps(Cap::Round);
    if let Some(ref dashes) = style.stroke_dasharray {
        stroke = stroke.with_dashes(0.0, dashes.iter().copied());
    }
    stroke
}

fn compose_affine(t: &Transform) -> Affine {
    match t {
        Transform::Identity => Affine::IDENTITY,
        Transform::Translate(dx, dy) => Affine::translate(Vec2::new(*dx, *dy)),
        Transform::Scale(sx, sy) => Affine::scale_non_uniform(*sx, *sy),
        Transform::Rotate { degrees, cx, cy } => {
            Affine::rotate_about(degrees.to_radians(), KPoint::new(*cx, *cy))
        }
    }
}

fn segments_to_bezpath(segments: &[PathSegment]) -> BezPath {
    let mut path = BezPath::new();
    for seg in segments {
        match seg {
            PathSegment::MoveTo(p) => path.move_to(to_kpoint(p)),
            PathSegment::LineTo(p) => path.line_to(to_kpoint(p)),
            PathSegment::CubicTo { cp1, cp2, to } => {
                path.curve_to(to_kpoint(cp1), to_kpoint(cp2), to_kpoint(to));
            }
            PathSegment::QuadTo { cp, to } => {
                path.quad_to(to_kpoint(cp), to_kpoint(to));
            }
            PathSegment::ArcTo { rx, ry, rotation, large_arc, sweep, to } => {
                // Convert SVG arc → kurbo Arc → bezier path segments
                let from = path.elements().last().map(|el| match el {
                    kurbo::PathEl::MoveTo(p) | kurbo::PathEl::LineTo(p) => *p,
                    kurbo::PathEl::QuadTo(_, p) | kurbo::PathEl::CurveTo(_, _, p) => *p,
                    kurbo::PathEl::ClosePath => KPoint::ORIGIN,
                }).unwrap_or(KPoint::ORIGIN);

                let svg_arc = kurbo::SvgArc {
                    from,
                    to: to_kpoint(to),
                    radii: Vec2::new(*rx, *ry),
                    x_rotation: rotation.to_radians(),
                    large_arc: *large_arc,
                    sweep: *sweep,
                };

                if let Some(arc) = kurbo::Arc::from_svg_arc(&svg_arc) {
                    // Append arc as bezier curves (skip the initial MoveTo)
                    use kurbo::Shape;
                    for el in arc.path_elements(0.1) {
                        match el {
                            kurbo::PathEl::MoveTo(_) => {} // skip — we're already at `from`
                            kurbo::PathEl::LineTo(p) => path.line_to(p),
                            kurbo::PathEl::QuadTo(c, p) => path.quad_to(c, p),
                            kurbo::PathEl::CurveTo(c1, c2, p) => path.curve_to(c1, c2, p),
                            kurbo::PathEl::ClosePath => path.close_path(),
                        }
                    }
                } else {
                    // Degenerate arc — straight line
                    path.line_to(to_kpoint(to));
                }
            }
            PathSegment::Close => path.close_path(),
        }
    }
    path
}

// ── Markers ──

fn paint_marker(
    scene: &mut VelloScene,
    marker: MarkerType,
    tip: Point,
    angle: f64,
    color: Color,
    stroke_width: f64,
    transform: Affine,
) {
    let geom = marker_geometry(marker);

    match &geom.shape {
        MarkerShape::FilledPath(_) => {
            let pts = transform_marker_points(&geom, tip, angle, stroke_width);
            let path = points_to_bezpath(&pts, true);
            scene.fill(Fill::NonZero, transform, to_vello_color(color), None, &path);
        }
        MarkerShape::StrokedPath { closed, stroke_width: sw, .. } => {
            let pts = transform_marker_points(&geom, tip, angle, stroke_width);
            let path = points_to_bezpath(&pts, *closed);
            let stroke = Stroke::new(sw * stroke_width / geom.vb_w * geom.marker_w);
            scene.stroke(&stroke, transform, to_vello_color(color), None, &path);
        }
        MarkerShape::FilledStrokedPath { fill_is_marker_color, stroke_width: sw, .. } => {
            let pts = transform_marker_points(&geom, tip, angle, stroke_width);
            let path = points_to_bezpath(&pts, true);
            let fill_color = if *fill_is_marker_color { color } else { Color::WHITE };
            scene.fill(Fill::NonZero, transform, to_vello_color(fill_color), None, &path);
            let stroke = Stroke::new(sw * stroke_width / geom.vb_w * geom.marker_w);
            scene.stroke(&stroke, transform, to_vello_color(color), None, &path);
        }
        MarkerShape::FilledCircle { .. } => {
            let (center, r) = transform_marker_circle(&geom, tip, angle, stroke_width);
            let circle = kurbo::Circle::new(KPoint::new(center.x, center.y), r);
            scene.fill(Fill::NonZero, transform, to_vello_color(color), None, &circle);
        }
        MarkerShape::StrokedCurves { stroke_width: sw, .. } => {
            let curves = transform_marker_curves(&geom, tip, angle, stroke_width);
            let stroke = Stroke::new(sw * stroke_width / geom.vb_w * geom.marker_w)
                .with_caps(Cap::Round);
            for curve in &curves {
                if curve.len() >= 3 {
                    let mut path = BezPath::new();
                    path.move_to(to_kpoint(&curve[0]));
                    path.quad_to(to_kpoint(&curve[1]), to_kpoint(&curve[2]));
                    scene.stroke(&stroke, transform, to_vello_color(color), None, &path);
                }
            }
        }
    }
}

fn points_to_bezpath(pts: &[Point], closed: bool) -> BezPath {
    let mut path = BezPath::new();
    if let Some(first) = pts.first() {
        path.move_to(to_kpoint(first));
        for p in &pts[1..] {
            path.line_to(to_kpoint(p));
        }
        if closed {
            path.close_path();
        }
    }
    path
}

// ── Arc (pie/annular sector) ──

fn paint_arc(
    scene: &mut VelloScene,
    center: &Point,
    inner_r: f64,
    outer_r: f64,
    start_angle: f64,
    end_angle: f64,
    style: &Style,
    theme: &Theme,
    transform: Affine,
) {
    let cx = center.x;
    let cy = center.y;
    let steps = 64;
    let angle_span = end_angle - start_angle;

    let mut path = BezPath::new();

    for i in 0..=steps {
        let t = start_angle + angle_span * (i as f64 / steps as f64);
        let x = cx + outer_r * t.cos();
        let y = cy + outer_r * t.sin();
        if i == 0 { path.move_to(KPoint::new(x, y)); } else { path.line_to(KPoint::new(x, y)); }
    }

    if inner_r > 0.0 {
        for i in (0..=steps).rev() {
            let t = start_angle + angle_span * (i as f64 / steps as f64);
            let x = cx + inner_r * t.cos();
            let y = cy + inner_r * t.sin();
            path.line_to(KPoint::new(x, y));
        }
    } else {
        path.line_to(KPoint::new(cx, cy));
    }
    path.close_path();

    if let Some(fill) = style.fill {
        scene.fill(Fill::NonZero, transform, to_vello_color(fill), None, &path);
    }
    if let Some((color, width)) = resolve_stroke(style, theme) {
        let stroke = make_stroke(width, style);
        scene.stroke(&stroke, transform, to_vello_color(color), None, &path);
    }
}

// ── Path endpoint angles for markers (shared from core) ──

use rusty_mermaid_core::{path_start_tangent, path_end_tangent};

// ── Text: deterministic font selection + shaping ──

use rusty_mermaid_core::font_fallback::{FontSlot, font_for_char};

/// Pre-loaded font data indexed by FontSlot. Initialized once.
struct FontSet {
    primary: FontData,       // Intel One Mono Regular
    primary_bold: FontData,
    primary_italic: FontData,
    primary_bold_italic: FontData,
    extended_text: FontData, // Noto Sans (proportional, Greek, Cyrillic)
    monospace: FontData,     // Noto Sans Mono (arrows, box drawing)
    dingbats: FontData,      // Noto Sans Symbols 2 (☕ ✔ ★ etc.)
    arabic: FontData,        // Noto Sans Arabic
    external: std::sync::Mutex<ExternalFonts>,
}

struct ExternalFonts {
    cjk: Option<FontData>,
    emoji: Option<FontData>,
}

static FONT_SET: OnceLock<FontSet> = OnceLock::new();

fn get_font_set() -> &'static FontSet {
    FONT_SET.get_or_init(|| {
        let load = |bytes: &[u8]| FontData::new(Blob::new(Arc::new(bytes.to_vec())), 0);
        FontSet {
            primary: load(include_bytes!("../../raster/fonts/IntelOneMono-Regular.ttf")),
            primary_bold: load(include_bytes!("../../raster/fonts/IntelOneMono-Bold.ttf")),
            primary_italic: load(include_bytes!("../../raster/fonts/IntelOneMono-Italic.ttf")),
            primary_bold_italic: load(include_bytes!("../../raster/fonts/IntelOneMono-BoldItalic.ttf")),
            extended_text: load(include_bytes!("../../raster/fonts/NotoSans-Regular.ttf")),
            monospace: load(include_bytes!("../../raster/fonts/NotoSansMono-Regular.ttf")),
            dingbats: load(include_bytes!("../../raster/fonts/NotoSansSymbols2-Regular.ttf")),
            arabic: load(include_bytes!("../../raster/fonts/NotoSansArabic-Regular.ttf")),
            external: std::sync::Mutex::new(ExternalFonts { cjk: None, emoji: None }),
        }
    })
}

/// Set an external font (called from WASM after CDN fetch).
pub fn set_external_font(slot: FontSlot, bytes: Vec<u8>) {
    let fs = get_font_set();
    if let Ok(mut ext) = fs.external.lock() {
        let fd = FontData::new(Blob::new(Arc::new(bytes)), 0);
        match slot {
            FontSlot::Cjk => ext.cjk = Some(fd),
            FontSlot::Emoji => ext.emoji = Some(fd),
            _ => {}
        }
    }
}

/// Resolve FontSlot to FontData. Returns None for unavailable external fonts.
fn font_for_slot(fs: &FontSet, slot: FontSlot, bold: bool, italic: bool) -> Option<FontData> {
    match slot {
        FontSlot::Primary => Some(match (bold, italic) {
            (true, true) => fs.primary_bold_italic.clone(),
            (true, false) => fs.primary_bold.clone(),
            (false, true) => fs.primary_italic.clone(),
            (false, false) => fs.primary.clone(),
        }),
        FontSlot::ExtendedText => Some(fs.extended_text.clone()),
        FontSlot::Monospace => Some(fs.monospace.clone()),
        FontSlot::Dingbats => Some(fs.dingbats.clone()),
        FontSlot::Arabic => Some(fs.arabic.clone()),
        FontSlot::Cjk => fs.external.lock().ok().and_then(|e| e.cjk.clone()),
        FontSlot::Emoji => fs.external.lock().ok().and_then(|e| e.emoji.clone()),
    }
}

struct ShapedGlyph {
    glyph_id: u32,
    x_advance: f32,
    x_offset: f32,
    y_offset: f32,
}

/// Shape text with rustybuzz, falling back to skrifa charmap.
fn shape_run(text: &str, font_data: &FontData, font_size: f32) -> Vec<ShapedGlyph> {
    let bytes = font_data.data.as_ref();

    if let Some(face) = rustybuzz::Face::from_slice(bytes, 0) {
        let scale = font_size / face.units_per_em() as f32;
        let mut buffer = rustybuzz::UnicodeBuffer::new();
        buffer.push_str(text);
        let output = rustybuzz::shape(&face, &[], buffer);
        return output.glyph_infos().iter().zip(output.glyph_positions()).map(|(info, pos)| {
            ShapedGlyph {
                glyph_id: info.glyph_id,
                x_advance: pos.x_advance as f32 * scale,
                x_offset: pos.x_offset as f32 * scale,
                y_offset: -(pos.y_offset as f32 * scale),
            }
        }).collect();
    }

    if let Ok(fr) = skrifa::FontRef::new(bytes) {
        let cm = skrifa::MetadataProvider::charmap(&fr);
        let gm = skrifa::MetadataProvider::glyph_metrics(&fr, skrifa::instance::Size::new(font_size), skrifa::instance::LocationRef::default());
        return text.chars().map(|ch| {
            let gid = cm.map(ch).unwrap_or_default();
            ShapedGlyph {
                glyph_id: gid.to_u32(),
                x_advance: gm.advance_width(gid).unwrap_or(font_size * 0.6),
                x_offset: 0.0, y_offset: 0.0,
            }
        }).collect();
    }

    text.chars().map(|_| ShapedGlyph {
        glyph_id: 0, x_advance: font_size * 0.6, x_offset: 0.0, y_offset: 0.0,
    }).collect()
}


// ── Text rendering ──

fn render_text(
    scene: &mut VelloScene,
    position: &Point,
    content: &str,
    anchor: TextAnchor,
    style: &rusty_mermaid_core::TextStyle,
    _theme: &Theme,
    transform: Affine,
) {
    let fs = get_font_set();
    let font_size = style.font_size as f32;
    let fill_color = style.fill.unwrap_or(Color::rgb(51, 51, 51));

    // Metrics from primary font for baseline centering
    let font_ref = skrifa::FontRef::from_index(fs.primary.data.as_ref(), fs.primary.index)
        .expect("embedded font must be valid");
    let metrics = skrifa::MetadataProvider::metrics(&font_ref, skrifa::instance::Size::new(font_size), skrifa::instance::LocationRef::default());
    let primary_gm = skrifa::MetadataProvider::glyph_metrics(&font_ref, skrifa::instance::Size::new(font_size), skrifa::instance::LocationRef::default());
    let primary_cm = skrifa::MetadataProvider::charmap(&font_ref);
    let visual_center_above_baseline = (metrics.ascent + metrics.descent) / 2.0;

    let lines: Vec<&str> = content.split('\n').collect();
    let line_height = font_size * 1.2;
    let block_height = (lines.len() as f32 - 1.0) * line_height;
    let first_baseline_y = position.y as f32 + visual_center_above_baseline - block_height / 2.0;

    for (line_idx, line) in lines.iter().enumerate() {
        let spans = parse_inline_markdown(line);
        let text_parts: Vec<(&str, bool, bool)> = if let Some(ref spans) = spans {
            spans.iter().map(|s| (s.text.as_str(), s.bold, s.italic)).collect()
        } else {
            vec![(line as &str, false, false)]
        };

        let line_y = first_baseline_y + line_idx as f32 * line_height;

        // First pass: shape all runs, compute actual rendered width.
        struct ShapedRun {
            glyphs: Vec<ShapedGlyph>,
            font: FontData,
            width: f32,
        }
        let mut runs: Vec<ShapedRun> = Vec::new();

        for (text, is_bold, is_italic) in &text_parts {
            // Assign each char a FontSlot, but spaces/punctuation inherit
            // from their neighbors to avoid splitting script runs.
            let chars: Vec<char> = text.chars().collect();
            let slots: Vec<FontSlot> = {
                let raw: Vec<FontSlot> = chars.iter().map(|&c| font_for_char(c)).collect();
                let mut resolved = raw.clone();
                // Spaces/common punctuation between same-script chars inherit that script
                for j in 0..resolved.len() {
                    if chars[j] == ' ' || chars[j] == ',' || chars[j] == '.' {
                        // Look for the nearest non-Primary neighbor
                        let prev = if j > 0 { Some(raw[j - 1]) } else { None };
                        let next = if j + 1 < raw.len() { Some(raw[j + 1]) } else { None };
                        if let Some(p) = prev {
                            if p != FontSlot::Primary { resolved[j] = p; }
                        } else if let Some(n) = next {
                            if n != FontSlot::Primary { resolved[j] = n; }
                        }
                    }
                }
                resolved
            };

            let mut i = 0;
            while i < chars.len() {
                let slot = slots[i];
                let run_start = i;
                while i < chars.len() && slots[i] == slot {
                    i += 1;
                }
                let run_text: String = chars[run_start..i].iter().collect();
                let fd = font_for_slot(fs, slot, *is_bold, *is_italic);
                if let Some(fd) = fd {
                    let shaped = shape_run(&run_text, &fd, font_size);
                    let width: f32 = shaped.iter().map(|sg| sg.x_advance).sum();
                    runs.push(ShapedRun { glyphs: shaped, font: fd, width });
                } else {
                    let w = (i - run_start) as f32 * font_size * 0.6;
                    runs.push(ShapedRun { glyphs: vec![], font: fs.primary.clone(), width: w });
                }
            }
        }

        // Compute actual total width and center
        let actual_w: f32 = runs.iter().map(|r| r.width).sum();
        let start_x = match anchor {
            TextAnchor::Start => position.x as f32,
            TextAnchor::Middle => position.x as f32 - actual_w / 2.0,
            TextAnchor::End => position.x as f32 - actual_w,
        };

        // Second pass: render at centered position
        let mut cursor_x = start_x;
        for run in &runs {
            if run.glyphs.is_empty() {
                cursor_x += run.width;
                continue;
            }
            let mut gx = cursor_x;
            let glyphs: Vec<_> = run.glyphs.iter().map(|sg| {
                let g = vello::Glyph {
                    id: sg.glyph_id,
                    x: gx + sg.x_offset,
                    y: line_y + sg.y_offset,
                };
                gx += sg.x_advance;
                g
            }).collect();
            scene.draw_glyphs(&run.font).font_size(font_size).transform(transform)
                .brush(to_vello_color(fill_color)).draw(Fill::NonZero, glyphs.into_iter());
            cursor_x = gx;
        }
    }
}
