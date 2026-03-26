pub mod ir;
pub mod parser;

use rusty_mermaid_core::{
    BBox, Color, Point, Primitive, Scene, SimpleTextMeasure, Style, TextAnchor, TextStyle, Theme,
};

use ir::{PacketDiagram, PacketField};

const BIT_WIDTH: f64 = 32.0;
const ROW_HEIGHT: f64 = 32.0;
const PAD_X: f64 = 5.0;
const PAD_Y: f64 = 5.0;
const BIT_FONT_SIZE: f64 = 10.0;
const SCENE_PAD: f64 = 20.0;
// Shared palette blue at 12% tint — same translucent look as other diagrams
const BLOCK_BASE: Color = Color::rgb(78, 121, 167);
const BLOCK_TINT: f64 = 0.12;
const BLOCK_FILL: Color = Color::rgb(
    (255.0 * (1.0 - BLOCK_TINT) + BLOCK_BASE.r as f64 * BLOCK_TINT) as u8,
    (255.0 * (1.0 - BLOCK_TINT) + BLOCK_BASE.g as f64 * BLOCK_TINT) as u8,
    (255.0 * (1.0 - BLOCK_TINT) + BLOCK_BASE.b as f64 * BLOCK_TINT) as u8,
);
const BLOCK_STROKE: Color = Color::rgb(
    (BLOCK_BASE.r as f64 * 0.6) as u8,
    (BLOCK_BASE.g as f64 * 0.6) as u8,
    (BLOCK_BASE.b as f64 * 0.6) as u8,
);

pub fn to_scene(diagram: &PacketDiagram) -> Scene {
    to_scene_themed(diagram, &Theme::default())
}

