pub mod ir;
pub mod parser;

use rusty_mermaid_core::{
    BBox, Color, Point, Primitive, Scene, SimpleTextMeasure, Style, TextAnchor, TextStyle, Theme,
};

use crate::common::palette::tint_color;
use ir::TreemapNode;

const CHART_W: f64 = 600.0;
const CHART_H: f64 = 400.0;
const SCENE_PAD: f64 = 16.0;
const INNER_PAD: f64 = 3.0;
const HEADER_H: f64 = 22.0;
const SECTION_PAD: f64 = 4.0;
const MIN_LABEL_SIZE: f64 = 8.0;

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

const TINT: f64 = 0.20;

pub fn to_scene(diagram: &ir::TreemapDiagram) -> Scene {
    to_scene_themed(diagram, &Theme::default())
}

pub fn to_scene_themed(diagram: &ir::TreemapDiagram, theme: &Theme) -> Scene {
    if diagram.roots.is_empty() {
        return Scene::empty();
    }

    let scene_w = CHART_W + SCENE_PAD * 2.0;
    let scene_h = CHART_H + SCENE_PAD * 2.0;
    let mut scene = Scene::new(scene_w, scene_h);

    // Treat all roots as children of a virtual root
    let rect = LayoutRect { x: SCENE_PAD, y: SCENE_PAD, w: CHART_W, h: CHART_H };
    layout_children(&diagram.roots, rect, 0, &mut scene, theme);

    scene
}

#[derive(Clone, Copy)]
struct LayoutRect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

fn layout_children(
    nodes: &[TreemapNode],
    rect: LayoutRect,
    depth: usize,
    scene: &mut Scene,
    theme: &Theme,
) {
    if nodes.is_empty() || rect.w < 2.0 || rect.h < 2.0 {
        return;
    }

    let total: f64 = nodes.iter().map(|n| n.total_value()).sum();
    if total <= 0.0 {
        return;
    }

    // Squarified layout: place items into the available rect
    let rects = squarify(nodes, rect, total);

    for (node, r) in nodes.iter().zip(rects.iter()) {
        let color = COLORS[(depth + nodes.iter().position(|n| std::ptr::eq(n, node)).unwrap_or(0)) % COLORS.len()];

        if node.is_leaf() {
            render_leaf(scene, node, *r, color, theme);
        } else {
            render_section(scene, node, *r, color, depth, theme);
        }
    }
}

