pub mod ir;
pub mod parser;

use rusty_mermaid_core::{
    BBox, Color, Point, Primitive, Scene, Style, TextAnchor, TextStyle, Theme, Transform,
};

use crate::common::palette::tint_color;
use ir::QuadrantChart;

const CHART_SIZE: f64 = 500.0;
const SCENE_PAD: f64 = 20.0;
const AXIS_LABEL_PAD: f64 = 28.0;
const POINT_RADIUS: f64 = 5.0;
const POINT_LABEL_GAP: f64 = 8.0;

/// Base colors from the shared palette — blended with white for translucent fills.
const QUAD_BASE: [Color; 4] = [
    Color::rgb(78, 121, 167), // Q1 blue
    Color::rgb(89, 161, 79),  // Q2 green
    Color::rgb(237, 201, 73), // Q3 yellow
    Color::rgb(242, 142, 44), // Q4 orange
];

const QUAD_TINT: f64 = 0.12;

fn quad_fill(idx: usize) -> Color {
    tint_color(QUAD_BASE[idx], QUAD_TINT)
}

pub fn to_scene(chart: &QuadrantChart) -> Scene {
    to_scene_themed(chart, &Theme::default())
}

/// Grid layout parameters computed from chart metadata.
struct GridLayout {
    grid_x: f64,
    grid_y: f64,
    half: f64,
    scene_w: f64,
    scene_h: f64,
    y_axis_w: f64,
}

impl GridLayout {
    fn from_chart(chart: &QuadrantChart) -> Self {
        let title_h = if chart.title.is_some() { 30.0 } else { 0.0 };
        let x_axis_h = if chart.x_axis.is_some() {
            AXIS_LABEL_PAD
        } else {
            0.0
        };
        let y_axis_w = if chart.y_axis.is_some() {
            AXIS_LABEL_PAD
        } else {
            0.0
        };
        Self {
            grid_x: SCENE_PAD + y_axis_w,
            grid_y: SCENE_PAD + title_h,
            half: CHART_SIZE / 2.0,
            scene_w: CHART_SIZE + SCENE_PAD * 2.0 + y_axis_w,
            scene_h: CHART_SIZE + SCENE_PAD * 2.0 + title_h + x_axis_h,
            y_axis_w,
        }
    }

    /// Q1=TR, Q2=TL, Q3=BL, Q4=BR center positions.
    fn quad_positions(&self) -> [(f64, f64); 4] {
        let (gx, gy, h) = (self.grid_x, self.grid_y, self.half);
        [
            (gx + h + h / 2.0, gy + h / 2.0),     // Q1
            (gx + h / 2.0, gy + h / 2.0),         // Q2
            (gx + h / 2.0, gy + h + h / 2.0),     // Q3
            (gx + h + h / 2.0, gy + h + h / 2.0), // Q4
        ]
    }
}

pub fn to_scene_themed(chart: &QuadrantChart, theme: &Theme) -> Scene {
    let layout = GridLayout::from_chart(chart);
    let mut scene = Scene::new(layout.scene_w, layout.scene_h);

    render_title(&mut scene, chart, &layout, theme);
    render_quadrants(&mut scene, chart, &layout);
    render_grid(&mut scene, &layout, theme);
    render_axes(&mut scene, chart, &layout, theme);
    render_points(&mut scene, chart, &layout, theme);

    scene
}

