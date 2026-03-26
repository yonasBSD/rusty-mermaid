pub mod ir;
pub mod parser;

use rusty_mermaid_core::{
    BBox, Color, PathSegment, Point, Primitive, Scene, Style, TextAnchor,
    TextStyle, Theme, Transform,
};

use ir::{AxisDef, PlotType, XyChart};

const CHART_W: f64 = 500.0;
const CHART_H: f64 = 300.0;
const MARGIN_LEFT: f64 = 60.0;
const MARGIN_RIGHT: f64 = 30.0;
const MARGIN_TOP: f64 = 60.0;
const MARGIN_BOTTOM: f64 = 60.0;
const TICK_LEN: f64 = 5.0;
const BAR_PAD_RATIO: f64 = 0.15;

/// Same palette as timeline/gantt/kanban for visual coherence.
const PLOT_COLORS: [Color; 8] = [
    Color::rgb(78, 121, 167),
    Color::rgb(242, 142, 44),
    Color::rgb(225, 87, 89),
    Color::rgb(118, 183, 178),
    Color::rgb(89, 161, 79),
    Color::rgb(237, 201, 73),
    Color::rgb(175, 122, 161),
    Color::rgb(255, 157, 167),
];

pub fn to_scene(chart: &XyChart) -> Scene {
    to_scene_themed(chart, &Theme::default())
}

