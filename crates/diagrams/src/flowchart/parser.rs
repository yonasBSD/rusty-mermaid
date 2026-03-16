use rusty_mermaid_core::{Direction, Shape};
use winnow::combinator::{alt, opt};
use winnow::prelude::*;
use winnow::token::{any, take_while};

use crate::common::error::{ParseError, ParseErrorKind};
use crate::common::styling::{class_apply_body, class_def_body, style_stmt_body};
use crate::common::tokens::{direction, node_id, quoted_string, skip, style_class, text_until, ws};

use super::ir::*;

/// Parse a complete flowchart from mermaid text.
pub fn parse(input: &str) -> Result<FlowDiagram, ParseError> {
    let mut rest = input;
    parse_flowchart(&mut rest).map_err(|_| {
        let offset = input.len() - rest.len();
        ParseError::new(ParseErrorKind::UnexpectedToken, offset..offset, input)
    })
}

/// Top-level: `graph DIR` or `flowchart DIR`, then statements.
fn parse_flowchart(input: &mut &str) -> ModalResult<FlowDiagram> {
    skip.parse_next(input)?;
    let dir = header(input)?;
    let mut diagram = FlowDiagram::new(dir);

    skip.parse_next(input)?;
    parse_statements(input, &mut diagram, None)?;

    Ok(diagram)
}

/// Parse the header: `graph TD` or `flowchart LR`.
fn header(input: &mut &str) -> ModalResult<Direction> {
    let _keyword = alt(("flowchart", "graph")).parse_next(input)?;
    ws.parse_next(input)?;
    direction(input)
}

/// Parse statements until EOF or `end` keyword.
/// If `subgraph_id` is Some, we're inside a subgraph and stop at `end`.
fn parse_statements(
    input: &mut &str,
    diagram: &mut FlowDiagram,
    subgraph_id: Option<&str>,
) -> ModalResult<()> {
    loop {
        skip.parse_next(input)?;
        if input.is_empty() {
            return Ok(());
        }

        // Check for `end` keyword (closes subgraph)
        if let Some(_sg_id) = subgraph_id {
            if input.starts_with("end") {
                let after = &input[3..];
                // `end` must be followed by whitespace, newline, EOF, or comment
                if after.is_empty()
                    || after.starts_with(|c: char| c.is_ascii_whitespace())
                    || after.starts_with("%%")
                {
                    *input = after;
                    return Ok(());
                }
            }
        }

        // Try each statement type
        if input.starts_with("subgraph") {
            parse_subgraph(input, diagram, subgraph_id)?;
        } else if input.starts_with("classDef") {
            *input = &input[8..];
            let cd = class_def_body.parse_next(input)?;
            diagram.class_defs.push(cd);
        } else if input.starts_with("style ") {
            *input = &input[5..];
            let ss = style_stmt_body.parse_next(input)?;
            diagram.style_stmts.push(FlowStyleStmt {
                ids: ss.ids,
                styles: ss.styles,
            });
        } else if input.starts_with("class ") {
            *input = &input[5..];
            let ca = class_apply_body.parse_next(input)?;
            // Apply classes to vertices
            for id in &ca.ids {
                if let Some(v) = diagram.vertices.iter_mut().find(|v| v.id == *id) {
                    v.classes.push(ca.class_name.clone());
                }
            }
        } else if input.starts_with("direction") {
            // Subgraph-level direction override — not yet wired
            *input = &input[9..];
            ws.parse_next(input)?;
            let _dir = direction(input)?;
        } else {
            // Must be a node/edge statement
            parse_node_edge_statement(input, diagram, subgraph_id)?;
        }
    }
}

/// Parse `subgraph id[Label] ... end` or `subgraph Title ... end`.
fn parse_subgraph(
    input: &mut &str,
    diagram: &mut FlowDiagram,
    parent_sg: Option<&str>,
) -> ModalResult<()> {
    // Consume "subgraph"
    "subgraph".parse_next(input)?;
    ws.parse_next(input)?;

    // Parse subgraph ID and optional label
    let (sg_id, sg_label) = parse_subgraph_header(input)?;

    let sg = FlowSubGraph {
        id: sg_id.clone(),
        label: sg_label,
        direction: None,
        node_ids: Vec::new(),
        subgraph_ids: Vec::new(),
    };
    diagram.subgraphs.push(sg);

    // Register this subgraph as a child of parent
    if let Some(parent) = parent_sg {
        if let Some(p) = diagram.subgraphs.iter_mut().find(|s| s.id == parent) {
            p.subgraph_ids.push(sg_id.clone());
        }
    }

    // Parse inner statements
    parse_statements(input, diagram, Some(&sg_id))?;

    Ok(())
}

