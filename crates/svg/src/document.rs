use std::fmt::Write;

/// Minimal XML/SVG builder. Accumulates elements into a string.
pub struct SvgDocument {
    buf: String,
    indent: usize,
}

impl SvgDocument {
    pub fn new(width: f64, height: f64) -> Self {
        let mut doc = Self {
            buf: String::with_capacity(4096),
            indent: 0,
        };
        let _ = writeln!(
            doc.buf,
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}">"#,
            w = fmt_f64(width),
            h = fmt_f64(height),
        );
        doc.indent = 1;
        doc
    }

    pub fn open_tag(&mut self, tag: &str, attrs: &[(&str, &str)]) {
        self.write_indent();
        let _ = write!(self.buf, "<{tag}");
        for (k, v) in attrs {
            let _ = write!(self.buf, r#" {k}="{v}""#);
        }
        let _ = writeln!(self.buf, ">");
        self.indent += 1;
    }

    pub fn close_tag(&mut self, tag: &str) {
        self.indent = self.indent.saturating_sub(1);
        self.write_indent();
        let _ = writeln!(self.buf, "</{tag}>");
    }

    pub fn empty_tag(&mut self, tag: &str, attrs: &[(&str, &str)]) {
        self.write_indent();
        let _ = write!(self.buf, "<{tag}");
        for (k, v) in attrs {
            let _ = write!(self.buf, r#" {k}="{v}""#);
        }
        let _ = writeln!(self.buf, " />");
    }

    pub fn text_element(&mut self, tag: &str, attrs: &[(&str, &str)], content: &str) {
        self.write_indent();
        let _ = write!(self.buf, "<{tag}");
        for (k, v) in attrs {
            let _ = write!(self.buf, r#" {k}="{v}""#);
        }
        let _ = writeln!(self.buf, ">{content}</{tag}>");
    }

    /// Write raw content (e.g. a <defs> block).
    pub fn raw(&mut self, content: &str) {
        for line in content.lines() {
            self.write_indent();
            let _ = writeln!(self.buf, "{line}");
        }
    }

    pub fn finish(mut self) -> String {
        self.indent = 0;
        let _ = writeln!(self.buf, "</svg>");
        self.buf
    }

    fn write_indent(&mut self) {
        for _ in 0..self.indent {
            self.buf.push_str("  ");
        }
    }
}

/// Format f64 without trailing zeros: `10.0` → `"10"`, `10.5` → `"10.5"`.
pub fn fmt_f64(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{}", v as i64)
    } else {
        // Up to 2 decimal places, strip trailing zeros
        let s = format!("{:.2}", v);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_svg() {
        let doc = SvgDocument::new(100.0, 50.0);
        let svg = doc.finish();
        assert!(svg.contains(r#"viewBox="0 0 100 50""#));
        assert!(svg.contains(r#"width="100""#));
        assert!(svg.starts_with("<svg"));
        assert!(svg.trim_end().ends_with("</svg>"));
    }

    #[test]
    fn open_close_tag() {
        let mut doc = SvgDocument::new(100.0, 100.0);
        doc.open_tag("g", &[("class", "nodes")]);
        doc.close_tag("g");
        let svg = doc.finish();
        assert!(svg.contains(r#"<g class="nodes">"#));
        assert!(svg.contains("</g>"));
    }

    #[test]
    fn empty_tag() {
        let mut doc = SvgDocument::new(100.0, 100.0);
        doc.empty_tag("rect", &[("x", "10"), ("y", "20"), ("width", "30"), ("height", "40")]);
        let svg = doc.finish();
        assert!(svg.contains(r#"<rect x="10" y="20" width="30" height="40" />"#));
    }

    #[test]
    fn text_element() {
        let mut doc = SvgDocument::new(100.0, 100.0);
        doc.text_element("text", &[("x", "50"), ("y", "50")], "Hello");
        let svg = doc.finish();
        assert!(svg.contains(r#"<text x="50" y="50">Hello</text>"#));
    }

    #[test]
    fn fmt_f64_integer() {
        assert_eq!(fmt_f64(10.0), "10");
        assert_eq!(fmt_f64(0.0), "0");
        assert_eq!(fmt_f64(-5.0), "-5");
    }

    #[test]
    fn fmt_f64_decimal() {
        assert_eq!(fmt_f64(10.5), "10.5");
        assert_eq!(fmt_f64(10.25), "10.25");
        assert_eq!(fmt_f64(10.10), "10.1");
    }
}
