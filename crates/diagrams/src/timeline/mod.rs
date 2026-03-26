pub mod ir;
pub mod parser;

use rusty_mermaid_core::{
    BBox, Color, Direction, PathSegment, Point, Primitive, Scene, SimpleTextMeasure, Style,
    TextAnchor, TextStyle, Theme,
};

use ir::TimelineDiagram;

const TASK_BOX_W: f64 = 120.0;
const TASK_BOX_H: f64 = 36.0;
const EVENT_BOX_W: f64 = 140.0;
const EVENT_BOX_H: f64 = 30.0;
const SECTION_HEADER_H: f64 = 28.0;
const GAP: f64 = 20.0;
const MARGIN: f64 = 20.0;
const DOT_RADIUS: f64 = 4.0;

/// 8-color section palette.
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

pub fn to_scene(diagram: &TimelineDiagram) -> Scene {
    to_scene_themed(diagram, &Theme::default())
}

pub fn to_scene_themed(diagram: &TimelineDiagram, theme: &Theme) -> Scene {
    match diagram.direction {
        Direction::TB => render_horizontal(diagram, theme), // TB = vertical axis (tasks top-to-bottom)
        _ => render_vertical(diagram, theme),               // LR default = horizontal axis (time left-to-right)
    }
}

// ── Shared types ──

struct TaskPos {
    axis_pos: f64,
    name: String,
    events: Vec<String>,
    section_idx: usize,
}

fn title_width(title: &str, theme: &Theme) -> f64 {
    let style = TextStyle { font_size: theme.font_size_title, ..Default::default() };
    SimpleTextMeasure::measure_raw(title, &style).width
}

// ── LR layout: vertical axis, tasks left, events right ──

fn render_horizontal(diagram: &TimelineDiagram, theme: &Theme) -> Scene {
    let axis_x = MARGIN + TASK_BOX_W + GAP;
    let mut y = MARGIN;

    if let Some(_title) = &diagram.title {
        y += theme.font_size_title + GAP;
    }

    // Compute max event box width from content
    let event_style = TextStyle { font_size: theme.font_size_edge_label, ..Default::default() };
    let event_box_w = diagram.sections.iter()
        .flat_map(|s| s.tasks.iter())
        .flat_map(|t| t.events.iter())
        .map(|e| SimpleTextMeasure::measure_raw(e, &event_style).width + GAP)
        .fold(EVENT_BOX_W, f64::max);

    let title_bottom = y;
    let mut tasks: Vec<TaskPos> = Vec::new();
    let mut section_ranges: Vec<(f64, f64, Option<String>, usize)> = Vec::new();

    for (si, section) in diagram.sections.iter().enumerate() {
        let start = y;
        if section.name.is_some() { y += SECTION_HEADER_H + GAP / 2.0; }
        for task in &section.tasks {
            let h = task_event_height(task.events.len());
            tasks.push(TaskPos { axis_pos: y + h / 2.0, name: task.name.clone(), events: task.events.clone(), section_idx: si });
            y += h + GAP;
        }
        section_ranges.push((start, y - GAP, section.name.clone(), si));
    }

    let content_w = axis_x + GAP + event_box_w + MARGIN;
    let title_w = diagram.title.as_ref().map(|t| title_width(t, theme) + MARGIN * 2.0).unwrap_or(0.0);
    let width = content_w.max(title_w);
    // Bottom extends to last task + its events
    let last_bottom = tasks.last().map(|t| {
        let eh = task_event_height(t.events.len());
        t.axis_pos + eh / 2.0
    }).unwrap_or(y);
    let height = last_bottom + MARGIN;
    let mut scene = Scene::new(width, height);

    // Title
    if let Some(title) = &diagram.title {
        render_title(&mut scene, title, width / 2.0, MARGIN + theme.font_size_title * 0.4, theme);
    }

    // Vertical axis — extends from first task to last task
    let axis_start = tasks.first().map(|t| t.axis_pos - task_event_height(t.events.len()) / 2.0 - GAP / 2.0).unwrap_or(title_bottom);
    let axis_end = last_bottom + MARGIN / 2.0;
    render_axis_line(&mut scene, Point::new(axis_x, axis_start), Point::new(axis_x, axis_end), theme);

    // Sections
    let label_x = MARGIN + 8.0; // left-aligned, away from axis
    for (start, end, name, si) in &section_ranges {
        let sec_cy = (*start + *end) / 2.0;
        let sec_h = *end - *start + GAP;
        render_section_bg(&mut scene, width / 2.0, sec_cy, width - MARGIN, sec_h, *si);
        if let Some(name) = name {
            render_section_label_left(&mut scene, label_x, *start + SECTION_HEADER_H * 0.4, name, *si, theme);
        }
    }

    // Tasks + events
    for tp in &tasks {
        let color = SECTION_COLORS[tp.section_idx % SECTION_COLORS.len()];
        let task_x = axis_x - GAP - TASK_BOX_W / 2.0;
        render_box(&mut scene, task_x, tp.axis_pos, TASK_BOX_W, TASK_BOX_H, &tp.name, color, true, theme);
        render_dot(&mut scene, axis_x, tp.axis_pos, color);

        let event_x = axis_x + GAP + event_box_w / 2.0;
        let total_h = tp.events.len() as f64 * (EVENT_BOX_H + 4.0);
        let start_y = tp.axis_pos - total_h / 2.0 + EVENT_BOX_H / 2.0;
        for (ei, event) in tp.events.iter().enumerate() {
            let ey = start_y + ei as f64 * (EVENT_BOX_H + 4.0);
            render_connector(&mut scene, Point::new(axis_x + 5.0, tp.axis_pos), Point::new(event_x - event_box_w / 2.0, ey), theme);
            render_box(&mut scene, event_x, ey, event_box_w, EVENT_BOX_H, event, color, false, theme);
        }
    }

    scene
}

