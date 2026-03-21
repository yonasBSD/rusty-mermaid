use gpui::{
    point, px, quad, font, App, Bounds, BorderStyle, Edges, Hsla, PathBuilder, Pixels,
    Point as GpuiPoint, TextRun, Window,
};
use rusty_mermaid_core::{
    marker_geometry, text_baseline_y_offset, transform_marker_circle, transform_marker_curves,
    transform_marker_points, Color, MarkerShape, MarkerType, PathSegment, Point, Primitive, Style,
    TextAnchor, Theme, Transform,
};
use rusty_mermaid_viewport::ViewportState;

pub fn paint_scene(
    scene: &rusty_mermaid_core::Scene,
    theme: &Theme,
    viewport: &ViewportState,
    bounds: Bounds<Pixels>,
    window: &mut Window,
    cx: &mut App,
) {
    let padding = theme.padding;
    let zoom = viewport.zoom as f32;
    let ox = viewport.offset.x as f32 + padding as f32;
    let oy = viewport.offset.y as f32 + padding as f32;

    for elem in scene.elements() {
        paint_primitive(&elem.primitive, theme, zoom, ox, oy, bounds, window, cx);
    }
}

fn paint_primitive(
    prim: &Primitive,
    theme: &Theme,
    zoom: f32,
    ox: f32,
    oy: f32,
    bounds: Bounds<Pixels>,
    window: &mut Window,
    cx: &mut App,
) {
    match prim {
        Primitive::Rect { bbox, rx, style, .. } => {
            let left = (bbox.x - bbox.width / 2.0) as f32 * zoom + ox;
            let top = (bbox.y - bbox.height / 2.0) as f32 * zoom + oy;
            let w = bbox.width as f32 * zoom;
            let h = bbox.height as f32 * zoom;
            let r = *rx as f32 * zoom;

            let rect_bounds = Bounds {
                origin: point(px(left), px(top)),
                size: gpui::size(px(w), px(h)),
            };

            let bg = style.fill.map(to_gpui_color).unwrap_or(gpui::transparent_black());
            let (border_color, border_width) = resolve_stroke_gpui(style, theme);

            window.paint_quad(quad(
                rect_bounds,
                px(r),
                bg,
                Edges::all(px(border_width * zoom)),
                border_color,
                BorderStyle::Solid,
            ));
        }

        Primitive::Circle { center, radius, style } => {
            let cx = center.x as f32 * zoom + ox;
            let cy = center.y as f32 * zoom + oy;
            let r = *radius as f32 * zoom;

            if let Some(fill) = style.fill {
                if let Ok(path) = circle_fill_path(cx, cy, r) {
                    window.paint_path(path, to_gpui_color(fill));
                }
            }
            if let Some((color, width)) = resolve_stroke_opt(style, theme) {
                if let Ok(path) = circle_stroke_path(cx, cy, r, width * zoom) {
                    window.paint_path(path, color);
                }
            }
        }

        Primitive::Ellipse { center, rx, ry, style } => {
            let ecx = center.x as f32 * zoom + ox;
            let ecy = center.y as f32 * zoom + oy;
            let erx = *rx as f32 * zoom;
            let ery = *ry as f32 * zoom;

            if let Some(fill) = style.fill {
                if let Ok(path) = ellipse_fill_path(ecx, ecy, erx, ery) {
                    window.paint_path(path, to_gpui_color(fill));
                }
            }
            if let Some((color, width)) = resolve_stroke_opt(style, theme) {
                if let Ok(path) = ellipse_stroke_path(ecx, ecy, erx, ery, width * zoom) {
                    window.paint_path(path, color);
                }
            }
        }

        Primitive::Path { segments, style, marker_start, marker_end } => {
            let (color, width) = stroke_or_default(style, theme);
            if let Ok(path) = build_stroke_path(segments, zoom, ox, oy, width * zoom, style) {
                window.paint_path(path, color);
            }

            if let Some(fill) = style.fill {
                if let Ok(path) = build_fill_path(segments, zoom, ox, oy) {
                    window.paint_path(path, to_gpui_color(fill));
                }
            }

            if let Some(marker) = marker_start {
                if let Some((tip, angle)) = first_point_angle(segments) {
                    paint_marker(window, *marker, tip, angle, color, width * zoom, zoom, ox, oy);
                }
            }
            if let Some(marker) = marker_end {
                if let Some((tip, angle)) = last_point_angle(segments) {
                    paint_marker(window, *marker, tip, angle, color, width * zoom, zoom, ox, oy);
                }
            }
        }

        Primitive::Text { position, content, anchor, style } => {
            let x = position.x as f32 * zoom + ox;
            let y = position.y as f32 * zoom + oy;
            let font_size = style.font_size as f32 * zoom;
            let fill = style.fill.unwrap_or(Color::rgb(51, 51, 51));

            let f = font("monospace");

            let runs = vec![TextRun {
                len: content.len(),
                font: f,
                color: to_gpui_color(fill),
                background_color: None,
                underline: None,
                strikethrough: None,
            }];

            let shaped = window.text_system().shape_line(
                content.clone().into(),
                px(font_size),
                &runs,
                None,
            );

            let text_w: f32 = shaped.width.into();
            let text_x = match anchor {
                TextAnchor::Start => x,
                TextAnchor::Middle => x - text_w / 2.0,
                TextAnchor::End => x - text_w,
            };
            let baseline_offset = text_baseline_y_offset(style.font_size, 1) as f32 * zoom;
            let text_y = y + baseline_offset - font_size; // gpui paints from top, not baseline

            let _ = shaped.paint(
                point(px(text_x), px(text_y)),
                px(font_size * 1.2),
                window,
                cx,
            );
        }

        Primitive::Polygon { points, style } => {
            if points.len() < 3 { return; }

            if let Some(fill) = style.fill {
                let mut pb = PathBuilder::fill();
                pb.move_to(transform_pt(&points[0], zoom, ox, oy));
                for p in &points[1..] {
                    pb.line_to(transform_pt(p, zoom, ox, oy));
                }
                pb.close();
                if let Ok(path) = pb.build() {
                    window.paint_path(path, to_gpui_color(fill));
                }
            }
            if let Some((color, width)) = resolve_stroke_opt(style, theme) {
                let mut pb = PathBuilder::stroke(px(width * zoom));
                pb.move_to(transform_pt(&points[0], zoom, ox, oy));
                for p in &points[1..] {
                    pb.line_to(transform_pt(p, zoom, ox, oy));
                }
                pb.close();
                if let Ok(path) = pb.build() {
                    window.paint_path(path, color);
                }
            }
        }

        Primitive::Group { transform, children } => {
            let (nzoom, nox, noy) = apply_transform(transform, zoom, ox, oy);
            for child in children {
                paint_primitive(child, theme, nzoom, nox, noy, bounds, window, cx);
            }
        }

        Primitive::Arc { center, inner_r, outer_r, start_angle, end_angle, style } => {
            paint_arc(window, center, *inner_r, *outer_r, *start_angle, *end_angle, style, theme, zoom, ox, oy);
        }
    }
}

