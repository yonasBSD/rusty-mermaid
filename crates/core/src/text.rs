use crate::TextStyle;

/// Measure text dimensions for layout purposes.
/// Generic parameter on dagre::layout — no vtable overhead.
pub trait TextMeasure {
    fn measure(&self, text: &str, style: &TextStyle) -> (f64, f64);
}

/// Monospace character width at the reference font size (14px).
/// Measured from Intel One Mono Regular: 8.596px at 14px.
/// Rounded up to ensure node boxes never undersize the text.
const MONOSPACE_CHAR_WIDTH_14PX: f64 = 8.6;
const MONOSPACE_LINE_HEIGHT_14PX: f64 = 16.8;

/// Monospace text measurer using fixed character advance width.
/// Uses accurate metrics for the default monospace font stack.
pub struct SimpleTextMeasure {
    pub avg_char_width: f64,
    pub line_height: f64,
}

impl Default for SimpleTextMeasure {
    fn default() -> Self {
        Self {
            avg_char_width: MONOSPACE_CHAR_WIDTH_14PX,
            line_height: MONOSPACE_LINE_HEIGHT_14PX,
        }
    }
}

impl SimpleTextMeasure {
    pub fn new(avg_char_width: f64, line_height: f64) -> Self {
        debug_assert!(avg_char_width > 0.0, "char width must be positive");
        debug_assert!(line_height > 0.0, "line height must be positive");
        Self {
            avg_char_width,
            line_height,
        }
    }
}

use crate::font_fallback::{FontSlot, font_for_char};

/// Width multiplier per FontSlot relative to Intel One Mono's advance.
/// These ratios account for the actual advance widths of each fallback font.
pub const fn char_width_ratio(ch: char) -> f64 {
    match font_for_char(ch) {
        FontSlot::Primary => 1.0,       // Intel One Mono: 0.6em monospace
        FontSlot::ExtendedText => 0.85,  // Noto Sans: proportional, narrower than mono
        FontSlot::Monospace => 1.0,      // Noto Sans Mono: same width as primary
        FontSlot::Dingbats => 1.4,       // Noto Sans Symbols 2: wider symbols
        FontSlot::Arabic => 0.8,         // Noto Sans Arabic: proportional, varies
        FontSlot::Cjk => 1.8,           // CJK: wide but proportional
        FontSlot::Emoji => 2.0,          // Color emoji: ~2x Latin mono width
    }
}

impl TextMeasure for SimpleTextMeasure {
    fn measure(&self, text: &str, style: &TextStyle) -> (f64, f64) {
        let scale = style.font_size / 14.0;
        let stripped = strip_markup(text);
        let mut max_width: f64 = 0.0;
        let mut line_count: usize = 0;
        for line in stripped.split('\n') {
            line_count += 1;
            let w: f64 = line
                .chars()
                .map(|c| char_width_ratio(c))
                .sum();
            max_width = max_width.max(w);
        }

        let width = max_width * self.avg_char_width * scale;
        let height = line_count as f64 * self.line_height * scale;
        (width, height)
    }
}

/// Compute the baseline Y for vertically centered text.
///
/// Given a target center Y position, returns the Y coordinate for the
/// **first line's baseline** so that the text block is visually centered.
///
/// SVG does this with `dominant-baseline: central`. Non-SVG backends
/// (raster, gpui, vello) call this to compute the baseline position.
///
/// Usage: `baseline_y = center_y + text_baseline_y_offset(font_size, line_count)`
///
/// Based on Intel One Mono metrics: ascent ≈ 0.8em, descent ≈ 0.2em.
/// Visual center above baseline = (ascent - |descent|) / 2 ≈ 0.3em.
pub fn text_baseline_y_offset(font_size: f64, line_count: usize) -> f64 {
    // For a single line, baseline should be below center by 0.3 * font_size
    // because glyphs extend more above baseline (ascent) than below (descent).
    let baseline_from_center = font_size * 0.3;
    // For multi-line, shift up by half the total block height (from first to last baseline).
    let line_height = font_size * 1.2;
    let block_offset = (line_count as f64 - 1.0) * line_height / 2.0;
    baseline_from_center - block_offset
}

