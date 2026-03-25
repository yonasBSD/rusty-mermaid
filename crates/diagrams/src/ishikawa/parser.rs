use crate::common::error::{ParseError, ParseErrorKind};
use super::ir::{Category, Cause, IshikawaDiagram};

pub fn parse(input: &str) -> Result<IshikawaDiagram, ParseError> {
    let mut header_found = false;
    let mut entries: Vec<(usize, String)> = Vec::new();

    for (_line_no, raw_line) in input.lines().enumerate() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with("%%") {
            continue;
        }
        if !header_found {
            if trimmed.starts_with("ishikawa") {
                header_found = true;
                continue;
            }
            return Err(ParseError::new(ParseErrorKind::UnexpectedToken, 0..1, input));
        }
        let indent = raw_line.len() - raw_line.trim_start().len();
        entries.push((indent, trimmed.trim_matches('"').to_string()));
    }

    if !header_found {
        return Err(ParseError::new(ParseErrorKind::UnexpectedToken, 0..input.len().min(10), input));
    }
    if entries.is_empty() {
        return Err(ParseError::new(ParseErrorKind::UnexpectedEof, input.len()..input.len(), input));
    }

    // Normalize indents
    let min_indent = entries.iter().map(|(i, _)| *i).filter(|&i| i > 0).min().unwrap_or(4);
    let entries: Vec<(usize, String)> = entries
        .into_iter()
        .map(|(indent, name)| (indent / min_indent.max(1), name))
        .collect();

    // First entry at minimum depth = effect
    let min_depth = entries.iter().map(|(d, _)| *d).min().unwrap_or(0);
    let effect = entries[0].1.clone();

    // Remaining entries at min_depth = categories, deeper = causes
    let mut categories = Vec::new();
    let mut i = 1;
    while i < entries.len() {
        if entries[i].0 == min_depth {
            let cat_name = entries[i].1.clone();
            i += 1;
            let mut causes = Vec::new();
            while i < entries.len() && entries[i].0 > min_depth {
                let (cause, next_i) = parse_cause(&entries, i, min_depth + 1);
                causes.push(cause);
                i = next_i;
            }
            categories.push(Category { name: cat_name, causes });
        } else {
            i += 1;
        }
    }

    Ok(IshikawaDiagram { effect, categories })
}

fn parse_cause(entries: &[(usize, String)], pos: usize, depth: usize) -> (Cause, usize) {
    let name = entries[pos].1.clone();
    let mut subcauses = Vec::new();
    let mut i = pos + 1;
    while i < entries.len() && entries[i].0 > depth {
        if entries[i].0 == depth + 1 {
            let (sub, next_i) = parse_cause(entries, i, depth + 1);
            subcauses.push(sub);
            i = next_i;
        } else {
            i += 1;
        }
    }
    (Cause { name, subcauses }, i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        let d = parse("ishikawa-beta\n    Problem\n    Cat A\n        Cause 1\n        Cause 2\n    Cat B\n        Cause 3").unwrap();
        assert_eq!(d.effect, "Problem");
        assert_eq!(d.categories.len(), 2);
        assert_eq!(d.categories[0].causes.len(), 2);
        assert_eq!(d.categories[1].causes.len(), 1);
    }

    #[test]
    fn parse_nested_causes() {
        let d = parse("ishikawa-beta\n    Effect\n    Cat\n        Cause\n            SubA\n            SubB").unwrap();
        assert_eq!(d.categories[0].causes[0].subcauses.len(), 2);
    }

    #[test]
    fn parse_no_causes() {
        let d = parse("ishikawa-beta\n    Effect\n    Cat A\n    Cat B").unwrap();
        assert_eq!(d.categories.len(), 2);
        assert!(d.categories[0].causes.is_empty());
    }

    #[test]
    fn parse_comments() {
        let d = parse("ishikawa-beta\n    %% comment\n    Effect\n    Cat\n        C1").unwrap();
        assert_eq!(d.effect, "Effect");
    }

    #[test]
    fn reject_no_header() {
        assert!(parse("    Effect\n    Cat").is_err());
    }

    #[test]
    fn reject_empty() {
        assert!(parse("ishikawa-beta").is_err());
    }
}
