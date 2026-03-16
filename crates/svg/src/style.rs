use rusty_mermaid_core::{FontWeight, Style, TextStyle};

use crate::document::fmt_f64;

/// Convert a Style to a list of SVG attribute key-value pairs.
pub fn style_attrs(style: &Style) -> Vec<(String, String)> {
    let mut attrs = Vec::new();

    if let Some(fill) = &style.fill {
        attrs.push(("fill".into(), fill.to_string()));
    }
    if let Some(stroke) = &style.stroke {
        attrs.push(("stroke".into(), stroke.to_string()));
    }
    if let Some(sw) = style.stroke_width {
        attrs.push(("stroke-width".into(), fmt_f64(sw)));
    }
    if let Some(da) = &style.stroke_dasharray {
        let s: Vec<String> = da.iter().map(|v| fmt_f64(*v)).collect();
        attrs.push(("stroke-dasharray".into(), s.join(" ")));
    }
    if let Some(op) = style.opacity {
        attrs.push(("opacity".into(), fmt_f64(op)));
    }
    if !style.css_classes.is_empty() {
        attrs.push(("class".into(), style.css_classes.join(" ")));
    }

    attrs
}

/// Convert a TextStyle to a list of SVG attribute key-value pairs.
pub fn text_style_attrs(style: &TextStyle) -> Vec<(String, String)> {
    let mut attrs = Vec::new();

    attrs.push(("font-size".into(), format!("{}px", fmt_f64(style.font_size))));
    attrs.push(("font-family".into(), style.font_family.clone()));

    if let Some(fill) = &style.fill {
        attrs.push(("fill".into(), fill.to_string()));
    }
    if style.font_weight == FontWeight::Bold {
        attrs.push(("font-weight".into(), "bold".into()));
    }

    attrs
}

/// Build a `style` attribute string from a Style (inline CSS).
pub fn style_to_inline_css(style: &Style) -> String {
    let mut parts = Vec::new();
    if let Some(fill) = &style.fill {
        parts.push(format!("fill:{fill}"));
    }
    if let Some(stroke) = &style.stroke {
        parts.push(format!("stroke:{stroke}"));
    }
    if let Some(sw) = style.stroke_width {
        parts.push(format!("stroke-width:{}", fmt_f64(sw)));
    }
    if let Some(da) = &style.stroke_dasharray {
        let s: Vec<String> = da.iter().map(|v| fmt_f64(*v)).collect();
        parts.push(format!("stroke-dasharray:{}", s.join(" ")));
    }
    if let Some(op) = style.opacity {
        parts.push(format!("opacity:{}", fmt_f64(op)));
    }
    parts.join(";")
}

#[cfg(test)]
mod tests {
    use rusty_mermaid_core::Color;

    use super::*;

    #[test]
    fn empty_style() {
        let attrs = style_attrs(&Style::default());
        assert!(attrs.is_empty());
    }

    #[test]
    fn fill_and_stroke() {
        let s = Style {
            fill: Some(Color::WHITE),
            stroke: Some(Color::BLACK),
            stroke_width: Some(2.0),
            ..Default::default()
        };
        let attrs = style_attrs(&s);
        assert!(attrs.iter().any(|(k, v)| k == "fill" && v == "#ffffff"));
        assert!(attrs.iter().any(|(k, v)| k == "stroke" && v == "#000000"));
        assert!(attrs.iter().any(|(k, v)| k == "stroke-width" && v == "2"));
    }

    #[test]
    fn dash_array() {
        let s = Style {
            stroke_dasharray: Some(vec![5.0, 3.0]),
            ..Default::default()
        };
        let attrs = style_attrs(&s);
        assert!(attrs.iter().any(|(k, v)| k == "stroke-dasharray" && v == "5 3"));
    }

    #[test]
    fn css_classes() {
        let s = Style {
            css_classes: vec!["node".into(), "active".into()],
            ..Default::default()
        };
        let attrs = style_attrs(&s);
        assert!(attrs.iter().any(|(k, v)| k == "class" && v == "node active"));
    }

    #[test]
    fn text_style_defaults() {
        let ts = TextStyle::default();
        let attrs = text_style_attrs(&ts);
        assert!(attrs.iter().any(|(k, v)| k == "font-size" && v == "14px"));
    }

    #[test]
    fn text_style_bold() {
        let ts = TextStyle {
            font_weight: FontWeight::Bold,
            ..Default::default()
        };
        let attrs = text_style_attrs(&ts);
        assert!(attrs.iter().any(|(k, v)| k == "font-weight" && v == "bold"));
    }

    #[test]
    fn inline_css() {
        let s = Style {
            fill: Some(Color::WHITE),
            stroke: Some(Color::BLACK),
            ..Default::default()
        };
        let css = style_to_inline_css(&s);
        assert!(css.contains("fill:#ffffff"));
        assert!(css.contains("stroke:#000000"));
    }
}
