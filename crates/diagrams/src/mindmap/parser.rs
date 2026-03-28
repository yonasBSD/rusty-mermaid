use crate::common::error::{ParseError, ParseErrorKind};
use crate::common::tokens::skip;
use winnow::prelude::*;

use super::ir::*;

pub fn parse(input: &str) -> Result<MindmapDiagram, ParseError> {
    let mut rest = input;
    parse_mindmap(&mut rest).map_err(|_| {
        let offset = input.len() - rest.len();
        ParseError::new(ParseErrorKind::UnexpectedToken, offset..offset, input)
    })
}

fn parse_mindmap(input: &mut &str) -> ModalResult<MindmapDiagram> {
    skip.parse_next(input)?;
    "mindmap".parse_next(input)?;

    // Collect all lines with their indentation
    let mut lines: Vec<(usize, MindmapNode)> = Vec::new();

    loop {
        // Don't use skip — we need to preserve indentation
        skip_empty_and_comments(input);
        if input.is_empty() {
            break;
        }

        let raw_line = take_raw_line(input);
        let indent = count_indent(raw_line);
        let content = raw_line.trim();
        if content.is_empty() || content.starts_with("%%") {
            continue;
        }

        let mut node = parse_node_content(content);

        // Check for decorations on following lines
        loop {
            let checkpoint = *input;
            skip_empty_and_comments(input);
            if input.is_empty() {
                break;
            }
            let peek = input.trim_start_matches([' ', '\t']);
            if peek.starts_with("::icon(") {
                let raw = take_raw_line(input);
                let trimmed = raw.trim();
                if let Some(icon_content) = trimmed.strip_prefix("::icon(")
                    && let Some(icon) = icon_content.strip_suffix(')')
                {
                    node.icon = Some(icon.to_string());
                }
            } else if peek.starts_with(":::") {
                let raw = take_raw_line(input);
                let trimmed = raw.trim();
                if let Some(classes) = trimmed.strip_prefix(":::") {
                    for cls in classes.split_whitespace() {
                        node.css_classes.push(cls.to_string());
                    }
                }
            } else {
                *input = checkpoint;
                break;
            }
        }

        lines.push((indent, node));
    }

    if lines.is_empty() {
        return Err(winnow::error::ErrMode::Backtrack(
            winnow::error::ContextError::new(),
        ));
    }

    // Build tree from indentation levels
    let (_, root) = lines.remove(0);
    let root = build_tree(root, &mut lines, 0);

    Ok(MindmapDiagram { root })
}

fn build_tree(
    mut parent: MindmapNode,
    lines: &mut Vec<(usize, MindmapNode)>,
    parent_indent: usize,
) -> MindmapNode {
    while !lines.is_empty() {
        let (indent, _) = lines[0];
        if indent <= parent_indent {
            break; // Back to parent's level or above
        }
        let (child_indent, child_node) = lines.remove(0);
        let child = build_tree(child_node, lines, child_indent);
        parent.children.push(child);
    }
    parent
}

fn parse_node_content(content: &str) -> MindmapNode {
    let content = content.trim();

    // Shape detection by delimiters
    // Order matters: check multi-char delimiters first
    if content.starts_with("((") && content.ends_with("))") {
        let text = &content[2..content.len() - 2];
        return MindmapNode {
            shape: MindmapShape::Circle,
            ..MindmapNode::new(text.trim())
        };
    }
    if content.starts_with("))") && content.ends_with("((") {
        let text = &content[2..content.len() - 2];
        return MindmapNode {
            shape: MindmapShape::Bang,
            ..MindmapNode::new(text.trim())
        };
    }
    if content.starts_with("{{") && content.ends_with("}}") {
        let text = &content[2..content.len() - 2];
        return MindmapNode {
            shape: MindmapShape::Hexagon,
            ..MindmapNode::new(text.trim())
        };
    }
    if content.starts_with(')') && content.ends_with('(') {
        let text = &content[1..content.len() - 1];
        return MindmapNode {
            shape: MindmapShape::Cloud,
            ..MindmapNode::new(text.trim())
        };
    }
    if content.starts_with('[') && content.ends_with(']') {
        let text = &content[1..content.len() - 1];
        return MindmapNode {
            shape: MindmapShape::Rect,
            ..MindmapNode::new(text.trim())
        };
    }
    if content.starts_with('(') && content.ends_with(')') {
        let text = &content[1..content.len() - 1];
        return MindmapNode {
            shape: MindmapShape::RoundedRect,
            ..MindmapNode::new(text.trim())
        };
    }

    // Default: no shape delimiters
    MindmapNode::new(content)
}

