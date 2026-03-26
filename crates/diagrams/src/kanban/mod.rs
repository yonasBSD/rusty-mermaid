pub mod ir;
pub mod parser;

use rusty_mermaid_core::{
    BBox, Color, Primitive, Scene, SimpleTextMeasure, Style, TextAnchor, TextStyle, Theme,
};

use ir::KanbanBoard;

const COLUMN_WIDTH: f64 = 200.0;
const COLUMN_GAP: f64 = 16.0;
const CARD_PADDING: f64 = 10.0;
const CARD_GAP: f64 = 8.0;
const CARD_RX: f64 = 6.0;
const HEADER_HEIGHT: f64 = 36.0;
const MARGIN: f64 = 16.0;

/// Column header colors.
const COLUMN_COLORS: [Color; 6] = [
    Color::rgb(78, 121, 167),
    Color::rgb(242, 142, 44),
    Color::rgb(89, 161, 79),
    Color::rgb(225, 87, 89),
    Color::rgb(175, 122, 161),
    Color::rgb(118, 183, 178),
];

pub fn to_scene(board: &KanbanBoard) -> Scene {
    to_scene_themed(board, &Theme::default())
}

/// Precomputed column metrics: widths and per-card heights.
struct ColumnMetrics {
    col_widths: Vec<f64>,
    col_card_heights: Vec<Vec<f64>>,
    max_col_height: f64,
    line_height: f64,
}

impl ColumnMetrics {
    fn from_board(board: &KanbanBoard, theme: &Theme) -> Self {
        let card_style = TextStyle { font_size: theme.font_size_edge_label, ..Default::default() };
        let line_height = theme.font_size_edge_label * rusty_mermaid_core::constants::LINE_HEIGHT_MULTIPLIER;

        let mut col_widths: Vec<f64> = Vec::new();
        let mut col_card_heights: Vec<Vec<f64>> = Vec::new();

        for col in &board.columns {
            let mut max_w = SimpleTextMeasure::measure_raw(&col.label, &TextStyle {
                font_size: theme.font_size_node, ..Default::default()
            }).width + CARD_PADDING * 4.0;

            let mut card_heights = Vec::new();
            for card in &col.cards {
                let text_w = SimpleTextMeasure::measure_raw(&card.label, &card_style).width;
                max_w = max_w.max(text_w + CARD_PADDING * 4.0);
                let mut h = line_height + CARD_PADDING * 2.0;
                if card.priority.is_some() || card.assigned.is_some() || card.ticket.is_some() {
                    h += line_height * 0.8;
                }
                card_heights.push(h);
            }

            col_widths.push(max_w.max(COLUMN_WIDTH));
            col_card_heights.push(card_heights);
        }

        let max_col_height: f64 = board.columns.iter().enumerate().map(|(ci, col)| {
            let cards_h: f64 = col_card_heights[ci].iter().sum::<f64>() + col.cards.len().saturating_sub(1) as f64 * CARD_GAP;
            HEADER_HEIGHT + CARD_GAP + cards_h + CARD_GAP
        }).fold(0.0f64, f64::max);

        Self { col_widths, col_card_heights, max_col_height, line_height }
    }
}

pub fn to_scene_themed(board: &KanbanBoard, theme: &Theme) -> Scene {
    if board.columns.is_empty() {
        return Scene::new(100.0, 50.0);
    }

    let metrics = ColumnMetrics::from_board(board, theme);

    let total_w: f64 = metrics.col_widths.iter().sum::<f64>() + (board.columns.len() - 1) as f64 * COLUMN_GAP + MARGIN * 2.0;
    let total_h = metrics.max_col_height + MARGIN * 2.0;
    let mut scene = Scene::new(total_w, total_h);

    render_columns(&mut scene, board, &metrics, theme);

    scene
}

fn render_columns(scene: &mut Scene, board: &KanbanBoard, metrics: &ColumnMetrics, theme: &Theme) {
    let mut x = MARGIN;
    for (ci, col) in board.columns.iter().enumerate() {
        let col_w = metrics.col_widths[ci];
        let color = COLUMN_COLORS[ci % COLUMN_COLORS.len()];
        let col_cx = x + col_w / 2.0;

        render_column_bg(scene, col_cx, col_w, metrics.max_col_height, color);
        render_column_header(scene, col, col_cx, col_w, color, theme);
        render_cards(scene, col, ci, col_cx, col_w, color, metrics, theme);

        x += col_w + COLUMN_GAP;
    }
}

fn render_column_bg(scene: &mut Scene, col_cx: f64, col_w: f64, max_col_height: f64, color: Color) {
    scene.push(Primitive::Rect {
        bbox: BBox::new(col_cx, MARGIN + max_col_height / 2.0, col_w, max_col_height),
        rx: 8.0, ry: 8.0,
        style: Style {
            fill: Some(Color::rgba(color.r, color.g, color.b, 20)),
            stroke: Some(Color::rgba(color.r, color.g, color.b, 60)),
            stroke_width: Some(1.0),
            ..Default::default()
        },
    });
}

