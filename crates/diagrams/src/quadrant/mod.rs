pub mod ir;
pub mod parser;

use rusty_mermaid_core::{
    BBox, Color, Point, Primitive, Scene, Style, TextAnchor, TextStyle, Theme, Transform,
};

use ir::QuadrantChart;

const CHART_SIZE: f64 = 500.0;
const SCENE_PAD: f64 = 20.0;
const AXIS_LABEL_PAD: f64 = 28.0;
const POINT_RADIUS: f64 = 5.0;
const POINT_LABEL_GAP: f64 = 8.0;

/// Base colors from the shared palette — blended with white for translucent fills.
const QUAD_BASE: [Color; 4] = [
    Color::rgb(78, 121, 167),  // Q1 blue
    Color::rgb(89, 161, 79),   // Q2 green
    Color::rgb(237, 201, 73),  // Q3 yellow
    Color::rgb(242, 142, 44),  // Q4 orange
];

const QUAD_TINT: f64 = 0.12;

fn quad_fill(idx: usize) -> Color {
    let c = QUAD_BASE[idx];
    let t = QUAD_TINT;
    Color::rgb(
        (255.0 * (1.0 - t) + c.r as f64 * t) as u8,
        (255.0 * (1.0 - t) + c.g as f64 * t) as u8,
        (255.0 * (1.0 - t) + c.b as f64 * t) as u8,
    )
}

pub fn to_scene(chart: &QuadrantChart) -> Scene {
    to_scene_themed(chart, &Theme::default())
}

