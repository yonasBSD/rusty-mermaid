pub mod ir;
pub mod parser;

use std::f64::consts::TAU;

use rusty_mermaid_core::{
    Color, PathSegment, Point, Primitive, Scene, Style, TextAnchor, TextStyle, Theme,
};

use ir::PieChart;

const RADIUS: f64 = 140.0;
const CENTER_X: f64 = 180.0;
const CENTER_Y: f64 = 200.0;
const LABEL_RADIUS_RATIO: f64 = 0.65;
const LEGEND_X_OFFSET: f64 = 40.0;
const LEGEND_SWATCH_SIZE: f64 = 12.0;
const LEGEND_LINE_HEIGHT: f64 = 20.0;
const TITLE_Y: f64 = 30.0;

/// 12-color palette matching mermaid.js pie sections.
const PIE_COLORS: [Color; 12] = [
    Color::rgb(78, 121, 167),
    Color::rgb(242, 142, 44),
    Color::rgb(225, 87, 89),
    Color::rgb(118, 183, 178),
    Color::rgb(89, 161, 79),
    Color::rgb(237, 201, 73),
    Color::rgb(175, 122, 161),
    Color::rgb(255, 157, 167),
    Color::rgb(156, 117, 95),
    Color::rgb(186, 176, 172),
    Color::rgb(0, 128, 128),
    Color::rgb(255, 127, 80),
];

pub fn to_scene(chart: &PieChart) -> Scene {
    to_scene_themed(chart, &Theme::default())
}

pub fn to_scene_themed(chart: &PieChart, theme: &Theme) -> Scene {
    let total = chart.total();
    if total <= 0.0 {
        return Scene::new(100.0, 100.0);
    }

    let legend_width = 200.0;
    let width = CENTER_X + RADIUS + LEGEND_X_OFFSET + legend_width + 20.0;
    let height = CENTER_Y + RADIUS + 40.0;
    let mut scene = Scene::new(width, height);

    render_pie_title(&mut scene, chart, theme);
    render_slices(&mut scene, chart, total, theme);
    render_legend(&mut scene, chart, theme);

    scene
}

