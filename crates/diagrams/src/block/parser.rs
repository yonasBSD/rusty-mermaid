use crate::common::error::{ParseError, ParseErrorKind};
use super::ir::*;

pub fn parse(input: &str) -> Result<BlockDiagram, ParseError> {
    let mut diagram = BlockDiagram::default();
    let mut header_found = false;

    for (_line_no, raw_line) in input.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        if !header_found {
            if line.starts_with("block-beta") || line == "block" {
                header_found = true;
                continue;
            }
            return Err(ParseError::new(ParseErrorKind::UnexpectedToken, 0..1, input));
        }

        // Skip styling lines
        if line.starts_with("classDef") || line.starts_with("class ") || line.starts_with("style ") {
            continue;
        }

        // Columns
        if let Some(rest) = line.strip_prefix("columns") {
            let rest = rest.trim();
            diagram.columns = if rest == "auto" { 0 } else { rest.parse().unwrap_or(0) };
            continue;
        }

        // Space
        if line.starts_with("space") {
            let span = line.strip_prefix("space").unwrap_or("")
                .trim_start_matches(':').trim().parse().unwrap_or(1usize);
            for _ in 0..span {
                diagram.blocks.push(Block {
                    id: format!("__space_{}", diagram.blocks.len()),
                    label: String::new(),
                    shape: BlockShape::Space,
                    children: Vec::new(),
                    span: 1,
                });
            }
            continue;
        }

        // Edge: id1 --> id2 or id1 -- "label" --> id2
        if let Some(edge) = try_parse_edge(line) {
            diagram.edges.push(edge);
            continue;
        }

        // Block node
        if let Some(block) = parse_block(line) {
            diagram.blocks.push(block);
        }
    }

    if !header_found {
        return Err(ParseError::new(ParseErrorKind::UnexpectedToken, 0..input.len().min(10), input));
    }

    // Default columns if not specified
    if diagram.columns == 0 {
        diagram.columns = diagram.blocks.len().min(4).max(1);
    }

    Ok(diagram)
}

fn parse_block(s: &str) -> Option<Block> {
    let s = s.trim();
    if s.is_empty() || s == "end" || s == "block" {
        return None;
    }

    // Extract ":N" span suffix from the very end before any other parsing.
    // e.g. a["Wide"]:2 → main="a[\"Wide\"]", span=2
    let (main, span) = {
        if let Some(colon) = s.rfind(':') {
            let after = s[colon + 1..].trim();
            if let Ok(n) = after.parse::<usize>() {
                (&s[..colon], n.max(1))
            } else {
                (s, 1)
            }
        } else {
            (s, 1)
        }
    };

    let (id, label, shape) = if let Some(bracket) = main.find('[') {
        let id = main[..bracket].trim().to_string();
        let rest = &main[bracket..];
        let (label, shape) = parse_shape_label(rest);
        (id, label, shape)
    } else if let Some(paren) = main.find('(') {
        let id = main[..paren].trim().to_string();
        let rest = &main[paren..];
        let (label, shape) = parse_shape_label(rest);
        (id, label, shape)
    } else if let Some(brace) = main.find('{') {
        let id = main[..brace].trim().to_string();
        let rest = &main[brace..];
        let (label, shape) = parse_shape_label(rest);
        (id, label, shape)
    } else {
        let id = main.to_string();
        (id.clone(), id, BlockShape::Rect)
    };

    Some(Block { id, label, shape, children: Vec::new(), span })
}

fn parse_shape_label(s: &str) -> (String, BlockShape) {
    if s.starts_with("[\"") || s.starts_with("['") || s.starts_with('[') {
        let label = s.trim_start_matches('[').trim_end_matches(']').trim_matches('"').trim_matches('\'').to_string();
        (label, BlockShape::Rect)
    } else if s.starts_with("((") {
        let label = s.trim_start_matches('(').trim_end_matches(')').trim_matches('"').to_string();
        (label, BlockShape::Circle)
    } else if s.starts_with("([") {
        let label = s.trim_start_matches("([").trim_end_matches("])").trim_matches('"').to_string();
        (label, BlockShape::Stadium)
    } else if s.starts_with('(') {
        let label = s.trim_start_matches('(').trim_end_matches(')').trim_matches('"').to_string();
        (label, BlockShape::Round)
    } else if s.starts_with("{{") {
        let label = s.trim_start_matches('{').trim_end_matches('}').trim_matches('"').to_string();
        (label, BlockShape::Hexagon)
    } else if s.starts_with('{') {
        let label = s.trim_start_matches('{').trim_end_matches('}').trim_matches('"').to_string();
        (label, BlockShape::Diamond)
    } else if s.starts_with("[(") {
        let label = s.trim_start_matches("[(").trim_end_matches(")]").trim_matches('"').to_string();
        (label, BlockShape::Cylinder)
    } else {
        (s.trim_matches('"').to_string(), BlockShape::Rect)
    }
}