// ── Helpers ──

fn to_gpui_color(c: Color) -> Hsla {
    let r = c.r as f32 / 255.0;
    let g = c.g as f32 / 255.0;
    let b = c.b as f32 / 255.0;
    let a = c.a as f32 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;

    if (max - min).abs() < 1e-6 {
        return Hsla { h: 0.0, s: 0.0, l, a };
    }

    let d = max - min;
    let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
    let h = if (max - r).abs() < 1e-6 {
        ((g - b) / d + if g < b { 6.0 } else { 0.0 }) / 6.0
    } else if (max - g).abs() < 1e-6 {
        ((b - r) / d + 2.0) / 6.0
    } else {
        ((r - g) / d + 4.0) / 6.0
    };

    Hsla { h, s, l, a }
}

fn resolve_stroke_gpui(style: &Style, theme: &Theme) -> (Hsla, f32) {
    let color = style.stroke.unwrap_or(theme.edge_stroke);
    let width = style.stroke_width.unwrap_or(theme.default_stroke_width) as f32;
    (to_gpui_color(color), width)
}

fn resolve_stroke_opt(style: &Style, theme: &Theme) -> Option<(Hsla, f32)> {
    match (style.stroke, style.stroke_width) {
        (Some(c), Some(w)) => Some((to_gpui_color(c), w as f32)),
        (Some(c), None) => Some((to_gpui_color(c), theme.default_stroke_width as f32)),
        (None, Some(w)) => Some((to_gpui_color(theme.edge_stroke), w as f32)),
        (None, None) => None,
    }
}

