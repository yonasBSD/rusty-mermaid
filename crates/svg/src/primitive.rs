use rusty_mermaid_core::{MarkerType, Primitive, TextAnchor, Transform};

use crate::document::{fmt_f64, SvgDocument};
use crate::markers::marker_id;
use crate::path::segments_to_d;
use crate::style::{style_attrs, text_style_attrs};

/// Render a single Primitive into the SVG document.
pub fn render_primitive(doc: &mut SvgDocument, prim: &Primitive) {
    match prim {
        Primitive::Rect { bbox, rx, ry, style } => {
            let mut attrs: Vec<(String, String)> = vec![
                ("x".into(), fmt_f64(bbox.x - bbox.width / 2.0)),
                ("y".into(), fmt_f64(bbox.y - bbox.height / 2.0)),
                ("width".into(), fmt_f64(bbox.width)),
                ("height".into(), fmt_f64(bbox.height)),
            ];
            if *rx > 0.0 {
                attrs.push(("rx".into(), fmt_f64(*rx)));
            }
            if *ry > 0.0 {
                attrs.push(("ry".into(), fmt_f64(*ry)));
            }
            attrs.extend(style_attrs(style));
            let refs: Vec<(&str, &str)> = attrs.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
            doc.empty_tag("rect", &refs);
        }

        Primitive::Circle { center, radius, style } => {
            let mut attrs: Vec<(String, String)> = vec![
                ("cx".into(), fmt_f64(center.x)),
                ("cy".into(), fmt_f64(center.y)),
                ("r".into(), fmt_f64(*radius)),
            ];
            attrs.extend(style_attrs(style));
            let refs: Vec<(&str, &str)> = attrs.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
            doc.empty_tag("circle", &refs);
        }

        Primitive::Ellipse { center, rx, ry, style } => {
            let mut attrs: Vec<(String, String)> = vec![
                ("cx".into(), fmt_f64(center.x)),
                ("cy".into(), fmt_f64(center.y)),
                ("rx".into(), fmt_f64(*rx)),
                ("ry".into(), fmt_f64(*ry)),
            ];
            attrs.extend(style_attrs(style));
            let refs: Vec<(&str, &str)> = attrs.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
            doc.empty_tag("ellipse", &refs);
        }

        Primitive::Path { segments, style, marker_start, marker_end } => {
            let d = segments_to_d(segments);
            let mut attrs: Vec<(String, String)> = vec![("d".into(), d)];
            // Default path: no fill, black stroke
            if style.fill.is_none() {
                attrs.push(("fill".into(), "none".into()));
            }
            if style.stroke.is_none() && style.stroke_width.is_none() {
                attrs.push(("stroke".into(), "#333".into()));
                attrs.push(("stroke-width".into(), "1.5".into()));
            }
            attrs.extend(style_attrs(style));
            if let Some(m) = marker_start {
                attrs.push(("marker-start".into(), format!("url(#{})", marker_id(*m))));
            }
            if let Some(m) = marker_end {
                attrs.push(("marker-end".into(), format!("url(#{})", marker_id(*m))));
            }
            let refs: Vec<(&str, &str)> = attrs.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
            doc.empty_tag("path", &refs);
        }

        Primitive::Text { position, content, anchor, style } => {
            let anchor_str = match anchor {
                TextAnchor::Start => "start",
                TextAnchor::Middle => "middle",
                TextAnchor::End => "end",
            };
            let lines: Vec<&str> = content.split('\n').collect();
            if lines.len() <= 1 {
                // Single line
                let mut attrs: Vec<(String, String)> = vec![
                    ("x".into(), fmt_f64(position.x)),
                    ("y".into(), fmt_f64(position.y)),
                    ("text-anchor".into(), anchor_str.into()),
                    ("dominant-baseline".into(), "central".into()),
                ];
                attrs.extend(text_style_attrs(style));
                let refs: Vec<(&str, &str)> = attrs.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
                if let Some(spans) = parse_inline_markdown(content) {
                    doc.open_tag("text", &refs);
                    render_md_spans(doc, &spans);
                    doc.close_tag("text");
                } else {
                    doc.text_element("text", &refs, &xml_escape(content));
                }
            } else {
                // Multi-line — tspan elements with dy offsets
                let line_height = style.font_size * 1.2;
                let total_h = line_height * (lines.len() - 1) as f64;
                let start_y = position.y - total_h / 2.0;

                let mut attrs: Vec<(String, String)> = vec![
                    ("x".into(), fmt_f64(position.x)),
                    ("y".into(), fmt_f64(start_y)),
                    ("text-anchor".into(), anchor_str.into()),
                    ("dominant-baseline".into(), "central".into()),
                ];
                attrs.extend(text_style_attrs(style));
                let refs: Vec<(&str, &str)> = attrs.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
                doc.open_tag("text", &refs);

                let x_str = fmt_f64(position.x);
                for (i, line) in lines.iter().enumerate() {
                    let dy = if i == 0 { "0".to_string() } else { fmt_f64(line_height) };
                    if let Some(spans) = parse_inline_markdown(line) {
                        let tspan_attrs: Vec<(&str, &str)> = vec![
                            ("x", &x_str),
                            ("dy", &dy),
                        ];
                        doc.open_tag("tspan", &tspan_attrs);
                        render_md_spans(doc, &spans);
                        doc.close_tag("tspan");
                    } else {
                        let tspan_attrs: Vec<(&str, &str)> = vec![
                            ("x", &x_str),
                            ("dy", &dy),
                        ];
                        doc.text_element("tspan", &tspan_attrs, &xml_escape(line));
                    }
                }

                doc.close_tag("text");
            }
        }

        Primitive::Polygon { points, style } => {
            let pts: Vec<String> = points
                .iter()
                .map(|p| format!("{},{}", fmt_f64(p.x), fmt_f64(p.y)))
                .collect();
            let mut attrs: Vec<(String, String)> = vec![("points".into(), pts.join(" "))];
            attrs.extend(style_attrs(style));
            let refs: Vec<(&str, &str)> = attrs.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
            doc.empty_tag("polygon", &refs);
        }

        Primitive::Group { transform, children } => {
            let transform_str = transform_to_attr(transform);
            let mut attrs: Vec<(String, String)> = Vec::new();
            if !transform_str.is_empty() {
                attrs.push(("transform".into(), transform_str));
            }
            let refs: Vec<(&str, &str)> = attrs.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
            doc.open_tag("g", &refs);
            for child in children {
                render_primitive(doc, child);
            }
            doc.close_tag("g");
        }

        Primitive::Arc { center, inner_r, outer_r, start_angle, end_angle, style } => {
            render_arc(doc, center, *inner_r, *outer_r, *start_angle, *end_angle, style);
        }
    }
}