fn render_pie_title(scene: &mut Scene, chart: &PieChart, theme: &Theme) {
    if let Some(title) = &chart.title {
        scene.push(Primitive::Text {
            position: Point::new(CENTER_X, TITLE_Y),
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

fn render_slices(scene: &mut Scene, chart: &PieChart, total: f64, theme: &Theme) {
    let mut start_angle: f64 = -TAU / 4.0; // start at top (12 o'clock)
    for (i, slice) in chart.slices.iter().enumerate() {
        let fraction = slice.value / total;
        if fraction < 0.001 {
            continue;
        } // skip negligible slices

        let sweep = fraction * TAU;
        let end_angle = start_angle + sweep;
        let color = PIE_COLORS[i % PIE_COLORS.len()];

        render_arc_slice(
            scene,
            CENTER_X,
            CENTER_Y,
            RADIUS,
            start_angle,
            end_angle,
            color,
            theme,
        );

        // Percentage label on slice
        let mid_angle = start_angle + sweep / 2.0;
        let label_r = RADIUS * LABEL_RADIUS_RATIO;
        let label_x = CENTER_X + label_r * mid_angle.cos();
        let label_y = CENTER_Y + label_r * mid_angle.sin();
        let pct = format!("{:.0}%", fraction * 100.0);
        scene.push(Primitive::Text {
            position: Point::new(label_x, label_y),
            content: pct,
            anchor: TextAnchor::Middle,
            style: TextStyle {
                font_size: theme.font_size_edge_label,
                fill: Some(Color::WHITE),
                font_weight: rusty_mermaid_core::FontWeight::Bold,
                ..Default::default()
            },
        });

        start_angle = end_angle;
    }
}

fn render_legend(scene: &mut Scene, chart: &PieChart, theme: &Theme) {
    let legend_x = CENTER_X + RADIUS + LEGEND_X_OFFSET;
    let mut legend_y = CENTER_Y - (chart.slices.len() as f64 * LEGEND_LINE_HEIGHT) / 2.0;
    for (i, slice) in chart.slices.iter().enumerate() {
        let color = PIE_COLORS[i % PIE_COLORS.len()];

        scene.push(Primitive::Rect {
            bbox: rusty_mermaid_core::BBox::new(
                legend_x + LEGEND_SWATCH_SIZE / 2.0,
                legend_y + LEGEND_SWATCH_SIZE / 2.0,
                LEGEND_SWATCH_SIZE,
                LEGEND_SWATCH_SIZE,
            ),
            rx: 2.0,
            ry: 2.0,
            style: Style {
                fill: Some(color),
                ..Default::default()
            },
        });

        let mut label = slice.label.clone();
        if chart.show_data {
            label.push_str(&format!(" [{}]", slice.value));
        }
        scene.push(Primitive::Text {
            position: Point::new(
                legend_x + LEGEND_SWATCH_SIZE + 8.0,
                legend_y + LEGEND_SWATCH_SIZE / 2.0,
            ),
            content: label,
            anchor: TextAnchor::Start,
            style: TextStyle {
                font_size: theme.font_size_edge_label,
                fill: Some(theme.node_text),
                ..Default::default()
            },
        });

        legend_y += LEGEND_LINE_HEIGHT;
    }
}

fn render_arc_slice(
    scene: &mut Scene,
    cx: f64,
    cy: f64,
    r: f64,
    start: f64,
    end: f64,
    fill: Color,
    theme: &Theme,
) {
    let sweep = end - start;
    let large_arc = sweep.abs() > std::f64::consts::PI;

    let x1 = cx + r * start.cos();
    let y1 = cy + r * start.sin();
    let x2 = cx + r * end.cos();
    let y2 = cy + r * end.sin();

    scene.push(Primitive::Path {
        segments: vec![
            PathSegment::MoveTo(Point::new(cx, cy)),
            PathSegment::LineTo(Point::new(x1, y1)),
            PathSegment::ArcTo {
                rx: r,
                ry: r,
                rotation: 0.0,
                large_arc,
                sweep: true,
                to: Point::new(x2, y2),
            },
            PathSegment::Close,
        ],
        style: Style {
            fill: Some(fill),
            stroke: Some(theme.background),
            stroke_width: Some(2.0),
            ..Default::default()
        },
        marker_start: None,
        marker_end: None,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(input: &str) -> Scene {
        let chart = parser::parse(input).unwrap();
        to_scene(&chart)
    }

    #[test]
    fn scene_has_slices() {
        let scene = render("pie\n    \"A\" : 50\n    \"B\" : 50");
        assert!(scene.len() >= 4, "2 slices + 2 labels + legend");
    }

    #[test]
    fn scene_with_title() {
        let scene = render("pie title My Chart\n    \"A\" : 100");
        let has_title = scene.elements().iter().any(|e| {
            if let Primitive::Text { content, .. } = &e.primitive {
                content == "My Chart"
            } else {
                false
            }
        });
        assert!(has_title);
    }

    #[test]
    fn three_slices_three_labels() {
        let scene = render("pie\n    \"X\" : 30\n    \"Y\" : 40\n    \"Z\" : 30");
        let pct_labels: Vec<_> = scene
            .elements()
            .iter()
            .filter(|e| {
                if let Primitive::Text { content, .. } = &e.primitive {
                    content.ends_with('%')
                } else {
                    false
                }
            })
            .collect();
        assert_eq!(pct_labels.len(), 3);
    }

    #[test]
    fn legend_has_swatches() {
        let scene = render("pie\n    \"A\" : 50\n    \"B\" : 50");
        let rects: Vec<_> = scene
            .elements()
            .iter()
            .filter(|e| matches!(&e.primitive, Primitive::Rect { .. }))
            .collect();
        assert!(rects.len() >= 2, "at least 2 legend swatches");
    }

    #[test]
    fn show_data_in_legend() {
        let scene = render("pie showData\n    \"Dogs\" : 40\n    \"Cats\" : 60");
        let has_value = scene.elements().iter().any(|e| {
            if let Primitive::Text { content, .. } = &e.primitive {
                content.contains("[40]")
            } else {
                false
            }
        });
        assert!(has_value, "showData should show values in legend");
    }
}