fn stroke_or_default(style: &Style, theme: &Theme) -> (Hsla, f32) {
    let color = style.stroke.unwrap_or(theme.edge_stroke);
    let width = style.stroke_width.unwrap_or(theme.default_stroke_width) as f32;
    (to_gpui_color(color), width)
}

fn transform_pt(p: &Point, zoom: f32, ox: f32, oy: f32) -> GpuiPoint<Pixels> {
    point(px(p.x as f32 * zoom + ox), px(p.y as f32 * zoom + oy))
}

fn apply_transform(t: &Transform, zoom: f32, ox: f32, oy: f32) -> (f32, f32, f32) {
    match t {
        Transform::Identity => (zoom, ox, oy),
        Transform::Translate(dx, dy) => (zoom, ox + *dx as f32 * zoom, oy + *dy as f32 * zoom),
        Transform::Scale(sx, _sy) => (*sx as f32 * zoom, ox, oy),
        Transform::Rotate { .. } => (zoom, ox, oy),
    }
}

// ── Path builders ──

fn build_stroke_path(
    segments: &[PathSegment],
    zoom: f32,
    ox: f32,
    oy: f32,
    width: f32,
    style: &Style,
) -> Result<gpui::Path<Pixels>, anyhow::Error> {
    let mut pb = PathBuilder::stroke(px(width));
    if let Some(ref dashes) = style.stroke_dasharray {
        let dash_px: Vec<Pixels> = dashes.iter().map(|d| px(*d as f32 * zoom)).collect();
        pb = pb.dash_array(&dash_px);
    }
    build_path_segments(&mut pb, segments, zoom, ox, oy);
    pb.build()
}

fn build_fill_path(
    segments: &[PathSegment],
    zoom: f32,
    ox: f32,
    oy: f32,
) -> Result<gpui::Path<Pixels>, anyhow::Error> {
    let mut pb = PathBuilder::fill();
    build_path_segments(&mut pb, segments, zoom, ox, oy);
    pb.build()
}

fn build_path_segments(pb: &mut PathBuilder, segments: &[PathSegment], zoom: f32, ox: f32, oy: f32) {
    for seg in segments {
        match seg {
            PathSegment::MoveTo(p) => pb.move_to(transform_pt(p, zoom, ox, oy)),
            PathSegment::LineTo(p) => pb.line_to(transform_pt(p, zoom, ox, oy)),
            PathSegment::CubicTo { cp1, cp2, to } => {
                pb.cubic_bezier_to(
                    transform_pt(to, zoom, ox, oy),
                    transform_pt(cp1, zoom, ox, oy),
                    transform_pt(cp2, zoom, ox, oy),
                );
            }
            PathSegment::QuadTo { cp, to } => {
                pb.curve_to(
                    transform_pt(to, zoom, ox, oy),
                    transform_pt(cp, zoom, ox, oy),
                );
            }
            PathSegment::ArcTo { rx, ry, rotation, large_arc, sweep, to } => {
                let radii = point(px(*rx as f32 * zoom), px(*ry as f32 * zoom));
                pb.arc_to(
                    radii,
                    px(*rotation as f32),
                    *large_arc,
                    *sweep,
                    transform_pt(to, zoom, ox, oy),
                );
            }
            PathSegment::Close => pb.close(),
        }
    }
}

// ── Circle/Ellipse ──

fn circle_fill_path(cx: f32, cy: f32, r: f32) -> Result<gpui::Path<Pixels>, anyhow::Error> {
    ellipse_fill_path(cx, cy, r, r)
}

