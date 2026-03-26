pub mod ir;
pub mod parser;

use rusty_mermaid_core::{
    BBox, Color, PathSegment, Point, Primitive, Scene, SimpleTextMeasure, Style, TextAnchor,
    TextStyle, Theme,
};

use ir::IshikawaDiagram;

const SPINE_BASE: f64 = 300.0;
const SPINE_PER_CAT: f64 = 120.0;
const BONE_BASE: f64 = 80.0;
const BONE_PER_CHILD: f64 = 20.0;
const SUB_BONE_LEN: f64 = 70.0;
const SUB_BONE_GAP: f64 = 22.0;
const ANGLE_DEG: f64 = 75.0;
const HEAD_W: f64 = 120.0;
const HEAD_H: f64 = 36.0;
const SCENE_PAD: f64 = 40.0;
const LABEL_FONT: f64 = 11.0;

const COLORS: [Color; 8] = [
    Color::rgb(78, 121, 167),
    Color::rgb(242, 142, 44),
    Color::rgb(225, 87, 89),
    Color::rgb(118, 183, 178),
    Color::rgb(89, 161, 79),
    Color::rgb(237, 201, 73),
    Color::rgb(175, 122, 161),
    Color::rgb(255, 157, 167),
];

pub fn to_scene(diagram: &IshikawaDiagram) -> Scene {
    to_scene_themed(diagram, &Theme::default())
}

/// Precomputed spine geometry shared across rendering helpers.
struct SpineLayout {
    spine_y: f64,
    spine_left: f64,
    spine_end_x: f64,
    head_w: f64,
    bone_lengths: Vec<f64>,
    cos_a: f64,
    sin_a: f64,
    cat_spacing: f64,
}

impl SpineLayout {
    fn from_diagram(diagram: &IshikawaDiagram, scene_w: f64, scene_h: f64) -> Self {
        let n_cats = diagram.categories.len();
        let angle = ANGLE_DEG.to_radians();
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        let spine_len = SPINE_BASE + n_cats as f64 * SPINE_PER_CAT;

        let bone_lengths: Vec<f64> = diagram
            .categories
            .iter()
            .map(|cat| BONE_BASE + cat.total_causes() as f64 * BONE_PER_CHILD)
            .collect();

        let spine_y = scene_h / 2.0;
        let spine_right = scene_w - SCENE_PAD;
        let spine_left = spine_right - spine_len;

        let label_style = TextStyle {
            font_size: 14.0,
            ..Default::default()
        };
        let effect_w = SimpleTextMeasure::measure_raw(&diagram.effect, &label_style).width + 24.0;
        let head_w = effect_w.max(HEAD_W);
        let spine_end_x = spine_right - head_w;

        let usable_spine = spine_end_x - spine_left - SCENE_PAD;
        let cat_spacing = usable_spine / (n_cats as f64 + 1.0);

        Self {
            spine_y,
            spine_left,
            spine_end_x,
            head_w,
            bone_lengths,
            cos_a,
            sin_a,
            cat_spacing,
        }
    }
}

pub fn to_scene_themed(diagram: &IshikawaDiagram, theme: &Theme) -> Scene {
    let n_cats = diagram.categories.len();
    if n_cats == 0 {
        return Scene::empty();
    }

    let angle = ANGLE_DEG.to_radians();
    let bone_lengths: Vec<f64> = diagram
        .categories
        .iter()
        .map(|cat| BONE_BASE + cat.total_causes() as f64 * BONE_PER_CHILD)
        .collect();
    let max_bone = bone_lengths.iter().copied().fold(0.0f64, f64::max);
    let max_vertical = max_bone * angle.sin();
    let spine_len = SPINE_BASE + n_cats as f64 * SPINE_PER_CAT;

    let scene_w = SCENE_PAD * 2.0 + spine_len + HEAD_W;
    let scene_h = SCENE_PAD * 2.0 + max_vertical * 2.0 + HEAD_H;
    let mut scene = Scene::new(scene_w, scene_h);

    let layout = SpineLayout::from_diagram(diagram, scene_w, scene_h);

    render_effect_head(&mut scene, diagram, &layout, theme);
    render_spine_line(&mut scene, &layout, theme);
    render_category_bones(&mut scene, diagram, &layout, theme);

    scene
}

