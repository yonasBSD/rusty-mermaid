use std::sync::{Arc, OnceLock};

use vello::Scene as VelloScene;
use vello::kurbo::{
    self, Affine, BezPath, Cap, Join, Point as KPoint, Rect, RoundedRect, Stroke, Vec2,
};
use vello::peniko::{Blob, Color as VelloColor, Fill, FontData};

use rusty_mermaid_core::{
    Color, MarkerShape, MarkerType, PathSegment, Point, Primitive, Style, TextAnchor, Theme,
    Transform, marker_geometry, parse_inline_markdown, transform_marker_circle,
    transform_marker_curves, transform_marker_points,
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

fn paint_primitive(scene: &mut VelloScene, prim: &Primitive, transform: Affine, theme: &Theme) {
    match prim {
        Primitive::Rect {
            bbox,
            rx,
            ry,
            style,
        } => {
            paint_rect(scene, bbox, *rx, *ry, style, theme, transform);
        }
        Primitive::Circle {
            center,
            radius,
            style,
        } => {
            let shape = kurbo::Circle::new(KPoint::new(center.x, center.y), *radius);
            fill_and_stroke(scene, &shape, style, theme, transform);
        }
        Primitive::Ellipse {
            center,
            rx,
            ry,
            style,
        } => {
            let shape =
                kurbo::Ellipse::new(KPoint::new(center.x, center.y), Vec2::new(*rx, *ry), 0.0);
            fill_and_stroke(scene, &shape, style, theme, transform);
        }
        Primitive::Path {
            segments,
            style,
            marker_start,
            marker_end,
        } => {
            paint_path_prim(
                scene,
                segments,
                style,
                *marker_start,
                *marker_end,
                theme,
                transform,
            );
        }
        Primitive::Text {
            position,
            content,
            anchor,
            style,
        } => {
            render_text(scene, position, content, *anchor, style, transform);
        }
        Primitive::Polygon { points, style } => {
            paint_polygon(scene, points, style, theme, transform);
        }
        Primitive::Group {
            transform: group_tf,
            children,
        } => {
            let child_transform = transform * compose_affine(group_tf);
            for child in children {
                paint_primitive(scene, child, child_transform, theme);
            }
        }
        Primitive::Arc {
            center,
            inner_r,
            outer_r,
            start_angle,
            end_angle,
            style,
        } => {
            paint_arc(
                scene,
                center,
                *inner_r,
                *outer_r,
                *start_angle,
                *end_angle,
                style,
                theme,
                transform,
            );
        }
    }
}

// ── Per-primitive rendering ──

fn fill_and_stroke(
    scene: &mut VelloScene,
    shape: &impl kurbo::Shape,
    style: &Style,
    theme: &Theme,
    transform: Affine,
) {
    if let Some(fill) = style.fill {
        scene.fill(Fill::NonZero, transform, to_vello_color(fill), None, shape);
    }
    if let Some((color, width)) = resolve_stroke(style, theme) {
        let stroke = make_stroke(width, style);
        scene.stroke(&stroke, transform, to_vello_color(color), None, shape);
    }
}

fn paint_rect(
    scene: &mut VelloScene,
    bbox: &rusty_mermaid_core::BBox,
    rx: f64,
    ry: f64,
    style: &Style,
    theme: &Theme,
    transform: Affine,
) {
    let left = bbox.x - bbox.width / 2.0;
    let top = bbox.y - bbox.height / 2.0;
    let rect = Rect::new(left, top, left + bbox.width, top + bbox.height);
    let r = rx.max(ry);
    if r > 0.0 {
        fill_and_stroke(
            scene,
            &RoundedRect::from_rect(rect, r),
            style,
            theme,
            transform,
        );
    } else {
        fill_and_stroke(scene, &rect, style, theme, transform);
    }
}

fn paint_path_prim(
    scene: &mut VelloScene,
    segments: &[rusty_mermaid_core::PathSegment],
    style: &Style,
    marker_start: Option<rusty_mermaid_core::MarkerType>,
    marker_end: Option<rusty_mermaid_core::MarkerType>,
    theme: &Theme,
    transform: Affine,
) {
    let path = segments_to_bezpath(segments);
    if let Some(fill) = style.fill {
        scene.fill(Fill::NonZero, transform, to_vello_color(fill), None, &path);
    }
    let (color, width) = stroke_or_default(style, theme);
    let stroke = make_stroke(width, style);
    scene.stroke(&stroke, transform, to_vello_color(color), None, &path);

    if let Some(marker) = marker_start {
        if let Some((tip, angle)) = path_start_tangent(segments) {
            paint_marker(scene, marker, tip, angle, color, width, transform);
        }
    }
    if let Some(marker) = marker_end {
        if let Some((tip, angle)) = path_end_tangent(segments) {
            paint_marker(scene, marker, tip, angle, color, width, transform);
        }
    }
}

fn paint_polygon(
    scene: &mut VelloScene,
    points: &[Point],
    style: &Style,
    theme: &Theme,
    transform: Affine,
) {
    if points.len() < 3 {
        return;
    }
    let mut path = BezPath::new();
    path.move_to(to_kpoint(&points[0]));
    for p in &points[1..] {
        path.line_to(to_kpoint(p));
    }
    path.close_path();
    fill_and_stroke(scene, &path, style, theme, transform);
}

// ── Helpers ──

fn to_vello_color(c: Color) -> VelloColor {
    VelloColor::from_rgba8(c.r, c.g, c.b, c.a)
}

fn to_kpoint(p: &Point) -> KPoint {
    KPoint::new(p.x, p.y)
}

fn resolve_stroke(style: &Style, theme: &Theme) -> Option<(Color, f64)> {
    style.resolve_stroke_opt(theme)
}

fn stroke_or_default(style: &Style, theme: &Theme) -> (Color, f64) {
    (
        style.resolved_stroke(theme),
        style.resolved_stroke_width(theme),
    )
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
            PathSegment::ArcTo {
                rx,
                ry,
                rotation,
                large_arc,
                sweep,
                to,
            } => {
                append_svg_arc(&mut path, *rx, *ry, *rotation, *large_arc, *sweep, to);
            }
            PathSegment::Close => path.close_path(),
        }
    }
    path
}

