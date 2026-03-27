pub mod ir;
pub mod parser;

use rusty_mermaid_core::{
    BBox, Color, PathSegment, Point, Primitive, Scene, Style, TextAnchor, TextStyle, Theme,
};

use ir::{GanttChart, TaskEnd, TaskStart, TaskTag};

const BAR_HEIGHT: f64 = 24.0;
const BAR_GAP: f64 = 6.0;
const SECTION_GAP: f64 = 10.0;
const LABEL_WIDTH: f64 = 160.0;
const CHART_WIDTH: f64 = 600.0;
const MARGIN: f64 = 20.0;
const AXIS_HEIGHT: f64 = 30.0;
const SECTION_HEADER_H: f64 = 20.0;

const SECTION_COLORS: [Color; 4] = [
    Color::rgb(78, 121, 167),
    Color::rgb(242, 142, 44),
    Color::rgb(89, 161, 79),
    Color::rgb(225, 87, 89),
];

/// Precomputed chart geometry shared across drawing helpers.
struct ChartArea {
    chart_left: f64,
    chart_right: f64,
    full_left: f64,
    full_right: f64,
    min_day: i32,
    total_days: f64,
    axis_y: f64,
    width: f64,
    height: f64,
}

impl ChartArea {
    fn day_to_x(&self, day: i32) -> f64 {
        self.chart_left + (day - self.min_day) as f64 / self.total_days * CHART_WIDTH
    }

    fn axis_line_y(&self) -> f64 {
        self.axis_y + AXIS_HEIGHT
    }
}

struct BarPos {
    name: String,
    x1: f64,
    x2: f64,
    y: f64,
    tags: Vec<TaskTag>,
    section_idx: usize,
}

struct SectionRange {
    start: f64,
    end: f64,
    name: Option<String>,
    idx: usize,
}