fn try_parse_edge(line: &str) -> Option<BlockEdge> {
    // Patterns: id1 --> id2, id1 -.-> id2, id1 ==> id2, id1 -- "label" --> id2
    let (style, arrow) = if line.contains("-.->") || line.contains("-..->") {
        (EdgeStyle::Dotted, "-.")
    } else if line.contains("==>") {
        (EdgeStyle::Thick, "==")
    } else if line.contains("-->") {
        (EdgeStyle::Arrow, "--")
    } else {
        return None;
    };

    // Split on arrow pattern
    let (left, right) = if style == EdgeStyle::Dotted {
        let parts: Vec<&str> = line.splitn(2, "-.").collect();
        if parts.len() != 2 { return None; }
        (parts[0], parts[1].trim_start_matches(['-', '.', '>']).trim())
    } else {
        let sep = if style == EdgeStyle::Thick { "==>" } else { "-->" };
        let parts: Vec<&str> = line.splitn(2, sep).collect();
        if parts.len() != 2 { return None; }
        (parts[0], parts[1].trim())
    };

    let from = left.split("--").next().unwrap_or(left).split("==").next().unwrap_or(left).trim();
    let label = if left.contains('"') {
        left.split('"').nth(1).map(|s| s.to_string())
    } else {
        None
    };

    // Target might have a shape definition: id2["Label"]
    let to = right.split('[').next().unwrap_or(right)
        .split('(').next().unwrap_or(right)
        .split('{').next().unwrap_or(right)
        .trim();

    if from.is_empty() || to.is_empty() { return None; }

    Some(BlockEdge {
        from: from.to_string(),
        to: to.to_string(),
        label,
        style,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        let d = parse("block-beta\n  columns 2\n  a[\"A\"]\n  b[\"B\"]\n  c[\"C\"]\n  d[\"D\"]").unwrap();
        assert_eq!(d.columns, 2);
        assert_eq!(d.blocks.len(), 4);
    }

    #[test]
    fn parse_shapes() {
        let d = parse("block-beta\n  a[\"Rect\"]\n  b(\"Round\")\n  c{\"Diamond\"}\n  d((\"Circle\"))").unwrap();
        assert_eq!(d.blocks[0].shape, BlockShape::Rect);
        assert_eq!(d.blocks[1].shape, BlockShape::Round);
        assert_eq!(d.blocks[2].shape, BlockShape::Diamond);
        assert_eq!(d.blocks[3].shape, BlockShape::Circle);
    }

    #[test]
    fn parse_edges() {
        let d = parse("block-beta\n  a[\"A\"]\n  b[\"B\"]\n  a --> b").unwrap();
        assert_eq!(d.edges.len(), 1);
        assert_eq!(d.edges[0].from, "a");
        assert_eq!(d.edges[0].to, "b");
    }

    #[test]
    fn parse_space() {
        let d = parse("block-beta\n  columns 3\n  a[\"A\"]\n  space\n  b[\"B\"]").unwrap();
        assert_eq!(d.blocks.len(), 3);
        assert_eq!(d.blocks[1].shape, BlockShape::Space);
    }

    #[test]
    fn parse_auto_columns() {
        let d = parse("block-beta\n  a\n  b\n  c").unwrap();
        assert_eq!(d.columns, 3);
    }

    #[test]
    fn parse_column_span() {
        let d = parse("block-beta\n  columns 3\n  a[\"Wide\"]:2\n  b[\"Normal\"]").unwrap();
        assert_eq!(d.blocks[0].span, 2);
        assert_eq!(d.blocks[0].label, "Wide");
        assert_eq!(d.blocks[1].span, 1);
    }

    #[test]
    fn reject_no_header() {
        assert!(parse("a[\"A\"]").is_err());
    }
}
