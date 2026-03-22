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

/// CSS font-family fallback stack for SVG rendering.
pub use crate::font_fallback::SVG_FONT_FAMILY as DEFAULT_FONT_FAMILY;

/// Diagram color theme. All rendering reads from this — no hardcoded values.
#[derive(Debug, Clone)]
pub struct Theme {
    // -- Colors --
    pub node_fill: Color,
    pub node_stroke: Color,
    pub node_text: Color,
    pub edge_stroke: Color,
    pub edge_label_text: Color,
    pub edge_label_bg: Color,
    pub start_fill: Color,
    pub end_inner_fill: Color,
    pub composite_fill: Color,
    pub composite_stroke: Color,
    pub composite_label: Color,
    pub note_fill: Color,
    pub note_stroke: Color,
    pub note_text: Color,
    pub subgraph_fill: Color,
    pub subgraph_stroke: Color,
    pub subgraph_label: Color,
    pub divider_stroke: Color,
    pub region_stroke: Color,
    pub lifeline_stroke: Color,
    pub activation_fill: Color,
    pub activation_stroke: Color,
    // -- Typography --
    pub font_size_node: f64,
    pub font_size_edge_label: f64,
    pub font_size_label: f64,
    pub font_size_small: f64,
    pub font_size_title: f64,
    // -- Stroke --
    pub default_stroke_width: f64,
    // -- Rendering --
    /// Padding around the diagram (pixels on each side).
    pub padding: f64,
    /// Background color for raster/interactive backends.
    pub background: Color,
    /// Custom font bytes (TTF/OTF). When `None`, backends use embedded default.
    pub custom_font: Option<Vec<u8>>,
}

impl Default for Theme {
    fn default() -> Self {
        Self::light()
    }
}

impl Theme {
    /// Mermaid.js-aligned light theme with lavender fills and purple borders.
    pub fn light() -> Self {
        Self {
            node_fill: Color::rgba(236, 236, 255, 178),   // lavender @ 70%
            node_stroke: Color::rgb(147, 112, 219),       // #9370DB purple
            node_text: Color::rgb(51, 51, 51),            // #333333
            edge_stroke: Color::rgb(51, 51, 51),          // #333333
            edge_label_text: Color::rgb(51, 51, 51),      // #333333
            edge_label_bg: Color::rgba(245, 243, 255, 191), // frosted lavender @ 75%
            start_fill: Color::rgb(51, 51, 51),           // #333333
            end_inner_fill: Color::rgb(147, 112, 219),    // #9370DB purple
            composite_fill: Color::rgba(255, 255, 255, 204), // white @ 80%
            composite_stroke: Color::rgb(147, 112, 219),  // #9370DB
            composite_label: Color::rgb(51, 51, 51),
            note_fill: Color::rgba(255, 248, 200, 178),   // warm yellow @ 70%
            note_stroke: Color::rgb(170, 170, 51),        // #aaaa33
            note_text: Color::rgb(51, 51, 51),
            subgraph_fill: Color::rgba(236, 242, 220, 153), // sage @ 60%
            subgraph_stroke: Color::rgb(168, 174, 142),   // #a8ae8e muted olive
            subgraph_label: Color::rgb(51, 51, 51),
            divider_stroke: Color::rgb(128, 128, 128),    // #808080
            region_stroke: Color::rgb(128, 128, 128),     // #808080
            lifeline_stroke: Color::rgb(175, 165, 200),   // gray-lavender blend
            activation_fill: Color::rgba(200, 190, 230, 180), // light lavender
            activation_stroke: Color::rgb(153, 153, 153), // #999999
            font_size_node: 14.0,
            font_size_edge_label: 12.0,
            font_size_label: 13.0,
            font_size_small: 11.0,
            font_size_title: 16.0,
            default_stroke_width: 1.5,
            padding: 20.0,
            background: Color::WHITE,
            custom_font: None,
        }
    }

    /// Dark theme for dark backgrounds.
    pub fn dark() -> Self {
        Self {
            node_fill: Color::rgb(45, 45, 68),            // #2d2d44
            node_stroke: Color::rgb(124, 111, 189),       // #7c6fbd
            node_text: Color::rgb(205, 214, 244),         // #cdd6f4
            edge_stroke: Color::rgb(166, 173, 200),       // #a6adc8
            edge_label_text: Color::rgb(186, 194, 222),   // #bac2de
            edge_label_bg: Color::rgba(30, 30, 46, 204),  // dark semi-transparent
            start_fill: Color::rgb(205, 214, 244),        // #cdd6f4
            end_inner_fill: Color::rgb(124, 111, 189),    // #7c6fbd
            composite_fill: Color::rgb(37, 37, 56),       // #252538
            composite_stroke: Color::rgb(124, 111, 189),
            composite_label: Color::rgb(186, 194, 222),
            note_fill: Color::rgb(62, 60, 40),            // dark yellow-brown
            note_stroke: Color::rgb(170, 170, 51),
            note_text: Color::rgb(205, 214, 244),
            subgraph_fill: Color::rgb(40, 43, 35),        // #282b23 dark sage
            subgraph_stroke: Color::rgb(105, 112, 85),    // #697055 muted dark olive
            subgraph_label: Color::rgb(205, 214, 244),
            divider_stroke: Color::rgb(88, 91, 112),
            region_stroke: Color::rgb(88, 91, 112),
            lifeline_stroke: Color::rgb(100, 95, 130),    // muted purple-gray
            activation_fill: Color::rgba(60, 55, 85, 180), // dark lavender
            activation_stroke: Color::rgb(88, 91, 112),   // #585b70
            font_size_node: 14.0,
            font_size_edge_label: 12.0,
            font_size_label: 13.0,
            font_size_small: 11.0,
            font_size_title: 16.0,
            default_stroke_width: 1.5,
            padding: 20.0,
            background: Color::rgb(30, 30, 46),  // #1e1e2e
            custom_font: None,
        }
    }
}

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
    fn theme_default_is_light() {
        let t = Theme::default();
        assert_eq!(t.node_fill, Color::rgba(236, 236, 255, 178));
        assert_eq!(t.node_stroke, Color::rgb(147, 112, 219));
    }

    #[test]
    fn theme_dark_has_dark_fills() {
        let t = Theme::dark();
        assert!(t.node_fill.luminance() < 0.1);
        assert!(t.node_text.luminance() > 0.5);
    }

    #[test]
    fn theme_light_typography_and_stroke() {
        let t = Theme::light();
        assert!((t.font_size_node - 14.0).abs() < f64::EPSILON);
        assert!((t.font_size_edge_label - 12.0).abs() < f64::EPSILON);
        assert!((t.font_size_label - 13.0).abs() < f64::EPSILON);
        assert!((t.font_size_small - 11.0).abs() < f64::EPSILON);
        assert!((t.font_size_title - 16.0).abs() < f64::EPSILON);
        assert!((t.default_stroke_width - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn theme_light_sequence_colors() {
        let t = Theme::light();
        assert_eq!(t.lifeline_stroke, Color::rgb(175, 165, 200));
        assert_eq!(t.activation_fill, Color::rgba(200, 190, 230, 180));
        assert_eq!(t.activation_stroke, Color::rgb(153, 153, 153));
    }

    #[test]
    fn theme_dark_has_all_new_fields() {
        let t = Theme::dark();
        assert!((t.font_size_node - 14.0).abs() < f64::EPSILON);
        assert!((t.default_stroke_width - 1.5).abs() < f64::EPSILON);
        assert!(t.lifeline_stroke.luminance() < 0.3);
        assert!(t.activation_fill.a < 255);
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
