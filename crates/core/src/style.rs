use crate::Color;

/// Visual style for shapes and paths.
#[derive(Debug, Clone, Default)]
pub struct Style {
    pub fill: Option<Color>,
    pub stroke: Option<Color>,
    pub stroke_width: Option<f64>,
    pub stroke_dasharray: Option<Vec<f64>>,
    pub opacity: Option<f64>,
    pub css_classes: Vec<String>,
}

/// Font weight for text rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FontWeight {
    #[default]
    Normal,
    Bold,
}

/// CSS font-family fallback stack.
/// "Intel One Mono" preferred, then platform monospace, then generic.
pub const DEFAULT_FONT_FAMILY: &str =
    "'Intel One Mono', 'SF Mono', 'Cascadia Code', 'JetBrains Mono', 'Fira Code', 'Consolas', 'Menlo', monospace";

/// Text styling properties.
#[derive(Debug, Clone)]
pub struct TextStyle {
    pub font_size: f64,
    pub font_family: String,
    pub fill: Option<Color>,
    pub font_weight: FontWeight,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_size: 14.0,
            font_family: String::from(DEFAULT_FONT_FAMILY),
            fill: None,
            font_weight: FontWeight::Normal,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_default_is_empty() {
        let s = Style::default();
        assert!(s.fill.is_none());
        assert!(s.stroke.is_none());
        assert!(s.stroke_width.is_none());
        assert!(s.stroke_dasharray.is_none());
        assert!(s.opacity.is_none());
        assert!(s.css_classes.is_empty());
    }

    #[test]
    fn text_style_default() {
        let ts = TextStyle::default();
        assert!((ts.font_size - 14.0).abs() < f64::EPSILON);
        assert_eq!(ts.font_family, DEFAULT_FONT_FAMILY);
        assert!(ts.font_family.starts_with("'Intel One Mono'"));
        assert!(ts.font_family.ends_with("monospace"));
        assert!(ts.fill.is_none());
        assert_eq!(ts.font_weight, FontWeight::Normal);
    }

    #[test]
    fn style_with_dash_array() {
        let s = Style {
            stroke_dasharray: Some(vec![5.0, 3.0]),
            ..Default::default()
        };
        assert_eq!(s.stroke_dasharray.as_ref().unwrap(), &[5.0, 3.0]);
    }

    #[test]
    fn style_with_css_classes() {
        let s = Style {
            css_classes: vec!["node".into(), "highlighted".into()],
            ..Default::default()
        };
        assert_eq!(s.css_classes.len(), 2);
        assert_eq!(s.css_classes[0], "node");
    }

    #[test]
    fn text_style_custom() {
        let ts = TextStyle {
            font_size: 24.0,
            font_family: String::from("monospace"),
            fill: Some(Color::BLACK),
            font_weight: FontWeight::Bold,
        };
        assert!((ts.font_size - 24.0).abs() < f64::EPSILON);
        assert_eq!(ts.font_family, "monospace");
        assert_eq!(ts.fill, Some(Color::BLACK));
        assert_eq!(ts.font_weight, FontWeight::Bold);
    }
}
