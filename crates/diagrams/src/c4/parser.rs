use crate::common::error::{ParseError, ParseErrorKind};
use super::ir::*;

pub fn parse(input: &str) -> Result<C4Diagram, ParseError> {
    let mut diagram = C4Diagram::default();
    let mut header_found = false;
    let mut boundary_stack: Vec<String> = Vec::new();

    for (_line_no, raw_line) in input.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        if !header_found {
            diagram.level = match line {
                l if l.starts_with("C4Context") => C4Level::Context,
                l if l.starts_with("C4Container") => C4Level::Container,
                l if l.starts_with("C4Component") => C4Level::Component,
                l if l.starts_with("C4Dynamic") => C4Level::Dynamic,
                _ => return Err(ParseError::new(ParseErrorKind::UnexpectedToken, 0..1, input)),
            };
            header_found = true;
            continue;
        }

        if let Some(rest) = line.strip_prefix("title") {
            diagram.title = Some(rest.trim().to_string());
            continue;
        }

        // Skip styling commands
        if line.starts_with("Update") || line.starts_with("AddProperty") {
            continue;
        }

        // Boundary open
        if let Some(boundary) = try_parse_boundary(line) {
            boundary_stack.push(boundary.alias.clone());
            diagram.boundaries.push(boundary);
            continue;
        }

        // Boundary close
        if line == "}" {
            boundary_stack.pop();
            continue;
        }

        // Relationship
        if let Some(rel) = try_parse_rel(line) {
            diagram.relationships.push(rel);
            continue;
        }

        // Element
        if let Some(mut elem) = try_parse_element(line) {
            elem.boundary = boundary_stack.last().cloned();
            diagram.elements.push(elem);
            continue;
        }
    }

    if !header_found {
        return Err(ParseError::new(ParseErrorKind::UnexpectedToken, 0..input.len().min(10), input));
    }

    Ok(diagram)
}

fn try_parse_boundary(line: &str) -> Option<C4Boundary> {
    let prefixes = ["System_Boundary", "Enterprise_Boundary", "Container_Boundary", "Boundary"];
    for prefix in prefixes {
        if let Some(rest) = line.strip_prefix(prefix) {
            let args = extract_args(rest.trim())?;
            let alias = args.first()?.clone();
            let label = args.get(1).cloned().unwrap_or_else(|| alias.clone());
            return Some(C4Boundary { alias, label });
        }
    }
    None
}

fn try_parse_rel(line: &str) -> Option<C4Rel> {
    let prefixes = ["BiRel", "Rel_U", "Rel_D", "Rel_L", "Rel_R", "Rel_B", "Rel"];
    for prefix in prefixes {
        if let Some(rest) = line.strip_prefix(prefix) {
            if rest.starts_with('(') || rest.starts_with('_') {
                // Rel_Back etc — skip the suffix
                let paren_rest = rest.find('(').map(|i| &rest[i..])?;
                let args = extract_args(paren_rest)?;
                if args.len() >= 3 {
                    return Some(C4Rel {
                        from: args[0].clone(),
                        to: args[1].clone(),
                        label: args[2].clone(),
                        technology: args.get(3).cloned(),
                    });
                }
            }
        }
    }
    None
}