/// Parse subgraph header: `id[Label]`, `id["Label"]`, or just `Title Text`.
fn parse_subgraph_header<'i>(input: &mut &'i str) -> ModalResult<(String, Option<String>)> {
    // Try: identifier followed by [label]
    let checkpoint = *input;
    if let Ok(id) = node_id.parse_next(input) {
        // Check for [label]
        if input.starts_with('[') {
            *input = &input[1..];
            let label = text_until(']', input)?;
            ']'.parse_next(input)?;
            return Ok((id.to_string(), Some(label.to_string())));
        }
        // Check for ["label"]
        if input.starts_with("[\"") {
            *input = &input[1..];
            let label = quoted_string(input)?;
            ']'.parse_next(input)?;
            return Ok((id.to_string(), Some(label.to_string())));
        }
        // No bracket — could be `subgraph Title Text`
        // Check if the rest of the line (before newline) is more text
        let remaining = take_while(0.., |c: char| c != '\n' && c != '\r')
            .parse_next(input)?;
        let full_title = format!("{}{}", id, remaining).trim().to_string();
        // Use a sanitized version as ID
        let sg_id = full_title.replace(' ', "_");
        return Ok((sg_id, Some(full_title)));
    }

    // Fallback: quoted string as title
    *input = checkpoint;
    if let Ok(title) = quoted_string(input) {
        let sg_id = title.replace(' ', "_");
        return Ok((sg_id, Some(title.to_string())));
    }

    Err(winnow::error::ErrMode::Backtrack(
        winnow::error::ContextError::new(),
    ))
}

/// Parse a node/edge statement like `A[Label] --> B[Label]` or `A --> B --> C`.
fn parse_node_edge_statement(
    input: &mut &str,
    diagram: &mut FlowDiagram,
    subgraph_id: Option<&str>,
) -> ModalResult<()> {
    // Parse first node
    let first_id = parse_node_ref(input, diagram, subgraph_id)?;

    // Check for chained edges: `A --> B --> C`
    let mut prev_id = first_id;
    loop {
        ws.parse_next(input)?;

        // Try to parse an edge operator
        let checkpoint = *input;
        if let Ok((label, stroke, start_arrow, end_arrow)) = parse_edge_operator(input) {
            ws.parse_next(input)?;
            let next_id = parse_node_ref(input, diagram, subgraph_id)?;

            diagram.edges.push(FlowEdge {
                src: prev_id.clone(),
                dst: next_id.clone(),
                label,
                stroke,
                start_arrow,
                end_arrow,
            });

            prev_id = next_id;
        } else {
            *input = checkpoint;
            break;
        }
    }

    Ok(())
}

/// Parse a node reference: `A`, `A[Label]`, `A{Label}`, `A[(Label)]`, etc.
/// Adds/updates the vertex in the diagram and returns the node ID.
fn parse_node_ref(
    input: &mut &str,
    diagram: &mut FlowDiagram,
    subgraph_id: Option<&str>,
) -> ModalResult<String> {
    let id = node_id(input)?;
    let id_str = id.to_string();

    // Try to parse a shape+label
    let shape_label = parse_node_shape(input);

    // Parse optional :::className
    let class = opt(style_class).parse_next(input)?;

    if let Ok((shape, label)) = shape_label {
        // Add or update vertex
        if let Some(v) = diagram.vertices.iter_mut().find(|v| v.id == id_str) {
            // Update label/shape if redefined
            v.label = label.clone();
            v.shape = shape;
        } else {
            let mut v = FlowVertex::new(&id_str, &label, shape);
            if let Some(c) = class {
                v.classes.push(c.to_string());
            }
            diagram.vertices.push(v);
        }
    } else if diagram.vertex(&id_str).is_none() {
        // Node referenced without shape — default to Rect with ID as label
        let mut v = FlowVertex::new(&id_str, &id_str, Shape::Rect);
        if let Some(c) = class {
            v.classes.push(c.to_string());
        }
        diagram.vertices.push(v);
    }

    // Register in subgraph
    if let Some(sg_id) = subgraph_id {
        if let Some(sg) = diagram.subgraphs.iter_mut().find(|s| s.id == sg_id) {
            if !sg.node_ids.contains(&id_str) {
                sg.node_ids.push(id_str.clone());
            }
        }
    }

    Ok(id_str)
}