fn render_title(scene: &mut Scene, chart: &QuadrantChart, layout: &GridLayout, theme: &Theme) {
    if let Some(title) = &chart.title {
        scene.push(Primitive::Text {
            position: Point::new(layout.grid_x + layout.half, SCENE_PAD + 10.0),
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

fn render_quadrants(scene: &mut Scene, chart: &QuadrantChart, layout: &GridLayout) {
    let positions = layout.quad_positions();
    let half = layout.half;

    for (i, &(cx, cy)) in positions.iter().enumerate() {
        scene.push(Primitive::Rect {
            bbox: BBox::new(cx, cy, half, half),
            rx: 0.0,
            ry: 0.0,
            style: Style {
                fill: Some(quad_fill(i)),
                ..Default::default()
            },
        });
    }

    for (i, &(cx, cy)) in positions.iter().enumerate() {
        if let Some(label) = &chart.quadrants[i] {
            let c = QUAD_BASE[i];
            scene.push(Primitive::Text {
                position: Point::new(cx, cy),
                content: label.clone(),
                anchor: TextAnchor::Middle,
                style: TextStyle {
                    font_size: 16.0,
                    fill: Some(Color::rgb(
                        (c.r as f64 * 0.7) as u8,
                        (c.g as f64 * 0.7) as u8,
                        (c.b as f64 * 0.7) as u8,
                    )),
                    ..Default::default()
                },
            });
        }
    }
}

fn render_grid(scene: &mut Scene, layout: &GridLayout, theme: &Theme) {
    use rusty_mermaid_core::PathSegment;

    let (gx, gy, half) = (layout.grid_x, layout.grid_y, layout.half);
    let divider_color = theme.grid_stroke;
    let border_style = Style {
        stroke: Some(divider_color),
        stroke_width: Some(1.0),
        ..Default::default()
    };

    scene.push(Primitive::Rect {
        bbox: BBox::new(gx + half, gy + half, CHART_SIZE, CHART_SIZE),
        rx: 0.0,
        ry: 0.0,
        style: Style {
            fill: Some(Color::rgba(0, 0, 0, 0)),
            stroke: Some(theme.grid_stroke),
            stroke_width: Some(1.5),
            ..Default::default()
        },
    });

    scene.push(Primitive::Path {
        segments: vec![
            PathSegment::MoveTo(Point::new(gx + half, gy)),
            PathSegment::LineTo(Point::new(gx + half, gy + CHART_SIZE)),
        ],
        style: border_style.clone(),
        marker_start: None,
        marker_end: None,
    });
    scene.push(Primitive::Path {
        segments: vec![
            PathSegment::MoveTo(Point::new(gx, gy + half)),
            PathSegment::LineTo(Point::new(gx + CHART_SIZE, gy + half)),
        ],
        style: border_style,
        marker_start: None,
        marker_end: None,
    });
}

fn render_axes(scene: &mut Scene, chart: &QuadrantChart, layout: &GridLayout, theme: &Theme) {
    let (gx, gy, half) = (layout.grid_x, layout.grid_y, layout.half);
    let axis_style = TextStyle {
        font_size: theme.font_size_node,
        fill: Some(theme.node_text),
        ..Default::default()
    };

    if let Some((left, right)) = &chart.x_axis {
        let y = gy + CHART_SIZE + AXIS_LABEL_PAD / 2.0;
        scene.push(Primitive::Text {
            position: Point::new(gx + half / 2.0, y),
            content: left.clone(),
            anchor: TextAnchor::Middle,
            style: axis_style.clone(),
        });
        if let Some(right_label) = right {
            scene.push(Primitive::Text {
                position: Point::new(gx + half + half / 2.0, y),
                content: right_label.clone(),
                anchor: TextAnchor::Middle,
                style: axis_style.clone(),
            });
        }
    }

    if let Some((bottom, top)) = &chart.y_axis {
        let x = SCENE_PAD + layout.y_axis_w / 2.0;
        let y_style = TextStyle {
            font_size: theme.font_size_node,
            fill: Some(theme.node_text),
            ..Default::default()
        };

        let y_bot = gy + half + half / 2.0;
        scene.push(Primitive::Group {
            transform: Transform::Rotate {
                degrees: -90.0,
                cx: x,
                cy: y_bot,
            },
            children: vec![Primitive::Text {
                position: Point::new(x, y_bot),
                content: bottom.clone(),
                anchor: TextAnchor::Middle,
                style: y_style.clone(),
            }],
        });

        if let Some(top_label) = top {
            let y_top = gy + half / 2.0;
            scene.push(Primitive::Group {
                transform: Transform::Rotate {
                    degrees: -90.0,
                    cx: x,
                    cy: y_top,
                },
                children: vec![Primitive::Text {
                    position: Point::new(x, y_top),
                    content: top_label.clone(),
                    anchor: TextAnchor::Middle,
                    style: y_style,
                }],
            });
        }
    }
}

fn render_points(scene: &mut Scene, chart: &QuadrantChart, layout: &GridLayout, theme: &Theme) {
    let point_color = crate::common::palette::palette_color(0);
    for pt in &chart.points {
        let px = layout.grid_x + pt.x * CHART_SIZE;
        let py = layout.grid_y + (1.0 - pt.y) * CHART_SIZE;

        scene.push(Primitive::Circle {
            center: Point::new(px, py),
            radius: POINT_RADIUS,
            style: Style {
                fill: Some(point_color),
                ..Default::default()
            },
        });

        scene.push(Primitive::Text {
            position: Point::new(px, py + POINT_RADIUS + POINT_LABEL_GAP),
            content: pt.label.clone(),
            anchor: TextAnchor::Middle,
            style: TextStyle {
                font_size: 11.0,
                fill: Some(theme.node_text),
                ..Default::default()
            },
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(input: &str) -> Scene {
        let c = parser::parse(input).unwrap();
        to_scene(&c)
    }

    #[test]
    fn basic_renders() {
        let scene = render("quadrantChart\n  A: [0.5, 0.5]");
        assert!(!scene.is_empty());
    }

    #[test]
    fn has_four_quadrant_rects() {
        let scene = render("quadrantChart\n  A: [0.5, 0.5]");
        let rects = scene
            .elements()
            .iter()
            .filter(|e| {
                if let Primitive::Rect { style, .. } = &e.primitive {
                    style.fill.is_some() && style.stroke.is_none()
                } else {
                    false
                }
            })
            .count();
        assert_eq!(rects, 4, "should have 4 quadrant background rects");
    }

    #[test]
    fn points_are_circles() {
        let scene = render("quadrantChart\n  A: [0.2, 0.8]\n  B: [0.7, 0.3]");
        let circles = scene
            .elements()
            .iter()
            .filter(|e| matches!(&e.primitive, Primitive::Circle { .. }))
            .count();
        assert_eq!(circles, 2);
    }

    #[test]
    fn quadrant_labels_render() {
        let scene =
            render("quadrantChart\n  quadrant-1 Leaders\n  quadrant-3 Niche\n  A: [0.5, 0.5]");
        let has_leaders = scene.elements().iter().any(|e| {
            if let Primitive::Text { content, .. } = &e.primitive {
                content == "Leaders"
            } else {
                false
            }
        });
        assert!(has_leaders);
    }

    #[test]
    fn axis_labels_render() {
        let scene = render("quadrantChart\n  x-axis Low --> High\n  A: [0.5, 0.5]");
        let has_low = scene.elements().iter().any(|e| {
            if let Primitive::Text { content, .. } = &e.primitive {
                content == "Low"
            } else {
                false
            }
        });
        let has_high = scene.elements().iter().any(|e| {
            if let Primitive::Text { content, .. } = &e.primitive {
                content == "High"
            } else {
                false
            }
        });
        assert!(has_low && has_high);
    }

    #[test]
    fn point_y_is_inverted() {
        // y=0 should be at bottom, y=1 at top
        let scene = render("quadrantChart\n  Bottom: [0.5, 0.0]\n  Top: [0.5, 1.0]");
        let circles: Vec<_> = scene
            .elements()
            .iter()
            .filter_map(|e| {
                if let Primitive::Circle { center, .. } = &e.primitive {
                    Some(center.y)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(circles.len(), 2);
        // Bottom point (y=0) should have larger pixel y than top point (y=1)
        assert!(circles[0] > circles[1], "y=0 should be below y=1");
    }
}