fn render_effect_head(
    scene: &mut Scene,
    diagram: &IshikawaDiagram,
    layout: &SpineLayout,
    theme: &Theme,
) {
    let spine_right = layout.spine_end_x + layout.head_w;
    let cx = spine_right - layout.head_w / 2.0;

    scene.push(Primitive::Rect {
        bbox: BBox::new(cx, layout.spine_y, layout.head_w, HEAD_H),
        rx: 4.0,
        ry: 4.0,
        style: Style {
            fill: Some(theme.node_stroke),
            ..Default::default()
        },
    });
    scene.push(Primitive::Text {
        position: Point::new(cx, layout.spine_y),
        content: diagram.effect.clone(),
        anchor: TextAnchor::Middle,
        style: TextStyle {
            font_size: theme.font_size_node,
            fill: Some(theme.background),
            font_weight: rusty_mermaid_core::FontWeight::Bold,
            ..Default::default()
        },
    });
}

fn render_spine_line(scene: &mut Scene, layout: &SpineLayout, theme: &Theme) {
    scene.push(Primitive::Path {
        segments: vec![
            PathSegment::MoveTo(Point::new(layout.spine_left, layout.spine_y)),
            PathSegment::LineTo(Point::new(layout.spine_end_x, layout.spine_y)),
        ],
        style: Style {
            stroke: Some(theme.edge_stroke),
            stroke_width: Some(2.5),
            ..Default::default()
        },
        marker_start: None,
        marker_end: None,
    });
}

fn render_category_bones(
    scene: &mut Scene,
    diagram: &IshikawaDiagram,
    layout: &SpineLayout,
    theme: &Theme,
) {
    for (ci, cat) in diagram.categories.iter().enumerate() {
        let color = COLORS[ci % COLORS.len()];
        let direction: f64 = if ci % 2 == 0 { -1.0 } else { 1.0 };

        let attach_x = layout.spine_left + layout.cat_spacing * (ci as f64 + 1.0);
        let bone_len = layout.bone_lengths[ci];

        let spine_half = 1.5;
        let start_x = attach_x - layout.cos_a * spine_half;
        let start_y = layout.spine_y + direction * layout.sin_a * spine_half;
        let tip_x = attach_x - bone_len * layout.cos_a;
        let tip_y = layout.spine_y + direction * bone_len * layout.sin_a;

        scene.push(Primitive::Path {
            segments: vec![
                PathSegment::MoveTo(Point::new(start_x, start_y)),
                PathSegment::LineTo(Point::new(tip_x, tip_y)),
            ],
            style: Style {
                stroke: Some(color),
                stroke_width: Some(2.0),
                ..Default::default()
            },
            marker_start: None,
            marker_end: None,
        });

        scene.push(Primitive::Text {
            position: Point::new(tip_x, tip_y + direction * 14.0),
            content: cat.name.clone(),
            anchor: TextAnchor::Middle,
            style: TextStyle {
                font_size: theme.font_size_node,
                fill: Some(color),
                font_weight: rusty_mermaid_core::FontWeight::Bold,
                ..Default::default()
            },
        });

        render_cause_bones(
            scene,
            cat,
            attach_x,
            tip_x,
            tip_y,
            layout.spine_y,
            direction,
            color,
            theme,
        );
    }
}