/// Parse node shape delimiter and label text. Returns (Shape, label).
fn parse_node_shape(input: &mut &str) -> ModalResult<(Shape, String)> {
    let c = input.chars().next().ok_or_else(|| {
        winnow::error::ErrMode::Backtrack(winnow::error::ContextError::new())
    })?;

    match c {
        '[' => {
            *input = &input[1..];
            // Check for special multi-char openers: [( )] cylinder, [[ ]] subroutine
            if input.starts_with('(') {
                *input = &input[1..];
                let label = text_until(')', input)?;
                ")]".parse_next(input)?;
                Ok((Shape::Cylinder, label.to_string()))
            } else if input.starts_with('[') {
                *input = &input[1..];
                let label = text_until(']', input)?;
                "]]".parse_next(input)?;
                Ok((Shape::Subroutine, label.to_string()))
            } else if input.starts_with('/') {
                // Trapezoid [/text\] or lean right [/text/]
                *input = &input[1..];
                let label = text_until_trap(input)?;
                Ok((Shape::Trapezoid, label))
            } else if input.starts_with('\\') {
                // Inv trapezoid [\text/] or lean left [\text\]
                *input = &input[1..];
                let label = text_until_trap(input)?;
                Ok((Shape::TrapezoidAlt, label))
            } else {
                // Regular rect [text] or quoted ["text"]
                let label = if input.starts_with('"') {
                    let s = quoted_string(input)?;
                    ']'.parse_next(input)?;
                    s.to_string()
                } else {
                    let s = text_until(']', input)?;
                    ']'.parse_next(input)?;
                    s.to_string()
                };
                Ok((Shape::Rect, label))
            }
        }
        '(' => {
            *input = &input[1..];
            if input.starts_with('[') {
                // Stadium ([text])
                *input = &input[1..];
                let label = text_until(']', input)?;
                "])".parse_next(input)?;
                Ok((Shape::Stadium, label.to_string()))
            } else if input.starts_with('(') {
                // Circle ((text)) or double circle (((text)))
                *input = &input[1..];
                if input.starts_with('(') {
                    *input = &input[1..];
                    let label = text_until(')', input)?;
                    ")))".parse_next(input)?;
                    Ok((Shape::DoubleCircle, label.to_string()))
                } else {
                    let label = text_until(')', input)?;
                    "))".parse_next(input)?;
                    Ok((Shape::Circle, label.to_string()))
                }
            } else {
                // Rounded rect (text)
                let label = text_until(')', input)?;
                ')'.parse_next(input)?;
                Ok((Shape::RoundedRect, label.to_string()))
            }
        }
        '{' => {
            *input = &input[1..];
            if input.starts_with('{') {
                // Hexagon {{text}}
                *input = &input[1..];
                let label = text_until('}', input)?;
                "}}".parse_next(input)?;
                Ok((Shape::Hexagon, label.to_string()))
            } else {
                // Diamond {text}
                let label = text_until('}', input)?;
                '}'.parse_next(input)?;
                Ok((Shape::Diamond, label.to_string()))
            }
        }
        '>' => {
            // Odd shape >text]
            *input = &input[1..];
            let label = text_until(']', input)?;
            ']'.parse_next(input)?;
            Ok((Shape::Rect, label.to_string())) // Map odd to Rect for now
        }
        _ => Err(winnow::error::ErrMode::Backtrack(
            winnow::error::ContextError::new(),
        )),
    }
}

/// Parse trapezoid label text until `\]` or `/]`.
fn text_until_trap(input: &mut &str) -> ModalResult<String> {
    let content = take_while(0.., |c: char| c != '\\' && c != '/' && c != ']')
        .parse_next(input)?;
    // Consume closing: `\]` or `/]`
    any.parse_next(input)?; // `\` or `/`
    ']'.parse_next(input)?;
    Ok(content.to_string())
}

