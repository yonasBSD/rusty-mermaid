use crate::common::error::{ParseError, ParseErrorKind};
use super::ir::*;

pub fn parse(input: &str) -> Result<ArchDiagram, ParseError> {
    let mut diagram = ArchDiagram::default();
    let mut header_found = false;

    for (_line_no, raw_line) in input.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with("%%") { continue; }

        if !header_found {
            if line.starts_with("architecture") {
                header_found = true;
                continue;
            }
            return Err(ParseError::new(ParseErrorKind::UnexpectedToken, 0..1, input));
        }

        if let Some(rest) = line.strip_prefix("group ") {
            if let Some(g) = parse_group(rest) {
                diagram.groups.push(g);
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix("service ") {
            if let Some(s) = parse_service(rest) {
                diagram.services.push(s);
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix("junction ") {
            if let Some(j) = parse_junction(rest) {
                diagram.junctions.push(j);
            }
            continue;
        }

        // Edge: id:DIR <--/--> DIR:id
        if line.contains("--") {
            if let Some(e) = parse_edge(line) {
                diagram.edges.push(e);
            }
        }
    }

    if !header_found {
        return Err(ParseError::new(ParseErrorKind::UnexpectedToken, 0..input.len().min(10), input));
    }

    Ok(diagram)
}

/// Parse "id(icon)[label] in parent"
fn parse_group(s: &str) -> Option<ArchGroup> {
    let (id, rest) = s.split_once('(')?;
    let (icon, rest) = rest.split_once(')')?;
    let label = rest.trim().strip_prefix('[').and_then(|r| r.split_once(']'))
        .map(|(l, r)| (l.to_string(), r))
        .unwrap_or_else(|| (id.trim().to_string(), rest));
    let parent = label.1.trim().strip_prefix("in ").map(|p| p.trim().to_string());
    Some(ArchGroup {
        id: id.trim().to_string(),
        icon: icon.trim().to_string(),
        label: label.0,
        parent,
    })
}

/// Parse "id(icon)[label] in group"
fn parse_service(s: &str) -> Option<ArchService> {
    let (id, rest) = s.split_once('(')?;
    let (icon, rest) = rest.split_once(')')?;
    let label = rest.trim().strip_prefix('[').and_then(|r| r.split_once(']'))
        .map(|(l, r)| (l.to_string(), r))
        .unwrap_or_else(|| (id.trim().to_string(), rest));
    let group = label.1.trim().strip_prefix("in ").map(|p| p.trim().to_string());
    Some(ArchService {
        id: id.trim().to_string(),
        icon: icon.trim().to_string(),
        label: label.0,
        group,
    })
}

/// Parse "id in group" or just "id"
fn parse_junction(s: &str) -> Option<ArchJunction> {
    let parts: Vec<&str> = s.splitn(3, ' ').collect();
    let id = parts[0].trim().to_string();
    let group = if parts.len() >= 3 && parts[1] == "in" {
        Some(parts[2].trim().to_string())
    } else { None };
    Some(ArchJunction { id, group })
}

/// Parse "from:DIR <--/--> DIR:to" or "from:DIR -- DIR:to"
fn parse_edge(line: &str) -> Option<ArchEdge> {
    // Split on the arrow pattern
    let (left, arrow, right) = if let Some(i) = line.find("-->") {
        (&line[..i], &line[i..i+3], &line[i+3..])
    } else if let Some(i) = line.find("<--") {
        (&line[..i], &line[i..i+3], &line[i+3..])
    } else if let Some(i) = line.find("--") {
        (&line[..i], &line[i..i+2], &line[i+2..])
    } else { return None };

    let arrow_right = arrow.contains('>');
    let arrow_left = arrow.contains('<');

    let (from_id, from_dir) = parse_endpoint(left.trim())?;
    let (to_id, to_dir) = parse_endpoint(right.trim())?;

    Some(ArchEdge { from: from_id, to: to_id, from_dir, to_dir, arrow_left, arrow_right })
}

/// Parse "id:DIR" or "DIR:id" — also handles "id{group}:DIR"
fn parse_endpoint(s: &str) -> Option<(String, Dir)> {
    let s = s.trim();
    // Strip {group} modifier
    let s = if let Some(brace) = s.find('{') {
        let end = s.find('}').unwrap_or(s.len());
        format!("{}{}", &s[..brace], &s[end+1..])
    } else { s.to_string() };

    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 { return None; }

    let (id, dir_str) = if parts[0].len() == 1 && "TBLR".contains(parts[0]) {
        (parts[1].trim(), parts[0].trim())
    } else {
        (parts[0].trim(), parts[1].trim())
    };

    let dir = match dir_str {
        "T" => Dir::T,
        "B" => Dir::B,
        "L" => Dir::L,
        "R" => Dir::R,
        _ => return None,
    };

    Some((id.to_string(), dir))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        let d = parse("architecture-beta\n  service db(database)[Database]\n  service srv(server)[Server]\n  db:R -- L:srv").unwrap();
        assert_eq!(d.services.len(), 2);
        assert_eq!(d.edges.len(), 1);
        assert_eq!(d.edges[0].from, "db");
        assert_eq!(d.edges[0].from_dir, Dir::R);
        assert_eq!(d.edges[0].to_dir, Dir::L);
    }

    #[test]
    fn parse_group() {
        let d = parse("architecture-beta\n  group api(cloud)[API]\n  service db(database)[DB] in api").unwrap();
        assert_eq!(d.groups.len(), 1);
        assert_eq!(d.groups[0].label, "API");
        assert_eq!(d.services[0].group.as_deref(), Some("api"));
    }

    #[test]
    fn parse_junction() {
        let d = parse("architecture-beta\n  junction mid in grp").unwrap();
        assert_eq!(d.junctions[0].id, "mid");
        assert_eq!(d.junctions[0].group.as_deref(), Some("grp"));
    }

    #[test]
    fn parse_arrows() {
        let d = parse("architecture-beta\n  service a(server)[A]\n  service b(server)[B]\n  a:R --> L:b").unwrap();
        assert!(d.edges[0].arrow_right);
        assert!(!d.edges[0].arrow_left);
    }

    #[test]
    fn reject_no_header() {
        assert!(parse("service a(server)[A]").is_err());
    }
}
