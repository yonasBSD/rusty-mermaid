use std::sync::OnceLock;

use rusty_mermaid_core::{
    marker_geometry, parse_inline_markdown, text_baseline_y_offset, transform_marker_circle,
    transform_marker_curves, transform_marker_points, Color, MarkerShape, PathSegment, Point,
    Primitive, Style, TextAnchor, Theme, Transform,
};
use tiny_skia::{
    FillRule, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke,
    Transform as SkTransform,
};

pub fn render_primitive(
    pixmap: &mut Pixmap,
    prim: &Primitive,
    transform: SkTransform,
    theme: &Theme,
) {
    match prim {
        Primitive::Rect { bbox, rx, ry, style } => {
            let left = (bbox.x - bbox.width / 2.0) as f32;
            let top = (bbox.y - bbox.height / 2.0) as f32;
            let w = bbox.width as f32;
            let h = bbox.height as f32;
            let rx = *rx as f32;
            let ry = *ry as f32;

            let rect = if rx > 0.0 || ry > 0.0 {
                rounded_rect_path(left, top, w, h, rx, ry)
            } else {
                let mut pb = PathBuilder::new();
                pb.move_to(left, top);
                pb.line_to(left + w, top);
                pb.line_to(left + w, top + h);
                pb.line_to(left, top + h);
                pb.close();
                pb.finish()
            };
            let Some(path) = rect else { return };

            if let Some(fill) = resolve_fill(style) {
                let mut paint = Paint::default();
                paint.set_color(fill);
                paint.anti_alias = true;
                pixmap.fill_path(&path, &paint, FillRule::Winding, transform, None);
            }
            if let Some((stroke_color, width)) = resolve_stroke(style, theme) {
                let mut paint = Paint::default();
                paint.set_color(stroke_color);
                paint.anti_alias = true;
                let stroke = make_stroke(width, style);
                pixmap.stroke_path(&path, &paint, &stroke, transform, None);
            }
        }

        Primitive::Circle { center, radius, style } => {
            let cx = center.x as f32;
            let cy = center.y as f32;
            let r = *radius as f32;

            let Some(path) = circle_path(cx, cy, r) else { return };

            if let Some(fill) = resolve_fill(style) {
                let mut paint = Paint::default();
                paint.set_color(fill);
                paint.anti_alias = true;
                pixmap.fill_path(&path, &paint, FillRule::Winding, transform, None);
            }
            if let Some((stroke_color, width)) = resolve_stroke(style, theme) {
                let mut paint = Paint::default();
                paint.set_color(stroke_color);
                paint.anti_alias = true;
                let stroke = make_stroke(width, style);
                pixmap.stroke_path(&path, &paint, &stroke, transform, None);
            }
        }

        Primitive::Ellipse { center, rx, ry, style } => {
            let cx = center.x as f32;
            let cy = center.y as f32;
            let rx = *rx as f32;
            let ry = *ry as f32;

            // Approximate ellipse with 4 cubic beziers
            let Some(path) = ellipse_path(cx, cy, rx, ry) else { return };

            if let Some(fill) = resolve_fill(style) {
                let mut paint = Paint::default();
                paint.set_color(fill);
                paint.anti_alias = true;
                pixmap.fill_path(&path, &paint, FillRule::Winding, transform, None);
            }
            if let Some((stroke_color, width)) = resolve_stroke(style, theme) {
                let mut paint = Paint::default();
                paint.set_color(stroke_color);
                paint.anti_alias = true;
                let stroke = make_stroke(width, style);
                pixmap.stroke_path(&path, &paint, &stroke, transform, None);
            }
        }

        Primitive::Path { segments, style, marker_start, marker_end } => {
            let Some(path) = segments_to_path(segments) else { return };

            if let Some(fill) = resolve_fill(style) {
                let mut paint = Paint::default();
                paint.set_color(fill);
                paint.anti_alias = true;
                pixmap.fill_path(&path, &paint, FillRule::Winding, transform, None);
            }

            let (stroke_color, width) = resolve_stroke(style, theme)
                .unwrap_or_else(|| (to_skia_color(style.resolved_stroke(theme)), style.resolved_stroke_width(theme) as f32));
            let mut paint = Paint::default();
            paint.set_color(stroke_color);
            paint.anti_alias = true;
            let stroke = make_stroke(width, style);
            pixmap.stroke_path(&path, &paint, &stroke, transform, None);

            // Draw markers as filled geometry at endpoints
            if let (Some(marker), Some((pt, angle))) = (marker_start, rusty_mermaid_core::path_start_tangent(segments)) {
                draw_marker(pixmap, *marker, pt, angle, stroke_color, width, transform);
            }
            if let (Some(marker), Some((pt, angle))) = (marker_end, rusty_mermaid_core::path_end_tangent(segments)) {
                draw_marker(pixmap, *marker, pt, angle, stroke_color, width, transform);
            }
        }

        Primitive::Text { position, content, anchor, style } => {
            render_text(pixmap, position, content, *anchor, style, transform, theme);
        }

        Primitive::Polygon { points, style } => {
            if points.len() < 3 { return; }
            let mut pb = PathBuilder::new();
            pb.move_to(points[0].x as f32, points[0].y as f32);
            for p in &points[1..] {
                pb.line_to(p.x as f32, p.y as f32);
            }
            pb.close();
            let Some(path) = pb.finish() else { return };

            if let Some(fill) = resolve_fill(style) {
                let mut paint = Paint::default();
                paint.set_color(fill);
                paint.anti_alias = true;
                pixmap.fill_path(&path, &paint, FillRule::Winding, transform, None);
            }
            if let Some((stroke_color, width)) = resolve_stroke(style, theme) {
                let mut paint = Paint::default();
                paint.set_color(stroke_color);
                paint.anti_alias = true;
                let stroke = make_stroke(width, style);
                pixmap.stroke_path(&path, &paint, &stroke, transform, None);
            }
        }

        Primitive::Group { transform: group_tf, children } => {
            let child_transform = compose_transform(transform, group_tf);
            for child in children {
                render_primitive(pixmap, child, child_transform, theme);
            }
        }

        Primitive::Arc { center, inner_r, outer_r, start_angle, end_angle, style } => {
            render_arc(pixmap, center, *inner_r, *outer_r, *start_angle, *end_angle, style, transform, theme);
        }
    }
}