pub fn to_scene(chart: &GanttChart, theme: &Theme) -> Scene {
    let resolved = resolve_tasks(chart);
    if resolved.is_empty() {
        return Scene::empty();
    }

    let min_day = resolved.iter().map(|t| t.start_day).min().unwrap_or(0);
    let max_day = resolved.iter().map(|t| t.end_day).max().unwrap_or(1);
    let total_days = (max_day - min_day).max(1) as f64;

    let chart_left = MARGIN + LABEL_WIDTH;
    let chart_right = MARGIN + LABEL_WIDTH + CHART_WIDTH;
    let full_right = chart_right + MARGIN;

    let mut y = MARGIN;
    if chart.title.is_some() {
        y += theme.font_size_title + MARGIN / 2.0;
    }
    let axis_y = y;
    y += AXIS_HEIGHT;

    let (bars, section_ranges) =
        compute_bar_layout(chart, &resolved, min_day, total_days, chart_left, &mut y);

    let area = ChartArea {
        chart_left,
        chart_right,
        full_left: MARGIN,
        full_right,
        min_day,
        total_days,
        axis_y,
        width: full_right,
        height: y + MARGIN,
    };

    let mut scene = Scene::new(area.width, area.height);

    if let Some(title) = &chart.title {
        scene.push(Primitive::Text {
            position: Point::new(area.width / 2.0, MARGIN + theme.font_size_title * 0.4),
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

    render_sections(&mut scene, &section_ranges, &area, theme);
    render_axis(&mut scene, &area, theme, min_day, max_day);
    render_bars(&mut scene, &bars, &area, theme);

    scene
}

fn compute_bar_layout(
    chart: &GanttChart,
    resolved: &[ResolvedTask],
    min_day: i32,
    total_days: f64,
    chart_left: f64,
    y: &mut f64,
) -> (Vec<BarPos>, Vec<SectionRange>) {
    let day_to_x =
        |day: i32| -> f64 { chart_left + (day - min_day) as f64 / total_days * CHART_WIDTH };

    let mut bars: Vec<BarPos> = Vec::new();
    let mut section_ranges: Vec<SectionRange> = Vec::new();

    for (si, section) in chart.sections.iter().enumerate() {
        let section_start = *y;
        if section.name.is_some() {
            *y += SECTION_HEADER_H;
        }
        for task in &section.tasks {
            if let Some(rt) = resolved.iter().find(|r| r.name == task.name) {
                bars.push(BarPos {
                    name: task.name.clone(),
                    x1: day_to_x(rt.start_day),
                    x2: day_to_x(rt.end_day),
                    y: *y + BAR_HEIGHT / 2.0,
                    tags: task.tags.clone(),
                    section_idx: si,
                });
                *y += BAR_HEIGHT + BAR_GAP;
            }
        }
        let section_end = *y + SECTION_GAP / 2.0;
        section_ranges.push(SectionRange {
            start: section_start,
            end: section_end,
            name: section.name.clone(),
            idx: si,
        });
        *y += SECTION_GAP;
    }

    (bars, section_ranges)
}

fn render_sections(scene: &mut Scene, ranges: &[SectionRange], area: &ChartArea, theme: &Theme) {
    for sr in ranges {
        let color = SECTION_COLORS[sr.idx % SECTION_COLORS.len()];
        let sec_h = sr.end - sr.start;
        scene.push(Primitive::Rect {
            bbox: BBox::new(
                (area.full_left + area.full_right) / 2.0,
                (sr.start + sr.end) / 2.0,
                area.full_right - area.full_left,
                sec_h,
            ),
            rx: 0.0,
            ry: 0.0,
            style: Style {
                fill: Some(Color::rgba(color.r, color.g, color.b, 18)),
                ..Default::default()
            },
        });
        if let Some(name) = &sr.name {
            scene.push(Primitive::Text {
                position: Point::new(MARGIN + 4.0, sr.start + SECTION_HEADER_H * 0.55),
                content: name.clone(),
                anchor: TextAnchor::Start,
                style: TextStyle {
                    font_size: theme.font_size_label,
                    fill: Some(color),
                    font_weight: rusty_mermaid_core::FontWeight::Bold,
                    ..Default::default()
                },
            });
        }
    }
}

fn render_axis(scene: &mut Scene, area: &ChartArea, theme: &Theme, min_day: i32, max_day: i32) {
    render_axis_line(scene, area, theme);
    render_axis_ticks(scene, area, theme, min_day, max_day);
    render_axis_grid(scene, area, theme, min_day, max_day);
}

fn render_axis_line(scene: &mut Scene, area: &ChartArea, theme: &Theme) {
    let axis_line_y = area.axis_line_y();

    // Top axis line
    scene.push(Primitive::Path {
        segments: vec![
            PathSegment::MoveTo(Point::new(area.chart_left, axis_line_y)),
            PathSegment::LineTo(Point::new(area.chart_right, axis_line_y)),
        ],
        style: Style {
            stroke: Some(theme.edge_stroke),
            stroke_width: Some(1.0),
            ..Default::default()
        },
        marker_start: None,
        marker_end: None,
    });

    let border_style = Style {
        stroke: Some(Color::rgba(
            theme.grid_stroke.r,
            theme.grid_stroke.g,
            theme.grid_stroke.b,
            100,
        )),
        stroke_width: Some(0.5),
        ..Default::default()
    };

    // Right edge border
    scene.push(Primitive::Path {
        segments: vec![
            PathSegment::MoveTo(Point::new(area.chart_right, axis_line_y)),
            PathSegment::LineTo(Point::new(area.chart_right, area.height - MARGIN)),
        ],
        style: border_style.clone(),
        marker_start: None,
        marker_end: None,
    });

    // Left edge border
    scene.push(Primitive::Path {
        segments: vec![
            PathSegment::MoveTo(Point::new(area.chart_left, axis_line_y)),
            PathSegment::LineTo(Point::new(area.chart_left, area.height - MARGIN)),
        ],
        style: border_style,
        marker_start: None,
        marker_end: None,
    });
}

fn render_axis_ticks(
    scene: &mut Scene,
    area: &ChartArea,
    theme: &Theme,
    min_day: i32,
    max_day: i32,
) {
    let axis_line_y = area.axis_line_y();
    let tick_interval = compute_tick_interval(area.total_days as i32);
    let mut day = min_day;
    while day <= max_day {
        let x = area.day_to_x(day);

        scene.push(Primitive::Path {
            segments: vec![
                PathSegment::MoveTo(Point::new(x, axis_line_y - 4.0)),
                PathSegment::LineTo(Point::new(x, axis_line_y + 4.0)),
            ],
            style: Style {
                stroke: Some(theme.edge_stroke),
                stroke_width: Some(1.0),
                ..Default::default()
            },
            marker_start: None,
            marker_end: None,
        });

        let label = format_day_label(day);
        scene.push(Primitive::Text {
            position: Point::new(x, area.axis_y + AXIS_HEIGHT * 0.4),
            content: label,
            anchor: TextAnchor::Middle,
            style: TextStyle {
                font_size: theme.font_size_small,
                fill: Some(theme.node_text),
                ..Default::default()
            },
        });

        day += tick_interval;
    }
}

fn render_axis_grid(
    scene: &mut Scene,
    area: &ChartArea,
    theme: &Theme,
    min_day: i32,
    max_day: i32,
) {
    let axis_line_y = area.axis_line_y();
    let tick_interval = compute_tick_interval(area.total_days as i32);
    let mut day = min_day;
    while day <= max_day {
        let x = area.day_to_x(day);

        scene.push(Primitive::Path {
            segments: vec![
                PathSegment::MoveTo(Point::new(x, axis_line_y)),
                PathSegment::LineTo(Point::new(x, area.height - MARGIN)),
            ],
            style: Style {
                stroke: Some(Color::rgba(
                    theme.grid_stroke.r,
                    theme.grid_stroke.g,
                    theme.grid_stroke.b,
                    60,
                )),
                stroke_width: Some(0.5),
                stroke_dasharray: Some(vec![4.0, 3.0]),
                ..Default::default()
            },
            marker_start: None,
            marker_end: None,
        });

        day += tick_interval;
    }
}

fn render_bars(scene: &mut Scene, bars: &[BarPos], area: &ChartArea, theme: &Theme) {
    for bar in bars {
        let color = SECTION_COLORS[bar.section_idx % SECTION_COLORS.len()];
        let is_milestone = bar.tags.contains(&TaskTag::Milestone);
        let is_done = bar.tags.contains(&TaskTag::Done);
        let is_crit = bar.tags.contains(&TaskTag::Crit);
        let is_active = bar.tags.contains(&TaskTag::Active);

        let bar_color = if is_crit {
            Color::rgb(220, 60, 60)
        } else if is_done {
            Color::rgba(color.r, color.g, color.b, 140)
        } else if is_active {
            color
        } else {
            Color::rgba(color.r, color.g, color.b, 200)
        };

        if is_milestone {
            let cx = (bar.x1 + bar.x2) / 2.0;
            let s = BAR_HEIGHT * 0.4;
            scene.push(Primitive::Polygon {
                points: vec![
                    Point::new(cx, bar.y - s),
                    Point::new(cx + s, bar.y),
                    Point::new(cx, bar.y + s),
                    Point::new(cx - s, bar.y),
                ],
                style: Style {
                    fill: Some(bar_color),
                    stroke: Some(color),
                    stroke_width: Some(1.0),
                    ..Default::default()
                },
            });
        } else {
            let bar_w = (bar.x2 - bar.x1).max(2.0);
            scene.push(Primitive::Rect {
                bbox: BBox::new(bar.x1 + bar_w / 2.0, bar.y, bar_w, BAR_HEIGHT),
                rx: 3.0,
                ry: 3.0,
                style: Style {
                    fill: Some(bar_color),
                    ..Default::default()
                },
            });
        }

        scene.push(Primitive::Text {
            position: Point::new(area.chart_left - 8.0, bar.y),
            content: bar.name.clone(),
            anchor: TextAnchor::End,
            style: TextStyle {
                font_size: theme.font_size_edge_label,
                fill: Some(theme.node_text),
                ..Default::default()
            },
        });
    }
}

// ── Date resolution ──

struct ResolvedTask {
    name: String,
    start_day: i32,
    end_day: i32,
}

fn resolve_tasks(chart: &GanttChart) -> Vec<ResolvedTask> {
    let mut resolved: Vec<ResolvedTask> = Vec::new();
    let mut prev_end: i32 = 0;

    for section in &chart.sections {
        for task in &section.tasks {
            let start = match &task.start {
                TaskStart::Date(d) => parse_date_to_day(d, &chart.date_format),
                TaskStart::After(id) => resolved
                    .iter()
                    .find(|r| {
                        r.name == *id
                            || chart
                                .sections
                                .iter()
                                .flat_map(|s| &s.tasks)
                                .any(|t| t.id.as_deref() == Some(id) && t.name == r.name)
                    })
                    .map(|r| r.end_day)
                    .unwrap_or(prev_end),
                TaskStart::Auto => prev_end,
            };

            let end = match &task.end {
                TaskEnd::Date(d) => parse_date_to_day(d, &chart.date_format),
                TaskEnd::Duration(dur) => start + parse_duration(dur),
                TaskEnd::Auto => start + 1,
            };

            prev_end = end;
            resolved.push(ResolvedTask {
                name: task.name.clone(),
                start_day: start,
                end_day: end.max(start + 1),
            });
        }
    }

    resolved
}

/// Convert YYYY-MM-DD to a day number. Uses 0-indexed day-of-month internally.
fn parse_date_to_day(date: &str, _format: &str) -> i32 {
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() >= 3 {
        let y: i32 = parts[0].parse().unwrap_or(2024);
        let m: i32 = parts[1].parse().unwrap_or(1);
        let d: i32 = parts[2].parse().unwrap_or(1);
        // Approximate: each month = 30 days, day is 0-indexed for consistency
        (y - 2000) * 365 + (m - 1) * 30 + (d - 1)
    } else {
        0
    }
}

fn parse_duration(dur: &str) -> i32 {
    let s = dur.trim();
    if s.is_empty() {
        return 1;
    }
    let unit = s.chars().last().unwrap_or('d');
    let num: i32 = s[..s.len() - 1].parse().unwrap_or(1);
    match unit {
        'd' => num,
        'w' => num * 7,
        'h' => (num + 23) / 24,
        'm' => 1,
        _ => num,
    }
}

fn compute_tick_interval(total_days: i32) -> i32 {
    if total_days <= 7 {
        1
    } else if total_days <= 30 {
        7
    } else if total_days <= 90 {
        14
    } else if total_days <= 365 {
        30
    } else {
        90
    }
}

/// Convert day number back to "Mon DD" label.
fn format_day_label(day: i32) -> String {
    let d_in_year = day.rem_euclid(365);
    let month = d_in_year / 30 + 1;
    let day_of_month = d_in_year % 30 + 1;
    let month_name = match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "Jan",
    };
    format!("{month_name} {day_of_month}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(input: &str) -> Scene {
        let c = parser::parse(input).unwrap();
        to_scene(&c, &Theme::default())
    }

    #[test]
    fn scene_has_primitives() {
        let scene =
            render("gantt\n    title Plan\n    Task A :2024-01-01, 5d\n    Task B :2024-01-06, 3d");
        assert!(scene.len() >= 5);
    }

    #[test]
    fn scene_with_sections() {
        let scene = render(
            "gantt\n    section Phase 1\n    Task A :2024-01-01, 5d\n    section Phase 2\n    Task B :2024-01-06, 3d",
        );
        assert!(scene.len() >= 8);
    }

    #[test]
    fn milestone_renders_as_polygon() {
        let scene = render("gantt\n    Milestone :milestone, 2024-01-15, 0d");
        let has_polygon = scene
            .elements()
            .iter()
            .any(|e| matches!(&e.primitive, Primitive::Polygon { .. }));
        assert!(has_polygon);
    }

    #[test]
    fn crit_task_renders() {
        let scene = render("gantt\n    Critical :crit, 2024-01-01, 5d");
        assert!(scene.len() >= 3);
    }

    #[test]
    fn after_dependency() {
        let scene = render("gantt\n    Task A :a1, 2024-01-01, 5d\n    Task B :after a1, 3d");
        let rects: Vec<_> = scene
            .elements()
            .iter()
            .filter(|e| matches!(&e.primitive, Primitive::Rect { .. }))
            .collect();
        assert!(rects.len() >= 2);
    }

    #[test]
    fn duration_parsing() {
        assert_eq!(parse_duration("5d"), 5);
        assert_eq!(parse_duration("2w"), 14);
        assert_eq!(parse_duration("48h"), 2);
    }

    #[test]
    fn date_roundtrip() {
        let day = parse_date_to_day("2024-01-01", "YYYY-MM-DD");
        let label = format_day_label(day);
        assert_eq!(label, "Jan 1");
    }

    #[test]
    fn date_jan_15() {
        let day = parse_date_to_day("2024-01-15", "YYYY-MM-DD");
        let label = format_day_label(day);
        assert_eq!(label, "Jan 15");
    }
}
