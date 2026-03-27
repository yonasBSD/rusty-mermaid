pub mod ir;
pub mod parser;

use rusty_mermaid_core::{
    BBox, Color, PathSegment, Point, Primitive, Scene, SimpleTextMeasure, Style, TextAnchor,
    TextStyle, Theme,
};

use crate::common::palette::{BORDER_RADIUS, MAX_SCORE, STROKE_WIDTH, tint_color};
use ir::JourneyDiagram;

const TASK_W: f64 = 120.0;
const TASK_H: f64 = 32.0;
const TASK_GAP: f64 = 10.0;
const SECTION_GAP: f64 = 20.0;
const SECTION_HEADER_H: f64 = 28.0;
const ACTOR_ROW_H: f64 = 32.0;
const SCORE_RANGE_H: f64 = 120.0;
const SCENE_PAD: f64 = 20.0;
const FACE_R: f64 = 12.0;
const ACTOR_R: f64 = 5.0;

const SECTION_COLORS: [Color; 8] = [
    Color::rgb(78, 121, 167),
    Color::rgb(242, 142, 44),
    Color::rgb(225, 87, 89),
    Color::rgb(118, 183, 178),
    Color::rgb(89, 161, 79),
    Color::rgb(237, 201, 73),
    Color::rgb(175, 122, 161),
    Color::rgb(255, 157, 167),
];

const TINT: f64 = 0.12;

pub fn to_scene(diagram: &JourneyDiagram) -> Scene {
    to_scene_themed(diagram, &Theme::default())
}

/// Vertical layout positions derived from the diagram.
struct JourneyLayout {
    ox: f64,
    oy: f64,
    task_top: f64,
    actor_row_y: f64,
    score_top: f64,
    score_bot: f64,
    scene_w: f64,
}

pub fn to_scene_themed(diagram: &JourneyDiagram, theme: &Theme) -> Scene {
    let total_tasks: usize = diagram.sections.iter().map(|s| s.tasks.len()).sum();
    if total_tasks == 0 {
        return Scene::empty();
    }

    let title_h = if diagram.title.is_some() { 30.0 } else { 0.0 };
    let n_sections = diagram.sections.len();

    let content_w = total_tasks as f64 * (TASK_W + TASK_GAP)
        + n_sections.saturating_sub(1) as f64 * SECTION_GAP
        + TASK_GAP;
    let content_h = SECTION_HEADER_H + TASK_H + ACTOR_ROW_H + SCORE_RANGE_H + FACE_R * 2.0 + 30.0;

    let scene_w = content_w + SCENE_PAD * 2.0;
    let scene_h = content_h + title_h + SCENE_PAD * 2.0;
    let mut scene = Scene::new(scene_w, scene_h);

    let oy = SCENE_PAD + title_h;
    let task_top = oy + SECTION_HEADER_H + 8.0;
    let actor_row_y = task_top + TASK_H + 4.0;
    let score_top = actor_row_y + ACTOR_ROW_H;
    let score_bot = score_top + SCORE_RANGE_H + FACE_R * 2.0;

    let layout = JourneyLayout {
        ox: SCENE_PAD,
        oy,
        task_top,
        actor_row_y,
        score_top,
        score_bot,
        scene_w,
    };

    let actors = diagram.all_actors();
    let actor_colors: Vec<Color> = actors
        .iter()
        .enumerate()
        .map(|(i, _)| SECTION_COLORS[i % SECTION_COLORS.len()])
        .collect();

    render_journey_title(&mut scene, diagram, &layout, theme);
    let x = render_sections(&mut scene, diagram, &layout, &actors, &actor_colors, theme);
    let _ = x;
    render_actor_legend(&mut scene, &actors, &actor_colors, &layout, theme);

    scene
}

