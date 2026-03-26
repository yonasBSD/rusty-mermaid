use super::ir::{TreemapDiagram, TreemapNode};
use crate::common::error::{ParseError, ParseErrorKind};

pub fn parse(input: &str) -> Result<TreemapDiagram, ParseError> {
    let mut header_found = false;
    let mut entries: Vec<(usize, String, Option<f64>)> = Vec::new();

    for (_line_no, raw_line) in input.lines().enumerate() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with("%%") || trimmed.starts_with("classDef") {
            continue;
        }
        if !header_found {
            if trimmed.starts_with("treemap") {
                header_found = true;
                continue;
            }
            return Err(ParseError::new(
                ParseErrorKind::UnexpectedToken,
                0..1,
                input,
            ));
        }

        let indent = raw_line.len() - raw_line.trim_start().len();
        let (name, value) = parse_name_value(trimmed);
        entries.push((indent, name, value));
    }

    if !header_found {
        return Err(ParseError::new(
            ParseErrorKind::UnexpectedToken,
            0..input.len().min(10),
            input,
        ));
    }

    // Normalize indents
    let min_indent = entries
        .iter()
        .map(|(i, _, _)| *i)
        .filter(|&i| i > 0)
        .min()
        .unwrap_or(4);
    let entries: Vec<(usize, String, Option<f64>)> = entries
        .into_iter()
        .map(|(indent, name, val)| (indent / min_indent.max(1), name, val))
        .collect();

    let min_depth = entries.iter().map(|(d, _, _)| *d).min().unwrap_or(0);
    let roots = build_children(&entries, &mut 0, min_depth);
    Ok(TreemapDiagram { roots })
}

fn parse_name_value(s: &str) -> (String, Option<f64>) {
    // Strip :::className
    let s = s.split(":::").next().unwrap_or(s).trim();

    // Try "name": value or "name", value or name: value
    if let Some((name_part, val_part)) = s.rsplit_once(':').or_else(|| s.rsplit_once(',')) {
        let name = name_part.trim().trim_matches('"').to_string();
        if let Ok(v) = val_part.trim().parse::<f64>() {
            return (name, Some(v));
        }
        // Colon was part of name, not a separator
        (s.trim_matches('"').to_string(), None)
    } else {
        (s.trim_matches('"').to_string(), None)
    }
}

fn build_children(
    entries: &[(usize, String, Option<f64>)],
    pos: &mut usize,
    depth: usize,
) -> Vec<TreemapNode> {
    let mut nodes = Vec::new();
    while *pos < entries.len() && entries[*pos].0 >= depth {
        if entries[*pos].0 == depth {
            let name = entries[*pos].1.clone();
            let value = entries[*pos].2;
            *pos += 1;
            let children = build_children(entries, pos, depth + 1);
            // If has children, value comes from children (section)
            let value = if children.is_empty() { value } else { None };
            nodes.push(TreemapNode {
                name,
                value,
                children,
            });
        } else {
            *pos += 1;
        }
    }
    nodes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        let d = parse("treemap\n    \"Root\"\n        \"A\": 60\n        \"B\": 40").unwrap();
        assert_eq!(d.roots.len(), 1);
        assert_eq!(d.roots[0].children.len(), 2);
        assert!((d.roots[0].children[0].total_value() - 60.0).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_nested() {
        let d = parse("treemap\n    Section\n        Sub\n            Leaf: 10").unwrap();
        assert!(!d.roots[0].is_leaf());
        assert!(d.roots[0].children[0].children[0].is_leaf());
    }

    #[test]
    fn parse_multiple_roots() {
        let d = parse("treemap\n    A: 50\n    B: 30").unwrap();
        assert_eq!(d.roots.len(), 2);
    }

    #[test]
    fn parse_comma_value() {
        let d = parse("treemap\n    \"Item\", 42").unwrap();
        assert!((d.roots[0].total_value() - 42.0).abs() < f64::EPSILON);
    }

    #[test]
    fn reject_no_header() {
        assert!(parse("    A: 10").is_err());
    }

    #[test]
    fn reject_wrong_header() {
        assert!(parse("pie\n    A: 10").is_err());
    }

    #[test]
    fn section_value_from_children() {
        let d = parse("treemap\n    Section\n        A: 60\n        B: 40").unwrap();
        assert!(
            d.roots[0].value.is_none(),
            "section should have no direct value"
        );
        assert!((d.roots[0].total_value() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn empty_treemap_ok() {
        assert!(parse("treemap").is_ok());
    }
}