/// A text span with inline markdown formatting.
#[derive(Debug, Clone, PartialEq)]
pub struct MdSpan {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
}

/// Parse inline markdown (`**bold**`, `*italic*`, `***both***`) into styled spans.
/// Returns `None` if the text contains no formatting markers.
pub fn parse_inline_markdown(text: &str) -> Option<Vec<MdSpan>> {
    if !text.contains('*') {
        return None;
    }

    let mut spans = Vec::new();
    let mut bold = false;
    let mut italic = false;
    let mut buf = String::new();
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '*' && chars.peek() == Some(&'*') {
            chars.next();
            if !buf.is_empty() {
                spans.push(MdSpan { text: std::mem::take(&mut buf), bold, italic });
            }
            bold = !bold;
        } else if c == '*' {
            if !buf.is_empty() {
                spans.push(MdSpan { text: std::mem::take(&mut buf), bold, italic });
            }
            italic = !italic;
        } else {
            buf.push(c);
        }
    }
    if !buf.is_empty() {
        spans.push(MdSpan { text: buf, bold, italic });
    }

    if spans.iter().any(|s| s.bold || s.italic) {
        Some(spans)
    } else {
        None
    }
}

/// Strip HTML tags and markdown markers from text in a single pass.
/// Replaces <br/> variants with newlines; removes `**` and `*` markers;
/// strips other HTML tags entirely.
fn strip_markup(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut in_tag = false;
    let mut tag_buf = String::with_capacity(8);
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if in_tag {
            if ch == '>' {
                in_tag = false;
                if tag_buf.eq_ignore_ascii_case("br")
                    || tag_buf.eq_ignore_ascii_case("br/")
                    || tag_buf.eq_ignore_ascii_case("br /")
                {
                    result.push('\n');
                }
            } else {
                tag_buf.push(ch);
            }
        } else if ch == '<' {
            in_tag = true;
            tag_buf.clear();
        } else if ch == '*' {
            // Skip markdown markers (* and **)
            if chars.peek() == Some(&'*') {
                chars.next();
            }
        } else {
            result.push(ch);
        }
    }
    result
}