fn render_journey_title(
    scene: &mut Scene,
    diagram: &JourneyDiagram,
    layout: &JourneyLayout,
    theme: &Theme,
) {
    if let Some(title) = &diagram.title {
        scene.push(Primitive::Text {
            position: Point::new(layout.scene_w / 2.0, SCENE_PAD + 10.0),
            content: title.clone(),
            anchor: TextAnchor::Middle,
            style: TextStyle {
                font_size: theme.font_size_title,
                fill: Some(theme.node_text),
                font_weight: rusty_mermaid_core::FontWeight::Bold,
                ..Default::default()
            },
        });
    }
}

fn render_sections(
    scene: &mut Scene,
    diagram: &JourneyDiagram,
    layout: &JourneyLayout,
    actors: &[String],
    actor_colors: &[Color],
    theme: &Theme,
) -> f64 {
    let mut x = layout.ox + TASK_GAP;

    for (si, section) in diagram.sections.iter().enumerate() {
        let color = SECTION_COLORS[si % SECTION_COLORS.len()];
        let section_w = section.tasks.len() as f64 * (TASK_W + TASK_GAP) - TASK_GAP;

        let fill = tint_color(color, TINT);
        scene.push(Primitive::Rect {
            bbox: BBox::new(
                x + section_w / 2.0,
                layout.oy + SECTION_HEADER_H / 2.0,
                section_w,
                SECTION_HEADER_H,
            ),
            rx: BORDER_RADIUS,
            ry: BORDER_RADIUS,
            style: Style {
                fill: Some(fill),
                ..Default::default()
            },
        });

        scene.push(Primitive::Text {
            position: Point::new(x + section_w / 2.0, layout.oy + SECTION_HEADER_H / 2.0),
            content: section.name.clone(),
            anchor: TextAnchor::Middle,
            style: TextStyle {
                font_size: theme.font_size_node,
                fill: Some(color),
                font_weight: rusty_mermaid_core::FontWeight::Bold,
                ..Default::default()
            },
        });

        for task in &section.tasks {
            let task_cx = x + TASK_W / 2.0;
            render_task(
                scene,
                task,
                task_cx,
                layout,
                color,
                actors,
                actor_colors,
                theme,
            );
            x += TASK_W + TASK_GAP;
        }

        x += SECTION_GAP;
    }

    x
}

fn render_task(
    scene: &mut Scene,
    task: &ir::JourneyTask,
    task_cx: f64,
    layout: &JourneyLayout,
    color: Color,
    actors: &[String],
    actor_colors: &[Color],
    theme: &Theme,
) {
    let task_fill = tint_color(color, TINT * 0.7);
    scene.push(Primitive::Rect {
        bbox: BBox::new(task_cx, layout.task_top + TASK_H / 2.0, TASK_W, TASK_H),
        rx: BORDER_RADIUS,
        ry: BORDER_RADIUS,
        style: Style {
            fill: Some(task_fill),
            stroke: Some(color),
            stroke_width: Some(1.0),
            ..Default::default()
        },
    });

    let label_style = TextStyle {
        font_size: theme.font_size_small,
        fill: Some(theme.node_text),
        ..Default::default()
    };
    let label_w = SimpleTextMeasure::measure_raw(&task.name, &label_style).width;
    if label_w < TASK_W - 8.0 {
        scene.push(Primitive::Text {
            position: Point::new(task_cx, layout.task_top + TASK_H / 2.0),
            content: task.name.clone(),
            anchor: TextAnchor::Middle,
            style: label_style,
        });
    }

    let face_top = layout.score_top + FACE_R;
    let face_bot = layout.score_bot - FACE_R;
    let score_y = face_bot - (task.score as f64 / MAX_SCORE) * (face_bot - face_top);

    scene.push(Primitive::Path {
        segments: vec![
            PathSegment::MoveTo(Point::new(task_cx, layout.score_top)),
            PathSegment::LineTo(Point::new(task_cx, score_y)),
        ],
        style: Style {
            stroke: Some(theme.grid_stroke),
            stroke_width: Some(1.0),
            stroke_dasharray: Some(vec![4.0, 3.0]),
            ..Default::default()
        },
        marker_start: None,
        marker_end: None,
    });

    render_face(scene, Point::new(task_cx, score_y), task.score, color, theme);

    for (ai, actor) in task.actors.iter().enumerate() {
        if let Some(idx) = actors.iter().position(|a| a == actor) {
            let spacing = ACTOR_R * 2.0 + 4.0;
            let dot_x =
                task_cx - (task.actors.len() as f64 - 1.0) * spacing / 2.0 + ai as f64 * spacing;
            let dot_y = layout.actor_row_y + ACTOR_ROW_H / 2.0;
            scene.push(Primitive::Circle {
                center: Point::new(dot_x, dot_y),
                radius: ACTOR_R,
                style: Style {
                    fill: Some(actor_colors[idx]),
                    ..Default::default()
                },
            });
        }
    }
}

