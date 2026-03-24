use crate::common::error::{ParseError, ParseErrorKind};
use super::ir::{TreeNode, TreeView};

pub fn parse(input: &str) -> Result<TreeView, ParseError> {
    let mut header_found = false;
    let mut entries: Vec<(usize, String)> = Vec::new(); // (indent_level, name)

    for (line_no, raw_line) in input.lines().enumerate() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with("%%") {
            continue;
        }

        if !header_found {
            if trimmed.starts_with("treeView") {
                header_found = true;
                continue;
            }
            return Err(make_err(input, line_no));
        }

        // Count leading spaces for indentation level
        let indent = raw_line.len() - raw_line.trim_start().len();
        let name = trimmed.trim_matches('"').to_string();
        if !name.is_empty() {
            entries.push((indent, name));
        }
    }

    if !header_found {
        return Err(ParseError::new(ParseErrorKind::UnexpectedToken, 0..input.len().min(10), input));
    }

    // Normalize indents to levels (find the indent unit)
    let min_indent = entries.iter().map(|(i, _)| *i).filter(|&i| i > 0).min().unwrap_or(4);
    let entries: Vec<(usize, String)> = entries
        .into_iter()
        .map(|(indent, name)| (indent / min_indent.max(1), name))
        .collect();

    let roots = build_tree(&entries);
    Ok(TreeView { roots })
}

fn build_tree(entries: &[(usize, String)]) -> Vec<TreeNode> {
    let min_depth = entries.iter().map(|(d, _)| *d).min().unwrap_or(0);
    build_children(entries, &mut 0, min_depth)
}

/// Recursively consume entries at `depth` or deeper, starting from `pos`.
fn build_children(entries: &[(usize, String)], pos: &mut usize, depth: usize) -> Vec<TreeNode> {
    let mut nodes = Vec::new();
    while *pos < entries.len() && entries[*pos].0 >= depth {
        if entries[*pos].0 == depth {
            let name = entries[*pos].1.clone();
            *pos += 1;
            let children = build_children(entries, pos, depth + 1);
            nodes.push(TreeNode { name, children });
        } else {
            // Deeper than expected — belongs to previous node's children,
            // but no parent at this depth yet. Skip to avoid infinite loop.
            *pos += 1;
        }
    }
    nodes
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
        let t = parse("treeView-beta\n    root\n        child1\n        child2").unwrap();
        assert_eq!(t.roots.len(), 1);
        assert_eq!(t.roots[0].children.len(), 2);
    }

    #[test]
    fn parse_multi_root() {
        let t = parse("treeView-beta\n    a\n    b\n    c").unwrap();
        assert_eq!(t.roots.len(), 3);
    }

    #[test]
    fn parse_deep_tree() {
        let t = parse("treeView-beta\n    a\n        b\n            c\n                d").unwrap();
        assert_eq!(t.roots[0].children[0].children[0].children[0].name, "d");
    }

    #[test]
    fn parse_quoted_names() {
        let t = parse("treeView-beta\n    \"folder name\"\n        \"file.ts\"").unwrap();
        assert_eq!(t.roots[0].name, "folder name");
    }

    #[test]
    fn parse_comments_blanks() {
        let t = parse("treeView-beta\n    %% comment\n\n    a\n        b").unwrap();
        assert_eq!(t.node_count(), 2);
    }

    #[test]
    fn reject_no_header() {
        assert!(parse("    a\n        b").is_err());
    }
}