// ── Helpers ──

fn to_skia_color(c: Color) -> tiny_skia::Color {
    tiny_skia::Color::from_rgba8(c.r, c.g, c.b, c.a)
}

fn resolve_fill(style: &Style) -> Option<tiny_skia::Color> {
    style.fill.map(|c| to_skia_color(c))
}

fn resolve_stroke(style: &Style, theme: &Theme) -> Option<(tiny_skia::Color, f32)> {
    style.resolve_stroke_opt(theme)
        .map(|(c, w)| (to_skia_color(c), w as f32))
}

fn make_stroke(width: f32, style: &Style) -> Stroke {
    let mut stroke = Stroke {
        width,
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        ..Default::default()
    };
    if let Some(ref dashes) = style.stroke_dasharray {
        let dash: Vec<f32> = dashes.iter().map(|d| *d as f32).collect();
        if let Some(d) = tiny_skia::StrokeDash::new(dash, 0.0) {
            stroke.dash = Some(d);
        }
    }
    stroke
}

fn segments_to_path(segments: &[PathSegment]) -> Option<tiny_skia::Path> {
    let mut pb = PathBuilder::new();
    let mut cur = Point::new(0.0, 0.0);
    for seg in segments {
        match seg {
            PathSegment::MoveTo(p) => {
                pb.move_to(p.x as f32, p.y as f32);
                cur = *p;
            }
            PathSegment::LineTo(p) => {
                pb.line_to(p.x as f32, p.y as f32);
                cur = *p;
            }
            PathSegment::CubicTo { cp1, cp2, to } => {
                pb.cubic_to(cp1.x as f32, cp1.y as f32, cp2.x as f32, cp2.y as f32, to.x as f32, to.y as f32);
                cur = *to;
            }
            PathSegment::QuadTo { cp, to } => {
                pb.quad_to(cp.x as f32, cp.y as f32, to.x as f32, to.y as f32);
                cur = *to;
            }
            PathSegment::ArcTo { rx, ry, rotation, large_arc, sweep, to } => {
                arc_to_cubics(&mut pb, cur, *rx, *ry, *rotation, *large_arc, *sweep, *to);
                cur = *to;
            }
            PathSegment::Close => pb.close(),
        }
    }
    pb.finish()
}

