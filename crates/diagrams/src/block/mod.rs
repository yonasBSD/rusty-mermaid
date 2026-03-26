pub mod ir;
pub mod parser;

use std::collections::HashMap;

use rusty_mermaid_core::{
    BBox, Color, PathSegment, Point, Primitive, Scene, SimpleTextMeasure, Style, TextAnchor,
    TextStyle, Theme, intersect_rect,
};

use crate::common::palette::tint_color;
use ir::{Block, BlockDiagram, BlockShape, EdgeStyle};

const CELL_W: f64 = 120.0;
const CELL_H: f64 = 60.0;
const GAP: f64 = 16.0;
const SCENE_PAD: f64 = 24.0;
const TINT: f64 = 0.12;

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

pub fn to_scene(diagram: &BlockDiagram) -> Scene {
    to_scene_themed(diagram, &Theme::default())
}

pub fn to_scene_themed(diagram: &BlockDiagram, theme: &Theme) -> Scene {
    if diagram.blocks.is_empty() {
        return Scene::new(100.0, 50.0);
    }

    let cols = diagram.columns.max(1);

    // Compute positions using a grid cursor that advances by block.span
    let mut positions: HashMap<String, (f64, f64, f64)> = HashMap::new(); // id → (cx, cy, width)
    let mut grid_pos: usize = 0; // linear position in the grid

    for block in &diagram.blocks {
        let span = block.span.min(cols);
        // If span doesn't fit in current row, wrap to next
        let col = grid_pos % cols;
        if col + span > cols {
            grid_pos += cols - col; // skip to next row
        }
        let col = grid_pos % cols;
        let row = grid_pos / cols;
        let block_w = span as f64 * (CELL_W + GAP) - GAP;
        let cx = SCENE_PAD + col as f64 * (CELL_W + GAP) + block_w / 2.0;
        let cy = SCENE_PAD + row as f64 * (CELL_H + GAP) + CELL_H / 2.0;
        positions.insert(block.id.clone(), (cx, cy, block_w));
        grid_pos += span;
    }

    let total_rows = (grid_pos + cols - 1) / cols;
    let grid_w = cols as f64 * (CELL_W + GAP) - GAP;
    let grid_h = total_rows as f64 * (CELL_H + GAP) - GAP;
    let scene_w = grid_w + SCENE_PAD * 2.0;
    let scene_h = grid_h + SCENE_PAD * 2.0;
    let mut scene = Scene::new(scene_w, scene_h);

    // Draw edges FIRST (behind blocks)
    for edge in &diagram.edges {
        let Some(&(x1, y1, w1)) = positions.get(&edge.from) else { continue };
        let Some(&(x2, y2, w2)) = positions.get(&edge.to) else { continue };

        // Clip endpoints to rect borders, then pull back by marker size
        let start = intersect_rect(&BBox::new(x1, y1, w1, CELL_H), Point::new(x2, y2));
        let raw_end = intersect_rect(&BBox::new(x2, y2, w2, CELL_H), Point::new(x1, y1));

        // Shorten end by ~8px (marker overshoot) along the line direction
        let dx = raw_end.x - start.x;
        let dy = raw_end.y - start.y;
        let len = (dx * dx + dy * dy).sqrt().max(1.0);
        let marker_inset = 3.0;
        let end = Point::new(
            raw_end.x - marker_inset * dx / len,
            raw_end.y - marker_inset * dy / len,
        );

        let style = match edge.style {
            EdgeStyle::Arrow => Style {
                stroke: Some(theme.edge_stroke),
                stroke_width: Some(1.5),
                ..Default::default()
            },
            EdgeStyle::Dotted => Style {
                stroke: Some(theme.edge_stroke),
                stroke_width: Some(1.5),
                stroke_dasharray: Some(vec![5.0, 3.0]),
                ..Default::default()
            },
            EdgeStyle::Thick => Style {
                stroke: Some(theme.edge_stroke),
                stroke_width: Some(3.0),
                ..Default::default()
            },
        };

        scene.push(Primitive::Path {
            segments: vec![
                PathSegment::MoveTo(start),
                PathSegment::LineTo(end),
            ],
            style,
            marker_start: None,
            marker_end: Some(rusty_mermaid_core::MarkerType::ArrowPoint),
        });

        // Edge label at midpoint
        if let Some(label) = &edge.label {
            let mx = (start.x + end.x) / 2.0;
            let my = (start.y + end.y) / 2.0 - 8.0;
            scene.push(Primitive::Text {
                position: Point::new(mx, my),
                content: label.clone(),
                anchor: TextAnchor::Middle,
                style: TextStyle {
                    font_size: 10.0,
                    fill: Some(theme.edge_label_text),
                    ..Default::default()
                },
            });
        }
    }

    // Draw blocks ON TOP of edges
    for (i, block) in diagram.blocks.iter().enumerate() {
        if block.shape == BlockShape::Space {
            continue;
        }
        let &(cx, cy, bw) = positions.get(&block.id).unwrap();
        let color = COLORS[i % COLORS.len()];
        render_block(&mut scene, block, cx, cy, bw, color, theme);
    }

    scene
}

