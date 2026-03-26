mod primitive;

use rusty_mermaid_core::{Color, Renderer, Scene, Theme};

/// Raster-specific rendering configuration.
#[derive(Debug, Clone)]
pub struct RasterConfig {
    /// DPI scale factor (1.0 = 1x, 2.0 = 2x / Retina).
    pub scale: f64,
    /// Theme providing colors, typography, padding, background, and optional custom font.
    pub theme: Theme,
}

impl Default for RasterConfig {
    fn default() -> Self {
        Self {
            scale: 2.0,
            theme: Theme::default(),
        }
    }
}

/// Raster rendering backend. Converts a Scene to PNG bytes.
pub struct RasterRenderer {
    pub config: RasterConfig,
}

impl RasterRenderer {
    pub fn new() -> Self {
        Self {
            config: RasterConfig::default(),
        }
    }

    pub fn with_config(config: RasterConfig) -> Self {
        Self { config }
    }
}

impl Default for RasterRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer for RasterRenderer {
    type Output = Vec<u8>;

    fn render(&self, scene: &Scene) -> Vec<u8> {
        let theme = &self.config.theme;
        let padding = theme.padding;
        let scale = self.config.scale;
        let w = ((scene.width + padding * 2.0) * scale).ceil() as u32;
        let h = ((scene.height + padding * 2.0) * scale).ceil() as u32;

        let mut pixmap = tiny_skia::Pixmap::new(w, h).expect("pixmap dimensions must be > 0");

        // Fill background
        let bg = to_skia_color(theme.background);
        pixmap.fill(bg);

        // Render all primitives with padding offset applied via transform
        let offset = tiny_skia::Transform::from_scale(scale as f32, scale as f32)
            .post_translate(padding as f32 * scale as f32, padding as f32 * scale as f32);

        for elem in scene.elements() {
            primitive::render_primitive(&mut pixmap, &elem.primitive, offset, theme);
        }

        encode_png(&pixmap)
    }
}

fn to_skia_color(c: Color) -> tiny_skia::Color {
    tiny_skia::Color::from_rgba8(c.r, c.g, c.b, c.a)
}