fn circle_path(cx: f32, cy: f32, r: f32) -> Option<tiny_skia::Path> {
    ellipse_path(cx, cy, r, r)
}

fn ellipse_path(cx: f32, cy: f32, rx: f32, ry: f32) -> Option<tiny_skia::Path> {
    // 4-segment cubic bezier approximation (kappa = 0.5522847498)
    let k: f32 = rusty_mermaid_core::constants::KAPPA_F32;
    let kx = rx * k;
    let ky = ry * k;

    let mut pb = PathBuilder::new();
    pb.move_to(cx + rx, cy);
    pb.cubic_to(cx + rx, cy + ky, cx + kx, cy + ry, cx, cy + ry);
    pb.cubic_to(cx - kx, cy + ry, cx - rx, cy + ky, cx - rx, cy);
    pb.cubic_to(cx - rx, cy - ky, cx - kx, cy - ry, cx, cy - ry);
    pb.cubic_to(cx + kx, cy - ry, cx + rx, cy - ky, cx + rx, cy);
    pb.close();
    pb.finish()
}

fn rounded_rect_path(x: f32, y: f32, w: f32, h: f32, rx: f32, ry: f32) -> Option<tiny_skia::Path> {
    let rx = rx.min(w / 2.0);
    let ry = ry.min(h / 2.0);
    let k: f32 = rusty_mermaid_core::constants::KAPPA_F32;
    let kx = rx * k;
    let ky = ry * k;

    let mut pb = PathBuilder::new();
    pb.move_to(x + rx, y);
    pb.line_to(x + w - rx, y);
    pb.cubic_to(x + w - rx + kx, y, x + w, y + ry - ky, x + w, y + ry);
    pb.line_to(x + w, y + h - ry);
    pb.cubic_to(x + w, y + h - ry + ky, x + w - rx + kx, y + h, x + w - rx, y + h);
    pb.line_to(x + rx, y + h);
    pb.cubic_to(x + rx - kx, y + h, x, y + h - ry + ky, x, y + h - ry);
    pb.line_to(x, y + ry);
    pb.cubic_to(x, y + ry - ky, x + rx - kx, y, x + rx, y);
    pb.close();
    pb.finish()
}

fn compose_transform(parent: SkTransform, child: &Transform) -> SkTransform {
    match child {
        Transform::Identity => parent,
        Transform::Translate(dx, dy) => parent.post_translate(*dx as f32, *dy as f32),
        Transform::Scale(sx, sy) => parent.post_scale(*sx as f32, *sy as f32),
        Transform::Rotate { degrees, cx, cy } => {
            let rad = (*degrees as f32).to_radians();
            let cos = rad.cos();
            let sin = rad.sin();
            let cx = *cx as f32;
            let cy = *cy as f32;
            // Translate to origin, rotate, translate back
            let rot = SkTransform::from_row(cos, sin, -sin, cos, cx - cx * cos + cy * sin, cy - cx * sin - cy * cos);
            parent.post_concat(rot)
        }
    }
}

// ── Markers ──