fn count_indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn take_raw_line<'i>(input: &mut &'i str) -> &'i str {
    let end = input.find('\n').unwrap_or(input.len());
    let line = &input[..end];
    *input = if end < input.len() {
        &input[end + 1..]
    } else {
        ""
    };
    line
}

fn skip_empty_and_comments(input: &mut &str) {
    loop {
        if input.is_empty() {
            break;
        }
        let line_end = input.find('\n').unwrap_or(input.len());
        let line = input[..line_end].trim();
        if line.is_empty() || line.starts_with("%%") {
            *input = if line_end < input.len() {
                &input[line_end + 1..]
            } else {
                ""
            };
        } else {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        let d = parse("mindmap\n    Root\n        Child 1\n        Child 2").unwrap();
        assert_eq!(d.root.text, "Root");
        assert_eq!(d.root.children.len(), 2);
        assert_eq!(d.root.children[0].text, "Child 1");
    }

    #[test]
    fn parse_deep_tree() {
        let d =
            parse("mindmap\n    Root\n        A\n            A1\n                A1a\n        B")
                .unwrap();
        assert_eq!(d.root.children.len(), 2);
        assert_eq!(d.root.children[0].children[0].children[0].text, "A1a");
    }

    #[test]
    fn parse_shapes() {
        let d = parse("mindmap\n    Root\n        [Rect]\n        (Rounded)\n        ((Circle))\n        )Cloud(\n        ))Bang((\n        {{Hexagon}}").unwrap();
        assert_eq!(d.root.children.len(), 6);
        assert_eq!(d.root.children[0].shape, MindmapShape::Rect);
        assert_eq!(d.root.children[1].shape, MindmapShape::RoundedRect);
        assert_eq!(d.root.children[2].shape, MindmapShape::Circle);
        assert_eq!(d.root.children[3].shape, MindmapShape::Cloud);
        assert_eq!(d.root.children[4].shape, MindmapShape::Bang);
        assert_eq!(d.root.children[5].shape, MindmapShape::Hexagon);
    }

    #[test]
    fn parse_icon_decoration() {
        let d = parse("mindmap\n    Root\n        Child\n        ::icon(star)").unwrap();
        assert_eq!(d.root.children[0].icon.as_deref(), Some("star"));
    }

    #[test]
    fn parse_css_class() {
        let d = parse("mindmap\n    Root\n        Child\n        :::highlight bold").unwrap();
        assert_eq!(d.root.children[0].css_classes, vec!["highlight", "bold"]);
    }

    #[test]
    fn parse_comments() {
        let d = parse("mindmap\n    %% comment\n    Root\n        Child").unwrap();
        assert_eq!(d.root.text, "Root");
        assert_eq!(d.root.children.len(), 1);
    }

    #[test]
    fn parse_wide_tree() {
        let d = parse("mindmap\n    Center\n        A\n        B\n        C\n        D\n        E")
            .unwrap();
        assert_eq!(d.root.children.len(), 5);
    }

    #[test]
    fn total_node_count() {
        let d =
            parse("mindmap\n    Root\n        A\n            A1\n        B\n        C").unwrap();
        assert_eq!(d.root.count(), 5);
    }

    #[test]
    fn reject_empty() {
        assert!(parse("").is_err());
    }

    #[test]
    fn reject_wrong_header() {
        assert!(parse("gantt\n    title X").is_err());
    }

    #[test]
    fn reject_no_root() {
        assert!(parse("mindmap").is_err());
    }
}
