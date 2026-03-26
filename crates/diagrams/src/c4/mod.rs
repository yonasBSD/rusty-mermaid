pub mod ir;
pub mod parser;

use std::collections::HashMap;

use rusty_mermaid_core::{
    BBox, Color, PathSegment, Point, Primitive, Scene, SimpleTextMeasure, Style, TextAnchor,
    TextStyle, Theme, intersect_rect,
};

use ir::{C4Diagram, C4Element, C4Shape};

const MIN_ELEM_W: f64 = 160.0;
const ELEM_H: f64 = 100.0;
const GAP: f64 = 30.0;
const COLS: usize = 3;
const SCENE_PAD: f64 = 30.0;
const BOUNDARY_PAD: f64 = 16.0;
const TINT: f64 = 0.10;

const INTERNAL_COLOR: Color = Color::rgb(68, 114, 196);
const EXTERNAL_COLOR: Color = Color::rgb(128, 128, 128);
const PERSON_COLOR: Color = Color::rgb(8, 100, 164);
const BOUNDARY_COLOR: Color = Color::rgb(68, 114, 196);

pub fn to_scene(diagram: &C4Diagram) -> Scene {
    to_scene_themed(diagram, &Theme::default())
}

pub fn to_scene_themed(diagram: &C4Diagram, theme: &Theme) -> Scene {
    if diagram.elements.is_empty() {
        return Scene::new(100.0, 50.0);
    }

    let title_h = if diagram.title.is_some() { 36.0 } else { 0.0 };

    // Group elements by boundary
    let mut free_elements: Vec<usize> = Vec::new();
    let mut boundary_elements: HashMap<String, Vec<usize>> = HashMap::new();

    for (i, elem) in diagram.elements.iter().enumerate() {
        if let Some(ref b) = elem.boundary {
            boundary_elements.entry(b.clone()).or_default().push(i);
        } else {
            free_elements.push(i);
        }
    }

    // Position all elements on a grid
    let mut positions: HashMap<String, (f64, f64, f64, f64)> = HashMap::new(); // alias → (cx, cy, w, h)
    let mut cursor_y = SCENE_PAD + title_h;
    let mut max_right = 0.0f64;

    // Free elements first
    layout_row(&diagram.elements, &free_elements, SCENE_PAD, &mut cursor_y, &mut positions, &mut max_right);

    // Boundary groups
    for boundary in &diagram.boundaries {
        let elems = boundary_elements.get(&boundary.alias).cloned().unwrap_or_default();
        if elems.is_empty() { continue; }

        let boundary_y_start = cursor_y;
        cursor_y += 28.0; // header

        layout_row(&diagram.elements, &elems, SCENE_PAD + BOUNDARY_PAD, &mut cursor_y, &mut positions, &mut max_right);

        cursor_y += BOUNDARY_PAD;

        // Store boundary rect
        let bw = max_right - SCENE_PAD + BOUNDARY_PAD;
        let bh = cursor_y - boundary_y_start;
        positions.insert(
            format!("__boundary_{}", boundary.alias),
            (SCENE_PAD + bw / 2.0, boundary_y_start + bh / 2.0, bw, bh),
        );

        // Store boundary info for rendering
        positions.insert(
            format!("__blabel_{}", boundary.alias),
            (SCENE_PAD + 8.0, boundary_y_start + 14.0, 0.0, 0.0),
        );
    }

    let scene_w = max_right + SCENE_PAD;
    let scene_h = cursor_y + SCENE_PAD;
    let mut scene = Scene::new(scene_w, scene_h);

    // Title
    if let Some(title) = &diagram.title {
        scene.push(Primitive::Text {
            position: Point::new(scene_w / 2.0, SCENE_PAD + 12.0),
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

    // Boundary rects (behind elements)
    for boundary in &diagram.boundaries {
        if let Some(&(cx, cy, w, h)) = positions.get(&format!("__boundary_{}", boundary.alias)) {
            scene.push(Primitive::Rect {
                bbox: BBox::new(cx, cy, w, h),
                rx: 4.0, ry: 4.0,
                style: Style {
                    fill: Some(Color::rgba(0, 0, 0, 0)),
                    stroke: Some(BOUNDARY_COLOR),
                    stroke_width: Some(1.5),
                    stroke_dasharray: Some(vec![7.0, 5.0]),
                    ..Default::default()
                },
            });
        }
        if let Some(&(lx, ly, _, _)) = positions.get(&format!("__blabel_{}", boundary.alias)) {
            scene.push(Primitive::Text {
                position: Point::new(lx, ly),
                content: boundary.label.clone(),
                anchor: TextAnchor::Start,
                style: TextStyle {
                    font_size: 12.0,
                    fill: Some(BOUNDARY_COLOR),
                    font_weight: rusty_mermaid_core::FontWeight::Bold,
                    ..Default::default()
                },
            });
        }
    }

    // Compute visual widths per element (Database is narrower than its grid cell)
    let visual_widths: HashMap<String, f64> = diagram.elements.iter().map(|e| {
        let &(_, _, ew, _) = positions.get(&e.alias).unwrap_or(&(0.0, 0.0, MIN_ELEM_W, ELEM_H));
        let vw = if e.shape == C4Shape::Database { ew * 0.7 } else { ew };
        (e.alias.clone(), vw)
    }).collect();

    // Edge lines (behind elements), labels collected for on-top rendering
    let mut edge_labels: Vec<(f64, f64, String)> = Vec::new();

    for rel in &diagram.relationships {
        let Some(&(x1, y1, _, h1)) = positions.get(&rel.from) else { continue };
        let Some(&(x2, y2, _, h2)) = positions.get(&rel.to) else { continue };
        let vw1 = visual_widths.get(&rel.from).copied().unwrap_or(MIN_ELEM_W);
        let vw2 = visual_widths.get(&rel.to).copied().unwrap_or(MIN_ELEM_W);

        let start = intersect_rect(&BBox::new(x1, y1, vw1, h1), Point::new(x2, y2));
        let raw_end = intersect_rect(&BBox::new(x2, y2, vw2, h2), Point::new(x1, y1));
        let dx = raw_end.x - start.x;
        let dy = raw_end.y - start.y;
        let len = (dx * dx + dy * dy).sqrt().max(1.0);
        let end = Point::new(raw_end.x - 2.0 * dx / len, raw_end.y - 2.0 * dy / len);

        scene.push(Primitive::Path {
            segments: vec![
                PathSegment::MoveTo(start),
                PathSegment::LineTo(end),
            ],
            style: Style {
                stroke: Some(Color::rgb(140, 140, 140)),
                stroke_width: Some(1.2),
                ..Default::default()
            },
            marker_start: None,
            marker_end: Some(rusty_mermaid_core::MarkerType::ArrowPoint),
        });

        // Label near source, above the line
        let label = if let Some(tech) = &rel.technology {
            format!("{} [{}]", rel.label, tech)
        } else {
            rel.label.clone()
        };
        let is_horizontal = dx.abs() > dy.abs();
        let (lx, ly) = if is_horizontal {
            (start.x + dx * 0.15, start.y - 14.0)
        } else {
            (start.x - 10.0, start.y + dy * 0.2)
        };
        edge_labels.push((lx, ly, label));
    }

    // Elements (on top of edges)
    for elem in &diagram.elements {
        let Some(&(cx, cy, ew, _)) = positions.get(&elem.alias) else { continue };
        render_element(&mut scene, elem, cx, cy, ew, theme);
    }

    // Edge labels ON TOP with translucent background
    let label_bg = Color::rgba(255, 255, 255, 200);
    for (lx, ly, label) in &edge_labels {
        let label_style = TextStyle { font_size: 10.0, ..Default::default() };
        let tw = SimpleTextMeasure::measure_raw(label, &label_style).width;
        // Background rect
        scene.push(Primitive::Rect {
            bbox: BBox::new(*lx, *ly, tw + 8.0, 14.0),
            rx: 3.0, ry: 3.0,
            style: Style { fill: Some(label_bg), ..Default::default() },
        });
        scene.push(Primitive::Text {
            position: Point::new(*lx, *ly),
            content: label.clone(),
            anchor: TextAnchor::Middle,
            style: TextStyle {
                font_size: 10.0,
                fill: Some(Color::rgb(80, 80, 80)),
                ..Default::default()
            },
        });
    }

    scene
}

/// Measure the width an element needs based on its text content.
fn measure_elem_width(elem: &C4Element) -> f64 {
    let style = TextStyle { font_size: 13.0, ..Default::default() };
    let name_w = SimpleTextMeasure::measure_raw(&elem.label, &style).width;
    let desc_w = elem.description.as_ref()
        .map(|d| SimpleTextMeasure::measure_raw(d, &TextStyle { font_size: 9.0, ..Default::default() }).width)
        .unwrap_or(0.0);
    let tech_w = elem.technology.as_ref()
        .map(|t| SimpleTextMeasure::measure_raw(&format!("[{}]", t), &TextStyle { font_size: 10.0, ..Default::default() }).width)
        .unwrap_or(0.0);
    name_w.max(desc_w).max(tech_w) + 40.0 // padding
}

fn layout_row(
    elements: &[C4Element],
    indices: &[usize],
    start_x: f64,
    cursor_y: &mut f64,
    positions: &mut HashMap<String, (f64, f64, f64, f64)>,
    max_right: &mut f64,
) {
    let cols = COLS.min(indices.len()).max(1);
    let rows = (indices.len() + cols - 1) / cols;

    // Compute per-element widths, then use the max per row for uniform sizing
    let widths: Vec<f64> = indices.iter()
        .map(|&idx| measure_elem_width(&elements[idx]).max(MIN_ELEM_W))
        .collect();
    // Use max width across all elements in this group for uniform columns
    let col_w = widths.iter().copied().fold(MIN_ELEM_W, f64::max);

    for (i, &idx) in indices.iter().enumerate() {
        let col = i % cols;
        let row = i / cols;
        let cx = start_x + col as f64 * (col_w + GAP) + col_w / 2.0;
        let cy = *cursor_y + row as f64 * (ELEM_H + GAP) + ELEM_H / 2.0;
        positions.insert(elements[idx].alias.clone(), (cx, cy, col_w, ELEM_H));
        *max_right = max_right.max(cx + col_w / 2.0);
    }

    *cursor_y += rows as f64 * (ELEM_H + GAP);
}


fn render_element(scene: &mut Scene, elem: &C4Element, cx: f64, cy: f64, elem_w: f64, _theme: &Theme) {
    let base_color = if elem.external { EXTERNAL_COLOR }
        else if elem.shape == C4Shape::Person { PERSON_COLOR }
        else { INTERNAL_COLOR };

    let fill = Color::rgb(
        (255.0 * (1.0 - TINT) + base_color.r as f64 * TINT) as u8,
        (255.0 * (1.0 - TINT) + base_color.g as f64 * TINT) as u8,
        (255.0 * (1.0 - TINT) + base_color.b as f64 * TINT) as u8,
    );
    let style = Style {
        fill: Some(fill),
        stroke: Some(base_color),
        stroke_width: Some(1.5),
        ..Default::default()
    };

    match elem.shape {
        C4Shape::Database => {
            scene.push(Primitive::Rect {
                bbox: BBox::new(cx, cy, elem_w * 0.7, ELEM_H),
                rx: elem_w * 0.35, ry: 8.0,
                style,
            });
        }
        _ => {
            scene.push(Primitive::Rect {
                bbox: BBox::new(cx, cy, elem_w, ELEM_H),
                rx: 4.0, ry: 4.0,
                style,
            });
        }
    }

    // Person icon (simple circle head)
    if elem.shape == C4Shape::Person {
        scene.push(Primitive::Circle {
            center: Point::new(cx, cy - ELEM_H / 2.0 + 14.0),
            radius: 10.0,
            style: Style { fill: Some(base_color), ..Default::default() },
        });
    }

    // Labels: name (bold), then technology in brackets, then description
    let mut y = cy - 8.0;
    if elem.shape == C4Shape::Person { y += 8.0; }

    scene.push(Primitive::Text {
        position: Point::new(cx, y),
        content: elem.label.clone(),
        anchor: TextAnchor::Middle,
        style: TextStyle {
            font_size: 13.0,
            fill: Some(base_color),
            font_weight: rusty_mermaid_core::FontWeight::Bold,
            ..Default::default()
        },
    });

    if let Some(tech) = &elem.technology {
        y += 14.0;
        scene.push(Primitive::Text {
            position: Point::new(cx, y),
            content: format!("[{}]", tech),
            anchor: TextAnchor::Middle,
            style: TextStyle {
                font_size: 10.0,
                fill: Some(Color::rgb(120, 120, 120)),
                ..Default::default()
            },
        });
    }

    if let Some(desc) = &elem.description {
        y += 13.0;
        let desc_style = TextStyle { font_size: 9.0, ..Default::default() };
        let desc_w = SimpleTextMeasure::measure_raw(desc, &desc_style).width;
        if desc_w < elem_w - 16.0 {
            scene.push(Primitive::Text {
                position: Point::new(cx, y),
                content: desc.clone(),
                anchor: TextAnchor::Middle,
                style: TextStyle {
                    font_size: 9.0,
                    fill: Some(Color::rgb(100, 100, 100)),
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
        let scene = render("C4Context\n  Person(user, \"User\")\n  System(sys, \"System\")\n  Rel(user, sys, \"Uses\")");
        assert!(!scene.is_empty());
    }

    #[test]
    fn has_element_rects() {
        let scene = render("C4Context\n  Person(u, \"User\")\n  System(s, \"Sys\")");
        let rects = scene.elements().iter().filter(|e| matches!(&e.primitive, Primitive::Rect { .. })).count();
        assert!(rects >= 2);
    }

    #[test]
    fn boundary_renders() {
        let scene = render("C4Container\n  System_Boundary(b, \"Bank\") {\n    Container(web, \"Web\")\n  }");
        let dashed = scene.elements().iter().any(|e| {
            if let Primitive::Rect { style, .. } = &e.primitive {
                style.stroke_dasharray.is_some()
            } else { false }
        });
        assert!(dashed, "boundary should have dashed rect");
    }

    #[test]
    fn person_has_circle() {
        let scene = render("C4Context\n  Person(u, \"User\")");
        let circles = scene.elements().iter().filter(|e| matches!(&e.primitive, Primitive::Circle { .. })).count();
        assert!(circles >= 1, "person should have circle head");
    }

    #[test]
    fn relationship_has_path() {
        let scene = render("C4Context\n  Person(u, \"U\")\n  System(s, \"S\")\n  Rel(u, s, \"Uses\")");
        let paths = scene.elements().iter().filter(|e| matches!(&e.primitive, Primitive::Path { .. })).count();
        assert!(paths >= 1);
    }

    #[test]
    fn all_positions_finite() {
        let scene = render("C4Context\n  title Test\n  Person(u, \"User\", \"Desc\")\n  System(s, \"System\", \"Desc\")\n  Rel(u, s, \"Uses\", \"HTTPS\")");
        for elem in scene.elements() {
            match &elem.primitive {
                Primitive::Rect { bbox, .. } => assert!(bbox.x.is_finite() && bbox.y.is_finite()),
                Primitive::Text { position, .. } => assert!(position.x.is_finite() && position.y.is_finite()),
                Primitive::Circle { center, .. } => assert!(center.x.is_finite() && center.y.is_finite()),
                _ => {}
            }
        }
    }
}