fn draw_marker(
    pixmap: &mut Pixmap,
    marker: rusty_mermaid_core::MarkerType,
    tip: Point,
    angle: f64,
    color: tiny_skia::Color,
    stroke_width: f32,
    transform: SkTransform,
) {
    let geom = marker_geometry(marker);
    let sw = stroke_width as f64;

    match &geom.shape {
        MarkerShape::FilledPath(_) => {
            let pts = transform_marker_points(&geom, tip, angle, sw);
            if let Some(path) = points_to_skia_path(&pts, true) {
                let mut paint = Paint::default();
                paint.set_color(color);
                paint.anti_alias = true;
                pixmap.fill_path(&path, &paint, FillRule::Winding, transform, None);
            }
        }
        MarkerShape::StrokedPath { closed, stroke_width: rel_sw, .. } => {
            let pts = transform_marker_points(&geom, tip, angle, sw);
            if let Some(path) = points_to_skia_path(&pts, *closed) {
                let mut paint = Paint::default();
                paint.set_color(color);
                paint.anti_alias = true;
                let w = (*rel_sw * sw / geom.vb_w * geom.marker_w) as f32;
                let stroke = Stroke { width: w, ..Default::default() };
                pixmap.stroke_path(&path, &paint, &stroke, transform, None);
            }
        }
        MarkerShape::FilledStrokedPath { fill_is_marker_color, stroke_width: rel_sw, .. } => {
            let pts = transform_marker_points(&geom, tip, angle, sw);
            if let Some(path) = points_to_skia_path(&pts, true) {
                let fill_color = if *fill_is_marker_color {
                    color
                } else {
                    tiny_skia::Color::from_rgba8(255, 255, 255, 255)
                };
                let mut paint = Paint::default();
                paint.set_color(fill_color);
                paint.anti_alias = true;
                pixmap.fill_path(&path, &paint, FillRule::Winding, transform, None);

                let w = (*rel_sw * sw / geom.vb_w * geom.marker_w) as f32;
                let mut stroke_paint = Paint::default();
                stroke_paint.set_color(color);
                stroke_paint.anti_alias = true;
                let stroke = Stroke { width: w, ..Default::default() };
                pixmap.stroke_path(&path, &stroke_paint, &stroke, transform, None);
            }
        }
        MarkerShape::FilledCircle { .. } => {
            let (center, r) = transform_marker_circle(&geom, tip, angle, sw);
            if let Some(path) = circle_path(center.x as f32, center.y as f32, r as f32) {
                let mut paint = Paint::default();
                paint.set_color(color);
                paint.anti_alias = true;
                pixmap.fill_path(&path, &paint, FillRule::Winding, transform, None);
            }
        }
        MarkerShape::StrokedCurves { stroke_width: rel_sw, .. } => {
            let curves = transform_marker_curves(&geom, tip, angle, sw);
            let w = (*rel_sw * sw / geom.vb_w * geom.marker_w) as f32;
            for curve in &curves {
                if curve.len() >= 3 {
                    let mut pb = PathBuilder::new();
                    pb.move_to(curve[0].x as f32, curve[0].y as f32);
                    pb.quad_to(
                        curve[1].x as f32, curve[1].y as f32,
                        curve[2].x as f32, curve[2].y as f32,
                    );
                    if let Some(path) = pb.finish() {
                        let mut paint = Paint::default();
                        paint.set_color(color);
                        paint.anti_alias = true;
                        let stroke = Stroke {
                            width: w,
                            line_cap: LineCap::Round,
                            ..Default::default()
                        };
                        pixmap.stroke_path(&path, &paint, &stroke, transform, None);
                    }
                }
            }
        }
    }
}

fn points_to_skia_path(pts: &[Point], closed: bool) -> Option<tiny_skia::Path> {
    if pts.is_empty() { return None; }
    let mut pb = PathBuilder::new();
    pb.move_to(pts[0].x as f32, pts[0].y as f32);
    for p in &pts[1..] {
        pb.line_to(p.x as f32, p.y as f32);
    }
    if closed { pb.close(); }
    pb.finish()
}


