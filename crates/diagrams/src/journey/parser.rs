use crate::common::error::{ParseError, ParseErrorKind};
use super::ir::{JourneyDiagram, JourneySection, JourneyTask};

pub fn parse(input: &str) -> Result<JourneyDiagram, ParseError> {
    let mut diagram = JourneyDiagram::default();
    let mut header_found = false;
    let mut current_section: Option<JourneySection> = None;

    for (line_no, raw_line) in input.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        if !header_found {
            if line.starts_with("journey") {
                header_found = true;
                continue;
            }
            return Err(make_err(input, line_no));
        }

        if let Some(rest) = line.strip_prefix("title") {
            diagram.title = Some(rest.trim().to_string());
            continue;
        }

        if let Some(rest) = line.strip_prefix("section") {
            // Flush previous section
            if let Some(sec) = current_section.take() {
                diagram.sections.push(sec);
            }
            current_section = Some(JourneySection {
                name: rest.trim().to_string(),
                tasks: Vec::new(),
            });
            continue;
        }

        // Task: "Name: score: actor1, actor2" or "Name: score"
        if let Some((name_part, rest)) = line.split_once(':') {
            let name = name_part.trim().to_string();
            let (score, actors) = parse_task_rest(rest);

            let task = JourneyTask { name, score, actors };

            if let Some(ref mut sec) = current_section {
                sec.tasks.push(task);
            } else {
                // No section yet — create implicit one
                current_section = Some(JourneySection {
                    name: String::new(),
                    tasks: vec![task],
                });
            }
            continue;
        }
    }

    // Flush last section
    if let Some(sec) = current_section {
        diagram.sections.push(sec);
    }

    if !header_found {
        return Err(ParseError::new(ParseErrorKind::UnexpectedToken, 0..input.len().min(10), input));
    }

    Ok(diagram)
}

fn parse_task_rest(rest: &str) -> (u8, Vec<String>) {
    let parts: Vec<&str> = rest.splitn(2, ':').collect();
    let score = parts[0].trim().parse::<u8>().unwrap_or(3).min(5);
    let actors = if parts.len() > 1 {
        parts[1]
            .split(',')
            .map(|a| a.trim().to_string())
            .filter(|a| !a.is_empty())
            .collect()
    } else {
        Vec::new()
    };
    (score, actors)
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
        let d = parse("journey\n  title My Day\n  section Morning\n    Wake up: 5: Me\n    Breakfast: 3: Me, Cat").unwrap();
        assert_eq!(d.title.as_deref(), Some("My Day"));
        assert_eq!(d.sections.len(), 1);
        assert_eq!(d.sections[0].tasks.len(), 2);
        assert_eq!(d.sections[0].tasks[0].score, 5);
        assert_eq!(d.sections[0].tasks[1].actors, vec!["Me", "Cat"]);
    }

    #[test]
    fn parse_multiple_sections() {
        let d = parse("journey\n  section A\n    T1: 4\n  section B\n    T2: 2: Bob").unwrap();
        assert_eq!(d.sections.len(), 2);
        assert_eq!(d.sections[1].tasks[0].actors, vec!["Bob"]);
    }

    #[test]
    fn parse_no_actors() {
        let d = parse("journey\n  section S\n    Task: 5").unwrap();
        assert!(d.sections[0].tasks[0].actors.is_empty());
    }

    #[test]
    fn parse_score_clamped() {
        let d = parse("journey\n  section S\n    T: 9").unwrap();
        assert_eq!(d.sections[0].tasks[0].score, 5);
    }

    #[test]
    fn parse_comments_blanks() {
        let d = parse("journey\n  %% comment\n\n  section S\n    T: 3").unwrap();
        assert_eq!(d.sections[0].tasks.len(), 1);
    }

    #[test]
    fn reject_no_header() {
        assert!(parse("section S\n  T: 3").is_err());
    }

    #[test]
    fn empty_journey_ok() {
        let d = parse("journey").unwrap();
        assert!(d.sections.is_empty());
    }

    #[test]
    fn reject_wrong_header() {
        assert!(parse("pie\n  section S\n    T: 3").is_err());
    }

    #[test]
    fn task_without_section() {
        let d = parse("journey\n    Task: 5: Me").unwrap();
        assert_eq!(d.sections.len(), 1, "implicit section created");
        assert_eq!(d.sections[0].tasks.len(), 1);
    }

    #[test]
    fn score_defaults_to_3() {
        let d = parse("journey\n  section S\n    Task: abc").unwrap();
        assert_eq!(d.sections[0].tasks[0].score, 3);
    }
}