/// Collect all marker types used in a scene.
pub fn collect_markers(primitives: &[Primitive]) -> Vec<MarkerType> {
    let mut markers = Vec::new();
    for prim in primitives {
        match prim {
            Primitive::Path { marker_start, marker_end, .. } => {
                if let Some(m) = marker_start {
                    if !markers.contains(m) {
                        markers.push(*m);
                    }
                }
                if let Some(m) = marker_end {
                    if !markers.contains(m) {
                        markers.push(*m);
                    }
                }
            }
            Primitive::Group { children, .. } => {
                for m in collect_markers(children) {
                    if !markers.contains(&m) {
                        markers.push(m);
                    }
                }
            }
            _ => {}
        }
    }
    markers
}

fn transform_to_attr(t: &Transform) -> String {
    match t {
        Transform::Identity => String::new(),
        Transform::Translate(x, y) => format!("translate({}, {})", fmt_f64(*x), fmt_f64(*y)),
        Transform::Scale(sx, sy) => format!("scale({}, {})", fmt_f64(*sx), fmt_f64(*sy)),
        Transform::Rotate { degrees, cx, cy } => {
            format!("rotate({}, {}, {})", fmt_f64(*degrees), fmt_f64(*cx), fmt_f64(*cy))
        }
    }
}

fn render_arc(
    doc: &mut SvgDocument,
    center: &rusty_mermaid_core::Point,
    _inner_r: f64,
    outer_r: f64,
    start_angle: f64,
    end_angle: f64,
    style: &rusty_mermaid_core::Style,
) {
    // Convert arc to SVG path
    let x1 = center.x + outer_r * start_angle.cos();
    let y1 = center.y + outer_r * start_angle.sin();
    let x2 = center.x + outer_r * end_angle.cos();
    let y2 = center.y + outer_r * end_angle.sin();
    let large_arc = if (end_angle - start_angle).abs() > std::f64::consts::PI { 1 } else { 0 };

    let d = format!(
        "M{} {} A{} {} 0 {} 1 {} {}",
        fmt_f64(x1), fmt_f64(y1),
        fmt_f64(outer_r), fmt_f64(outer_r),
        large_arc,
        fmt_f64(x2), fmt_f64(y2),
    );

    let mut attrs: Vec<(String, String)> = vec![("d".into(), d)];
    attrs.extend(style_attrs(style));
    if style.fill.is_none() {
        attrs.push(("fill".into(), "none".into()));
    }
    if style.stroke.is_none() {
        attrs.push(("stroke".into(), "#333".into()));
    }
    let refs: Vec<(&str, &str)> = attrs.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    doc.empty_tag("path", &refs);
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Inline markdown span with bold/italic styling.
#[derive(Debug, Clone, PartialEq, Eq)]
struct MdSpan {
    text: String,
    bold: bool,
    italic: bool,
}

/// Parse inline markdown (`**bold**`, `*italic*`) into styled spans.
/// Returns `None` if the text contains no markdown markers.
fn parse_inline_markdown(text: &str) -> Option<Vec<MdSpan>> {
    if !text.contains('*') {
        return None;
    }

    let mut spans = Vec::new();
    let mut bold = false;
    let mut italic = false;
    let mut buf = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' {
            // Toggle bold
            if !buf.is_empty() {
                spans.push(MdSpan { text: std::mem::take(&mut buf), bold, italic });
            }
            bold = !bold;
            i += 2;
        } else if chars[i] == '*' {
            // Toggle italic
            if !buf.is_empty() {
                spans.push(MdSpan { text: std::mem::take(&mut buf), bold, italic });
            }
            italic = !italic;
            i += 1;
        } else {
            buf.push(chars[i]);
            i += 1;
        }
    }
    if !buf.is_empty() {
        spans.push(MdSpan { text: buf, bold, italic });
    }

    // Only return spans if we actually found some formatting
    if spans.iter().any(|s| s.bold || s.italic) {
        Some(spans)
    } else {
        None
    }
}

