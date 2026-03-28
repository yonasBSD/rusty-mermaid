use super::ir::{TreeNode, TreeView};
use crate::common::error::{ParseError, ParseErrorKind};

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
        return Err(ParseError::new(
            ParseErrorKind::UnexpectedToken,
            0..input.len().min(10),
            input,
        ));
    }

    // Normalize indents to levels (find the indent unit)
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

    #[test]
    fn reject_wrong_header() {
        assert!(parse("mindmap\n    a").is_err());
    }

    #[test]
    fn empty_tree_ok() {
        let t = parse("treeView-beta").unwrap();
        assert!(t.roots.is_empty());
    }

    #[test]
    fn single_node_is_root() {
        let t = parse("treeView-beta\n    solo").unwrap();
        assert_eq!(t.roots.len(), 1);
        assert!(t.roots[0].children.is_empty());
    }

    #[test]
    fn deep_nesting_five_levels() {
        let t = parse(
            "treeView-beta\n    a\n        b\n            c\n                d\n                    e",
        )
        .unwrap();
        let mut node = &t.roots[0];
        for expected in ["a", "b", "c", "d", "e"] {
            assert_eq!(node.name, expected);
            if expected != "e" {
                node = &node.children[0];
            }
        }
        assert!(node.children.is_empty());
    }

    #[test]
    fn special_chars_in_names() {
        let t = parse("treeView-beta\n    src/main.rs\n        fn main() {}").unwrap();
        assert_eq!(t.roots[0].name, "src/main.rs");
        assert_eq!(t.roots[0].children[0].name, "fn main() {}");
    }

    #[test]
    fn siblings_at_same_depth() {
        let t = parse("treeView-beta\n    root\n        child1\n        child2\n        child3")
            .unwrap();
        assert_eq!(t.roots[0].children.len(), 3);
        assert_eq!(t.roots[0].children[2].name, "child3");
    }

    #[test]
    fn mixed_depths_under_root() {
        let t = parse(
            "treeView-beta\n    root\n        a\n            a1\n        b\n            b1\n            b2",
        )
        .unwrap();
        assert_eq!(t.roots[0].children.len(), 2);
        assert_eq!(t.roots[0].children[0].children.len(), 1);
        assert_eq!(t.roots[0].children[1].children.len(), 2);
    }

    #[test]
    fn node_count_complex_tree() {
        let t = parse("treeView-beta\n    r\n        a\n            a1\n        b").unwrap();
        assert_eq!(t.node_count(), 4);
    }
}