/// Parse an edge operator and optional label.
/// Returns (label, stroke, start_arrow, end_arrow).
fn parse_edge_operator(
    input: &mut &str,
) -> ModalResult<(Option<String>, StrokeType, ArrowEnd, ArrowEnd)> {
    // Detect start arrow: `<`, `o`, `x`
    let start_arrow = parse_start_arrow(input);

    // Detect stroke type from first chars
    let (stroke, label) = if input.starts_with("-.") {
        // Dotted: `-.->` or `-. text .->`
        *input = &input[2..];
        let label = parse_inline_edge_label(input, ".-")?;
        take_while(0.., |c: char| c == '.' || c == '-').parse_next(input)?;
        (StrokeType::Dotted, label)
    } else if input.starts_with("==") {
        // Thick: `==>` or `== text ==>`
        *input = &input[2..];
        let label = parse_inline_edge_label(input, "=")?;
        take_while(0.., |c: char| c == '=').parse_next(input)?;
        (StrokeType::Thick, label)
    } else if input.starts_with("--") {
        // Normal: `-->` or `-- text -->` or `---`
        *input = &input[2..];
        let label = parse_inline_edge_label(input, "-")?;
        take_while(0.., |c: char| c == '-').parse_next(input)?;
        (StrokeType::Normal, label)
    } else {
        return Err(winnow::error::ErrMode::Backtrack(
            winnow::error::ContextError::new(),
        ));
    };

    // End arrow
    let end_arrow = parse_end_arrow(input);

    // Pipe-delimited label: `-->|text|`
    let label = if label.is_some() {
        label
    } else if input.starts_with('|') {
        *input = &input[1..];
        let text = text_until('|', input)?;
        '|'.parse_next(input)?;
        Some(text.to_string())
    } else {
        None
    };

    Ok((label, stroke, start_arrow, end_arrow))
}

fn parse_start_arrow(input: &mut &str) -> ArrowEnd {
    if input.starts_with('<') {
        *input = &input[1..];
        ArrowEnd::Arrow
    } else if input.starts_with("o-") || input.starts_with("o=") || input.starts_with("o.") {
        *input = &input[1..];
        ArrowEnd::Circle
    } else if input.starts_with("x-") || input.starts_with("x=") || input.starts_with("x.") {
        *input = &input[1..];
        ArrowEnd::Cross
    } else {
        ArrowEnd::None
    }
}

fn parse_end_arrow(input: &mut &str) -> ArrowEnd {
    if input.starts_with('>') {
        *input = &input[1..];
        ArrowEnd::Arrow
    } else if input.starts_with('x') {
        *input = &input[1..];
        ArrowEnd::Cross
    } else if input.starts_with('o') {
        *input = &input[1..];
        ArrowEnd::Circle
    } else {
        ArrowEnd::None
    }
}

