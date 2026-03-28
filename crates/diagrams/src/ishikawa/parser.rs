use super::ir::{Category, Cause, IshikawaDiagram};
use crate::common::error::{ParseError, ParseErrorKind};

pub fn parse(input: &str) -> Result<IshikawaDiagram, ParseError> {
    let mut header_found = false;
    let mut entries: Vec<(usize, String)> = Vec::new();

    for raw_line in input.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with("%%") {
            continue;
        }
        if !header_found {
            if trimmed.starts_with("ishikawa") {
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
        entries.push((indent, trimmed.trim_matches('"').to_string()));
    }

    if !header_found {
        return Err(ParseError::new(
            ParseErrorKind::UnexpectedToken,
            0..input.len().min(10),
            input,
        ));
    }
    if entries.is_empty() {
        return Err(ParseError::new(
            ParseErrorKind::UnexpectedEof,
            input.len()..input.len(),
            input,
        ));
    }

    // Normalize indents
    let min_indent = entries
        .iter()
        .map(|(i, _)| *i)
        .filter(|&i| i > 0)
        .min()
        .unwrap_or(4);
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
            categories.push(Category {
                name: cat_name,
                causes,
            });
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
        let d = parse(
            "ishikawa-beta\n    Effect\n    Cat\n        Cause\n            SubA\n            SubB",
        )
        .unwrap();
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

    #[test]
    fn single_category_no_causes() {
        let d = parse("ishikawa-beta\n    Defect\n    Materials").unwrap();
        assert_eq!(d.effect, "Defect");
        assert_eq!(d.categories.len(), 1);
        assert_eq!(d.categories[0].name, "Materials");
        assert!(d.categories[0].causes.is_empty());
    }

    #[test]
    fn special_chars_in_names() {
        let d = parse("ishikawa-beta\n    Bug #42\n    Cat & Dog\n        Cause: yes").unwrap();
        assert_eq!(d.effect, "Bug #42");
        assert_eq!(d.categories[0].name, "Cat & Dog");
        assert_eq!(d.categories[0].causes[0].name, "Cause: yes");
    }

    #[test]
    fn whitespace_only_lines_ignored() {
        let d = parse("ishikawa-beta\n    Effect\n    \n      \n    Cat\n        C1").unwrap();
        assert_eq!(d.categories.len(), 1);
        assert_eq!(d.categories[0].causes.len(), 1);
    }

    #[test]
    fn deeply_nested_subcauses() {
        let d = parse(
            "ishikawa-beta\n    Effect\n    Cat\n        L1\n            L2\n                L3",
        )
        .unwrap();
        let l1 = &d.categories[0].causes[0];
        assert_eq!(l1.name, "L1");
        assert_eq!(l1.subcauses[0].name, "L2");
        assert_eq!(l1.subcauses[0].subcauses[0].name, "L3");
        assert!(l1.subcauses[0].subcauses[0].subcauses.is_empty());
    }

    #[test]
    fn multiple_categories_mixed_depths() {
        let d = parse(
            "ishikawa-beta\n    Problem\n    People\n        Skill\n    Process\n    Machine\n        Wear\n            Bearing",
        )
        .unwrap();
        assert_eq!(d.categories.len(), 3);
        assert_eq!(d.categories[0].causes.len(), 1);
        assert!(d.categories[1].causes.is_empty());
        assert_eq!(d.categories[2].causes[0].subcauses.len(), 1);
    }

    #[test]
    fn quoted_names_stripped() {
        let d =
            parse("ishikawa-beta\n    \"My Effect\"\n    \"My Category\"\n        \"My Cause\"")
                .unwrap();
        assert_eq!(d.effect, "My Effect");
        assert_eq!(d.categories[0].name, "My Category");
        assert_eq!(d.categories[0].causes[0].name, "My Cause");
    }
}