fn render_column_header(scene: &mut Scene, col: &ir::KanbanColumn, col_cx: f64, col_w: f64, color: Color, theme: &Theme) {
    scene.push(Primitive::Rect {
        bbox: BBox::new(col_cx, MARGIN + HEADER_HEIGHT / 2.0, col_w, HEADER_HEIGHT),
        rx: 8.0, ry: 8.0,
        style: Style { fill: Some(Color::rgba(color.r, color.g, color.b, 50)), ..Default::default() },
    });
    scene.push(Primitive::Text {
        position: rusty_mermaid_core::Point::new(col_cx, MARGIN + HEADER_HEIGHT / 2.0),
        content: col.label.clone(),
        anchor: TextAnchor::Middle,
        style: TextStyle {
            font_size: theme.font_size_node,
            fill: Some(color),
            font_weight: rusty_mermaid_core::FontWeight::Bold,
            ..Default::default()
        },
    });
}

fn render_cards(scene: &mut Scene, col: &ir::KanbanColumn, ci: usize, col_cx: f64, col_w: f64, color: Color, metrics: &ColumnMetrics, theme: &Theme) {
    let mut cy = MARGIN + HEADER_HEIGHT + CARD_GAP;
    for (cardi, card) in col.cards.iter().enumerate() {
        let card_h = metrics.col_card_heights[ci][cardi];
        let card_cy = cy + card_h / 2.0;

        scene.push(Primitive::Rect {
            bbox: BBox::new(col_cx, card_cy, col_w - CARD_PADDING * 2.0, card_h),
            rx: CARD_RX, ry: CARD_RX,
            style: Style {
                fill: Some(theme.background),
                stroke: Some(Color::rgba(color.r, color.g, color.b, 80)),
                stroke_width: Some(1.0),
                ..Default::default()
            },
        });

        let label_y = if card.priority.is_some() || card.assigned.is_some() || card.ticket.is_some() {
            card_cy - metrics.line_height * 0.3
        } else {
            card_cy
        };
        scene.push(Primitive::Text {
            position: rusty_mermaid_core::Point::new(col_cx, label_y),
            content: card.label.clone(),
            anchor: TextAnchor::Middle,
            style: TextStyle {
                font_size: theme.font_size_edge_label,
                fill: Some(theme.node_text),
                ..Default::default()
            },
        });

        let mut meta_parts: Vec<String> = Vec::new();
        if let Some(p) = card.priority {
            meta_parts.push(p.label().to_string());
        }
        if let Some(a) = &card.assigned {
            meta_parts.push(format!("@{a}"));
        }
        if let Some(t) = &card.ticket {
            meta_parts.push(t.clone());
        }
        if !meta_parts.is_empty() {
            scene.push(Primitive::Text {
                position: rusty_mermaid_core::Point::new(col_cx, card_cy + metrics.line_height * 0.5),
                content: meta_parts.join(" · "),
                anchor: TextAnchor::Middle,
                style: TextStyle {
                    font_size: theme.font_size_small,
                    fill: Some(Color::rgb(140, 140, 140)),
                    ..Default::default()
                },
            });
        }

        cy += card_h + CARD_GAP;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(input: &str) -> Scene {
        let b = parser::parse(input).unwrap();
        to_scene(&b)
    }

    #[test]
    fn scene_has_primitives() {
        let scene = render("kanban\n    Todo\n        task1[Buy milk]\n    Done\n        task2[Laundry]");
        assert!(scene.len() >= 8, "2 columns (bg + header + label) + 2 cards");
    }

    #[test]
    fn column_count_matches() {
        let scene = render("kanban\n    A\n    B\n    C");
        // 3 columns × (bg rect + header rect + label text) = 9 minimum
        let rects: Vec<_> = scene.elements().iter().filter(|e| matches!(&e.primitive, Primitive::Rect { .. })).collect();
        assert!(rects.len() >= 6, "3 columns × 2 rects each");
    }

    #[test]
    fn cards_rendered() {
        let scene = render("kanban\n    Col\n        a[Card A]\n        b[Card B]\n        c[Card C]");
        let texts: Vec<_> = scene.elements().iter().filter_map(|e| {
            if let Primitive::Text { content, .. } = &e.primitive { Some(content.as_str()) } else { None }
        }).collect();
        assert!(texts.contains(&"Card A"));
        assert!(texts.contains(&"Card B"));
        assert!(texts.contains(&"Card C"));
    }

    #[test]
    fn metadata_rendered() {
        let scene = render("kanban\n    Col\n        t[Task] @{priority: high, assigned: alice}");
        let texts: Vec<_> = scene.elements().iter().filter_map(|e| {
            if let Primitive::Text { content, .. } = &e.primitive { Some(content.clone()) } else { None }
        }).collect();
        assert!(texts.iter().any(|t| t.contains("High") && t.contains("@alice")));
    }
}