pub fn to_scene_themed(chart: &QuadrantChart, theme: &Theme) -> Scene {
    let title_h = if chart.title.is_some() { 30.0 } else { 0.0 };
    let x_axis_h = if chart.x_axis.is_some() { AXIS_LABEL_PAD } else { 0.0 };
    let y_axis_w = if chart.y_axis.is_some() { AXIS_LABEL_PAD } else { 0.0 };

    let grid_x = SCENE_PAD + y_axis_w;
    let grid_y = SCENE_PAD + title_h;
    let half = CHART_SIZE / 2.0;

    let scene_w = CHART_SIZE + SCENE_PAD * 2.0 + y_axis_w;
    let scene_h = CHART_SIZE + SCENE_PAD * 2.0 + title_h + x_axis_h;
    let mut scene = Scene::new(scene_w, scene_h);

    // Title
    if let Some(title) = &chart.title {
        scene.push(Primitive::Text {
            position: Point::new(grid_x + half, SCENE_PAD + 10.0),
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

    // Quadrant backgrounds (BBox is center-based)
    // Q2=TL, Q1=TR, Q3=BL, Q4=BR
    let quad_positions = [
        (grid_x + half + half / 2.0, grid_y + half / 2.0),         // Q1 top-right
        (grid_x + half / 2.0, grid_y + half / 2.0),                 // Q2 top-left
        (grid_x + half / 2.0, grid_y + half + half / 2.0),          // Q3 bottom-left
        (grid_x + half + half / 2.0, grid_y + half + half / 2.0),   // Q4 bottom-right
    ];

    for (i, &(cx, cy)) in quad_positions.iter().enumerate() {
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

    // Quadrant labels (use base color, slightly muted)
    for (i, &(cx, cy)) in quad_positions.iter().enumerate() {
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

    // Grid borders
    let divider_color = Color::rgb(200, 200, 200);
    let border_style = Style {
        stroke: Some(divider_color),
        stroke_width: Some(1.0),
        ..Default::default()
    };

    // Outer border (transparent fill — quadrant backgrounds must show through)
    scene.push(Primitive::Rect {
        bbox: BBox::new(grid_x + half, grid_y + half, CHART_SIZE, CHART_SIZE),
        rx: 0.0,
        ry: 0.0,
        style: Style {
            fill: Some(Color::rgba(0, 0, 0, 0)),
            stroke: Some(Color::rgb(180, 180, 180)),
            stroke_width: Some(1.5),
            ..Default::default()
        },
    });

    // Center dividers
    use rusty_mermaid_core::PathSegment;
    scene.push(Primitive::Path {
        segments: vec![
            PathSegment::MoveTo(Point::new(grid_x + half, grid_y)),
            PathSegment::LineTo(Point::new(grid_x + half, grid_y + CHART_SIZE)),
        ],
        style: border_style.clone(),
        marker_start: None,
        marker_end: None,
    });
    scene.push(Primitive::Path {
        segments: vec![
            PathSegment::MoveTo(Point::new(grid_x, grid_y + half)),
            PathSegment::LineTo(Point::new(grid_x + CHART_SIZE, grid_y + half)),
        ],
        style: border_style,
        marker_start: None,
        marker_end: None,
    });

    // X-axis labels
    if let Some((left, right)) = &chart.x_axis {
        let y = grid_y + CHART_SIZE + AXIS_LABEL_PAD / 2.0;
        scene.push(Primitive::Text {
            position: Point::new(grid_x + half / 2.0, y),
            content: left.clone(),
            anchor: TextAnchor::Middle,
            style: TextStyle {
                font_size: theme.font_size_node,
                fill: Some(theme.node_text),
                ..Default::default()
            },
        });
        if let Some(right_label) = right {
            scene.push(Primitive::Text {
                position: Point::new(grid_x + half + half / 2.0, y),
                content: right_label.clone(),
                anchor: TextAnchor::Middle,
                style: TextStyle {
                    font_size: theme.font_size_node,
                    fill: Some(theme.node_text),
                    ..Default::default()
                },
            });
        }
    }

    // Y-axis labels (rotated -90°)
    if let Some((bottom, top)) = &chart.y_axis {
        let x = SCENE_PAD + y_axis_w / 2.0;
        let y_style = TextStyle {
            font_size: theme.font_size_node,
            fill: Some(theme.node_text),
            ..Default::default()
        };

        let y_bot = grid_y + half + half / 2.0;
        scene.push(Primitive::Group {
            transform: Transform::Rotate { degrees: -90.0, cx: x, cy: y_bot },
            children: vec![Primitive::Text {
                position: Point::new(x, y_bot),
                content: bottom.clone(),
                anchor: TextAnchor::Middle,
                style: y_style.clone(),
            }],
        });

        if let Some(top_label) = top {
            let y_top = grid_y + half / 2.0;
            scene.push(Primitive::Group {
                transform: Transform::Rotate { degrees: -90.0, cx: x, cy: y_top },
                children: vec![Primitive::Text {
                    position: Point::new(x, y_top),
                    content: top_label.clone(),
                    anchor: TextAnchor::Middle,
                    style: y_style,
                }],
            });
        }
    }

    // Points
    let point_color = Color::rgb(78, 121, 167);
    for pt in &chart.points {
        // Map [0,1] → pixel coords (y inverted: 0=bottom, 1=top)
        let px = grid_x + pt.x * CHART_SIZE;
        let py = grid_y + (1.0 - pt.y) * CHART_SIZE;

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

    scene
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
        let rects = scene.elements().iter().filter(|e| {
            if let Primitive::Rect { style, .. } = &e.primitive {
                style.fill.is_some() && style.stroke.is_none()
            } else { false }
        }).count();
        assert_eq!(rects, 4, "should have 4 quadrant background rects");
    }

    #[test]
    fn points_are_circles() {
        let scene = render("quadrantChart\n  A: [0.2, 0.8]\n  B: [0.7, 0.3]");
        let circles = scene.elements().iter().filter(|e| {
            matches!(&e.primitive, Primitive::Circle { .. })
        }).count();
        assert_eq!(circles, 2);
    }

    #[test]
    fn quadrant_labels_render() {
        let scene = render("quadrantChart\n  quadrant-1 Leaders\n  quadrant-3 Niche\n  A: [0.5, 0.5]");
        let has_leaders = scene.elements().iter().any(|e| {
            if let Primitive::Text { content, .. } = &e.primitive { content == "Leaders" } else { false }
        });
        assert!(has_leaders);
    }

    #[test]
    fn axis_labels_render() {
        let scene = render("quadrantChart\n  x-axis Low --> High\n  A: [0.5, 0.5]");
        let has_low = scene.elements().iter().any(|e| {
            if let Primitive::Text { content, .. } = &e.primitive { content == "Low" } else { false }
        });
        let has_high = scene.elements().iter().any(|e| {
            if let Primitive::Text { content, .. } = &e.primitive { content == "High" } else { false }
        });
        assert!(has_low && has_high);
    }

    #[test]
    fn point_y_is_inverted() {
        // y=0 should be at bottom, y=1 at top
        let scene = render("quadrantChart\n  Bottom: [0.5, 0.0]\n  Top: [0.5, 1.0]");
        let circles: Vec<_> = scene.elements().iter().filter_map(|e| {
            if let Primitive::Circle { center, .. } = &e.primitive { Some(center.y) } else { None }
        }).collect();
        assert_eq!(circles.len(), 2);
        // Bottom point (y=0) should have larger pixel y than top point (y=1)
        assert!(circles[0] > circles[1], "y=0 should be below y=1");
    }
}