fn render_actor_legend(
    scene: &mut Scene,
    actors: &[String],
    actor_colors: &[Color],
    layout: &JourneyLayout,
    theme: &Theme,
) {
    let legend_y = layout.score_bot + FACE_R + 16.0;
    let mut lx = layout.ox + TASK_GAP;
    for (i, actor) in actors.iter().enumerate() {
        scene.push(Primitive::Circle {
            center: Point::new(lx + ACTOR_R, legend_y),
            radius: ACTOR_R,
            style: Style {
                fill: Some(actor_colors[i]),
                ..Default::default()
            },
        });
        scene.push(Primitive::Text {
            position: Point::new(lx + ACTOR_R * 2.0 + 4.0, legend_y),
            content: actor.clone(),
            anchor: TextAnchor::Start,
            style: TextStyle {
                font_size: theme.font_size_small,
                fill: Some(theme.node_text),
                ..Default::default()
            },
        });
        lx += SimpleTextMeasure::measure_raw(
            actor,
            &TextStyle {
                font_size: theme.font_size_small,
                ..Default::default()
            },
        )
        .width
            + ACTOR_R * 2.0
            + 16.0;
    }
}

fn render_face(scene: &mut Scene, center: Point, score: u8, color: Color, theme: &Theme) {
    let (cx, cy) = (center.x, center.y);
    let face_fill = theme.face_fill;

    // Face circle
    scene.push(Primitive::Circle {
        center: Point::new(cx, cy),
        radius: FACE_R,
        style: Style {
            fill: Some(face_fill),
            stroke: Some(color),
            stroke_width: Some(STROKE_WIDTH),
            ..Default::default()
        },
    });

    // Eyes
    for dx in [-3.5, 3.5] {
        scene.push(Primitive::Circle {
            center: Point::new(cx + dx, cy - 3.0),
            radius: 1.5,
            style: Style {
                fill: Some(theme.detail_stroke),
                ..Default::default()
            },
        });
    }

    // Mouth — arc based on score
    let mouth_y = cy + 3.0;
    if score > 3 {
        // Smile
        scene.push(Primitive::Path {
            segments: vec![
                PathSegment::MoveTo(Point::new(cx - 4.0, mouth_y)),
                PathSegment::CubicTo {
                    cp1: Point::new(cx - 2.0, mouth_y + 4.0),
                    cp2: Point::new(cx + 2.0, mouth_y + 4.0),
                    to: Point::new(cx + 4.0, mouth_y),
                },
            ],
            style: Style {
                stroke: Some(theme.detail_stroke),
                stroke_width: Some(STROKE_WIDTH),
                ..Default::default()
            },
            marker_start: None,
            marker_end: None,
        });
    } else if score < 3 {
        // Frown
        scene.push(Primitive::Path {
            segments: vec![
                PathSegment::MoveTo(Point::new(cx - 4.0, mouth_y + 3.0)),
                PathSegment::CubicTo {
                    cp1: Point::new(cx - 2.0, mouth_y - 1.0),
                    cp2: Point::new(cx + 2.0, mouth_y - 1.0),
                    to: Point::new(cx + 4.0, mouth_y + 3.0),
                },
            ],
            style: Style {
                stroke: Some(theme.detail_stroke),
                stroke_width: Some(STROKE_WIDTH),
                ..Default::default()
            },
            marker_start: None,
            marker_end: None,
        });
    } else {
        // Neutral line
        scene.push(Primitive::Path {
            segments: vec![
                PathSegment::MoveTo(Point::new(cx - 4.0, mouth_y + 1.0)),
                PathSegment::LineTo(Point::new(cx + 4.0, mouth_y + 1.0)),
            ],
            style: Style {
                stroke: Some(theme.detail_stroke),
                stroke_width: Some(STROKE_WIDTH),
                ..Default::default()
            },
            marker_start: None,
            marker_end: None,
        });
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
        let scene = render("journey\n  title Day\n  section Morning\n    Wake: 5: Me");
        assert!(!scene.is_empty());
    }

    #[test]
    fn has_task_rects() {
        let scene = render("journey\n  section S\n    A: 4\n    B: 2\n    C: 5");
        let rects = scene
            .elements()
            .iter()
            .filter(|e| {
                if let Primitive::Rect { style, .. } = &e.primitive {
                    style.stroke.is_some()
                } else {
                    false
                }
            })
            .count();
        assert_eq!(rects, 3, "3 task rects");
    }

    #[test]
    fn face_emojis_per_task() {
        let scene = render("journey\n  section S\n    Happy: 5\n    Sad: 1\n    Meh: 3");
        // Each face has 1 face circle + 2 eye circles = 3 circles per task
        let circles = scene
            .elements()
            .iter()
            .filter(|e| matches!(&e.primitive, Primitive::Circle { .. }))
            .count();
        assert!(circles >= 9, "at least 9 circles (3 faces × 3 parts)");
    }

    #[test]
    fn score_positions_ordered() {
        let scene = render("journey\n  section S\n    High: 5\n    Low: 1");
        // Face circles: the high-score face should be higher (smaller y)
        let face_circles: Vec<f64> = scene
            .elements()
            .iter()
            .filter_map(|e| {
                if let Primitive::Circle { center, radius, .. } = &e.primitive {
                    if (*radius - FACE_R).abs() < 0.1 {
                        Some(center.y)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(face_circles.len(), 2);
        assert!(
            face_circles[0] < face_circles[1],
            "score 5 should be above score 1"
        );
    }

    #[test]
    fn multiple_sections() {
        let scene = render("journey\n  section A\n    T1: 4\n  section B\n    T2: 2");
        let section_labels: Vec<&str> = scene
            .elements()
            .iter()
            .filter_map(|e| {
                if let Primitive::Text { content, style, .. } = &e.primitive {
                    if style.font_weight == rusty_mermaid_core::FontWeight::Bold
                        && (content == "A" || content == "B")
                    {
                        Some(content.as_str())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        assert!(section_labels.contains(&"A"));
        assert!(section_labels.contains(&"B"));
    }

    #[test]
    fn all_positions_finite() {
        let scene = render(
            "journey\n  section Go\n    Walk: 4: Me\n    Run: 2: Me, You\n  section Rest\n    Sit: 5: Me",
        );
        for elem in scene.elements() {
            match &elem.primitive {
                Primitive::Rect { bbox, .. } => {
                    assert!(bbox.x.is_finite() && bbox.y.is_finite());
                }
                Primitive::Text { position, .. } => {
                    assert!(position.x.is_finite() && position.y.is_finite());
                }
                Primitive::Circle { center, .. } => {
                    assert!(center.x.is_finite() && center.y.is_finite());
                }
                _ => {}
            }
        }
    }
}