/// Squarified treemap: greedily fill rows along the shorter side.
fn squarify(nodes: &[TreemapNode], rect: LayoutRect, total: f64) -> Vec<LayoutRect> {
    let mut result = vec![LayoutRect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 }; nodes.len()];

    // Sort indices by value descending
    let mut indices: Vec<usize> = (0..nodes.len()).collect();
    indices.sort_by(|&a, &b| {
        nodes[b].total_value().partial_cmp(&nodes[a].total_value()).unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut remaining = LayoutRect { x: rect.x, y: rect.y, w: rect.w, h: rect.h };
    let mut remaining_value = total;
    let mut i = 0;

    while i < indices.len() {
        let shorter = remaining.w.min(remaining.h);
        if shorter <= 0.0 { break; }

        // Build a row: keep adding items while aspect ratio improves
        let mut row = vec![indices[i]];
        let mut row_value = nodes[indices[i]].total_value();
        i += 1;

        while i < indices.len() {
            let next_val = nodes[indices[i]].total_value();
            let with = worst_aspect(nodes, &row, row_value, shorter, remaining_value, &remaining);
            row.push(indices[i]);
            let with_next = worst_aspect(nodes, &row, row_value + next_val, shorter, remaining_value, &remaining);
            if with_next > with {
                row.pop();
                break;
            }
            row_value += next_val;
            i += 1;
        }

        // Lay out this row
        let fraction = row_value / remaining_value;
        let horizontal = remaining.w >= remaining.h;

        let (row_w, row_h) = if horizontal {
            (remaining.w * fraction, remaining.h)
        } else {
            (remaining.w, remaining.h * fraction)
        };

        let mut offset = 0.0;
        for &idx in &row {
            let item_frac = nodes[idx].total_value() / row_value;
            if horizontal {
                let h = row_h * item_frac;
                result[idx] = LayoutRect {
                    x: remaining.x,
                    y: remaining.y + offset,
                    w: row_w,
                    h,
                };
                offset += h;
            } else {
                let w = row_w * item_frac;
                result[idx] = LayoutRect {
                    x: remaining.x + offset,
                    y: remaining.y,
                    w,
                    h: row_h,
                };
                offset += w;
            }
        }

        // Shrink remaining rect
        if horizontal {
            remaining.x += row_w;
            remaining.w -= row_w;
        } else {
            remaining.y += row_h;
            remaining.h -= row_h;
        }
        remaining_value -= row_value;
    }

    result
}

fn worst_aspect(
    nodes: &[TreemapNode],
    row: &[usize],
    row_value: f64,
    shorter: f64,
    _remaining_value: f64,
    remaining: &LayoutRect,
) -> f64 {
    let area_scale = (remaining.w * remaining.h) / _remaining_value.max(1.0);
    row.iter()
        .map(|&idx| {
            let item_area = nodes[idx].total_value() * area_scale;
            let row_area = row_value * area_scale;
            let row_len = row_area / shorter;
            let item_len = item_area / row_len.max(0.001);
            let ratio = (item_len / row_len.max(0.001)).max(row_len / item_len.max(0.001));
            ratio
        })
        .fold(0.0f64, f64::max)
}

fn render_leaf(scene: &mut Scene, node: &TreemapNode, r: LayoutRect, color: Color, theme: &Theme) {
    if r.w < 1.0 || r.h < 1.0 { return; } // skip degenerate rects
    let fill = tint_color(color, TINT);

    scene.push(Primitive::Rect {
        bbox: BBox::new(r.x + r.w / 2.0, r.y + r.h / 2.0, (r.w - INNER_PAD).max(1.0), (r.h - INNER_PAD).max(1.0)),
        rx: 3.0,
        ry: 3.0,
        style: Style {
            fill: Some(fill),
            stroke: Some(color),
            stroke_width: Some(1.0),
            ..Default::default()
        },
    });

    // Label: name + value, centered
    let _label = format!("{}\n{:.0}", node.name, node.total_value());
    let font_size = (r.h * 0.25).clamp(MIN_LABEL_SIZE, theme.font_size_node);
    let style = TextStyle { font_size, ..Default::default() };
    let label_w = SimpleTextMeasure::measure_raw(&node.name, &style).width;

    if label_w < r.w - 8.0 {
        scene.push(Primitive::Text {
            position: Point::new(r.x + r.w / 2.0, r.y + r.h / 2.0 - font_size * 0.3),
            content: node.name.clone(),
            anchor: TextAnchor::Middle,
            style: TextStyle {
                font_size,
                fill: Some(theme.node_text),
                font_weight: rusty_mermaid_core::FontWeight::Bold,
                ..Default::default()
            },
        });
        scene.push(Primitive::Text {
            position: Point::new(r.x + r.w / 2.0, r.y + r.h / 2.0 + font_size * 0.8),
            content: format!("{:.0}", node.total_value()),
            anchor: TextAnchor::Middle,
            style: TextStyle {
                font_size: font_size * 0.85,
                fill: Some(Color::rgb(100, 100, 100)),
                ..Default::default()
            },
        });
    }
}

fn render_section(
    scene: &mut Scene,
    node: &TreemapNode,
    r: LayoutRect,
    color: Color,
    depth: usize,
    theme: &Theme,
) {
    if r.w < 1.0 || r.h < 1.0 { return; }
    let fill = tint_color(color, TINT * 0.5);

    // Section background
    scene.push(Primitive::Rect {
        bbox: BBox::new(r.x + r.w / 2.0, r.y + r.h / 2.0, r.w - INNER_PAD, r.h - INNER_PAD),
        rx: 3.0,
        ry: 3.0,
        style: Style {
            fill: Some(fill),
            stroke: Some(color),
            stroke_width: Some(1.5),
            ..Default::default()
        },
    });

    // Header label
    if r.h > HEADER_H + 10.0 {
        let header_text = format!("{} ({:.0})", node.name, node.total_value());
        scene.push(Primitive::Text {
            position: Point::new(r.x + SECTION_PAD + 4.0, r.y + HEADER_H * 0.6),
            content: header_text,
            anchor: TextAnchor::Start,
            style: TextStyle {
                font_size: 11.0,
                fill: Some(color),
                font_weight: rusty_mermaid_core::FontWeight::Bold,
                ..Default::default()
            },
        });
    }

    // Recursively layout children in the area below the header
    let child_rect = LayoutRect {
        x: r.x + SECTION_PAD,
        y: r.y + HEADER_H + SECTION_PAD,
        w: r.w - SECTION_PAD * 2.0,
        h: r.h - HEADER_H - SECTION_PAD * 2.0,
    };
    if child_rect.w > 4.0 && child_rect.h > 4.0 {
        layout_children(&node.children, child_rect, depth + 1, scene, theme);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ir::TreemapDiagram;

    fn render(input: &str) -> Scene {
        let d = parser::parse(input).unwrap();
        to_scene(&d)
    }

    #[test]
    fn basic_renders() {
        let scene = render("treemap\n    A: 60\n    B: 40");
        assert!(!scene.is_empty());
    }

    #[test]
    fn has_rects_for_leaves() {
        let scene = render("treemap\n    A: 60\n    B: 40");
        let rects = scene.elements().iter().filter(|e| matches!(&e.primitive, Primitive::Rect { .. })).count();
        assert_eq!(rects, 2, "two leaf rects");
    }

    #[test]
    fn section_with_children() {
        let scene = render("treemap\n    Section\n        A: 60\n        B: 40");
        // 1 section rect + 2 leaf rects = 3
        let rects = scene.elements().iter().filter(|e| matches!(&e.primitive, Primitive::Rect { .. })).count();
        assert_eq!(rects, 3);
    }

    #[test]
    fn larger_value_gets_more_area() {
        let scene = render("treemap\n    Big: 90\n    Small: 10");
        let rects: Vec<_> = scene.elements().iter().filter_map(|e| {
            if let Primitive::Rect { bbox, .. } = &e.primitive { Some(bbox.width * bbox.height) } else { None }
        }).collect();
        assert_eq!(rects.len(), 2);
        assert!(rects[0] > rects[1], "bigger value should get larger rect");
    }

    #[test]
    fn all_positions_finite() {
        let scene = render("treemap\n    Sec\n        A: 50\n        B: 30\n        C: 20\n    D: 40");
        for elem in scene.elements() {
            match &elem.primitive {
                Primitive::Rect { bbox, .. } => {
                    assert!(bbox.x.is_finite() && bbox.y.is_finite());
                    assert!(bbox.width >= 0.0 && bbox.height >= 0.0);
                }
                Primitive::Text { position, .. } => {
                    assert!(position.x.is_finite() && position.y.is_finite());
                }
                _ => {}
            }
        }
    }

    #[test]
    fn squarify_produces_valid_rects() {
        let nodes = vec![
            TreemapNode { name: "A".into(), value: Some(60.0), children: vec![] },
            TreemapNode { name: "B".into(), value: Some(30.0), children: vec![] },
            TreemapNode { name: "C".into(), value: Some(10.0), children: vec![] },
        ];
        let rect = LayoutRect { x: 0.0, y: 0.0, w: 600.0, h: 400.0 };
        let rects = squarify(&nodes, rect, 100.0);
        assert_eq!(rects.len(), 3);
        for r in &rects {
            assert!(r.w > 0.0, "width should be positive");
            assert!(r.h > 0.0, "height should be positive");
        }
    }
}
