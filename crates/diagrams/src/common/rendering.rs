use rusty_mermaid_core::{BBox, Color, Point, Primitive, Scene, Style, TextAnchor, TextStyle, Theme};

use super::styling::StyleProperty;

/// Default node style from theme.
pub fn node_style(theme: &Theme) -> Style {
    Style {
        fill: Some(theme.node_fill),
        stroke: Some(theme.node_stroke),
        stroke_width: Some(1.5),
        ..Default::default()
    }
}

/// Default node label style from theme.
pub fn label_style(theme: &Theme) -> TextStyle {
    TextStyle {
        fill: Some(theme.node_text),
        ..Default::default()
    }
}

/// Default edge label text style from theme.
pub fn edge_label_style(theme: &Theme) -> TextStyle {
    TextStyle {
        font_size: 12.0,
        fill: Some(theme.edge_label_text),
        ..Default::default()
    }
}

/// Default edge label background style from theme.
pub fn edge_label_bg_style(theme: &Theme) -> Style {
    Style {
        fill: Some(theme.edge_label_bg),
        stroke: Some(theme.edge_label_bg),
        stroke_width: Some(0.5),
        ..Default::default()
    }
}

/// Overlay a custom style onto a base style, field by field.
pub fn overlay_style(base: &mut Style, custom: &Style) {
    if custom.fill.is_some() { base.fill = custom.fill; }
    if custom.stroke.is_some() { base.stroke = custom.stroke; }
    if custom.stroke_width.is_some() { base.stroke_width = custom.stroke_width; }
    if custom.stroke_dasharray.is_some() { base.stroke_dasharray = custom.stroke_dasharray.clone(); }
    if custom.opacity.is_some() { base.opacity = custom.opacity; }
}

/// Merge a node's custom style onto the theme default.
pub fn merge_custom_style(custom: Option<&Style>, theme: &Theme) -> Style {
    let mut style = node_style(theme);
    if let Some(custom) = custom {
        overlay_style(&mut style, custom);
    }
    style
}

/// Apply parsed CSS-like style properties onto a Style.
pub fn apply_style_properties(style: &mut Style, props: &[StyleProperty]) {
    for prop in props {
        match prop.key.as_str() {
            "fill" => { style.fill = Color::from_css(&prop.value); }
            "stroke" => { style.stroke = Color::from_css(&prop.value); }
            "stroke-width" => {
                let v = prop.value.trim_end_matches("px");
                if let Ok(w) = v.parse::<f64>() {
                    style.stroke_width = Some(w);
                }
            }
            "stroke-dasharray" => {
                let vals: Vec<f64> = prop.value
                    .split_whitespace()
                    .flat_map(|s| s.split(','))
                    .filter_map(|s| s.trim().parse().ok())
                    .collect();
                if !vals.is_empty() {
                    style.stroke_dasharray = Some(vals);
                }
            }
            "opacity" => {
                if let Ok(o) = prop.value.parse::<f64>() {
                    style.opacity = Some(o);
                }
            }
            _ => {}
        }
    }
}

/// Render a background rect behind an edge label for readability.
pub fn render_edge_label_bg(
    scene: &mut Scene,
    center: Point,
    label_size: (f64, f64),
    theme: &Theme,
) {
    let pad = 4.0;
    scene.push(Primitive::Rect {
        bbox: BBox::new(center.x, center.y, label_size.0 + pad * 2.0, label_size.1 + pad * 2.0),
        rx: 2.0,
        ry: 2.0,
        style: edge_label_bg_style(theme),
    });
}

/// Pick a contrasting text color based on the node fill luminance.
pub fn contrasting_label_style(node_fill: Option<Color>, theme: &Theme) -> TextStyle {
    let mut lstyle = label_style(theme);
    if let Some(fill) = node_fill {
        let lum = fill.luminance();
        if lum < 0.4 {
            lstyle.fill = Some(Color::WHITE);
        } else if lum > 0.9 {
            lstyle.fill = Some(Color::BLACK);
        }
    }
    lstyle
}

/// Render an edge label with optional background rect.
pub fn render_edge_label(
    scene: &mut Scene,
    mid: Point,
    label: &str,
    label_size: Option<(f64, f64)>,
    theme: &Theme,
) {
    if let Some(size) = label_size {
        render_edge_label_bg(scene, mid, size, theme);
    }
    scene.push(Primitive::Text {
        position: mid,
        content: label.to_string(),
        anchor: TextAnchor::Middle,
        style: edge_label_style(theme),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_style_uses_theme() {
        let theme = Theme::light();
        let s = node_style(&theme);
        assert_eq!(s.fill, Some(theme.node_fill));
        assert_eq!(s.stroke, Some(theme.node_stroke));
        assert_eq!(s.stroke_width, Some(1.5));
    }

    #[test]
    fn overlay_style_replaces_set_fields() {
        let mut base = Style {
            fill: Some(Color::WHITE),
            stroke: Some(Color::BLACK),
            stroke_width: Some(1.0),
            ..Default::default()
        };
        let custom = Style {
            fill: Some(Color::rgb(255, 0, 0)),
            ..Default::default()
        };
        overlay_style(&mut base, &custom);
        assert_eq!(base.fill, Some(Color::rgb(255, 0, 0)));
        assert_eq!(base.stroke, Some(Color::BLACK)); // unchanged
    }

    #[test]
    fn apply_style_properties_parses_css() {
        let props = vec![
            StyleProperty { key: "fill".into(), value: "#f9f".into() },
            StyleProperty { key: "stroke-width".into(), value: "4px".into() },
            StyleProperty { key: "opacity".into(), value: "0.5".into() },
        ];
        let mut style = Style::default();
        apply_style_properties(&mut style, &props);
        assert!(style.fill.is_some());
        assert_eq!(style.stroke_width, Some(4.0));
        assert_eq!(style.opacity, Some(0.5));
    }

    #[test]
    fn merge_custom_style_with_none() {
        let theme = Theme::light();
        let s = merge_custom_style(None, &theme);
        assert_eq!(s.fill, Some(theme.node_fill));
    }

    #[test]
    fn merge_custom_style_overrides() {
        let theme = Theme::light();
        let custom = Style {
            fill: Some(Color::rgb(255, 0, 0)),
            ..Default::default()
        };
        let s = merge_custom_style(Some(&custom), &theme);
        assert_eq!(s.fill, Some(Color::rgb(255, 0, 0)));
        assert_eq!(s.stroke, Some(theme.node_stroke));
    }

    #[test]
    fn contrasting_label_dark_fill() {
        let theme = Theme::light();
        let lstyle = contrasting_label_style(Some(Color::rgb(20, 20, 20)), &theme);
        assert_eq!(lstyle.fill, Some(Color::WHITE));
    }

    #[test]
    fn contrasting_label_light_fill() {
        let theme = Theme::light();
        let lstyle = contrasting_label_style(Some(Color::rgb(250, 250, 250)), &theme);
        assert_eq!(lstyle.fill, Some(Color::BLACK));
    }

    #[test]
    fn contrasting_label_mid_fill() {
        let theme = Theme::light();
        // rgb(180, 180, 180) has luminance ~0.46 — in the mid range, no override
        let lstyle = contrasting_label_style(Some(Color::rgb(180, 180, 180)), &theme);
        assert_eq!(lstyle.fill, Some(theme.node_text)); // unchanged
    }
}