fn render_cause_bones(
    scene: &mut Scene,
    cat: &ir::Category,
    attach_x: f64,
    tip_x: f64,
    tip_y: f64,
    spine_y: f64,
    direction: f64,
    color: Color,
    theme: &Theme,
) {
    let n_causes = cat.causes.len();
    for (ki, cause) in cat.causes.iter().enumerate() {
        let t = (ki as f64 + 1.0) / (n_causes as f64 + 1.0);
        let cx = attach_x + t * (tip_x - attach_x);
        let cy = spine_y + t * (tip_y - spine_y);
        let sub_len = SUB_BONE_LEN + cause.subcauses.len() as f64 * 10.0;

        scene.push(Primitive::Path {
            segments: vec![
                PathSegment::MoveTo(Point::new(cx, cy)),
                PathSegment::LineTo(Point::new(cx - sub_len, cy)),
            ],
            style: Style {
                stroke: Some(color),
                stroke_width: Some(1.2),
                ..Default::default()
            },
            marker_start: None,
            marker_end: None,
        });

        scene.push(Primitive::Text {
            position: Point::new(cx - sub_len - 4.0, cy),
            content: cause.name.clone(),
            anchor: TextAnchor::End,
            style: TextStyle {
                font_size: LABEL_FONT,
                fill: Some(theme.node_text),
                ..Default::default()
            },
        });

        let n_subs = cause.subcauses.len();
        for (si, sub) in cause.subcauses.iter().enumerate() {
            let gap = SUB_BONE_GAP + 8.0;
            let sx = cx - (si as f64 + 1.0) * gap;
            let tick_len = 20.0 + (n_subs - 1 - si) as f64 * 8.0;
            let sub_tip_y = cy + direction * tick_len;

            scene.push(Primitive::Path {
                segments: vec![
                    PathSegment::MoveTo(Point::new(sx, cy)),
                    PathSegment::LineTo(Point::new(sx - tick_len * 0.15, sub_tip_y)),
                ],
                style: Style {
                    stroke: Some(color),
                    stroke_width: Some(0.8),
                    ..Default::default()
                },
                marker_start: None,
                marker_end: None,
            });

            scene.push(Primitive::Text {
                position: Point::new(sx - tick_len * 0.15 - 3.0, sub_tip_y + direction * 10.0),
                content: sub.name.clone(),
                anchor: TextAnchor::End,
                style: TextStyle {
                    font_size: 9.0,
                    fill: Some(theme.muted_text),
                    ..Default::default()
                },
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(input: &str) -> Scene {
        let d = parser::parse(input).unwrap();
        to_scene(&d)
    }

    #[test]
    fn basic_renders() {
        let scene =
            render("ishikawa-beta\n    Problem\n    Cat A\n        C1\n    Cat B\n        C2");
        assert!(!scene.is_empty());
    }

    #[test]
    fn has_spine_line() {
        let scene = render("ishikawa-beta\n    Effect\n    Cat\n        Cause");
        let paths = scene
            .elements()
            .iter()
            .filter(|e| {
                if let Primitive::Path { style, .. } = &e.primitive {
                    style.stroke_width == Some(2.5)
                } else {
                    false
                }
            })
            .count();
        assert_eq!(paths, 1, "should have 1 spine line");
    }

    #[test]
    fn categories_alternate_above_below() {
        let scene = render(
            "ishikawa-beta\n    E\n    A\n        c1\n    B\n        c2\n    C\n        c3\n    D\n        c4",
        );
        // Category bone endpoints should alternate y position relative to spine
        let bone_endpoints: Vec<f64> = scene
            .elements()
            .iter()
            .filter_map(|e| {
                if let Primitive::Path {
                    segments, style, ..
                } = &e.primitive
                {
                    if style.stroke_width == Some(2.0) {
                        if let Some(PathSegment::LineTo(p)) = segments.last() {
                            return Some(p.y);
                        }
                    }
                }
                None
            })
            .collect();
        // First should be above center, second below (or vice versa)
        assert!(bone_endpoints.len() >= 4);
        // Check alternation: signs of (y - center) should flip
        let center = scene.height / 2.0;
        for w in bone_endpoints.windows(2) {
            let sign_a = (w[0] - center).signum();
            let sign_b = (w[1] - center).signum();
            assert!(
                sign_a != sign_b,
                "consecutive bones should alternate above/below"
            );
        }
    }

    #[test]
    fn effect_label_present() {
        let scene = render("ishikawa-beta\n    Bug\n    Code\n        Typo");
        let has_bug = scene.elements().iter().any(|e| {
            if let Primitive::Text { content, .. } = &e.primitive {
                content == "Bug"
            } else {
                false
            }
        });
        assert!(has_bug);
    }

    #[test]
    fn all_positions_finite() {
        let scene = render(
            "ishikawa-beta\n    E\n    A\n        a1\n            sub1\n        a2\n    B\n        b1",
        );
        for elem in scene.elements() {
            match &elem.primitive {
                Primitive::Rect { bbox, .. } => {
                    assert!(bbox.x.is_finite() && bbox.y.is_finite());
                }
                Primitive::Text { position, .. } => {
                    assert!(position.x.is_finite() && position.y.is_finite());
                }
                Primitive::Path { segments, .. } => {
                    for seg in segments {
                        match seg {
                            PathSegment::MoveTo(p) | PathSegment::LineTo(p) => {
                                assert!(p.x.is_finite() && p.y.is_finite());
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