/// Strip HTML tags from text, replacing <br/> variants with newlines.
#[cfg(test)]
fn strip_html_tags(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut in_tag = false;
    let mut tag_buf = String::with_capacity(8);

    for ch in text.chars() {
        match ch {
            '<' => {
                in_tag = true;
                tag_buf.clear();
            }
            '>' if in_tag => {
                in_tag = false;
                if tag_buf.eq_ignore_ascii_case("br")
                    || tag_buf.eq_ignore_ascii_case("br/")
                    || tag_buf.eq_ignore_ascii_case("br /")
                {
                    result.push('\n');
                }
            }
            _ if in_tag => tag_buf.push(ch),
            _ => result.push(ch),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    const W: f64 = MONOSPACE_CHAR_WIDTH_14PX;
    const LH: f64 = MONOSPACE_LINE_HEIGHT_14PX;

    fn default_style() -> TextStyle {
        TextStyle::default()
    }

    #[test]
    fn simple_measure_basic() {
        let m = SimpleTextMeasure::default();
        let (w, h) = m.measure("hello", &default_style());
        assert!((w - 5.0 * W).abs() < f64::EPSILON);
        assert!((h - LH).abs() < f64::EPSILON);
    }

    #[test]
    fn simple_measure_empty() {
        let m = SimpleTextMeasure::default();
        let (w, h) = m.measure("", &default_style());
        assert!((w - 0.0).abs() < f64::EPSILON);
        assert!((h - LH).abs() < f64::EPSILON);
    }

    #[test]
    fn simple_measure_strips_html() {
        let m = SimpleTextMeasure::default();
        let (w, _) = m.measure("<b>bold</b>", &default_style());
        assert!((w - 4.0 * W).abs() < f64::EPSILON);
    }

    #[test]
    fn simple_measure_br_adds_lines() {
        let m = SimpleTextMeasure::default();
        let (_, h) = m.measure("line1<br/>line2", &default_style());
        assert!((h - 2.0 * LH).abs() < f64::EPSILON);
    }

    #[test]
    fn simple_measure_font_size_scales() {
        let m = SimpleTextMeasure::default();
        let mut style = default_style();
        style.font_size = 28.0; // 2x default
        let (w, h) = m.measure("ab", &style);
        assert!((w - 2.0 * W * 2.0).abs() < f64::EPSILON);
        assert!((h - LH * 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn simple_measure_custom_char_width() {
        let m = SimpleTextMeasure::new(10.0, 20.0);
        let (w, h) = m.measure("abc", &default_style());
        assert!((w - 30.0).abs() < f64::EPSILON);
        assert!((h - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn measure_strips_markdown() {
        let m = SimpleTextMeasure::default();
        let (w_plain, _) = m.measure("bold", &default_style());
        let (w_md, _) = m.measure("**bold**", &default_style());
        assert!((w_plain - w_md).abs() < f64::EPSILON,
            "markdown markers should be stripped: plain={w_plain} md={w_md}");
    }

    #[test]
    fn strip_html_basic() {
        assert_eq!(strip_html_tags("<b>bold</b>"), "bold");
        assert_eq!(strip_html_tags("<i>italic</i>"), "italic");
        assert_eq!(strip_html_tags("no tags"), "no tags");
    }

    #[test]
    fn strip_html_br_to_newline() {
        assert_eq!(strip_html_tags("a<br/>b"), "a\nb");
        assert_eq!(strip_html_tags("a<br>b"), "a\nb");
        assert_eq!(strip_html_tags("a<br />b"), "a\nb");
    }

    #[test]
    fn strip_html_nested() {
        assert_eq!(strip_html_tags("<b><i>text</i></b>"), "text");
    }

    #[test]
    fn default_trait() {
        let m = SimpleTextMeasure::default();
        assert!((m.avg_char_width - W).abs() < f64::EPSILON);
        assert!((m.line_height - LH).abs() < f64::EPSILON);
    }

    #[test]
    fn cjk_chars_wider_than_latin() {
        let m = SimpleTextMeasure::default();
        let (w, _) = m.measure("你好世界", &default_style());
        assert!((w - 4.0 * 1.8 * W).abs() < 1e-10);
    }

    #[test]
    fn japanese_kana_wider_than_latin() {
        let m = SimpleTextMeasure::default();
        let (w, _) = m.measure("こんにちは世界", &default_style());
        assert!((w - 7.0 * 1.8 * W).abs() < 1e-10);
    }

    #[test]
    fn mixed_latin_cjk() {
        let m = SimpleTextMeasure::default();
        let (w, _) = m.measure("Hi你好", &default_style());
        assert!((w - (2.0 + 2.0 * 1.8) * W).abs() < 1e-10);
    }

    #[test]
    fn latin_and_cyrillic_widths() {
        let m = SimpleTextMeasure::default();
        let (w_latin, _) = m.measure("hello", &default_style());
        let (w_cyrillic, _) = m.measure("приве", &default_style());
        assert!((w_latin - 5.0 * W).abs() < 1e-10);
        assert!((w_cyrillic - 5.0 * 0.85 * W).abs() < 1e-10);
    }

    #[test]
    fn char_width_ratios() {
        assert!((char_width_ratio('A') - 1.0).abs() < f64::EPSILON);   // Primary
        assert!((char_width_ratio('你') - 1.8).abs() < f64::EPSILON);  // CJK
        assert!((char_width_ratio('α') - 0.85).abs() < f64::EPSILON);  // ExtendedText
        assert!((char_width_ratio('★') - 1.4).abs() < f64::EPSILON);   // Dingbats
        assert!((char_width_ratio('→') - 1.0).abs() < f64::EPSILON);   // Monospace
        assert!((char_width_ratio('م') - 0.8).abs() < f64::EPSILON);   // Arabic
    }
}
