use vello::kurbo::{self, Affine, BezPath, Cap, Join, Point as KPoint, Rect, RoundedRect, Stroke, Vec2};
use vello::peniko::{Color as VelloColor, Fill};
use vello::Scene as VelloScene;

use rusty_mermaid_core::{
    Color, MarkerType, PathSegment, Point, Primitive, Style, TextAnchor, Theme, Transform,
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
            // Text rendering via vello requires skrifa font loading + manual glyph layout.
            // For now, render a subtle rect placeholder (same approach as raster initial).
            // Full glyph rendering will be wired when parley/skrifa integration is added.
            let font_size = style.font_size;
            let char_w = font_size * 0.6;
            let text_w = char_w * content.len() as f64;

            let x = match anchor {
                TextAnchor::Start => position.x,
                TextAnchor::Middle => position.x - text_w / 2.0,
                TextAnchor::End => position.x - text_w,
            };
            let y = position.y - font_size / 2.0;
            let rect = Rect::new(x, y, x + text_w, y + font_size);

            let fill = style.fill.unwrap_or(Color::rgb(51, 51, 51));
            let c = to_vello_color(fill).multiply_alpha(0.15);
            scene.fill(Fill::NonZero, transform, c, None, &rect);
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
    let size = (stroke_width * 4.0).max(6.0);
    let (sin, cos) = angle.sin_cos();
    let tx = tip.x;
    let ty = tip.y;

    match marker {
        MarkerType::ArrowPoint | MarkerType::ArrowBarb => {
            let (p1x, p1y) = rotate_point(-size, -size * 0.5, sin, cos, tx, ty);
            let (p2x, p2y) = rotate_point(-size, size * 0.5, sin, cos, tx, ty);

            let mut path = BezPath::new();
            path.move_to(KPoint::new(tx, ty));
            path.line_to(KPoint::new(p1x, p1y));
            path.line_to(KPoint::new(p2x, p2y));
            path.close_path();
            scene.fill(Fill::NonZero, transform, to_vello_color(color), None, &path);
        }
        MarkerType::ArrowOpen | MarkerType::Dependency => {
            let (p1x, p1y) = rotate_point(-size, -size * 0.5, sin, cos, tx, ty);
            let (p2x, p2y) = rotate_point(-size, size * 0.5, sin, cos, tx, ty);

            let mut path = BezPath::new();
            path.move_to(KPoint::new(p1x, p1y));
            path.line_to(KPoint::new(tx, ty));
            path.line_to(KPoint::new(p2x, p2y));
            let stroke = Stroke::new(stroke_width);
            scene.stroke(&stroke, transform, to_vello_color(color), None, &path);
        }
        MarkerType::Circle => {
            let r = size * 0.4;
            let (mcx, mcy) = rotate_point(-r, 0.0, sin, cos, tx, ty);
            let circle = kurbo::Circle::new(KPoint::new(mcx, mcy), r);
            scene.fill(Fill::NonZero, transform, to_vello_color(color), None, &circle);
        }
        MarkerType::Cross => {
            let half = size * 0.4;
            let (mcx, mcy) = rotate_point(-half, 0.0, sin, cos, tx, ty);
            let mut path = BezPath::new();
            path.move_to(KPoint::new(mcx - half, mcy - half));
            path.line_to(KPoint::new(mcx + half, mcy + half));
            path.move_to(KPoint::new(mcx - half, mcy + half));
            path.line_to(KPoint::new(mcx + half, mcy - half));
            let stroke = Stroke::new(stroke_width);
            scene.stroke(&stroke, transform, to_vello_color(color), None, &path);
        }
        MarkerType::Aggregation | MarkerType::Composition => {
            let half = size * 0.5;
            let (p1x, p1y) = rotate_point(-half, 0.0, sin, cos, tx, ty);
            let (p2x, p2y) = rotate_point(-half * 0.5, -half * 0.4, sin, cos, tx, ty);
            let (p3x, p3y) = rotate_point(-half * 0.5, half * 0.4, sin, cos, tx, ty);

            let mut path = BezPath::new();
            path.move_to(KPoint::new(tx, ty));
            path.line_to(KPoint::new(p2x, p2y));
            path.line_to(KPoint::new(p1x, p1y));
            path.line_to(KPoint::new(p3x, p3y));
            path.close_path();

            let fill = if matches!(marker, MarkerType::Composition) {
                color
            } else {
                Color::WHITE
            };
            scene.fill(Fill::NonZero, transform, to_vello_color(fill), None, &path);
            let stroke = Stroke::new(stroke_width * 0.5);
            scene.stroke(&stroke, transform, to_vello_color(color), None, &path);
        }
        _ => {}
    }
}

fn rotate_point(dx: f64, dy: f64, sin: f64, cos: f64, cx: f64, cy: f64) -> (f64, f64) {
    (cx + dx * cos - dy * sin, cy + dx * sin + dy * cos)
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