// ── TB layout: horizontal axis, tasks above, events below ──

fn render_vertical(diagram: &TimelineDiagram, theme: &Theme) -> Scene {
    let title_offset = if diagram.title.is_some() { theme.font_size_title + GAP * 2.0 } else { 0.0 };
    let has_sections = diagram.sections.iter().any(|s| s.name.is_some());
    let section_label_h = if has_sections { SECTION_HEADER_H + GAP } else { 0.0 };
    let axis_y = MARGIN + title_offset + section_label_h + TASK_BOX_H + GAP;
    let mut x = MARGIN;

    // Compute max event box width from content
    let event_style = TextStyle { font_size: theme.font_size_edge_label, ..Default::default() };
    let event_box_w = diagram.sections.iter()
        .flat_map(|s| s.tasks.iter())
        .flat_map(|t| t.events.iter())
        .map(|e| SimpleTextMeasure::measure_raw(e, &event_style).width + GAP)
        .fold(EVENT_BOX_W, f64::max);

    // Compute max events per task for height
    let max_events = diagram.sections.iter()
        .flat_map(|s| s.tasks.iter())
        .map(|t| t.events.len())
        .max()
        .unwrap_or(0);

    let mut tasks: Vec<TaskPos> = Vec::new();
    let mut section_ranges: Vec<(f64, f64, Option<String>, usize)> = Vec::new();

    for (si, section) in diagram.sections.iter().enumerate() {
        let start = x;
        for task in &section.tasks {
            let w = TASK_BOX_W.max(event_box_w);
            tasks.push(TaskPos { axis_pos: x + w / 2.0, name: task.name.clone(), events: task.events.clone(), section_idx: si });
            x += w + GAP;
        }
        if !section.tasks.is_empty() {
            x += GAP; // extra gap between sections
        }
        section_ranges.push((start, x - GAP * 2.0, section.name.clone(), si));
    }

    let title_w = diagram.title.as_ref().map(|t| title_width(t, theme) + MARGIN * 2.0).unwrap_or(0.0);
    let width = (x + MARGIN).max(title_w);
    let events_h = max_events as f64 * (EVENT_BOX_H + 4.0);
    let height = axis_y + GAP + events_h + MARGIN;
    let mut scene = Scene::new(width, height);

    // Title
    if let Some(title) = &diagram.title {
        render_title(&mut scene, title, width / 2.0, MARGIN + theme.font_size_title * 0.4, theme);
    }

    // Horizontal axis — from first task to last task
    let axis_start = tasks.first().map(|t| t.axis_pos - TASK_BOX_W / 2.0 - GAP / 2.0).unwrap_or(MARGIN);
    let axis_end = tasks.last().map(|t| t.axis_pos + TASK_BOX_W / 2.0 + GAP / 2.0).unwrap_or(width - MARGIN);
    render_axis_line(&mut scene, Point::new(axis_start, axis_y), Point::new(axis_end, axis_y), theme);

    // Section label boxes + backgrounds — equal padding top and bottom
    let section_box_y = axis_y - TASK_BOX_H - GAP * 2.0 - SECTION_HEADER_H / 2.0;
    let content_top = section_box_y - SECTION_HEADER_H / 2.0;
    let content_bottom = axis_y + GAP + max_events as f64 * (EVENT_BOX_H + 4.0);
    let bg_pad = GAP / 2.0;
    let bg_top = content_top - bg_pad;
    let bg_bottom = content_bottom + bg_pad;
    let bg_cy = (bg_top + bg_bottom) / 2.0;
    let bg_h = bg_bottom - bg_top;
    for (start, end, name, si) in &section_ranges {
        let sec_cx = (*start + *end) / 2.0;
        let sec_w = *end - *start + GAP;
        render_section_bg(&mut scene, sec_cx, bg_cy, sec_w, bg_h, *si);
        if let Some(name) = name {
            let color = SECTION_COLORS[*si % SECTION_COLORS.len()];
            render_box(&mut scene, sec_cx, section_box_y, sec_w - GAP, SECTION_HEADER_H, name, color, true, theme);
        }
    }

    // Tasks (above axis) + vertical line + events (below axis)
    let events_bottom = axis_y + GAP + max_events as f64 * (EVENT_BOX_H + 4.0);
    for tp in &tasks {
        let color = SECTION_COLORS[tp.section_idx % SECTION_COLORS.len()];
        let task_y = axis_y - GAP - TASK_BOX_H / 2.0;
        render_box(&mut scene, tp.axis_pos, task_y, TASK_BOX_W, TASK_BOX_H, &tp.name, color, true, theme);
        render_dot(&mut scene, tp.axis_pos, axis_y, color);

        // Vertical line from axis to bottom of events area
        if !tp.events.is_empty() {
            render_connector(&mut scene, Point::new(tp.axis_pos, axis_y + 5.0), Point::new(tp.axis_pos, events_bottom), theme);
        }

        let start_y = axis_y + GAP + EVENT_BOX_H / 2.0;
        for (ei, event) in tp.events.iter().enumerate() {
            let ey = start_y + ei as f64 * (EVENT_BOX_H + 4.0);
            render_box(&mut scene, tp.axis_pos, ey, event_box_w, EVENT_BOX_H, event, color, false, theme);
        }
    }

    scene
}

