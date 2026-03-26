//! Shared color palette and tint helpers used across diagram types.

use rusty_mermaid_core::Color;

/// 8-color palette shared across all diagram types.
pub const PALETTE: [Color; 8] = [
    Color::rgb(78, 121, 167),
    Color::rgb(242, 142, 44),
    Color::rgb(225, 87, 89),
    Color::rgb(118, 183, 178),
    Color::rgb(89, 161, 79),
    Color::rgb(237, 201, 73),
    Color::rgb(175, 122, 161),
    Color::rgb(255, 157, 167),
];

/// Pick a color from the palette by index (wraps around).
pub fn palette_color(idx: usize) -> Color {
    PALETTE[idx % PALETTE.len()]
}

/// Blend a color with white at the given tint ratio (0.0 = white, 1.0 = full color).
/// Used for the translucent/glassy fill effect across all diagrams.
pub fn tint_color(color: Color, tint: f64) -> Color {
    Color::rgb(
        (255.0 * (1.0 - tint) + color.r as f64 * tint) as u8,
        (255.0 * (1.0 - tint) + color.g as f64 * tint) as u8,
        (255.0 * (1.0 - tint) + color.b as f64 * tint) as u8,
    )
}

/// Default tint ratio for glassy fills.
pub const DEFAULT_TINT: f64 = 0.12;

/// Standard border radius for rounded rectangles.
pub const BORDER_RADIUS: f64 = 4.0;

/// Standard stroke width for element borders.
pub const STROKE_WIDTH: f64 = 1.5;

/// Standard stroke width for thin lines (grid, dividers).
pub const STROKE_WIDTH_THIN: f64 = 1.0;

/// Standard dash pattern for boundaries/containers.
pub const DASH_PATTERN: [f64; 2] = [7.0, 5.0];

/// Max score for journey diagram face emojis.
pub const MAX_SCORE: f64 = 5.0;

/// Database width ratio relative to element width (cylinder shape).
pub const DATABASE_WIDTH_RATIO: f64 = 0.7;

/// Standard label padding around edge/node text.
pub const LABEL_PAD: f64 = 4.0;

/// Standard edge label font size.
pub const EDGE_LABEL_FONT: f64 = 12.0;

/// Dotted line dash pattern (shorter than boundary DASH_PATTERN).
pub const DOTTED_PATTERN: [f64; 2] = [6.0, 4.0];

/// Marker inset — pull arrow endpoint back to avoid piercing.
pub const MARKER_INSET: f64 = 3.0;

/// Standard title extra height when present.
pub const TITLE_HEIGHT: f64 = 36.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_wraps() {
        assert_eq!(palette_color(0), palette_color(8));
        assert_eq!(palette_color(1), palette_color(9));
    }

    #[test]
    fn tint_zero_is_white() {
        let c = tint_color(Color::rgb(100, 0, 0), 0.0);
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 255);
        assert_eq!(c.b, 255);
    }

    #[test]
    fn tint_one_is_original() {
        let c = tint_color(Color::rgb(100, 50, 200), 1.0);
        assert_eq!(c.r, 100);
        assert_eq!(c.g, 50);
        assert_eq!(c.b, 200);
    }

    #[test]
    fn default_tint_is_light() {
        let c = tint_color(Color::rgb(78, 121, 167), DEFAULT_TINT);
        assert!(c.r > 200, "should be mostly white");
        assert!(c.g > 200);
        assert!(c.b > 200);
    }
}