fn encode_png(pixmap: &tiny_skia::Pixmap) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut encoder = png::Encoder::new(&mut buf, pixmap.width(), pixmap.height());
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("PNG header write failed");
    writer
        .write_image_data(pixmap.data())
        .expect("PNG data write failed");
    writer.finish().expect("PNG finish failed");
    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusty_mermaid_core::{BBox, Point, Primitive, Style};

    #[test]
    fn render_empty_scene() {
        let renderer = RasterRenderer::new();
        let scene = Scene::new(100.0, 50.0);
        let png = renderer.render(&scene);
        // Valid PNG starts with magic bytes
        assert_eq!(&png[..4], &[0x89, b'P', b'N', b'G']);
    }

    #[test]
    fn render_rect() {
        let renderer = RasterRenderer::new();
        let mut scene = Scene::new(200.0, 100.0);
        scene.push(Primitive::Rect {
            bbox: BBox::new(100.0, 50.0, 80.0, 40.0),
            rx: 5.0,
            ry: 5.0,
            style: Style {
                fill: Some(Color::rgb(236, 236, 255)),
                stroke: Some(Color::rgb(147, 112, 219)),
                stroke_width: Some(1.5),
                ..Default::default()
            },
        });
        let png = renderer.render(&scene);
        assert_eq!(&png[..4], &[0x89, b'P', b'N', b'G']);
        assert!(png.len() > 100); // non-trivial output
    }

    #[test]
    fn render_circle() {
        let renderer = RasterRenderer::new();
        let mut scene = Scene::new(100.0, 100.0);
        scene.push(Primitive::Circle {
            center: Point::new(50.0, 50.0),
            radius: 20.0,
            style: Style {
                fill: Some(Color::rgb(51, 51, 51)),
                stroke: Some(Color::rgb(51, 51, 51)),
                ..Default::default()
            },
        });
        let png = renderer.render(&scene);
        assert_eq!(&png[..4], &[0x89, b'P', b'N', b'G']);
    }

    #[test]
    fn render_path() {
        use rusty_mermaid_core::PathSegment;
        let renderer = RasterRenderer::new();
        let mut scene = Scene::new(200.0, 200.0);
        scene.push(Primitive::Path {
            segments: vec![
                PathSegment::MoveTo(Point::new(10.0, 10.0)),
                PathSegment::LineTo(Point::new(190.0, 190.0)),
            ],
            style: Style {
                stroke: Some(Color::rgb(51, 51, 51)),
                stroke_width: Some(1.5),
                ..Default::default()
            },
            marker_start: None,
            marker_end: None,
        });
        let png = renderer.render(&scene);
        assert_eq!(&png[..4], &[0x89, b'P', b'N', b'G']);
    }

    #[test]
    fn scale_affects_pixel_dimensions() {
        let renderer_1x = RasterRenderer::with_config(RasterConfig {
            scale: 1.0,
            ..Default::default()
        });
        let renderer_2x = RasterRenderer::with_config(RasterConfig {
            scale: 2.0,
            ..Default::default()
        });
        let scene = Scene::new(100.0, 50.0);
        let png_1x = renderer_1x.render(&scene);
        let png_2x = renderer_2x.render(&scene);
        // 2x should produce larger output
        assert!(png_2x.len() > png_1x.len());
    }

    #[test]
    fn render_polygon() {
        let renderer = RasterRenderer::new();
        let mut scene = Scene::new(100.0, 100.0);
        scene.push(Primitive::Polygon {
            points: vec![
                Point::new(50.0, 10.0),
                Point::new(90.0, 90.0),
                Point::new(10.0, 90.0),
            ],
            style: Style {
                fill: Some(Color::rgb(200, 200, 255)),
                stroke: Some(Color::rgb(100, 100, 200)),
                stroke_width: Some(2.0),
                ..Default::default()
            },
        });
        let png = renderer.render(&scene);
        assert_eq!(&png[..4], &[0x89, b'P', b'N', b'G']);
    }

    #[test]
    fn render_text() {
        use rusty_mermaid_core::{TextAnchor, TextStyle};
        let renderer = RasterRenderer::new();
        let mut scene = Scene::new(200.0, 50.0);
        scene.push(Primitive::Text {
            position: Point::new(100.0, 25.0),
            content: String::from("Hello"),
            anchor: TextAnchor::Middle,
            style: TextStyle {
                font_size: 14.0,
                fill: Some(Color::rgb(51, 51, 51)),
                ..Default::default()
            },
        });
        let png = renderer.render(&scene);
        assert_eq!(&png[..4], &[0x89, b'P', b'N', b'G']);
        // Text rendering produces more pixel data than empty scene
        let empty_png = RasterRenderer::new().render(&Scene::new(200.0, 50.0));
        assert_ne!(png, empty_png);
    }

    #[test]
    fn render_text_multiline() {
        use rusty_mermaid_core::{TextAnchor, TextStyle};
        let renderer = RasterRenderer::new();
        let mut scene = Scene::new(200.0, 100.0);
        scene.push(Primitive::Text {
            position: Point::new(100.0, 50.0),
            content: String::from("Line 1\nLine 2"),
            anchor: TextAnchor::Middle,
            style: TextStyle::default(),
        });
        let png = renderer.render(&scene);
        assert_eq!(&png[..4], &[0x89, b'P', b'N', b'G']);
    }

    #[test]
    fn render_full_diagram_to_file() {
        use rusty_mermaid_core::Renderer;
        let scene = rusty_mermaid_diagrams::render_to_scene(
            "stateDiagram-v2\n    [*] --> Active\n    Active --> Paused : pause\n    Paused --> Active : resume\n    Active --> [*] : done",
        ).unwrap();
        let renderer = RasterRenderer::new();
        let png = renderer.render(&scene);
        let path = std::env::temp_dir().join("rusty_mermaid_state.png");
        std::fs::write(&path, &png).unwrap();
        eprintln!("wrote PNG to {}", path.display());
        assert!(png.len() > 1000);
    }

    #[test]
    fn render_flowchart_to_file() {
        use rusty_mermaid_core::Renderer;
        let scene = rusty_mermaid_diagrams::render_to_scene(
            "flowchart TD\n    A[Start] --> B{Decision}\n    B -->|Yes| C[OK]\n    B -->|No| D[Fail]",
        ).unwrap();
        let renderer = RasterRenderer::new();
        let png = renderer.render(&scene);
        let path = std::env::temp_dir().join("rusty_mermaid_flowchart.png");
        std::fs::write(&path, &png).unwrap();
        eprintln!("wrote PNG to {}", path.display());
        assert!(png.len() > 1000);
    }
}
