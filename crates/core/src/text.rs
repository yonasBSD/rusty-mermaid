use crate::TextStyle;

/// Measure text dimensions for layout purposes.
/// Generic parameter on dagre::layout — no vtable overhead.
pub trait TextMeasure {
    fn measure(&self, text: &str, style: &TextStyle) -> (f64, f64);
}

/// Simple character-counting text measurer.
/// Good enough for tests and when no real font metrics are available.
pub struct SimpleTextMeasure {
    pub avg_char_width: f64,
    pub line_height: f64,
}

impl Default for SimpleTextMeasure {
    fn default() -> Self {
        Self {
            avg_char_width: 8.0,
            line_height: 16.0,
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
        let stripped = strip_html_tags(text);
        let lines: Vec<&str> = stripped.split('\n').collect();
        let max_chars = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);

        let width = max_chars as f64 * self.avg_char_width * scale;
        let height = lines.len() as f64 * self.line_height * scale;
        (width, height)
    }
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
        assert!((w - 40.0).abs() < f64::EPSILON); // 5 chars * 8.0
        assert!((h - 16.0).abs() < f64::EPSILON); // 1 line * 16.0
    }

    #[test]
    fn simple_measure_empty() {
        let m = SimpleTextMeasure::default();
        let (w, h) = m.measure("", &default_style());
        assert!((w - 0.0).abs() < f64::EPSILON);
        assert!((h - 16.0).abs() < f64::EPSILON); // 1 line minimum
    }

    #[test]
    fn simple_measure_strips_html() {
        let m = SimpleTextMeasure::default();
        let (w, _) = m.measure("<b>bold</b>", &default_style());
        // "bold" = 4 chars
        assert!((w - 32.0).abs() < f64::EPSILON);
    }

    #[test]
    fn simple_measure_br_adds_lines() {
        let m = SimpleTextMeasure::default();
        let (_, h) = m.measure("line1<br/>line2", &default_style());
        assert!((h - 32.0).abs() < f64::EPSILON); // 2 lines
    }

    #[test]
    fn simple_measure_font_size_scales() {
        let m = SimpleTextMeasure::default();
        let mut style = default_style();
        style.font_size = 28.0; // 2x default
        let (w, h) = m.measure("ab", &style);
        assert!((w - 32.0).abs() < f64::EPSILON); // 2 * 8.0 * 2.0
        assert!((h - 32.0).abs() < f64::EPSILON); // 1 * 16.0 * 2.0
    }

    #[test]
    fn simple_measure_custom_char_width() {
        let m = SimpleTextMeasure::new(10.0, 20.0);
        let (w, h) = m.measure("abc", &default_style());
        assert!((w - 30.0).abs() < f64::EPSILON);
        assert!((h - 20.0).abs() < f64::EPSILON);
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
        assert!((m.avg_char_width - 8.0).abs() < f64::EPSILON);
        assert!((m.line_height - 16.0).abs() < f64::EPSILON);
    }
}