pub fn to_scene_themed(chart: &XyChart, theme: &Theme) -> Scene {
    let title_h = if chart.title.is_some() { theme.font_size_title + 20.0 } else { 0.0 };
    let width = MARGIN_LEFT + CHART_W + MARGIN_RIGHT;
    let height = title_h + MARGIN_TOP + CHART_H + MARGIN_BOTTOM;
    let plot_left = MARGIN_LEFT;
    let plot_right = MARGIN_LEFT + CHART_W;
    let plot_top = title_h + MARGIN_TOP;
    let plot_bottom = title_h + MARGIN_TOP + CHART_H;

    let mut scene = Scene::new(width, height);

    // Title
    if let Some(title) = &chart.title {
        scene.push(Primitive::Text {
            position: Point::new(width / 2.0, title_h / 2.0 + 5.0),
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

    // Resolve axis ranges from data
    let (y_min, y_max) = resolve_y_range(chart);
    let y_span = (y_max - y_min).max(1.0);
    let val_to_y = |v: f64| -> f64 {
        plot_bottom - (v - y_min) / y_span * CHART_H
    };

    let n_categories = match &chart.x_axis {
        AxisDef::Band { categories, .. } => categories.len(),
        AxisDef::Linear { .. } => chart.plots.first().map(|p| p.values.len()).unwrap_or(1),
    };
    let n_cats = n_categories.max(1);
    let cat_width = CHART_W / n_cats as f64;
    let cat_to_x = |i: usize| -> f64 {
        plot_left + i as f64 * cat_width + cat_width / 2.0
    };

    // Plot area background
    scene.push(Primitive::Rect {
        bbox: BBox::new(plot_left + CHART_W / 2.0, plot_top + CHART_H / 2.0, CHART_W, CHART_H),
        rx: 0.0, ry: 0.0,
        style: Style { fill: Some(Color::rgba(245, 245, 250, 200)), ..Default::default() },
    });

    // Y-axis
    scene.push(Primitive::Path {
        segments: vec![
            PathSegment::MoveTo(Point::new(plot_left, plot_top)),
            PathSegment::LineTo(Point::new(plot_left, plot_bottom)),
        ],
        style: Style { stroke: Some(theme.edge_stroke), stroke_width: Some(1.0), ..Default::default() },
        marker_start: None,
        marker_end: None,
    });

    // Y-axis ticks and labels
    let y_ticks = compute_nice_ticks(y_min, y_max, 5);
    for &val in &y_ticks {
        let y = val_to_y(val);
        // Tick
        scene.push(Primitive::Path {
            segments: vec![
                PathSegment::MoveTo(Point::new(plot_left - TICK_LEN, y)),
                PathSegment::LineTo(Point::new(plot_left, y)),
            ],
            style: Style { stroke: Some(theme.edge_stroke), stroke_width: Some(1.0), ..Default::default() },
            marker_start: None,
            marker_end: None,
        });
        // Grid line
        scene.push(Primitive::Path {
            segments: vec![
                PathSegment::MoveTo(Point::new(plot_left, y)),
                PathSegment::LineTo(Point::new(plot_right, y)),
            ],
            style: Style {
                stroke: Some(Color::rgba(200, 200, 200, 80)),
                stroke_width: Some(0.5),
                stroke_dasharray: Some(vec![4.0, 3.0]),
                ..Default::default()
            },
            marker_start: None,
            marker_end: None,
        });
        // Label
        let label = if val == val.floor() { format!("{:.0}", val) } else { format!("{:.1}", val) };
        scene.push(Primitive::Text {
            position: Point::new(plot_left - TICK_LEN - 4.0, y),
            content: label,
            anchor: TextAnchor::End,
            style: TextStyle {
                font_size: theme.font_size_small,
                fill: Some(theme.node_text),
                ..Default::default()
            },
        });
    }

    // Y-axis title — rotated 90° on the left, vertically centered
    if let AxisDef::Linear { title: Some(title), .. } = &chart.y_axis {
        let title_x = 15.0;
        let title_y = plot_top + CHART_H / 2.0;
        scene.push(Primitive::Group {
            transform: Transform::Rotate { degrees: -90.0, cx: title_x, cy: title_y },
            children: vec![Primitive::Text {
                position: Point::new(title_x, title_y),
                content: title.clone(),
                anchor: TextAnchor::Middle,
                style: TextStyle {
                    font_size: theme.font_size_label,
                    fill: Some(theme.node_text),
                    ..Default::default()
                },
            }],
        });
    }

    // X-axis
    scene.push(Primitive::Path {
        segments: vec![
            PathSegment::MoveTo(Point::new(plot_left, plot_bottom)),
            PathSegment::LineTo(Point::new(plot_right, plot_bottom)),
        ],
        style: Style { stroke: Some(theme.edge_stroke), stroke_width: Some(1.0), ..Default::default() },
        marker_start: None,
        marker_end: None,
    });

    // X-axis ticks and labels
    let categories = match &chart.x_axis {
        AxisDef::Band { categories, .. } => categories.clone(),
        AxisDef::Linear { .. } => (0..n_cats).map(|i| format!("{}", i + 1)).collect(),
    };
    for (i, cat) in categories.iter().enumerate() {
        let x = cat_to_x(i);
        // Tick
        scene.push(Primitive::Path {
            segments: vec![
                PathSegment::MoveTo(Point::new(x, plot_bottom)),
                PathSegment::LineTo(Point::new(x, plot_bottom + TICK_LEN)),
            ],
            style: Style { stroke: Some(theme.edge_stroke), stroke_width: Some(1.0), ..Default::default() },
            marker_start: None,
            marker_end: None,
        });
        // Label
        scene.push(Primitive::Text {
            position: Point::new(x, plot_bottom + TICK_LEN + 12.0),
            content: cat.clone(),
            anchor: TextAnchor::Middle,
            style: TextStyle {
                font_size: theme.font_size_small,
                fill: Some(theme.node_text),
                ..Default::default()
            },
        });
    }

    // X-axis title
    let x_title = match &chart.x_axis {
        AxisDef::Band { title, .. } | AxisDef::Linear { title, .. } => title.clone(),
    };
    if let Some(title) = x_title {
        scene.push(Primitive::Text {
            position: Point::new(plot_left + CHART_W / 2.0, plot_bottom + MARGIN_BOTTOM - 10.0),
            content: title,
            anchor: TextAnchor::Middle,
            style: TextStyle {
                font_size: theme.font_size_label,
                fill: Some(theme.node_text),
                ..Default::default()
            },
        });
    }

    // Count bar plots for grouping
    let bar_plots: Vec<usize> = chart.plots.iter().enumerate()
        .filter(|(_, p)| p.plot_type == PlotType::Bar)
        .map(|(i, _)| i)
        .collect();
    let n_bar_groups = bar_plots.len().max(1);

    // Render plots
    for (pi, plot) in chart.plots.iter().enumerate() {
        let color = PLOT_COLORS[pi % PLOT_COLORS.len()];

        match plot.plot_type {
            PlotType::Bar => {
                let bar_group_idx = bar_plots.iter().position(|&i| i == pi).unwrap_or(0);
                let group_width = cat_width * (1.0 - BAR_PAD_RATIO * 2.0);
                let bar_w = group_width / n_bar_groups as f64;
                let bar_offset = -group_width / 2.0 + bar_group_idx as f64 * bar_w;

                for (i, &val) in plot.values.iter().enumerate() {
                    if i >= n_cats { break; }
                    let x = cat_to_x(i) + bar_offset + bar_w / 2.0;
                    let y_top = val_to_y(val);
                    let y_bottom = val_to_y(y_min.max(0.0));
                    let h = (y_bottom - y_top).abs();
                    scene.push(Primitive::Rect {
                        bbox: BBox::new(x, y_top + h / 2.0, bar_w * 0.9, h),
                        rx: 1.0, ry: 1.0,
                        style: Style { fill: Some(color), ..Default::default() },
                    });
                }
            }
            PlotType::Line => {
                let mut segments = Vec::new();
                for (i, &val) in plot.values.iter().enumerate() {
                    if i >= n_cats { break; }
                    let x = cat_to_x(i);
                    let y = val_to_y(val);
                    if i == 0 {
                        segments.push(PathSegment::MoveTo(Point::new(x, y)));
                    } else {
                        segments.push(PathSegment::LineTo(Point::new(x, y)));
                    }
                }
                if !segments.is_empty() {
                    scene.push(Primitive::Path {
                        segments,
                        style: Style { stroke: Some(color), stroke_width: Some(2.0), ..Default::default() },
                        marker_start: None,
                        marker_end: None,
                    });
                }
                // Data points
                for (i, &val) in plot.values.iter().enumerate() {
                    if i >= n_cats { break; }
                    scene.push(Primitive::Circle {
                        center: Point::new(cat_to_x(i), val_to_y(val)),
                        radius: 3.5,
                        style: Style { fill: Some(color), stroke: Some(Color::WHITE), stroke_width: Some(1.5), ..Default::default() },
                    });
                }
            }
        }
    }

    scene
}

fn resolve_y_range(chart: &XyChart) -> (f64, f64) {
    let (explicit_min, explicit_max) = match &chart.y_axis {
        AxisDef::Linear { min, max, .. } => (*min, *max),
        _ => (None, None),
    };

    let data_min = chart.plots.iter()
        .flat_map(|p| &p.values)
        .copied()
        .fold(f64::INFINITY, f64::min);
    let data_max = chart.plots.iter()
        .flat_map(|p| &p.values)
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);

    let min = explicit_min.unwrap_or(data_min.min(0.0));
    let max = explicit_max.unwrap_or(data_max * 1.1);
    (min, max)
}

fn compute_nice_ticks(min: f64, max: f64, target_count: usize) -> Vec<f64> {
    let range = max - min;
    if range <= 0.0 { return vec![min]; }

    let rough_step = range / target_count as f64;
    let magnitude = 10.0f64.powf(rough_step.log10().floor());
    let nice_step = if rough_step / magnitude < 1.5 { magnitude }
        else if rough_step / magnitude < 3.5 { magnitude * 2.0 }
        else if rough_step / magnitude < 7.5 { magnitude * 5.0 }
        else { magnitude * 10.0 };

    let start = (min / nice_step).floor() * nice_step;
    let mut ticks = Vec::new();
    let mut v = start;
    while v <= max + nice_step * 0.01 {
        if v >= min - nice_step * 0.01 {
            ticks.push((v * 1e10).round() / 1e10); // avoid float noise
        }
        v += nice_step;
    }
    ticks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(input: &str) -> Scene {
        let c = parser::parse(input).unwrap();
        to_scene(&c)
    }

    #[test]
    fn scene_has_primitives() {
        let scene = render("xychart-beta\n    x-axis [A, B, C]\n    bar [10, 20, 30]");
        assert!(scene.len() >= 8, "axes + bars + ticks + labels");
    }

    #[test]
    fn scene_with_title() {
        let scene = render("xychart-beta\n    title \"Chart\"\n    bar [10, 20]");
        let has_title = scene.elements().iter().any(|e| {
            if let Primitive::Text { content, .. } = &e.primitive { content == "Chart" } else { false }
        });
        assert!(has_title);
    }

    #[test]
    fn bar_plot_renders_rects() {
        let scene = render("xychart-beta\n    x-axis [A, B, C]\n    bar [10, 20, 30]");
        let rects: Vec<_> = scene.elements().iter().filter(|e| matches!(&e.primitive, Primitive::Rect { .. })).collect();
        assert!(rects.len() >= 4, "plot bg + 3 bars");
    }

    #[test]
    fn line_plot_renders_path_and_dots() {
        let scene = render("xychart-beta\n    x-axis [A, B, C]\n    line [10, 20, 30]");
        let paths: Vec<_> = scene.elements().iter().filter(|e| matches!(&e.primitive, Primitive::Path { .. })).collect();
        let circles: Vec<_> = scene.elements().iter().filter(|e| matches!(&e.primitive, Primitive::Circle { .. })).collect();
        assert!(paths.len() >= 3, "axes + line path");
        assert_eq!(circles.len(), 3, "3 data point dots");
    }

    #[test]
    fn nice_ticks_produces_round_numbers() {
        let ticks = compute_nice_ticks(0.0, 100.0, 5);
        assert!(ticks.len() >= 3);
        for &t in &ticks {
            assert!((t % 10.0).abs() < 1e-6 || (t % 20.0).abs() < 1e-6, "tick {t} should be round");
        }
    }
}