fn try_parse_element(line: &str) -> Option<C4Element> {
    let patterns: &[(&str, C4Shape, bool)] = &[
        ("Person_Ext", C4Shape::Person, true),
        ("Person", C4Shape::Person, false),
        ("SystemDb_Ext", C4Shape::Database, true),
        ("SystemDb", C4Shape::Database, false),
        ("SystemQueue_Ext", C4Shape::Queue, true),
        ("SystemQueue", C4Shape::Queue, false),
        ("System_Ext", C4Shape::System, true),
        ("System", C4Shape::System, false),
        ("ContainerDb_Ext", C4Shape::Database, true),
        ("ContainerDb", C4Shape::Database, false),
        ("ContainerQueue_Ext", C4Shape::Queue, true),
        ("ContainerQueue", C4Shape::Queue, false),
        ("Container_Ext", C4Shape::Container, true),
        ("Container", C4Shape::Container, false),
        ("ComponentDb_Ext", C4Shape::Database, true),
        ("ComponentDb", C4Shape::Database, false),
        ("ComponentQueue_Ext", C4Shape::Queue, true),
        ("ComponentQueue", C4Shape::Queue, false),
        ("Component_Ext", C4Shape::Component, true),
        ("Component", C4Shape::Component, false),
    ];

    for &(prefix, shape, external) in patterns {
        if let Some(rest) = line.strip_prefix(prefix) {
            let rest = rest.trim();
            if rest.starts_with('(') {
                let args = extract_args(rest)?;
                let alias = args.first()?.clone();
                let label = args.get(1).cloned().unwrap_or_else(|| alias.clone());
                let technology = args.get(2).cloned();
                let description = args.get(3).cloned();
                return Some(C4Element {
                    alias, label, technology, description,
                    shape, external, boundary: None,
                });
            }
        }
    }
    None
}

/// Extract comma-separated args from "(arg1, arg2, ...)" or "(arg1, arg2, ...) {"
fn extract_args(s: &str) -> Option<Vec<String>> {
    let s = s.trim();
    let open = s.find('(')?;
    let close = s.rfind(')')?;
    if close <= open { return None; }
    let inner = &s[open + 1..close];
    Some(
        inner
            .split(',')
            .map(|a| a.trim().trim_matches('"').to_string())
            .filter(|a| !a.is_empty())
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_context() {
        let d = parse("C4Context\n  Person(user, \"User\", \"A user\")\n  System(sys, \"System\", \"The system\")\n  Rel(user, sys, \"Uses\")").unwrap();
        assert_eq!(d.level, C4Level::Context);
        assert_eq!(d.elements.len(), 2);
        assert_eq!(d.relationships.len(), 1);
    }

    #[test]
    fn parse_boundary() {
        let d = parse("C4Container\n  System_Boundary(bank, \"Bank\") {\n    Container(web, \"Web\")\n  }").unwrap();
        assert_eq!(d.boundaries.len(), 1);
        assert_eq!(d.elements[0].boundary.as_deref(), Some("bank"));
    }

    #[test]
    fn parse_shapes() {
        let d = parse("C4Context\n  Person(p, \"P\")\n  SystemDb(db, \"DB\")\n  SystemQueue(q, \"Q\")").unwrap();
        assert_eq!(d.elements[0].shape, C4Shape::Person);
        assert_eq!(d.elements[1].shape, C4Shape::Database);
        assert_eq!(d.elements[2].shape, C4Shape::Queue);
    }

    #[test]
    fn parse_external() {
        let d = parse("C4Context\n  System_Ext(ext, \"External\")").unwrap();
        assert!(d.elements[0].external);
    }

    #[test]
    fn parse_title() {
        let d = parse("C4Context\n  title My System\n  Person(u, \"User\")").unwrap();
        assert_eq!(d.title.as_deref(), Some("My System"));
    }

    #[test]
    fn reject_no_header() {
        assert!(parse("Person(u, \"User\")").is_err());
    }

    #[test]
    fn reject_wrong_header() {
        assert!(parse("flowchart TD\n  Person(u, \"U\")").is_err());
    }

    #[test]
    fn empty_c4_ok() {
        let d = parse("C4Context").unwrap();
        assert!(d.elements.is_empty());
    }

    #[test]
    fn parse_all_levels() {
        assert_eq!(parse("C4Context").unwrap().level, C4Level::Context);
        assert_eq!(parse("C4Container").unwrap().level, C4Level::Container);
        assert_eq!(parse("C4Component").unwrap().level, C4Level::Component);
        assert_eq!(parse("C4Dynamic").unwrap().level, C4Level::Dynamic);
    }
}
