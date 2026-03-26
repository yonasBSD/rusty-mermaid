use crate::common::error::{ParseError, ParseErrorKind};
use super::ir::{VennDiagram, VennSet, VennUnion};

/// Parse a venn diagram.
///
/// ```text
/// venn-beta
///   set A["Alpha"]:20
///   set B["Beta"]:12
///   union A,B["Overlap"]:5
/// ```
pub fn parse(input: &str) -> Result<VennDiagram, ParseError> {
    let mut diagram = VennDiagram::default();
    let mut header_found = false;

    for (line_no, raw_line) in input.lines().enumerate() {
        let line = raw_line.trim();

        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        if !header_found {
            if line.starts_with("venn") {
                header_found = true;
                continue;
            }
            return Err(make_err(input, line_no));
        }

        if let Some(rest) = line.strip_prefix("title") {
            diagram.title = Some(rest.trim().trim_matches('"').to_string());
            continue;
        }

        // Skip style/text lines for now
        if line.starts_with("style") || line.starts_with("text") {
            continue;
        }

        if let Some(rest) = line.strip_prefix("set") {
            let rest = rest.trim();
            let (id, label, size) = parse_set_def(rest);
            diagram.sets.push(VennSet {
                id: id.clone(),
                label: label.unwrap_or(id),
                size: size.unwrap_or(10.0),
            });
            continue;
        }

        if let Some(rest) = line.strip_prefix("union") {
            let rest = rest.trim();
            if let Some((ids_part, remainder)) = split_at_bracket_or_colon(rest) {
                let set_ids: Vec<String> = ids_part.split(',').map(|s| s.trim().to_string()).collect();
                let (label, size) = parse_label_size(remainder);
                let default_size = 10.0 / (set_ids.len() as f64).powi(2);
                diagram.unions.push(VennUnion {
                    set_ids,
                    label,
                    size: size.unwrap_or(default_size),
                });
            }
            continue;
        }

        // Unknown lines silently skipped
    }

    if !header_found {
        return Err(ParseError::new(ParseErrorKind::UnexpectedToken, 0..input.len().min(10), input));
    }

    Ok(diagram)
}

/// Parse "ID[\"Label\"]:size" or "ID:size" or "ID[\"Label\"]" or "ID"
fn parse_set_def(s: &str) -> (String, Option<String>, Option<f64>) {
    let (id_part, rest) = if let Some(bracket) = s.find('[') {
        (&s[..bracket], &s[bracket..])
    } else if let Some(colon) = s.find(':') {
        (&s[..colon], &s[colon..])
    } else {
        (s, "")
    };

    let id = id_part.trim().trim_matches('"').to_string();
    let (label, size) = parse_label_size(rest);
    (id, label, size)
}

/// Parse "[\"Label\"]:size" or ":size" or "[\"Label\"]" or ""
fn parse_label_size(s: &str) -> (Option<String>, Option<f64>) {
    let s = s.trim();
    let mut label = None;
    let mut rest = s;

    if rest.starts_with('[') {
        if let Some(end) = rest.find(']') {
            label = Some(rest[1..end].trim().trim_matches('"').to_string());
            rest = &rest[end + 1..];
        }
    }

    let size = if let Some(size_str) = rest.strip_prefix(':') {
        size_str.trim().parse::<f64>().ok()
    } else {
        None
    };

    (label, size)
}

/// Split at the first '[' or ':' that follows comma-separated IDs
fn split_at_bracket_or_colon(s: &str) -> Option<(&str, &str)> {
    // Find end of comma-separated IDs
    for (i, ch) in s.char_indices() {
        if ch == '[' || ch == ':' {
            return Some((&s[..i], &s[i..]));
        }
    }
    // All IDs, no label/size
    Some((s, ""))
}

fn make_err(input: &str, line_no: usize) -> ParseError {
    let offset: usize = input.lines().take(line_no).map(|l| l.len() + 1).sum();
    ParseError::new(ParseErrorKind::UnexpectedToken, offset..offset + 1, input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        let d = parse("venn-beta\n  set A\n  set B\n  union A,B").unwrap();
        assert_eq!(d.sets.len(), 2);
        assert_eq!(d.unions.len(), 1);
        assert_eq!(d.unions[0].set_ids, vec!["A", "B"]);
    }

    #[test]
    fn parse_with_labels_and_sizes() {
        let d = parse("venn-beta\n  set A[\"Alpha\"]:20\n  set B[\"Beta\"]:12\n  union A,B[\"Both\"]:5").unwrap();
        assert_eq!(d.sets[0].label, "Alpha");
        assert!((d.sets[0].size - 20.0).abs() < f64::EPSILON);
        assert_eq!(d.unions[0].label.as_deref(), Some("Both"));
        assert!((d.unions[0].size - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_title() {
        let d = parse("venn-beta\n  title My Venn\n  set A\n  set B").unwrap();
        assert_eq!(d.title.as_deref(), Some("My Venn"));
    }

    #[test]
    fn parse_three_sets() {
        let d = parse("venn-beta\n  set A:30\n  set B:20\n  set C:15\n  union A,B:8\n  union B,C:5\n  union A,B,C:2").unwrap();
        assert_eq!(d.sets.len(), 3);
        assert_eq!(d.unions.len(), 3);
        assert_eq!(d.unions[2].set_ids.len(), 3);
    }

    #[test]
    fn parse_venn_header() {
        assert!(parse("venn-beta\n  set A").is_ok());
    }

    #[test]
    fn reject_no_header() {
        assert!(parse("set A\nset B").is_err());
    }

    #[test]
    fn default_sizes() {
        let d = parse("venn-beta\n  set A\n  set B").unwrap();
        assert!((d.sets[0].size - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn empty_diagram_ok() {
        let d = parse("venn-beta").unwrap();
        assert!(d.sets.is_empty());
    }

    #[test]
    fn union_without_sets_ok() {
        let d = parse("venn-beta\n  union X,Y").unwrap();
        assert_eq!(d.unions.len(), 1);
        assert!(d.sets.is_empty());
    }

    #[test]
    fn reject_wrong_header() {
        assert!(parse("pie\n  set A").is_err());
    }
}