fn circle_stroke_path(cx: f32, cy: f32, r: f32, width: f32) -> Result<gpui::Path<Pixels>, anyhow::Error> {
    ellipse_stroke_path(cx, cy, r, r, width)
}

fn ellipse_fill_path(cx: f32, cy: f32, rx: f32, ry: f32) -> Result<gpui::Path<Pixels>, anyhow::Error> {
    let mut pb = PathBuilder::fill();
    let radii = point(px(rx), px(ry));
    pb.move_to(point(px(cx + rx), px(cy)));
    pb.arc_to(radii, px(0.0), false, true, point(px(cx - rx), px(cy)));
    pb.arc_to(radii, px(0.0), false, true, point(px(cx + rx), px(cy)));
    pb.close();
    pb.build()
}

fn ellipse_stroke_path(cx: f32, cy: f32, rx: f32, ry: f32, width: f32) -> Result<gpui::Path<Pixels>, anyhow::Error> {
    let mut pb = PathBuilder::stroke(px(width));
    let radii = point(px(rx), px(ry));
    pb.move_to(point(px(cx + rx), px(cy)));
    pb.arc_to(radii, px(0.0), false, true, point(px(cx - rx), px(cy)));
    pb.arc_to(radii, px(0.0), false, true, point(px(cx + rx), px(cy)));
    pb.close();
    pb.build()
}

// ── Markers ──

fn paint_marker(
    window: &mut Window,
    marker: MarkerType,
    tip: Point,
    angle: f64,
    color: Hsla,
    stroke_width: f32,
    zoom: f32,
    ox: f32,
    oy: f32,
) {
    let geom = marker_geometry(marker);
    let sw = stroke_width as f64 / zoom as f64; // undo zoom for geometry, re-applied via transform_pt

    // Transform shared geometry to scene coords, then apply zoom+offset
    let scene_to_screen = |p: &Point| -> GpuiPoint<Pixels> {
        point(px(p.x as f32 * zoom + ox), px(p.y as f32 * zoom + oy))
    };

    match &geom.shape {
        MarkerShape::FilledPath(_) => {
            let pts = transform_marker_points(&geom, tip, angle, sw);
            let mut pb = PathBuilder::fill();
            if let Some(first) = pts.first() { pb.move_to(scene_to_screen(first)); }
            for p in pts.iter().skip(1) { pb.line_to(scene_to_screen(p)); }
            pb.close();
            if let Ok(path) = pb.build() { window.paint_path(path, color); }
        }
        MarkerShape::StrokedPath { closed, stroke_width: rel_sw, .. } => {
            let pts = transform_marker_points(&geom, tip, angle, sw);
            let w = (*rel_sw * sw / geom.vb_w * geom.marker_w) as f32 * zoom;
            let mut pb = PathBuilder::stroke(px(w));
            if let Some(first) = pts.first() { pb.move_to(scene_to_screen(first)); }
            for p in pts.iter().skip(1) { pb.line_to(scene_to_screen(p)); }
            if *closed { pb.close(); }
            if let Ok(path) = pb.build() { window.paint_path(path, color); }
        }
        MarkerShape::FilledStrokedPath { fill_is_marker_color, stroke_width: rel_sw, .. } => {
            let pts = transform_marker_points(&geom, tip, angle, sw);
            let fill = if *fill_is_marker_color { color } else { Hsla { h: 0.0, s: 0.0, l: 1.0, a: 1.0 } };
            let mut pb = PathBuilder::fill();
            if let Some(first) = pts.first() { pb.move_to(scene_to_screen(first)); }
            for p in pts.iter().skip(1) { pb.line_to(scene_to_screen(p)); }
            pb.close();
            if let Ok(path) = pb.build() { window.paint_path(path, fill); }
            let w = (*rel_sw * sw / geom.vb_w * geom.marker_w) as f32 * zoom;
            let mut pb = PathBuilder::stroke(px(w));
            if let Some(first) = pts.first() { pb.move_to(scene_to_screen(first)); }
            for p in pts.iter().skip(1) { pb.line_to(scene_to_screen(p)); }
            pb.close();
            if let Ok(path) = pb.build() { window.paint_path(path, color); }
        }
        MarkerShape::FilledCircle { .. } => {
            let (center, r) = transform_marker_circle(&geom, tip, angle, sw);
            if let Ok(path) = circle_fill_path(center.x as f32 * zoom + ox, center.y as f32 * zoom + oy, r as f32 * zoom) {
                window.paint_path(path, color);
            }
        }
        MarkerShape::StrokedCurves { stroke_width: rel_sw, .. } => {
            let curves = transform_marker_curves(&geom, tip, angle, sw);
            let w = (*rel_sw * sw / geom.vb_w * geom.marker_w) as f32 * zoom;
            for curve in &curves {
                if curve.len() >= 3 {
                    let mut pb = PathBuilder::stroke(px(w));
                    pb.move_to(scene_to_screen(&curve[0]));
                    pb.curve_to(scene_to_screen(&curve[2]), scene_to_screen(&curve[1]));
                    if let Ok(path) = pb.build() { window.paint_path(path, color); }
                }
            }
        }
    }
}