fn render_block(scene: &mut Scene, block: &Block, cx: f64, cy: f64, w: f64, color: Color, theme: &Theme) {
    let cell_w = w;
    let fill = tint_color(color, TINT);
    let stroke_style = Style {
        fill: Some(fill),
        stroke: Some(color),
        stroke_width: Some(1.5),
        ..Default::default()
    };

    match block.shape {
        BlockShape::Rect => {
            scene.push(Primitive::Rect {
                bbox: BBox::new(cx, cy, cell_w, CELL_H),
                rx: 3.0, ry: 3.0,
                style: stroke_style,
            });
        }
        BlockShape::Round => {
            scene.push(Primitive::Rect {
                bbox: BBox::new(cx, cy, cell_w, CELL_H),
                rx: 12.0, ry: 12.0,
                style: stroke_style,
            });
        }
        BlockShape::Stadium => {
            scene.push(Primitive::Rect {
                bbox: BBox::new(cx, cy, cell_w, CELL_H),
                rx: CELL_H / 2.0, ry: CELL_H / 2.0,
                style: stroke_style,
            });
        }
        BlockShape::Diamond => {
            let hw = cell_w / 2.0;
            let hh = CELL_H / 2.0;
            scene.push(Primitive::Polygon {
                points: vec![
                    Point::new(cx, cy - hh),
                    Point::new(cx + hw, cy),
                    Point::new(cx, cy + hh),
                    Point::new(cx - hw, cy),
                ],
                style: stroke_style,
            });
        }
        BlockShape::Hexagon => {
            let hw = cell_w / 2.0;
            let hh = CELL_H / 2.0;
            let inset = hh * 0.5;
            scene.push(Primitive::Polygon {
                points: vec![
                    Point::new(cx - hw + inset, cy - hh),
                    Point::new(cx + hw - inset, cy - hh),
                    Point::new(cx + hw, cy),
                    Point::new(cx + hw - inset, cy + hh),
                    Point::new(cx - hw + inset, cy + hh),
                    Point::new(cx - hw, cy),
                ],
                style: stroke_style,
            });
        }
        BlockShape::Circle => {
            let r = CELL_H / 2.0;
            scene.push(Primitive::Circle {
                center: Point::new(cx, cy),
                radius: r,
                style: stroke_style,
            });
        }
        BlockShape::Cylinder => {
            // Approximate as rect with rounded top/bottom
            scene.push(Primitive::Rect {
                bbox: BBox::new(cx, cy, cell_w * 0.7, CELL_H),
                rx: cell_w * 0.35, ry: 8.0,
                style: stroke_style,
            });
        }
        BlockShape::Space => {}
    }

    // Label
    if !block.label.is_empty() {
        let label_style = TextStyle {
            font_size: theme.font_size_node,
            ..Default::default()
        };
        let label_w = SimpleTextMeasure::measure_raw(&block.label, &label_style).width;
        if label_w < cell_w - 8.0 || block.shape == BlockShape::Diamond {
            scene.push(Primitive::Text {
                position: Point::new(cx, cy),
                content: block.label.clone(),
                anchor: TextAnchor::Middle,
                style: TextStyle {
                    font_size: theme.font_size_node,
                    fill: Some(theme.node_text),
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
        let scene = render("block-beta\n  columns 2\n  a[\"A\"]\n  b[\"B\"]\n  c[\"C\"]\n  d[\"D\"]");
        assert!(!scene.is_empty());
    }

    #[test]
    fn has_block_rects() {
        let scene = render("block-beta\n  a[\"A\"]\n  b[\"B\"]");
        let rects = scene.elements().iter().filter(|e| matches!(&e.primitive, Primitive::Rect { .. })).count();
        assert_eq!(rects, 2);
    }

    #[test]
    fn edges_render() {
        let scene = render("block-beta\n  a[\"A\"]\n  b[\"B\"]\n  a --> b");
        let paths = scene.elements().iter().filter(|e| matches!(&e.primitive, Primitive::Path { .. })).count();
        assert!(paths >= 1, "should have edge path");
    }

    #[test]
    fn diamond_shape() {
        let scene = render("block-beta\n  a{\"Decision\"}");
        let polys = scene.elements().iter().filter(|e| matches!(&e.primitive, Primitive::Polygon { .. })).count();
        assert_eq!(polys, 1);
    }

    #[test]
    fn space_creates_gap() {
        let scene = render("block-beta\n  columns 3\n  a[\"A\"]\n  space\n  b[\"B\"]");
        let rects = scene.elements().iter().filter(|e| matches!(&e.primitive, Primitive::Rect { .. })).count();
        assert_eq!(rects, 2, "space should not create a visible rect");
    }

    #[test]
    fn all_positions_finite() {
        let scene = render("block-beta\n  columns 2\n  a[\"Start\"]\n  b(\"Process\")\n  c{\"Check\"}\n  d[\"End\"]\n  a --> b\n  b --> c");
        for elem in scene.elements() {
            match &elem.primitive {
                Primitive::Rect { bbox, .. } => {
                    assert!(bbox.x.is_finite() && bbox.y.is_finite());
                }
                Primitive::Text { position, .. } => {
                    assert!(position.x.is_finite() && position.y.is_finite());
                }
                _ => {}
            }
        }
    }
}
