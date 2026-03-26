use rusty_mermaid_core::Direction;

use crate::common::error::{ParseError, ParseErrorKind};
use crate::common::tokens::skip;
use winnow::prelude::*;

use super::ir::*;

pub fn parse(input: &str) -> Result<TimelineDiagram, ParseError> {
    let mut rest = input;
    parse_timeline(&mut rest).map_err(|_| {
        let offset = input.len() - rest.len();
        ParseError::new(ParseErrorKind::UnexpectedToken, offset..offset, input)
    })
}

fn parse_timeline(input: &mut &str) -> ModalResult<TimelineDiagram> {
    skip.parse_next(input)?;
    "timeline".parse_next(input)?;

    let mut diagram = TimelineDiagram::new();

    // Optional direction on same line
    skip_horizontal_ws(input);
    if input.starts_with("LR") {
        *input = &input[2..];
        diagram.direction = Direction::LR;
    } else if input.starts_with("TD") || input.starts_with("TB") {
        *input = &input[2..];
        diagram.direction = Direction::TB;
    }

    // Current section accumulator
    let mut current_section = TimelineSection {
        name: None,
        tasks: Vec::new(),
    };

    loop {
        skip.parse_next(input)?;
        if input.is_empty() {
            break;
        }

        let line = take_line(input);
        if line.is_empty() {
            continue;
        }

        // Title
        if let Some(rest) = line.strip_prefix("title") {
            diagram.title = Some(rest.trim().to_string());
            continue;
        }

        // Section header
        if let Some(rest) = line.strip_prefix("section") {
            // Flush previous section
            if !current_section.tasks.is_empty() || current_section.name.is_some() {
                diagram.sections.push(current_section);
            }
            current_section = TimelineSection {
                name: Some(rest.trim().to_string()),
                tasks: Vec::new(),
            };
            continue;
        }

        // Task with optional events: `task name : event1 : event2`
        let parts: Vec<&str> = line.splitn(2, ':').collect();
        let task_name = parts[0].trim().to_string();
        let events: Vec<String> = if parts.len() > 1 {
            parts[1]
                .split(':')
                .map(|e| e.trim().to_string())
                .filter(|e| !e.is_empty())
                .collect()
        } else {
            Vec::new()
        };

        current_section.tasks.push(TimelineTask {
            name: task_name,
            events,
        });
    }

    // Flush last section
    if !current_section.tasks.is_empty() || current_section.name.is_some() {
        diagram.sections.push(current_section);
    }

    Ok(diagram)
}

fn skip_horizontal_ws(input: &mut &str) {
    *input = input.trim_start_matches(|c: char| c == ' ' || c == '\t');
}

fn take_line<'i>(input: &mut &'i str) -> &'i str {
    let end = input.find('\n').unwrap_or(input.len());
    let line = input[..end].trim();
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
        let d =
            parse("timeline\n    title History\n    2020 : Event A\n    2021 : Event B : Event C")
                .unwrap();
        assert_eq!(d.title.as_deref(), Some("History"));
        assert_eq!(d.sections.len(), 1);
        assert_eq!(d.sections[0].tasks.len(), 2);
        assert_eq!(d.sections[0].tasks[0].name, "2020");
        assert_eq!(d.sections[0].tasks[0].events, vec!["Event A"]);
        assert_eq!(d.sections[0].tasks[1].events, vec!["Event B", "Event C"]);
    }

    #[test]
    fn parse_with_sections() {
        let d = parse("timeline\n    section Ancient\n        3000BC : Pyramids\n    section Modern\n        2000 : Internet").unwrap();
        assert_eq!(d.sections.len(), 2);
        assert_eq!(d.sections[0].name.as_deref(), Some("Ancient"));
        assert_eq!(d.sections[1].name.as_deref(), Some("Modern"));
    }

    #[test]
    fn parse_no_events() {
        let d = parse("timeline\n    2020\n    2021").unwrap();
        assert_eq!(d.sections[0].tasks.len(), 2);
        assert!(d.sections[0].tasks[0].events.is_empty());
    }

    #[test]
    fn parse_direction_td() {
        let d = parse("timeline TD\n    2020 : X").unwrap();
        assert_eq!(d.direction, Direction::TB);
    }

    #[test]
    fn parse_comments() {
        let d = parse("timeline\n    %% comment\n    2020 : X").unwrap();
        assert_eq!(d.sections[0].tasks.len(), 1);
    }

    #[test]
    fn reject_empty() {
        assert!(parse("").is_err());
    }

    #[test]
    fn reject_wrong_header() {
        assert!(parse("gantt\n    title X").is_err());
    }
}
