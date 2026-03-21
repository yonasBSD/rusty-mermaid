use std::sync::{Arc, OnceLock};

use vello::kurbo::{self, Affine, BezPath, Cap, Join, Point as KPoint, Rect, RoundedRect, Stroke, Vec2};
use vello::peniko::{Blob, Color as VelloColor, Fill, FontData};
use vello::Scene as VelloScene;

use rusty_mermaid_core::{
    marker_geometry, transform_marker_circle, transform_marker_curves, transform_marker_points,
    Color, MarkerShape, MarkerType, PathSegment, Point, Primitive, Style, TextAnchor, Theme,
    Transform,
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
                if let Some((tip, angle)) = first_point_angle(segments) {
                    paint_marker(scene, *marker, tip, angle, color, width, transform);
                }
            }
            if let Some(marker) = marker_end {
                if let Some((tip, angle)) = last_point_angle(segments) {
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
                // kurbo Arc expects center parameterization; convert via line for now.
                // Full SVG arc conversion deferred to parley/kurbo integration.
                path.line_to(to_kpoint(to));
                let _ = (rx, ry, rotation, large_arc, sweep);
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

// ── Path endpoint angles for markers ──

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

// ── Text rendering ──

static DEFAULT_FONT_DATA: OnceLock<FontData> = OnceLock::new();

fn get_font_data(theme: &Theme) -> FontData {
    if let Some(ref custom_bytes) = theme.custom_font {
        FontData::new(Blob::new(Arc::new(custom_bytes.clone())), 0)
    } else {
        DEFAULT_FONT_DATA
            .get_or_init(|| {
                let bytes = include_bytes!("../../raster/fonts/IntelOneMono-Regular.ttf");
                FontData::new(Blob::new(Arc::new(bytes.to_vec())), 0)
            })
            .clone()
    }
}

fn render_text(
    scene: &mut VelloScene,
    position: &Point,
    content: &str,
    anchor: TextAnchor,
    style: &rusty_mermaid_core::TextStyle,
    theme: &Theme,
    transform: Affine,
) {
    let font_data = get_font_data(theme);
    let font_size = style.font_size as f32;
    let fill_color = style.fill.unwrap_or(Color::rgb(51, 51, 51));

    let font_ref = skrifa::FontRef::from_index(font_data.data.as_ref(), font_data.index)
        .expect("embedded font must be valid");
    let charmap = skrifa::MetadataProvider::charmap(&font_ref);
    let glyph_metrics = skrifa::MetadataProvider::glyph_metrics(&font_ref, skrifa::instance::Size::new(font_size), skrifa::instance::LocationRef::default());

    let lines: Vec<&str> = content.split('\n').collect();
    let line_height = font_size * 1.2;
    let total_h = line_height * (lines.len() - 1) as f32;
    let base_y = position.y as f32 - total_h / 2.0;

    for (line_idx, line) in lines.iter().enumerate() {
        // Measure line width
        let mut line_w: f32 = 0.0;
        let mut glyphs = Vec::new();
        for ch in line.chars() {
            let gid = charmap.map(ch).unwrap_or_default();
            let advance = glyph_metrics.advance_width(gid).unwrap_or(font_size * 0.6);
            glyphs.push((gid, line_w));
            line_w += advance;
        }

        let start_x = match anchor {
            TextAnchor::Start => position.x as f32,
            TextAnchor::Middle => position.x as f32 - line_w / 2.0,
            TextAnchor::End => position.x as f32 - line_w,
        };
        let line_y = base_y + line_idx as f32 * line_height;

        let glyph_iter = glyphs.into_iter().map(|(gid, x_off)| vello::Glyph {
            id: gid.to_u32(),
            x: start_x + x_off,
            y: line_y,
        });

        scene
            .draw_glyphs(&font_data)
            .font_size(font_size)
            .transform(transform)
            .brush(to_vello_color(fill_color))
            .draw(Fill::NonZero, glyph_iter);
    }
}