/// Render a line of text with inline markdown as styled tspans.
fn render_md_spans(doc: &mut SvgDocument, spans: &[MdSpan]) {
    for span in spans {
        let escaped = xml_escape(&span.text);
        let mut attrs: Vec<(&str, &str)> = Vec::new();
        if span.bold {
            attrs.push(("font-weight", "bold"));
        }
        if span.italic {
            attrs.push(("font-style", "italic"));
        }
        if attrs.is_empty() {
            // Plain text — no wrapping tspan needed, but we use one for consistency.
            doc.text_element("tspan", &[], &escaped);
        } else {
            doc.text_element("tspan", &attrs, &escaped);
        }
    }
}

#[cfg(test)]
mod tests {
    use rusty_mermaid_core::*;

    use super::*;

    fn render_one(prim: &Primitive) -> String {
        let mut doc = SvgDocument::new(200.0, 200.0);
        render_primitive(&mut doc, prim);
        doc.finish()
    }

    #[test]
    fn render_rect() {
        let svg = render_one(&Primitive::Rect {
            bbox: BBox::new(50.0, 50.0, 80.0, 40.0),
            rx: 5.0,
            ry: 5.0,
            style: Style {
                fill: Some(Color::WHITE),
                stroke: Some(Color::BLACK),
                ..Default::default()
            },
        });
        assert!(svg.contains("<rect"));
        assert!(svg.contains(r#"x="10""#));
        assert!(svg.contains(r#"y="30""#));
        assert!(svg.contains(r#"width="80""#));
        assert!(svg.contains(r#"height="40""#));
        assert!(svg.contains(r#"rx="5""#));
        assert!(svg.contains(r##"fill="#ffffff""##));
    }

    #[test]
    fn render_circle() {
        let svg = render_one(&Primitive::Circle {
            center: Point::new(100.0, 100.0),
            radius: 30.0,
            style: Style::default(),
        });
        assert!(svg.contains("<circle"));
        assert!(svg.contains(r#"cx="100""#));
        assert!(svg.contains(r#"r="30""#));
    }

    #[test]
    fn render_path_with_marker() {
        let svg = render_one(&Primitive::Path {
            segments: vec![
                PathSegment::MoveTo(Point::new(0.0, 0.0)),
                PathSegment::LineTo(Point::new(100.0, 100.0)),
            ],
            style: Style::default(),
            marker_start: None,
            marker_end: Some(MarkerType::ArrowPoint),
        });
        assert!(svg.contains(r#"d="M0 0 L100 100""#));
        assert!(svg.contains("marker-end"));
        assert!(svg.contains("url(#arrow-point)"));
    }

    #[test]
    fn render_text() {
        let svg = render_one(&Primitive::Text {
            position: Point::new(50.0, 50.0),
            content: "Hello <World>".into(),
            anchor: TextAnchor::Middle,
            style: TextStyle::default(),
        });
        assert!(svg.contains("<text"));
        assert!(svg.contains("Hello &lt;World&gt;"));
        assert!(svg.contains(r#"text-anchor="middle""#));
    }

    #[test]
    fn render_polygon() {
        let svg = render_one(&Primitive::Polygon {
            points: vec![
                Point::new(0.0, 0.0),
                Point::new(100.0, 0.0),
                Point::new(50.0, 86.6),
            ],
            style: Style::default(),
        });
        assert!(svg.contains("<polygon"));
        assert!(svg.contains("points="));
    }

    #[test]
    fn render_group() {
        let svg = render_one(&Primitive::Group {
            transform: Transform::Translate(10.0, 20.0),
            children: vec![Primitive::Circle {
                center: Point::new(0.0, 0.0),
                radius: 5.0,
                style: Style::default(),
            }],
        });
        assert!(svg.contains("<g"));
        assert!(svg.contains("translate(10, 20)"));
        assert!(svg.contains("<circle"));
        assert!(svg.contains("</g>"));
    }

    #[test]
    fn collect_markers_from_paths() {
        let prims = vec![
            Primitive::Path {
                segments: vec![],
                style: Style::default(),
                marker_start: None,
                marker_end: Some(MarkerType::ArrowPoint),
            },
            Primitive::Path {
                segments: vec![],
                style: Style::default(),
                marker_start: Some(MarkerType::Circle),
                marker_end: Some(MarkerType::ArrowPoint),
            },
        ];
        let markers = collect_markers(&prims);
        assert_eq!(markers.len(), 2);
        assert!(markers.contains(&MarkerType::ArrowPoint));
        assert!(markers.contains(&MarkerType::Circle));
    }

    #[test]
    fn xml_escape_special_chars() {
        assert_eq!(xml_escape("a < b & c > d"), "a &lt; b &amp; c &gt; d");
        assert_eq!(xml_escape(r#"say "hello""#), "say &quot;hello&quot;");
    }

    #[test]
    fn parse_md_bold() {
        let spans = parse_inline_markdown("hello **world**").unwrap();
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0], MdSpan { text: "hello ".into(), bold: false, italic: false });
        assert_eq!(spans[1], MdSpan { text: "world".into(), bold: true, italic: false });
    }

    #[test]
    fn parse_md_italic() {
        let spans = parse_inline_markdown("hello *world*").unwrap();
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0], MdSpan { text: "hello ".into(), bold: false, italic: false });
        assert_eq!(spans[1], MdSpan { text: "world".into(), bold: false, italic: true });
    }

    #[test]
    fn parse_md_bold_italic() {
        let spans = parse_inline_markdown("***both***").unwrap();
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0], MdSpan { text: "both".into(), bold: true, italic: true });
    }

    #[test]
    fn parse_md_no_markers() {
        assert!(parse_inline_markdown("plain text").is_none());
    }

    #[test]
    fn render_text_bold() {
        let svg = render_one(&Primitive::Text {
            position: Point::new(50.0, 50.0),
            content: "hello **world**".into(),
            anchor: TextAnchor::Middle,
            style: TextStyle::default(),
        });
        assert!(svg.contains(r#"font-weight="bold"#));
        assert!(svg.contains(">world</tspan>"));
        assert!(svg.contains(">hello </tspan>"));
    }

    #[test]
    fn render_text_italic() {
        let svg = render_one(&Primitive::Text {
            position: Point::new(50.0, 50.0),
            content: "hello *world*".into(),
            anchor: TextAnchor::Middle,
            style: TextStyle::default(),
        });
        assert!(svg.contains(r#"font-style="italic"#));
        assert!(svg.contains(">world</tspan>"));
    }

    #[test]
    fn render_multiline_with_markdown() {
        let svg = render_one(&Primitive::Text {
            position: Point::new(50.0, 50.0),
            content: "**bold line**\nplain line".into(),
            anchor: TextAnchor::Middle,
            style: TextStyle::default(),
        });
        assert!(svg.contains(r#"font-weight="bold"#));
        assert!(svg.contains(">bold line</tspan>"));
        assert!(svg.contains(">plain line</tspan>"));
    }
}