pub fn to_scene_themed(diagram: &PacketDiagram, theme: &Theme) -> Scene {
    if diagram.fields.is_empty() {
        return Scene::empty();
    }

    let bpr = diagram.bits_per_row;

    // Split fields into row blocks (fields that cross row boundaries get split)
    let blocks = split_into_blocks(&diagram.fields, bpr);

    // Determine number of rows
    let max_bit = diagram.fields.last().map(|f| f.end).unwrap_or(0);
    let num_rows = max_bit / bpr + 1;

    let title_h = if diagram.title.is_some() { 24.0 } else { 0.0 };
    let grid_w = bpr as f64 * BIT_WIDTH;
    let grid_h = num_rows as f64 * (ROW_HEIGHT + PAD_Y);

    let scene_w = grid_w + SCENE_PAD * 2.0;
    let scene_h = grid_h + title_h + SCENE_PAD * 2.0;
    let mut scene = Scene::new(scene_w, scene_h);

    let ox = SCENE_PAD;
    let oy = SCENE_PAD + title_h;

    // Title
    if let Some(title) = &diagram.title {
        scene.push(Primitive::Text {
            position: Point::new(ox + grid_w / 2.0, SCENE_PAD + 8.0),
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

    // Render blocks
    for block in &blocks {
        let row = block.start / bpr;
        let col_start = block.start % bpr;
        let col_end = block.end % bpr;
        let width = (col_end - col_start + 1) as f64 * BIT_WIDTH - PAD_X;

        let x = ox + col_start as f64 * BIT_WIDTH + PAD_X / 2.0;
        let y = oy + row as f64 * (ROW_HEIGHT + PAD_Y);

        // Block rectangle (BBox is center-based)
        scene.push(Primitive::Rect {
            bbox: BBox::new(x + width / 2.0, y + ROW_HEIGHT / 2.0, width, ROW_HEIGHT),
            rx: 0.0,
            ry: 0.0,
            style: Style {
                fill: Some(BLOCK_FILL),
                stroke: Some(BLOCK_STROKE),
                stroke_width: Some(1.0),
                ..Default::default()
            },
        });

        // Label (centered in block)
        let label_style = TextStyle {
            font_size: theme.font_size_node,
            fill: Some(theme.node_text),
            ..Default::default()
        };
        let label_w = SimpleTextMeasure::measure_raw(&block.label, &label_style).width;
        if label_w < width - 4.0 {
            scene.push(Primitive::Text {
                position: Point::new(x + width / 2.0, y + ROW_HEIGHT / 2.0),
                content: block.label.clone(),
                anchor: TextAnchor::Middle,
                style: label_style,
            });
        }

        // Bit numbers inside block, pinned to top corners
        let bit_style = TextStyle {
            font_size: BIT_FONT_SIZE,
            fill: Some(Color::rgb(120, 120, 120)),
            ..Default::default()
        };

        if block.start == block.end {
            // Single bit: top-center
            scene.push(Primitive::Text {
                position: Point::new(x + width / 2.0, y + BIT_FONT_SIZE + 1.0),
                content: block.start.to_string(),
                anchor: TextAnchor::Middle,
                style: bit_style,
            });
        } else {
            // Range: start top-left, end top-right
            scene.push(Primitive::Text {
                position: Point::new(x + 3.0, y + BIT_FONT_SIZE + 1.0),
                content: block.start.to_string(),
                anchor: TextAnchor::Start,
                style: bit_style.clone(),
            });
            scene.push(Primitive::Text {
                position: Point::new(x + width - 3.0, y + BIT_FONT_SIZE + 1.0),
                content: block.end.to_string(),
                anchor: TextAnchor::End,
                style: bit_style,
            });
        }
    }

    scene
}

/// A block is a portion of a field that fits within one row.
#[derive(Debug, Clone)]
struct Block {
    start: usize,
    end: usize,
    label: String,
}

/// Split fields that cross row boundaries into per-row blocks.
fn split_into_blocks(fields: &[PacketField], bpr: usize) -> Vec<Block> {
    let mut blocks = Vec::new();

    for field in fields {
        let mut s = field.start;
        while s <= field.end {
            let row_end = (s / bpr + 1) * bpr - 1;
            let e = field.end.min(row_end);
            blocks.push(Block { start: s, end: e, label: field.label.clone() });
            s = e + 1;
        }
    }

    blocks
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
        let scene = render("packet-beta\n0-15: \"Source Port\"\n16-31: \"Dest Port\"");
        assert!(!scene.is_empty());
    }

    #[test]
    fn one_row_has_blocks() {
        let scene = render("packet-beta\n0-15: \"A\"\n16-31: \"B\"");
        let rects = scene.elements().iter().filter(|e| matches!(&e.primitive, Primitive::Rect { .. })).count();
        assert_eq!(rects, 2, "2 field rects");
    }

    #[test]
    fn field_splits_across_rows() {
        let blocks = split_into_blocks(
            &[PacketField { start: 0, end: 63, label: "Big".into() }],
            32,
        );
        assert_eq!(blocks.len(), 2, "64-bit field should split into 2 rows of 32");
        assert_eq!(blocks[0].start, 0);
        assert_eq!(blocks[0].end, 31);
        assert_eq!(blocks[1].start, 32);
        assert_eq!(blocks[1].end, 63);
    }

    #[test]
    fn single_bit_fields() {
        let scene = render("packet-beta\n0-7: \"Flags\"\n8: \"A\"\n9: \"B\"\n10-15: \"Pad\"");
        assert!(!scene.is_empty());
        let rects = scene.elements().iter().filter(|e| matches!(&e.primitive, Primitive::Rect { .. })).count();
        assert_eq!(rects, 4);
    }

    #[test]
    fn bit_numbers_present() {
        let scene = render("packet-beta\n0-15: \"Port\"");
        let texts: Vec<&str> = scene.elements().iter().filter_map(|e| {
            if let Primitive::Text { content, .. } = &e.primitive { Some(content.as_str()) } else { None }
        }).collect();
        assert!(texts.contains(&"0"), "should show start bit 0");
        assert!(texts.contains(&"15"), "should show end bit 15");
    }

    #[test]
    fn title_renders() {
        let scene = render("packet-beta\ntitle \"TCP Header\"\n0-31: \"Row\"");
        let has_title = scene.elements().iter().any(|e| {
            if let Primitive::Text { content, .. } = &e.primitive { content == "TCP Header" } else { false }
        });
        assert!(has_title);
    }

    #[test]
    fn all_positions_finite() {
        let scene = render("packet-beta\n0-15: \"A\"\n16-31: \"B\"\n32-63: \"C\"\n64-95: \"D\"");
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
