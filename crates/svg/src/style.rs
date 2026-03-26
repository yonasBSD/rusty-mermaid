use rusty_mermaid_core::{FontWeight, Style, TextStyle};

use crate::document::{fmt_f64, write_f64};

/// Append style attributes to an existing attribute list.
pub fn push_style_attrs(attrs: &mut Vec<(String, String)>, style: &Style) {
    if let Some(fill) = &style.fill {
        if fill.a == 0 {
            attrs.push(("fill".into(), "none".into()));
        } else {
            attrs.push(("fill".into(), fill.to_string()));
        }
    }
    if let Some(stroke) = &style.stroke {
        attrs.push(("stroke".into(), stroke.to_string()));
    }
    if let Some(sw) = style.stroke_width {
        attrs.push(("stroke-width".into(), fmt_f64(sw)));
    }
    if let Some(da) = &style.stroke_dasharray {
        let mut dash = String::with_capacity(da.len() * 6);
        for (i, v) in da.iter().enumerate() {
            if i > 0 {
                dash.push(' ');
            }
            write_f64(&mut dash, *v);
        }
        attrs.push(("stroke-dasharray".into(), dash));
    }
    if let Some(op) = style.opacity {
        attrs.push(("opacity".into(), fmt_f64(op)));
    }
    if !style.css_classes.is_empty() {
        attrs.push(("class".into(), style.css_classes.join(" ")));
    }
}

/// Append text style attributes to an existing attribute list.
pub fn push_text_style_attrs(attrs: &mut Vec<(String, String)>, style: &TextStyle) {
    let mut font_size = String::with_capacity(8);
    write_f64(&mut font_size, style.font_size);
    font_size.push_str("px");
    attrs.push(("font-size".into(), font_size));
    attrs.push(("font-family".into(), style.font_family.clone()));

    if let Some(fill) = &style.fill {
        attrs.push(("fill".into(), fill.to_string()));
    }
    if style.font_weight == FontWeight::Bold {
        attrs.push(("font-weight".into(), "bold".into()));
    }
}

#[cfg(test)]
mod tests {
    use rusty_mermaid_core::Color;

    use super::*;

    fn style_attrs(style: &Style) -> Vec<(String, String)> {
        let mut attrs = Vec::new();
        push_style_attrs(&mut attrs, style);
        attrs
    }

    fn text_style_attrs(style: &TextStyle) -> Vec<(String, String)> {
        let mut attrs = Vec::new();
        push_text_style_attrs(&mut attrs, style);
        attrs
    }

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
        assert!(
            attrs
                .iter()
                .any(|(k, v)| k == "stroke-dasharray" && v == "5 3")
        );
    }

    #[test]
    fn css_classes() {
        let s = Style {
            css_classes: vec!["node".into(), "active".into()],
            ..Default::default()
        };
        let attrs = style_attrs(&s);
        assert!(
            attrs
                .iter()
                .any(|(k, v)| k == "class" && v == "node active")
        );
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
}