fn render_arc(
    pixmap: &mut Pixmap,
    center: &Point,
    inner_r: f64,
    outer_r: f64,
    start_angle: f64,
    end_angle: f64,
    style: &Style,
    transform: SkTransform,
    theme: &Theme,
) {
    let segs = rusty_mermaid_core::arc_sector_segments(
        *center, inner_r, outer_r, start_angle, end_angle,
    );
    let mut pb = PathBuilder::new();
    for seg in &segs {
        match seg {
            rusty_mermaid_core::PathSegment::MoveTo(p) => pb.move_to(p.x as f32, p.y as f32),
            rusty_mermaid_core::PathSegment::LineTo(p) => pb.line_to(p.x as f32, p.y as f32),
            rusty_mermaid_core::PathSegment::Close => pb.close(),
            _ => {}
        }
    }
    let Some(path) = pb.finish() else { return };

    if let Some(fill) = resolve_fill(style) {
        let mut paint = Paint::default();
        paint.set_color(fill);
        paint.anti_alias = true;
        pixmap.fill_path(&path, &paint, FillRule::Winding, transform, None);
    }
    if let Some((stroke_color, width)) = resolve_stroke(style, theme) {
        let mut paint = Paint::default();
        paint.set_color(stroke_color);
        paint.anti_alias = true;
        let stroke = make_stroke(width, style);
        pixmap.stroke_path(&path, &paint, &stroke, transform, None);
    }
}

// ── Text rendering ──

struct RasterFontFamily {
    regular: fontdue::Font,
    bold: fontdue::Font,
    italic: fontdue::Font,
    bold_italic: fontdue::Font,
}

static FONT_FAMILY: OnceLock<RasterFontFamily> = OnceLock::new();

fn get_font_family() -> &'static RasterFontFamily {
    FONT_FAMILY.get_or_init(|| {
        let load = |bytes: &[u8]| {
            fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default())
                .expect("embedded font must be valid")
        };

        RasterFontFamily {
            regular: load(include_bytes!("../fonts/IntelOneMono-Regular.ttf")),
            bold: load(include_bytes!("../fonts/IntelOneMono-Bold.ttf")),
            italic: load(include_bytes!("../fonts/IntelOneMono-Italic.ttf")),
            bold_italic: load(include_bytes!("../fonts/IntelOneMono-BoldItalic.ttf")),
        }
    })
}

fn select_raster_font(family: &RasterFontFamily, bold: bool, italic: bool) -> &fontdue::Font {
    match (bold, italic) {
        (true, true) => &family.bold_italic,
        (true, false) => &family.bold,
        (false, true) => &family.italic,
        (false, false) => &family.regular,
    }
}

fn render_text(
    pixmap: &mut Pixmap,
    position: &Point,
    content: &str,
    anchor: TextAnchor,
    style: &rusty_mermaid_core::TextStyle,
    transform: SkTransform,
    _theme: &Theme,
) {
    let family = get_font_family();
    let px = style.font_size as f32;
    let fill = style.fill.unwrap_or(Color::rgb(51, 51, 51));

    let lines: Vec<&str> = content.split('\n').collect();
    let line_height = px * rusty_mermaid_core::constants::LINE_HEIGHT_MULTIPLIER_F32;
    let baseline_offset = text_baseline_y_offset(style.font_size, lines.len()) as f32;
    let first_baseline_y = position.y as f32 + baseline_offset;

    for (line_idx, line) in lines.iter().enumerate() {
        let spans = parse_inline_markdown(line);
        let text_parts: Vec<(&str, bool, bool)> = if let Some(ref spans) = spans {
            spans.iter().map(|s| (s.text.as_str(), s.bold, s.italic)).collect()
        } else {
            vec![(line as &str, false, false)]
        };

        // Measure total line width (using regular font — monospace, all same width)
        let mut line_w: f32 = 0.0;
        for (text, _, _) in &text_parts {
            for ch in text.chars() {
                let (metrics, _) = family.regular.rasterize(ch, px);
                line_w += metrics.advance_width;
            }
        }

        let start_x = match anchor {
            TextAnchor::Start => position.x as f32,
            TextAnchor::Middle => position.x as f32 - line_w / 2.0,
            TextAnchor::End => position.x as f32 - line_w,
        };
        let line_y = first_baseline_y + line_idx as f32 * line_height;

        let mut cursor_x = start_x;
        for (text, is_bold, is_italic) in &text_parts {
            let font = select_raster_font(family, *is_bold, *is_italic);
            for ch in text.chars() {
                let (metrics, bitmap) = font.rasterize(ch, px);
                if metrics.width > 0 && metrics.height > 0 {
                    blit_glyph(
                        pixmap,
                        &bitmap,
                        metrics.width,
                        metrics.height,
                        cursor_x + metrics.xmin as f32,
                        line_y - metrics.ymin as f32 - metrics.height as f32,
                        fill,
                        transform,
                    );
                }
                cursor_x += metrics.advance_width;
            }
        }
    }
}

