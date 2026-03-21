use crate::TextStyle;

/// Measure text dimensions for layout purposes.
/// Generic parameter on dagre::layout — no vtable overhead.
pub trait TextMeasure {
    fn measure(&self, text: &str, style: &TextStyle) -> (f64, f64);
}

/// Monospace character width at the reference font size (14px).
/// Derived from typical monospace advance width ratio: 0.6 * em.
/// Applies to Intel One Mono, Cascadia Code, JetBrains Mono, Fira Code, etc.
const MONOSPACE_CHAR_WIDTH_14PX: f64 = 8.4;
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

/// CJK glyph width relative to Latin in monospace fonts.
/// Terminal convention is 2.0, but actual monospace fonts (Intel One Mono,
/// JetBrains Mono, etc.) render CJK at roughly 1.0em vs Latin's 0.6em,
/// giving a ratio of ~1.67. We use 1.7 for comfortable padding.
const CJK_WIDTH_RATIO: f64 = 1.7;

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
                .map(|c| if is_wide_char(c) { CJK_WIDTH_RATIO } else { 1.0 })
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

/// Check if a character is East Asian Wide or Fullwidth per UAX #11.
/// Covers the most common ranges; not exhaustive but sufficient for
/// CJK, Japanese kana, Korean, and fullwidth Latin/symbols.
fn is_wide_char(c: char) -> bool {
    let cp = c as u32;
    matches!(cp,
        // CJK Unified Ideographs
        0x4E00..=0x9FFF |
        // CJK Extension A
        0x3400..=0x4DBF |
        // CJK Extension B+
        0x20000..=0x2A6DF |
        // CJK Compatibility Ideographs
        0xF900..=0xFAFF |
        // Hiragana
        0x3040..=0x309F |
        // Katakana
        0x30A0..=0x30FF |
        // CJK Symbols and Punctuation
        0x3000..=0x303F |
        // Hangul Syllables
        0xAC00..=0xD7AF |
        // Hangul Jamo
        0x1100..=0x11FF |
        // Hangul Compatibility Jamo
        0x3130..=0x318F |
        // Fullwidth Forms (fullwidth Latin, etc.)
        0xFF01..=0xFF60 |
        // Halfwidth CJK punctuation (these are actually narrow, skip)
        // Enclosed CJK Letters
        0x3200..=0x32FF |
        // CJK Compatibility
        0x3300..=0x33FF |
        // Katakana Phonetic Extensions
        0x31F0..=0x31FF |
        // Bopomofo
        0x3100..=0x312F |
        // CJK Radicals Supplement
        0x2E80..=0x2EFF |
        // Kangxi Radicals
        0x2F00..=0x2FDF
    )
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
    fn default_style() -> TextStyle {
        TextStyle::default()
    }

    #[test]
    fn simple_measure_basic() {
        let m = SimpleTextMeasure::default();
        let (w, h) = m.measure("hello", &default_style());
        assert!((w - 42.0).abs() < f64::EPSILON); // 5 chars * 8.4
        assert!((h - 16.8).abs() < f64::EPSILON); // 1 line * 16.8
    }

    #[test]
    fn simple_measure_empty() {
        let m = SimpleTextMeasure::default();
        let (w, h) = m.measure("", &default_style());
        assert!((w - 0.0).abs() < f64::EPSILON);
        assert!((h - 16.8).abs() < f64::EPSILON); // 1 line minimum
    }

    #[test]
    fn simple_measure_strips_html() {
        let m = SimpleTextMeasure::default();
        let (w, _) = m.measure("<b>bold</b>", &default_style());
        // "bold" = 4 chars * 8.4
        assert!((w - 33.6).abs() < f64::EPSILON);
    }

    #[test]
    fn simple_measure_br_adds_lines() {
        let m = SimpleTextMeasure::default();
        let (_, h) = m.measure("line1<br/>line2", &default_style());
        assert!((h - 33.6).abs() < f64::EPSILON); // 2 lines * 16.8
    }

    #[test]
    fn simple_measure_font_size_scales() {
        let m = SimpleTextMeasure::default();
        let mut style = default_style();
        style.font_size = 28.0; // 2x default
        let (w, h) = m.measure("ab", &style);
        assert!((w - 33.6).abs() < f64::EPSILON); // 2 * 8.4 * 2.0
        assert!((h - 33.6).abs() < f64::EPSILON); // 1 * 16.8 * 2.0
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
        assert!((m.avg_char_width - 8.4).abs() < f64::EPSILON);
        assert!((m.line_height - 16.8).abs() < f64::EPSILON);
    }

    #[test]
    fn cjk_chars_wider_than_latin() {
        let m = SimpleTextMeasure::default();
        // "你好世界" = 4 CJK chars × 1.7 × 8.4 = 57.12
        let (w, _) = m.measure("你好世界", &default_style());
        assert!((w - 4.0 * 1.7 * 8.4).abs() < 1e-10);
    }

    #[test]
    fn japanese_kana_wider_than_latin() {
        let m = SimpleTextMeasure::default();
        // "こんにちは世界" = 7 wide chars × 1.7 × 8.4
        let (w, _) = m.measure("こんにちは世界", &default_style());
        assert!((w - 7.0 * 1.7 * 8.4).abs() < 1e-10);
    }

    #[test]
    fn mixed_latin_cjk() {
        let m = SimpleTextMeasure::default();
        // "Hi你好" = 2×1.0 + 2×1.7 = 5.4 units × 8.4 = 45.36
        let (w, _) = m.measure("Hi你好", &default_style());
        assert!((w - (2.0 + 2.0 * 1.7) * 8.4).abs() < 1e-10);
    }

    #[test]
    fn latin_and_cyrillic_are_narrow() {
        let m = SimpleTextMeasure::default();
        let (w_latin, _) = m.measure("hello", &default_style());
        let (w_cyrillic, _) = m.measure("приве", &default_style());
        // Both 5 chars × 1.0 × 8.4 = 42.0
        assert!((w_latin - 42.0).abs() < 1e-10);
        assert!((w_cyrillic - 42.0).abs() < 1e-10);
    }

    #[test]
    fn wide_char_detection() {
        assert!(is_wide_char('你'));  // CJK ideograph
        assert!(is_wide_char('こ'));  // Hiragana
        assert!(is_wide_char('ア'));  // Katakana
        assert!(is_wide_char('한'));  // Hangul
        assert!(!is_wide_char('A'));  // Latin
        assert!(!is_wide_char('Я'));  // Cyrillic
        assert!(!is_wide_char('م'));  // Arabic
    }
}
