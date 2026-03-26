use crate::common::error::{ParseError, ParseErrorKind};
use crate::common::tokens::skip;
use winnow::prelude::*;

use super::ir::*;

pub fn parse(input: &str) -> Result<KanbanBoard, ParseError> {
    let mut rest = input;
    parse_kanban(&mut rest).map_err(|_| {
        let offset = input.len() - rest.len();
        ParseError::new(ParseErrorKind::UnexpectedToken, offset..offset, input)
    })
}

fn parse_kanban(input: &mut &str) -> ModalResult<KanbanBoard> {
    skip.parse_next(input)?;
    "kanban".parse_next(input)?;

    let mut board = KanbanBoard::new();
    let mut current_column: Option<KanbanColumn> = None;
    let mut col_indent: Option<usize> = None;

    loop {
        if input.is_empty() {
            break;
        }

        let line = take_line(input);
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("%%") {
            continue;
        }

        let indent = count_indent(line);

        // First non-empty line sets the column indent level
        let col_level = *col_indent.get_or_insert(indent);
        let is_column = indent <= col_level;

        if is_column {
            // Column header — flush previous
            if let Some(col) = current_column.take() {
                board.columns.push(col);
            }
            let (id, label) = parse_node_id_label(trimmed);
            current_column = Some(KanbanColumn {
                id,
                label,
                cards: Vec::new(),
            });
        } else {
            // Card within current column
            let (id, label) = parse_node_id_label(trimmed);
            let mut card = KanbanCard::new(id, label);

            // Check for inline metadata @{ ... }
            if let Some(meta_start) = trimmed.find("@{") {
                if let Some(meta_end) = trimmed[meta_start..].find('}') {
                    let meta = &trimmed[meta_start + 2..meta_start + meta_end];
                    parse_metadata(meta, &mut card);
                }
            }

            if let Some(col) = &mut current_column {
                col.cards.push(card);
            }
        }
    }

    // Flush last column
    if let Some(col) = current_column {
        board.columns.push(col);
    }

    Ok(board)
}

fn parse_node_id_label(content: &str) -> (String, String) {
    // Remove metadata @{...} if present
    let content = if let Some(idx) = content.find("@{") {
        content[..idx].trim()
    } else {
        content
    };

    // id[label] or id(label) or just id
    if let Some(bracket_start) = content.find('[') {
        if let Some(bracket_end) = content.rfind(']') {
            let id = content[..bracket_start].trim();
            let label = &content[bracket_start + 1..bracket_end];
            return (id.to_string(), label.to_string());
        }
    }
    if let Some(paren_start) = content.find('(') {
        if let Some(paren_end) = content.rfind(')') {
            let id = content[..paren_start].trim();
            let label = &content[paren_start + 1..paren_end];
            return (id.to_string(), label.to_string());
        }
    }
    // Plain text: id = label
    let id = content.split_whitespace().next().unwrap_or(content);
    (id.to_string(), content.to_string())
}

fn parse_metadata(meta: &str, card: &mut KanbanCard) {
    for part in meta.split(',') {
        let part = part.trim();
        if let Some((key, val)) = part.split_once(':') {
            let key = key.trim().to_ascii_lowercase();
            let val = val.trim().trim_matches('"').trim_matches('\'');
            match key.as_str() {
                "priority" => card.priority = parse_priority(val),
                "assigned" => card.assigned = Some(val.to_string()),
                "ticket" => card.ticket = Some(val.to_string()),
                _ => {}
            }
        }
    }
}

fn parse_priority(s: &str) -> Option<Priority> {
    match s.to_ascii_lowercase().as_str() {
        "very high" => Some(Priority::VeryHigh),
        "high" => Some(Priority::High),
        "medium" => Some(Priority::Medium),
        "low" => Some(Priority::Low),
        "very low" => Some(Priority::VeryLow),
        _ => None,
    }
}

fn count_indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn take_line<'i>(input: &mut &'i str) -> &'i str {
    let end = input.find('\n').unwrap_or(input.len());
    let line = &input[..end];
    *input = if end < input.len() {
        &input[end + 1..]
    } else {
        ""
    };
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        let b = parse("kanban\n    Todo\n        task1[Buy groceries]\n        task2[Clean house]\n    Done\n        task3[Laundry]").unwrap();
        assert_eq!(b.columns.len(), 2);
        assert_eq!(b.columns[0].label, "Todo");
        assert_eq!(b.columns[0].cards.len(), 2);
        assert_eq!(b.columns[0].cards[0].label, "Buy groceries");
        assert_eq!(b.columns[1].label, "Done");
        assert_eq!(b.columns[1].cards.len(), 1);
    }

    #[test]
    fn parse_rounded_brackets() {
        let b = parse("kanban\n    Col\n        t1(Round card)").unwrap();
        assert_eq!(b.columns[0].cards[0].label, "Round card");
    }

    #[test]
    fn parse_plain_text() {
        let b = parse("kanban\n    Backlog\n        Fix bug\n        Add feature").unwrap();
        assert_eq!(b.columns[0].cards.len(), 2);
        assert_eq!(b.columns[0].cards[0].label, "Fix bug");
    }

    #[test]
    fn parse_metadata() {
        let b = parse("kanban\n    Todo\n        task1[Do it] @{priority: high, assigned: alice, ticket: ABC-123}").unwrap();
        let card = &b.columns[0].cards[0];
        assert_eq!(card.priority, Some(Priority::High));
        assert_eq!(card.assigned.as_deref(), Some("alice"));
        assert_eq!(card.ticket.as_deref(), Some("ABC-123"));
    }

    #[test]
    fn parse_comments() {
        let b = parse("kanban\n    %% comment\n    Col\n        card").unwrap();
        assert_eq!(b.columns.len(), 1);
    }

    #[test]
    fn parse_empty_columns() {
        let b = parse("kanban\n    Empty\n    Also Empty").unwrap();
        assert_eq!(b.columns.len(), 2);
        assert!(b.columns[0].cards.is_empty());
    }

    #[test]
    fn reject_empty() {
        assert!(parse("").is_err());
    }

    #[test]
    fn reject_wrong_header() {
        assert!(parse("timeline\n    X").is_err());
    }
}