fn blit_glyph(
    pixmap: &mut Pixmap,
    bitmap: &[u8],
    glyph_w: usize,
    glyph_h: usize,
    x: f32,
    y: f32,
    color: Color,
    transform: SkTransform,
) {
    let pw = pixmap.width() as usize;
    let ph = pixmap.height() as usize;

    for gy in 0..glyph_h {
        for gx in 0..glyph_w {
            let alpha = bitmap[gy * glyph_w + gx];
            if alpha == 0 {
                continue;
            }

            // Apply transform to glyph pixel position
            let sx = x + gx as f32;
            let sy = y + gy as f32;
            let tx = transform.sx * sx + transform.kx * sy + transform.tx;
            let ty = transform.ky * sx + transform.sy * sy + transform.ty;

            let px_x = tx as usize;
            let px_y = ty as usize;
            if px_x >= pw || px_y >= ph {
                continue;
            }

            // Alpha-blend onto existing pixel
            let idx = px_y * pw + px_x;
            let dst = pixmap.pixels_mut();
            let bg = dst[idx];
            let a = alpha as f32 / 255.0;
            let inv_a = 1.0 - a;

            let r = (color.r as f32 * a + bg.red() as f32 * inv_a) as u8;
            let g = (color.g as f32 * a + bg.green() as f32 * inv_a) as u8;
            let b = (color.b as f32 * a + bg.blue() as f32 * inv_a) as u8;
            let out_a = ((a + bg.alpha() as f32 / 255.0 * inv_a) * 255.0) as u8;

            dst[idx] = tiny_skia::PremultipliedColorU8::from_rgba(r, g, b, out_a).unwrap();
        }
    }
}

// ── SVG Arc to Cubic Bezier conversion ──