// ── Arc ──

fn paint_arc(
    window: &mut Window,
    center: &Point,
    inner_r: f64,
    outer_r: f64,
    start_angle: f64,
    end_angle: f64,
    style: &Style,
    _theme: &Theme,
    zoom: f32,
    ox: f32,
    oy: f32,
) {
    let cx = center.x as f32 * zoom + ox;
    let cy = center.y as f32 * zoom + oy;
    let or = outer_r as f32 * zoom;
    let ir = inner_r as f32 * zoom;
    let steps = 64;
    let angle_span = end_angle - start_angle;

    if let Some(fill) = style.fill {
        let mut pb = PathBuilder::fill();
        for i in 0..=steps {
            let t = start_angle + angle_span * (i as f64 / steps as f64);
            let x = cx + or * t.cos() as f32;
            let y = cy + or * t.sin() as f32;
            if i == 0 { pb.move_to(point(px(x), px(y))); } else { pb.line_to(point(px(x), px(y))); }
        }
        if ir > 0.0 {
            for i in (0..=steps).rev() {
                let t = start_angle + angle_span * (i as f64 / steps as f64);
                let x = cx + ir * t.cos() as f32;
                let y = cy + ir * t.sin() as f32;
                pb.line_to(point(px(x), px(y)));
            }
        } else {
            pb.line_to(point(px(cx), px(cy)));
        }
        pb.close();
        if let Ok(path) = pb.build() {
            window.paint_path(path, to_gpui_color(fill));
        }
    }
}

// ── Path endpoint angles ──

fn first_point_angle(segments: &[PathSegment]) -> Option<(Point, f64)> {
    if segments.len() < 2 { return None; }
    let p0 = match &segments[0] {
        PathSegment::MoveTo(p) => *p,
        _ => return None,
    };
    let p1 = match &segments[1] {
        PathSegment::LineTo(p) | PathSegment::MoveTo(p) => *p,
        PathSegment::CubicTo { cp1, .. } => *cp1,
        PathSegment::QuadTo { cp, .. } => *cp,
        PathSegment::ArcTo { to, .. } => *to,
        PathSegment::Close => return None,
    };
    Some((p0, (p0.y - p1.y).atan2(p0.x - p1.x)))
}

fn last_point_angle(segments: &[PathSegment]) -> Option<(Point, f64)> {
    let points: Vec<Point> = segments.iter().filter_map(|s| match s {
        PathSegment::MoveTo(p) | PathSegment::LineTo(p) => Some(*p),
        PathSegment::CubicTo { to, .. } | PathSegment::QuadTo { to, .. } | PathSegment::ArcTo { to, .. } => Some(*to),
        PathSegment::Close => None,
    }).collect();
    if points.len() < 2 { return None; }
    let last = points[points.len() - 1];
    let prev = points[points.len() - 2];
    Some((last, (last.y - prev.y).atan2(last.x - prev.x)))
}
