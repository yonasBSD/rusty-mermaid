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

impl TextMeasure for SimpleTextMeasure {
    fn measure(&self, text: &str, style: &TextStyle) -> (f64, f64) {
        let scale = style.font_size / 14.0;
        let stripped = strip_markup(text);
        let lines: Vec<&str> = stripped.split('\n').collect();
        let max_chars = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);

        let width = max_chars as f64 * self.avg_char_width * scale;
        let height = lines.len() as f64 * self.line_height * scale;
        (width, height)
    }
}

/// Strip HTML tags and markdown markers from text for measurement.
/// Replaces <br/> variants with newlines; removes `**` and `*` markers.
fn strip_markup(text: &str) -> String {
    let html_stripped = strip_html_tags(text);
    strip_markdown_markers(&html_stripped)
}

/// Strip HTML tags from text, replacing <br/> variants with newlines.
fn strip_html_tags(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut in_tag = false;
    let mut tag_buf = String::new();

    for ch in text.chars() {
        match ch {
            '<' => {
                in_tag = true;
                tag_buf.clear();
            }
            '>' if in_tag => {
                in_tag = false;
                let tag = tag_buf.to_lowercase();
                if tag == "br/" || tag == "br" || tag == "br /" {
                    result.push('\n');
                }
                // Other tags (b, i, code, etc.) are simply stripped
            }
            _ if in_tag => tag_buf.push(ch),
            _ => result.push(ch),
        }
    }
    result
}

/// Strip markdown bold/italic markers (`**`, `*`) from text.
fn strip_markdown_markers(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' {
            i += 2;
        } else if chars[i] == '*' {
            i += 1;
        } else {
            result.push(chars[i]);
            i += 1;
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
}