fn arc_to_cubics(
    pb: &mut PathBuilder,
    from: Point,
    rx: f64,
    ry: f64,
    x_rotation: f64,
    large_arc: bool,
    sweep: bool,
    to: Point,
) {
    // Implementation of the SVG arc endpoint-to-center parameterization
    // Reference: https://www.w3.org/TR/SVG/implnote.html#ArcImplementationNotes
    let mut rx = rx.abs();
    let mut ry = ry.abs();

    if rx < 1e-10 || ry < 1e-10 {
        pb.line_to(to.x as f32, to.y as f32);
        return;
    }

    let phi = x_rotation.to_radians();
    let (sin_phi, cos_phi) = phi.sin_cos();

    let dx = (from.x - to.x) / 2.0;
    let dy = (from.y - to.y) / 2.0;
    let x1p = cos_phi * dx + sin_phi * dy;
    let y1p = -sin_phi * dx + cos_phi * dy;

    // Scale radii if needed
    let lambda = (x1p * x1p) / (rx * rx) + (y1p * y1p) / (ry * ry);
    if lambda > 1.0 {
        let s = lambda.sqrt();
        rx *= s;
        ry *= s;
    }

    let rxsq = rx * rx;
    let rysq = ry * ry;
    let x1psq = x1p * x1p;
    let y1psq = y1p * y1p;

    let num = (rxsq * rysq - rxsq * y1psq - rysq * x1psq).max(0.0);
    let den = rxsq * y1psq + rysq * x1psq;
    let sq = if den < 1e-10 { 0.0 } else { (num / den).sqrt() };
    let sign = if large_arc == sweep { -1.0 } else { 1.0 };

    let cxp = sign * sq * rx * y1p / ry;
    let cyp = sign * sq * -ry * x1p / rx;

    let cx = cos_phi * cxp - sin_phi * cyp + (from.x + to.x) / 2.0;
    let cy = sin_phi * cxp + cos_phi * cyp + (from.y + to.y) / 2.0;

    let theta1 = vec_angle(1.0, 0.0, (x1p - cxp) / rx, (y1p - cyp) / ry);
    let mut dtheta = vec_angle(
        (x1p - cxp) / rx,
        (y1p - cyp) / ry,
        (-x1p - cxp) / rx,
        (-y1p - cyp) / ry,
    );

    if !sweep && dtheta > 0.0 {
        dtheta -= std::f64::consts::TAU;
    } else if sweep && dtheta < 0.0 {
        dtheta += std::f64::consts::TAU;
    }

    // Split into segments of at most pi/2
    let n_segs = (dtheta.abs() / (std::f64::consts::FRAC_PI_2 + 0.001)).ceil() as usize;
    let seg_angle = dtheta / n_segs as f64;

    for i in 0..n_segs {
        let t1 = theta1 + seg_angle * i as f64;
        let t2 = theta1 + seg_angle * (i + 1) as f64;
        arc_segment_to_cubic(pb, cx, cy, rx, ry, sin_phi, cos_phi, t1, t2);
    }
}

fn arc_segment_to_cubic(
    pb: &mut PathBuilder,
    cx: f64, cy: f64,
    rx: f64, ry: f64,
    sin_phi: f64, cos_phi: f64,
    t1: f64, t2: f64,
) {
    let alpha = (4.0 / 3.0) * ((t2 - t1) / 4.0).tan();

    let (sin1, cos1) = t1.sin_cos();
    let (sin2, cos2) = t2.sin_cos();

    let ex1 = rx * cos1;
    let ey1 = ry * sin1;
    let ex2 = rx * cos2;
    let ey2 = ry * sin2;

    let cp1x = ex1 - alpha * rx * sin1;
    let cp1y = ey1 + alpha * ry * cos1;
    let cp2x = ex2 + alpha * rx * sin2;
    let cp2y = ey2 - alpha * ry * cos2;

    // Rotate and translate
    let c1x = cos_phi * cp1x - sin_phi * cp1y + cx;
    let c1y = sin_phi * cp1x + cos_phi * cp1y + cy;
    let c2x = cos_phi * cp2x - sin_phi * cp2y + cx;
    let c2y = sin_phi * cp2x + cos_phi * cp2y + cy;
    let x = cos_phi * ex2 - sin_phi * ey2 + cx;
    let y = sin_phi * ex2 + cos_phi * ey2 + cy;

    pb.cubic_to(c1x as f32, c1y as f32, c2x as f32, c2y as f32, x as f32, y as f32);
}

fn vec_angle(ux: f64, uy: f64, vx: f64, vy: f64) -> f64 {
    let sign = if ux * vy - uy * vx < 0.0 { -1.0 } else { 1.0 };
    let dot = ux * vx + uy * vy;
    let len = (ux * ux + uy * uy).sqrt() * (vx * vx + vy * vy).sqrt();
    let cos = if len < 1e-10 { 1.0 } else { dot / len };
    sign * cos.clamp(-1.0, 1.0).acos()
}