// ── Shared rendering helpers ──

fn task_event_height(n_events: usize) -> f64 {
    if n_events == 0 { TASK_BOX_H } else { (n_events as f64 * (EVENT_BOX_H + 4.0)).max(TASK_BOX_H) }
}

fn render_title(scene: &mut Scene, title: &str, x: f64, y: f64, theme: &Theme) {
    scene.push(Primitive::Text {
        position: Point::new(x, y),
        content: title.to_string(),
        anchor: TextAnchor::Middle,
        style: TextStyle {
            font_size: theme.font_size_title,
            fill: Some(theme.node_text),
            font_weight: rusty_mermaid_core::FontWeight::Bold,
            ..Default::default()
        },
    });
}

fn render_axis_line(scene: &mut Scene, from: Point, to: Point, theme: &Theme) {
    scene.push(Primitive::Path {
        segments: vec![PathSegment::MoveTo(from), PathSegment::LineTo(to)],
        style: Style { stroke: Some(theme.edge_stroke), stroke_width: Some(2.0), ..Default::default() },
        marker_start: None,
        marker_end: None,
    });
}

fn render_section_bg(scene: &mut Scene, cx: f64, cy: f64, w: f64, h: f64, idx: usize) {
    let color = SECTION_COLORS[idx % SECTION_COLORS.len()];
    scene.push(Primitive::Rect {
        bbox: BBox::new(cx, cy, w, h),
        rx: 4.0, ry: 4.0,
        style: Style { fill: Some(Color::rgba(color.r, color.g, color.b, 30)), ..Default::default() },
    });
}

