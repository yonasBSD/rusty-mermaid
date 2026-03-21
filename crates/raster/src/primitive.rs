use rusty_mermaid_core::{Color, PathSegment, Point, Primitive, Style, TextAnchor, Transform};
use tiny_skia::{
    FillRule, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke,
    Transform as SkTransform,
};

use crate::RasterConfig;

pub fn render_primitive(
    pixmap: &mut Pixmap,
    prim: &Primitive,
    transform: SkTransform,
    config: &RasterConfig,
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
            if let Some((stroke_color, width)) = resolve_stroke(style, config) {
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
            if let Some((stroke_color, width)) = resolve_stroke(style, config) {
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
            if let Some((stroke_color, width)) = resolve_stroke(style, config) {
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

            let (stroke_color, width) = if let Some(sw) = resolve_stroke(style, config) {
                sw
            } else {
                // Paths default to stroked
                (to_skia_color(config.default_stroke), config.default_stroke_width as f32)
            };
            let mut paint = Paint::default();
            paint.set_color(stroke_color);
            paint.anti_alias = true;
            let stroke = make_stroke(width, style);
            pixmap.stroke_path(&path, &paint, &stroke, transform, None);

            // Draw markers as filled geometry at endpoints
            if let (Some(marker), Some(pt)) = (marker_start, first_point(segments)) {
                draw_marker(pixmap, *marker, pt, start_angle(segments), stroke_color, width, transform);
            }
            if let (Some(marker), Some(pt)) = (marker_end, last_point(segments)) {
                draw_marker(pixmap, *marker, pt, end_angle(segments), stroke_color, width, transform);
            }
        }

        Primitive::Text { position, content, anchor, style } => {
            // Text rendering placeholder: draw a small rectangle where text would be.
            // Full glyph rendering (cosmic-text/fontdue) deferred to 11.8.
            let font_size = style.font_size as f32;
            let char_w = font_size * 0.6;
            let text_w = char_w * content.len() as f32;
            let text_h = font_size;

            let x = match anchor {
                TextAnchor::Start => position.x as f32,
                TextAnchor::Middle => position.x as f32 - text_w / 2.0,
                TextAnchor::End => position.x as f32 - text_w,
            };
            let y = position.y as f32 - text_h / 2.0;

            let mut pb = PathBuilder::new();
            pb.move_to(x, y);
            pb.line_to(x + text_w, y);
            pb.line_to(x + text_w, y + text_h);
            pb.line_to(x, y + text_h);
            pb.close();
            let Some(path) = pb.finish() else { return };

            let fill_color = style.fill.unwrap_or(Color::rgb(51, 51, 51));
            let mut paint = Paint::default();
            paint.set_color(to_skia_color(fill_color));
            paint.anti_alias = true;
            // Draw a subtle filled rect as text placeholder
            paint.set_color(tiny_skia::Color::from_rgba8(
                fill_color.r, fill_color.g, fill_color.b, 40,
            ));
            pixmap.fill_path(&path, &paint, FillRule::Winding, transform, None);
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
            if let Some((stroke_color, width)) = resolve_stroke(style, config) {
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
                render_primitive(pixmap, child, child_transform, config);
            }
        }

        Primitive::Arc { center, inner_r, outer_r, start_angle, end_angle, style } => {
            render_arc(pixmap, center, *inner_r, *outer_r, *start_angle, *end_angle, style, transform, config);
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

fn resolve_stroke(style: &Style, config: &RasterConfig) -> Option<(tiny_skia::Color, f32)> {
    match (style.stroke, style.stroke_width) {
        (Some(c), Some(w)) => Some((to_skia_color(c), w as f32)),
        (Some(c), None) => Some((to_skia_color(c), config.default_stroke_width as f32)),
        (None, Some(w)) => Some((to_skia_color(config.default_stroke), w as f32)),
        (None, None) => None,
    }
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
    for seg in segments {
        match seg {
            PathSegment::MoveTo(p) => pb.move_to(p.x as f32, p.y as f32),
            PathSegment::LineTo(p) => pb.line_to(p.x as f32, p.y as f32),
            PathSegment::CubicTo { cp1, cp2, to } => {
                pb.cubic_to(cp1.x as f32, cp1.y as f32, cp2.x as f32, cp2.y as f32, to.x as f32, to.y as f32);
            }
            PathSegment::QuadTo { cp, to } => {
                pb.quad_to(cp.x as f32, cp.y as f32, to.x as f32, to.y as f32);
            }
            PathSegment::ArcTo { rx, ry, rotation, large_arc, sweep, to } => {
                // Convert SVG arc to cubic beziers via tiny-skia's arc support
                // tiny-skia doesn't have arc_to, so approximate with line
                // (full arc conversion deferred to 11.8)
                pb.line_to(to.x as f32, to.y as f32);
                let _ = (rx, ry, rotation, large_arc, sweep);
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
    let k: f32 = 0.5522848;
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
    let k: f32 = 0.5522848;
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
    use rusty_mermaid_core::MarkerType;
    let size = (stroke_width * 4.0).max(6.0);
    let (sin, cos) = (angle as f32).sin_cos();
    let tx = tip.x as f32;
    let ty = tip.y as f32;

    match marker {
        MarkerType::ArrowPoint | MarkerType::ArrowBarb => {
            // Filled triangle pointing in the direction of `angle`
            let (p1x, p1y) = rotate_point(-size, -size * 0.5, sin, cos, tx, ty);
            let (p2x, p2y) = rotate_point(-size, size * 0.5, sin, cos, tx, ty);

            let mut pb = PathBuilder::new();
            pb.move_to(tx, ty);
            pb.line_to(p1x, p1y);
            pb.line_to(p2x, p2y);
            pb.close();
            if let Some(path) = pb.finish() {
                let mut paint = Paint::default();
                paint.set_color(color);
                paint.anti_alias = true;
                pixmap.fill_path(&path, &paint, FillRule::Winding, transform, None);
            }
        }
        MarkerType::ArrowOpen => {
            // Open chevron (no fill, just stroked lines)
            let (p1x, p1y) = rotate_point(-size, -size * 0.5, sin, cos, tx, ty);
            let (p2x, p2y) = rotate_point(-size, size * 0.5, sin, cos, tx, ty);

            let mut pb = PathBuilder::new();
            pb.move_to(p1x, p1y);
            pb.line_to(tx, ty);
            pb.line_to(p2x, p2y);
            if let Some(path) = pb.finish() {
                let mut paint = Paint::default();
                paint.set_color(color);
                paint.anti_alias = true;
                let stroke = Stroke { width: stroke_width, ..Default::default() };
                pixmap.stroke_path(&path, &paint, &stroke, transform, None);
            }
        }
        MarkerType::Circle => {
            let r = size * 0.4;
            let (cx, cy) = rotate_point(-r, 0.0, sin, cos, tx, ty);
            if let Some(path) = circle_path(cx, cy, r) {
                let mut paint = Paint::default();
                paint.set_color(color);
                paint.anti_alias = true;
                pixmap.fill_path(&path, &paint, FillRule::Winding, transform, None);
            }
        }
        MarkerType::Cross => {
            let half = size * 0.4;
            let (cx, cy) = rotate_point(-half, 0.0, sin, cos, tx, ty);
            let mut pb = PathBuilder::new();
            pb.move_to(cx - half, cy - half);
            pb.line_to(cx + half, cy + half);
            pb.move_to(cx - half, cy + half);
            pb.line_to(cx + half, cy - half);
            if let Some(path) = pb.finish() {
                let mut paint = Paint::default();
                paint.set_color(color);
                paint.anti_alias = true;
                let stroke = Stroke { width: stroke_width, ..Default::default() };
                pixmap.stroke_path(&path, &paint, &stroke, transform, None);
            }
        }
        MarkerType::Aggregation | MarkerType::Composition => {
            // Diamond shape
            let half = size * 0.5;
            let (p1x, p1y) = rotate_point(-half, 0.0, sin, cos, tx, ty);
            let (p2x, p2y) = rotate_point(-half * 0.5, -half * 0.4, sin, cos, tx, ty);
            let (p3x, p3y) = rotate_point(-half * 0.5, half * 0.4, sin, cos, tx, ty);

            let mut pb = PathBuilder::new();
            pb.move_to(tx, ty);
            pb.line_to(p2x, p2y);
            pb.line_to(p1x, p1y);
            pb.line_to(p3x, p3y);
            pb.close();
            if let Some(path) = pb.finish() {
                let mut paint = Paint::default();
                paint.set_color(if matches!(marker, MarkerType::Composition) {
                    color
                } else {
                    tiny_skia::Color::from_rgba8(255, 255, 255, 255)
                });
                paint.anti_alias = true;
                pixmap.fill_path(&path, &paint, FillRule::Winding, transform, None);

                let mut stroke_paint = Paint::default();
                stroke_paint.set_color(color);
                stroke_paint.anti_alias = true;
                let stroke = Stroke { width: stroke_width * 0.5, ..Default::default() };
                pixmap.stroke_path(&path, &stroke_paint, &stroke, transform, None);
            }
        }
        MarkerType::Dependency => {
            // Open arrow (same as ArrowOpen)
            let (p1x, p1y) = rotate_point(-size, -size * 0.5, sin, cos, tx, ty);
            let (p2x, p2y) = rotate_point(-size, size * 0.5, sin, cos, tx, ty);

            let mut pb = PathBuilder::new();
            pb.move_to(p1x, p1y);
            pb.line_to(tx, ty);
            pb.line_to(p2x, p2y);
            if let Some(path) = pb.finish() {
                let mut paint = Paint::default();
                paint.set_color(color);
                paint.anti_alias = true;
                let stroke = Stroke { width: stroke_width, ..Default::default() };
                pixmap.stroke_path(&path, &paint, &stroke, transform, None);
            }
        }
        _ => {} // Future marker types
    }
}

fn rotate_point(dx: f32, dy: f32, sin: f32, cos: f32, cx: f32, cy: f32) -> (f32, f32) {
    (cx + dx * cos - dy * sin, cy + dx * sin + dy * cos)
}

fn first_point(segments: &[PathSegment]) -> Option<Point> {
    segments.first().map(|s| match s {
        PathSegment::MoveTo(p) | PathSegment::LineTo(p) => *p,
        PathSegment::CubicTo { to, .. } | PathSegment::QuadTo { to, .. } | PathSegment::ArcTo { to, .. } => *to,
        PathSegment::Close => Point::new(0.0, 0.0),
    })
}

fn last_point(segments: &[PathSegment]) -> Option<Point> {
    for seg in segments.iter().rev() {
        match seg {
            PathSegment::MoveTo(p) | PathSegment::LineTo(p) => return Some(*p),
            PathSegment::CubicTo { to, .. } | PathSegment::QuadTo { to, .. } | PathSegment::ArcTo { to, .. } => return Some(*to),
            PathSegment::Close => continue,
        }
    }
    None
}

fn start_angle(segments: &[PathSegment]) -> f64 {
    if segments.len() < 2 { return 0.0; }
    let p0 = match &segments[0] {
        PathSegment::MoveTo(p) => *p,
        _ => return 0.0,
    };
    let p1 = match &segments[1] {
        PathSegment::LineTo(p) | PathSegment::MoveTo(p) => *p,
        PathSegment::CubicTo { cp1, .. } => *cp1,
        PathSegment::QuadTo { cp, .. } => *cp,
        PathSegment::ArcTo { to, .. } => *to,
        PathSegment::Close => return 0.0,
    };
    // Angle from p1 to p0 (marker points backwards)
    (p0.y - p1.y).atan2(p0.x - p1.x)
}

fn end_angle(segments: &[PathSegment]) -> f64 {
    let points: Vec<Point> = segments.iter().filter_map(|s| match s {
        PathSegment::MoveTo(p) | PathSegment::LineTo(p) => Some(*p),
        PathSegment::CubicTo { to, .. } | PathSegment::QuadTo { to, .. } | PathSegment::ArcTo { to, .. } => Some(*to),
        PathSegment::Close => None,
    }).collect();
    if points.len() < 2 { return 0.0; }
    let last = points[points.len() - 1];
    let prev = points[points.len() - 2];
    (last.y - prev.y).atan2(last.x - prev.x)
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
    config: &RasterConfig,
) {
    // Build arc wedge/annular sector as a path
    let cx = center.x as f32;
    let cy = center.y as f32;
    let or = outer_r as f32;
    let ir = inner_r as f32;

    let steps = 64;
    let angle_span = end_angle - start_angle;

    let mut pb = PathBuilder::new();

    // Outer arc
    for i in 0..=steps {
        let t = start_angle + angle_span * (i as f64 / steps as f64);
        let x = cx + or * t.cos() as f32;
        let y = cy + or * t.sin() as f32;
        if i == 0 { pb.move_to(x, y); } else { pb.line_to(x, y); }
    }

    if ir > 0.0 {
        // Inner arc (reverse)
        for i in (0..=steps).rev() {
            let t = start_angle + angle_span * (i as f64 / steps as f64);
            let x = cx + ir * t.cos() as f32;
            let y = cy + ir * t.sin() as f32;
            pb.line_to(x, y);
        }
    } else {
        // Pie slice: connect to center
        pb.line_to(cx, cy);
    }
    pb.close();

    let Some(path) = pb.finish() else { return };

    if let Some(fill) = resolve_fill(style) {
        let mut paint = Paint::default();
        paint.set_color(fill);
        paint.anti_alias = true;
        pixmap.fill_path(&path, &paint, FillRule::Winding, transform, None);
    }
    if let Some((stroke_color, width)) = resolve_stroke(style, config) {
        let mut paint = Paint::default();
        paint.set_color(stroke_color);
        paint.anti_alias = true;
        let stroke = make_stroke(width, style);
        pixmap.stroke_path(&path, &paint, &stroke, transform, None);
    }
}