/// Try to parse an inline edge label: `-- text -->` (text between dashes).
/// Returns None if there's no label (just more dashes/dots/equals).
/// `stop_chars` contains characters that signal "no label, continue to arrow".
fn parse_inline_edge_label(input: &mut &str, stop_chars: &str) -> ModalResult<Option<String>> {
    if input.is_empty() {
        return Ok(None);
    }
    let next = input.chars().next().unwrap();
    // If next char is a stop char or arrow endpoint, no label
    if stop_chars.contains(next) || next == '>' || next == 'x' || next == 'o' {
        return Ok(None);
    }

    // There's a label: consume until we hit a stop char
    ws.parse_next(input)?;
    let mut label = String::new();
    while !input.is_empty() {
        let c = input.chars().next().unwrap();
        if stop_chars.contains(c) {
            break;
        }
        label.push(c);
        *input = &input[c.len_utf8()..];
    }
    let label = label.trim().to_string();
    if label.is_empty() {
        Ok(None)
    } else {
        Ok(Some(label))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_node() {
        let d = parse("graph TD\n    A[Only Node]").unwrap();
        assert_eq!(d.direction, Direction::TB);
        assert_eq!(d.vertices.len(), 1);
        assert_eq!(d.vertices[0].id, "A");
        assert_eq!(d.vertices[0].label, "Only Node");
        assert_eq!(d.vertices[0].shape, Shape::Rect);
    }

    #[test]
    fn parse_linear_chain() {
        let d = parse("graph TD\n    A[Start] --> B[Middle] --> C[End]").unwrap();
        assert_eq!(d.vertices.len(), 3);
        assert_eq!(d.edges.len(), 2);
        assert_eq!(d.edges[0].src, "A");
        assert_eq!(d.edges[0].dst, "B");
        assert_eq!(d.edges[1].src, "B");
        assert_eq!(d.edges[1].dst, "C");
    }

    #[test]
    fn parse_diamond() {
        let d = parse("graph TD\n    A{Decision?}").unwrap();
        assert_eq!(d.vertices[0].shape, Shape::Diamond);
        assert_eq!(d.vertices[0].label, "Decision?");
    }

    #[test]
    fn parse_cylinder() {
        let d = parse("graph TD\n    A[(Database)]").unwrap();
        assert_eq!(d.vertices[0].shape, Shape::Cylinder);
    }

    #[test]
    fn parse_edge_label_pipe() {
        let d = parse("graph TD\n    A -->|yes| B").unwrap();
        assert_eq!(d.edges[0].label.as_deref(), Some("yes"));
    }

    #[test]
    fn parse_dotted_edge() {
        let d = parse("graph TD\n    A -.-> B").unwrap();
        assert_eq!(d.edges[0].stroke, StrokeType::Dotted);
        assert_eq!(d.edges[0].end_arrow, ArrowEnd::Arrow);
    }

    #[test]
    fn parse_flowchart_lr() {
        let d = parse("flowchart LR\n    A --> B").unwrap();
        assert_eq!(d.direction, Direction::LR);
    }

    #[test]
    fn parse_quoted_label() {
        let d = parse("graph TD\n    A[\"<b>Bold</b>\"]").unwrap();
        assert_eq!(d.vertices[0].label, "<b>Bold</b>");
    }

    #[test]
    fn parse_subgraph() {
        let d = parse(
            "graph TD\n    subgraph cluster[Group]\n        A --> B\n    end\n    B --> C",
        )
        .unwrap();
        assert_eq!(d.subgraphs.len(), 1);
        assert_eq!(d.subgraphs[0].id, "cluster");
        assert_eq!(d.subgraphs[0].label.as_deref(), Some("Group"));
        assert!(d.subgraphs[0].node_ids.contains(&"A".to_string()));
        assert!(d.subgraphs[0].node_ids.contains(&"B".to_string()));
    }

    #[test]
    fn parse_subgraph_unbracketed_title() {
        let d = parse("graph TD\n    subgraph Frontend\n        A --> B\n    end").unwrap();
        assert_eq!(d.subgraphs[0].label.as_deref(), Some("Frontend"));
    }

    #[test]
    fn parse_nested_subgraphs() {
        let input = "\
graph TD
    subgraph outer[Outer]
        subgraph inner[Inner]
            A --> B
        end
        C
    end";
        let d = parse(input).unwrap();
        assert_eq!(d.subgraphs.len(), 2);
        let outer = d.subgraphs.iter().find(|s| s.id == "outer").unwrap();
        assert!(outer.subgraph_ids.contains(&"inner".to_string()));
    }

    #[test]
    fn parse_comments_ignored() {
        let d = parse("graph TD\n    %% This is a comment\n    A --> B").unwrap();
        assert_eq!(d.vertices.len(), 2);
        assert_eq!(d.edges.len(), 1);
    }

    #[test]
    fn parse_node_reuse_without_shape() {
        let d = parse("graph TD\n    A[Start] --> B\n    B --> C[End]").unwrap();
        assert_eq!(d.vertices.len(), 3);
        // B should exist with default shape
        let b = d.vertex("B").unwrap();
        assert_eq!(b.shape, Shape::Rect);
    }

    #[test]
    fn parse_self_loop() {
        let d = parse("graph TD\n    A[Node] --> B\n    A --> A").unwrap();
        let self_edge = d.edges.iter().find(|e| e.src == "A" && e.dst == "A");
        assert!(self_edge.is_some());
    }

    #[test]
    fn parse_realistic_flowchart() {
        let input = "\
graph TD
    start[Start] --> input[Get Input]
    input --> validate{Valid?}
    validate -->|No| error[Show Error]
    validate -->|Yes| process[Process Data]
    error --> input
    process --> decide{Choose Path}
    decide -->|A| optA[Option A]
    decide -->|B| optB[Option B]
    optA --> merge[Merge]
    optB --> merge
    merge --> output[Output Result]
    output --> done[End]";
        let d = parse(input).unwrap();
        assert_eq!(d.vertices.len(), 11);
        assert_eq!(d.edges.len(), 12);
        assert_eq!(d.vertex("validate").unwrap().shape, Shape::Diamond);
    }

    #[test]
    fn parse_all_directions() {
        for (keyword, expected) in [
            ("graph TB", Direction::TB),
            ("graph TD", Direction::TB),
            ("graph BT", Direction::BT),
            ("graph LR", Direction::LR),
            ("graph RL", Direction::RL),
        ] {
            let d = parse(&format!("{keyword}\n    A --> B")).unwrap();
            assert_eq!(d.direction, expected, "failed for {keyword}");
        }
    }
}