fn append_svg_arc(
    path: &mut BezPath,
    rx: f64,
    ry: f64,
    rotation: f64,
    large_arc: bool,
    sweep: bool,
    to: &Point,
) {
    let from = path
        .elements()
        .last()
        .map(|el| match el {
            kurbo::PathEl::MoveTo(p) | kurbo::PathEl::LineTo(p) => *p,
            kurbo::PathEl::QuadTo(_, p) | kurbo::PathEl::CurveTo(_, _, p) => *p,
            kurbo::PathEl::ClosePath => KPoint::ORIGIN,
        })
        .unwrap_or(KPoint::ORIGIN);

    let svg_arc = kurbo::SvgArc {
        from,
        to: to_kpoint(to),
        radii: Vec2::new(rx, ry),
        x_rotation: rotation.to_radians(),
        large_arc,
        sweep,
    };

    let Some(arc) = kurbo::Arc::from_svg_arc(&svg_arc) else {
        path.line_to(to_kpoint(to));
        return;
    };

    use kurbo::Shape;
    for el in arc.path_elements(0.1) {
        match el {
            kurbo::PathEl::MoveTo(_) => {}
            kurbo::PathEl::LineTo(p) => path.line_to(p),
            kurbo::PathEl::QuadTo(c, p) => path.quad_to(c, p),
            kurbo::PathEl::CurveTo(c1, c2, p) => path.curve_to(c1, c2, p),
            kurbo::PathEl::ClosePath => path.close_path(),
        }
    }
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
    use rusty_mermaid_core::MarkerPath;
    let mp = rusty_mermaid_core::marker_path(marker, tip, angle, stroke_width);
    let vc = to_vello_color(color);

    match mp {
        MarkerPath::FillPolygon { points } => {
            let path = points_to_bezpath(&points, true);
            scene.fill(Fill::NonZero, transform, vc, None, &path);
        }
        MarkerPath::StrokePolyline {
            points,
            width,
            closed,
        } => {
            let path = points_to_bezpath(&points, closed);
            scene.stroke(&Stroke::new(width), transform, vc, None, &path);
        }
        MarkerPath::FillAndStrokePolygon {
            points,
            stroke_width: sw,
            fill_is_marker_color,
        } => {
            let path = points_to_bezpath(&points, true);
            let fc = if fill_is_marker_color {
                color
            } else {
                Color::WHITE
            };
            scene.fill(Fill::NonZero, transform, to_vello_color(fc), None, &path);
            scene.stroke(&Stroke::new(sw), transform, vc, None, &path);
        }
        MarkerPath::FillCircle { center, radius } => {
            let circle = kurbo::Circle::new(KPoint::new(center.x, center.y), radius);
            scene.fill(Fill::NonZero, transform, vc, None, &circle);
        }
        MarkerPath::StrokeCurves { curves, width } => {
            let stroke = Stroke::new(width).with_caps(Cap::Round);
            for [start, cp, end] in &curves {
                let mut path = BezPath::new();
                path.move_to(to_kpoint(start));
                path.quad_to(to_kpoint(cp), to_kpoint(end));
                scene.stroke(&stroke, transform, vc, None, &path);
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
    let segs =
        rusty_mermaid_core::arc_sector_segments(*center, inner_r, outer_r, start_angle, end_angle);
    let mut path = BezPath::new();
    for seg in &segs {
        match seg {
            rusty_mermaid_core::PathSegment::MoveTo(p) => path.move_to(KPoint::new(p.x, p.y)),
            rusty_mermaid_core::PathSegment::LineTo(p) => path.line_to(KPoint::new(p.x, p.y)),
            rusty_mermaid_core::PathSegment::Close => path.close_path(),
            _ => {}
        }
    }

    if let Some(fill) = style.fill {
        scene.fill(Fill::NonZero, transform, to_vello_color(fill), None, &path);
    }
    if let Some((color, width)) = resolve_stroke(style, theme) {
        let stroke = make_stroke(width, style);
        scene.stroke(&stroke, transform, to_vello_color(color), None, &path);
    }
}

// ── Path endpoint angles for markers (shared from core) ──

use rusty_mermaid_core::{path_end_tangent, path_start_tangent};

// ── Text: deterministic font selection + shaping ──

use rusty_mermaid_core::font_fallback::{FontSlot, font_for_char};

/// Pre-loaded font data indexed by FontSlot. Initialized once.
struct FontSet {
    primary: FontData, // Intel One Mono Regular
    primary_bold: FontData,
    primary_italic: FontData,
    primary_bold_italic: FontData,
    extended_text: FontData, // Noto Sans (proportional, Greek, Cyrillic)
    monospace: FontData,     // Noto Sans Mono (arrows, box drawing)
    dingbats: FontData,      // Noto Sans Symbols 2 (☕ ✔ ★ etc.)
    arabic: FontData,        // Noto Naskh Arabic
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
            primary: load(include_bytes!(
                "../../raster/fonts/IntelOneMono-Regular.ttf"
            )),
            primary_bold: load(include_bytes!("../../raster/fonts/IntelOneMono-Bold.ttf")),
            primary_italic: load(include_bytes!("../../raster/fonts/IntelOneMono-Italic.ttf")),
            primary_bold_italic: load(include_bytes!(
                "../../raster/fonts/IntelOneMono-BoldItalic.ttf"
            )),
            extended_text: load(include_bytes!("../../raster/fonts/NotoSans-Regular.ttf")),
            monospace: load(include_bytes!(
                "../../raster/fonts/NotoSansMono-Regular.ttf"
            )),
            dingbats: load(include_bytes!(
                "../../raster/fonts/NotoSansSymbols2-Regular.ttf"
            )),
            arabic: load(include_bytes!(
                "../../raster/fonts/NotoNaskhArabic-Regular.ttf"
            )),
            external: std::sync::Mutex::new(ExternalFonts {
                cjk: None,
                emoji: None,
            }),
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
        return output
            .glyph_infos()
            .iter()
            .zip(output.glyph_positions())
            .map(|(info, pos)| ShapedGlyph {
                glyph_id: info.glyph_id,
                x_advance: pos.x_advance as f32 * scale,
                x_offset: pos.x_offset as f32 * scale,
                y_offset: -(pos.y_offset as f32 * scale),
            })
            .collect();
    }

    if let Ok(fr) = skrifa::FontRef::new(bytes) {
        let cm = skrifa::MetadataProvider::charmap(&fr);
        let gm = skrifa::MetadataProvider::glyph_metrics(
            &fr,
            skrifa::instance::Size::new(font_size),
            skrifa::instance::LocationRef::default(),
        );
        return text
            .chars()
            .map(|ch| {
                let gid = cm.map(ch).unwrap_or_default();
                ShapedGlyph {
                    glyph_id: gid.to_u32(),
                    x_advance: gm.advance_width(gid).unwrap_or(font_size * 0.6),
                    x_offset: 0.0,
                    y_offset: 0.0,
                }
            })
            .collect();
    }

    text.chars()
        .map(|_| ShapedGlyph {
            glyph_id: 0,
            x_advance: font_size * 0.6,
            x_offset: 0.0,
            y_offset: 0.0,
        })
        .collect()
}

// ── Text rendering ──

fn render_text(
    scene: &mut VelloScene,
    position: &Point,
    content: &str,
    anchor: TextAnchor,
    style: &rusty_mermaid_core::TextStyle,
    transform: Affine,
) {
    let fs = get_font_set();
    let font_size = style.font_size as f32;
    let fill_color = style.fill.unwrap_or(Color::rgb(51, 51, 51));

    // Metrics from primary font for baseline centering
    let font_ref = skrifa::FontRef::from_index(fs.primary.data.as_ref(), fs.primary.index)
        .expect("embedded font must be valid");
    let metrics = skrifa::MetadataProvider::metrics(
        &font_ref,
        skrifa::instance::Size::new(font_size),
        skrifa::instance::LocationRef::default(),
    );
    let visual_center_above_baseline = (metrics.ascent + metrics.descent) / 2.0;

    let lines: Vec<&str> = content.split('\n').collect();
    let line_height = font_size * rusty_mermaid_core::constants::LINE_HEIGHT_MULTIPLIER_F32;
    let block_height = (lines.len() as f32 - 1.0) * line_height;
    let first_baseline_y = position.y as f32 + visual_center_above_baseline - block_height / 2.0;

    for (line_idx, line) in lines.iter().enumerate() {
        let spans = parse_inline_markdown(line);
        let text_parts: Vec<(&str, bool, bool)> = if let Some(ref spans) = spans {
            spans
                .iter()
                .map(|s| (s.text.as_str(), s.bold, s.italic))
                .collect()
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
                        let next = if j + 1 < raw.len() {
                            Some(raw[j + 1])
                        } else {
                            None
                        };
                        if let Some(p) = prev {
                            if p != FontSlot::Primary {
                                resolved[j] = p;
                            }
                        } else if let Some(n) = next {
                            if n != FontSlot::Primary {
                                resolved[j] = n;
                            }
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
                    runs.push(ShapedRun {
                        glyphs: shaped,
                        font: fd,
                        width,
                    });
                } else {
                    let w = (i - run_start) as f32 * font_size * 0.6;
                    runs.push(ShapedRun {
                        glyphs: vec![],
                        font: fs.primary.clone(),
                        width: w,
                    });
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
            let glyphs: Vec<_> = run
                .glyphs
                .iter()
                .map(|sg| {
                    let g = vello::Glyph {
                        id: sg.glyph_id,
                        x: gx + sg.x_offset,
                        y: line_y + sg.y_offset,
                    };
                    gx += sg.x_advance;
                    g
                })
                .collect();
            scene
                .draw_glyphs(&run.font)
                .font_size(font_size)
                .transform(transform)
                .brush(to_vello_color(fill_color))
                .draw(Fill::NonZero, glyphs.into_iter());
            cursor_x = gx;
        }
    }
}