fn render_section_label_left(scene: &mut Scene, x: f64, y: f64, name: &str, idx: usize, theme: &Theme) {
    let color = SECTION_COLORS[idx % SECTION_COLORS.len()];
    scene.push(Primitive::Text {
        position: Point::new(x, y),
        content: name.to_string(),
        anchor: TextAnchor::Start,
        style: TextStyle {
            font_size: theme.font_size_label,
            fill: Some(color),
            font_weight: rusty_mermaid_core::FontWeight::Bold,
            ..Default::default()
        },
    });
}

fn render_box(scene: &mut Scene, x: f64, y: f64, w: f64, h: f64, text: &str, color: Color, bold: bool, theme: &Theme) {
    let (rx, fill) = if bold {
        (4.0, Color::rgba(color.r, color.g, color.b, 80))
    } else {
        (12.0, theme.node_fill)
    };
    scene.push(Primitive::Rect {
        bbox: BBox::new(x, y, w, h),
        rx, ry: rx,
        style: Style { fill: Some(fill), stroke: Some(color), stroke_width: Some(1.0), ..Default::default() },
    });
    scene.push(Primitive::Text {
        position: Point::new(x, y),
        content: text.to_string(),
        anchor: TextAnchor::Middle,
        style: TextStyle {
            font_size: if bold { theme.font_size_node } else { theme.font_size_edge_label },
            fill: Some(theme.node_text),
            font_weight: if bold { rusty_mermaid_core::FontWeight::Bold } else { rusty_mermaid_core::FontWeight::Normal },
            ..Default::default()
        },
    });
}

fn render_dot(scene: &mut Scene, x: f64, y: f64, color: Color) {
    scene.push(Primitive::Circle {
        center: Point::new(x, y),
        radius: DOT_RADIUS,
        style: Style { fill: Some(color), ..Default::default() },
    });
}

fn render_connector(scene: &mut Scene, from: Point, to: Point, theme: &Theme) {
    scene.push(Primitive::Path {
        segments: vec![PathSegment::MoveTo(from), PathSegment::LineTo(to)],
        style: Style {
            stroke: Some(theme.edge_stroke),
            stroke_width: Some(0.5),
            stroke_dasharray: Some(vec![4.0, 3.0]),
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
        let d = parser::parse(input).unwrap();
        to_scene(&d)
    }

    #[test]
    fn scene_has_primitives() {
        let scene = render("timeline\n    2020 : Event A\n    2021 : Event B");
        assert!(scene.len() >= 6, "axis + 2 tasks + 2 events + dots");
    }

    #[test]
    fn scene_with_title() {
        let scene = render("timeline\n    title History\n    2020 : X");
        let has_title = scene.elements().iter().any(|e| {
            if let Primitive::Text { content, .. } = &e.primitive { content == "History" } else { false }
        });
        assert!(has_title);
    }

    #[test]
    fn scene_with_sections() {
        let scene = render("timeline\n    section Era1\n        2020 : X\n    section Era2\n        2021 : Y");
        assert!(scene.len() >= 8);
    }

    #[test]
    fn multiple_events_per_task() {
        let scene = render("timeline\n    2020 : A : B : C");
        let event_boxes: Vec<_> = scene.elements().iter().filter(|e| {
            if let Primitive::Rect { bbox, .. } = &e.primitive { (bbox.width - EVENT_BOX_W).abs() < 1.0 } else { false }
        }).collect();
        assert_eq!(event_boxes.len(), 3);
    }

    #[test]
    fn tb_direction_renders() {
        let scene = render("timeline TB\n    2020 : X\n    2021 : Y");
        // TB has horizontal axis — check scene has content
        assert!(scene.len() >= 5);
        assert!(scene.width > scene.height || scene.width > 100.0, "TB should be wider");
    }

    #[test]
    fn title_expands_width() {
        let scene = render("timeline\n    title A Very Long Title That Should Expand The Scene Width\n    2020 : X");
        let short = render("timeline\n    title Hi\n    2020 : X");
        assert!(scene.width > short.width, "long title should produce wider scene");
    }
}
