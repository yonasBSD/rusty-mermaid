use crate::common::error::{ParseError, ParseErrorKind};
use crate::common::tokens::skip;
use winnow::prelude::*;

use super::ir::*;

pub fn parse(input: &str) -> Result<GanttChart, ParseError> {
    let mut rest = input;
    parse_gantt(&mut rest).map_err(|_| {
        let offset = input.len() - rest.len();
        ParseError::new(ParseErrorKind::UnexpectedToken, offset..offset, input)
    })
}

fn parse_gantt(input: &mut &str) -> ModalResult<GanttChart> {
    skip.parse_next(input)?;
    "gantt".parse_next(input)?;

    let mut chart = GanttChart::new();
    let mut current_section = GanttSection {
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

        // Directives
        if let Some(rest) = line.strip_prefix("title") {
            chart.title = Some(rest.trim().to_string());
            continue;
        }
        if let Some(rest) = line.strip_prefix("dateFormat") {
            chart.date_format = rest.trim().to_string();
            continue;
        }
        if let Some(rest) = line.strip_prefix("axisFormat") {
            chart.axis_format = Some(rest.trim().to_string());
            continue;
        }
        if let Some(rest) = line.strip_prefix("section") {
            // Flush previous section
            if !current_section.tasks.is_empty() || current_section.name.is_some() {
                chart.sections.push(current_section);
            }
            current_section = GanttSection {
                name: Some(rest.trim().to_string()),
                tasks: Vec::new(),
            };
            continue;
        }

        // Skip other directives we don't handle yet
        if line.starts_with("excludes")
            || line.starts_with("includes")
            || line.starts_with("todayMarker")
            || line.starts_with("inclusiveEndDates")
            || line.starts_with("topAxis")
            || line.starts_with("weekday")
            || line.starts_with("weekend")
            || line.starts_with("tickInterval")
        {
            continue;
        }

        // Task line: name :tags, id, start, end
        if let Some(task) = parse_task_line(line) {
            current_section.tasks.push(task);
        }
    }

    if !current_section.tasks.is_empty() || current_section.name.is_some() {
        chart.sections.push(current_section);
    }

    Ok(chart)
}

fn parse_task_line(line: &str) -> Option<GanttTask> {
    // Split on first colon: "Task Name :spec1, spec2, ..."
    let (name, specs) = if let Some(colon_pos) = line.find(':') {
        let name = line[..colon_pos].trim();
        let specs = line[colon_pos + 1..].trim();
        (name, specs)
    } else {
        // No colon — just a task name with auto dates
        return Some(GanttTask {
            name: line.trim().to_string(),
            id: None,
            tags: Vec::new(),
            start: TaskStart::Auto,
            end: TaskEnd::Auto,
        });
    };

    let parts: Vec<&str> = specs
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    let mut tags = Vec::new();
    let mut start = TaskStart::Auto;
    let mut date_parts: Vec<&str> = Vec::new();

    for part in &parts {
        match *part {
            "done" => tags.push(TaskTag::Done),
            "active" => tags.push(TaskTag::Active),
            "crit" => tags.push(TaskTag::Crit),
            "milestone" => tags.push(TaskTag::Milestone),
            p if p.starts_with("after ") => {
                start = TaskStart::After(p[6..].trim().to_string());
            }
            p => date_parts.push(p),
        }
    }

    let (parsed_id, parsed_start, parsed_end) = interpret_date_parts(&date_parts, start);

    Some(GanttTask {
        name: name.to_string(),
        id: parsed_id,
        tags,
        start: parsed_start,
        end: parsed_end,
    })
}

fn interpret_date_parts(
    date_parts: &[&str],
    current_start: TaskStart,
) -> (Option<String>, TaskStart, TaskEnd) {
    let mut id = None;
    let mut start = current_start;
    let mut end = TaskEnd::Auto;

    match date_parts.len() {
        0 => {}
        1 => {
            let p = date_parts[0];
            if is_duration(p) {
                end = TaskEnd::Duration(p.to_string());
            } else if looks_like_date(p) {
                if matches!(start, TaskStart::Auto) {
                    start = TaskStart::Date(p.to_string());
                } else {
                    end = TaskEnd::Date(p.to_string());
                }
            } else {
                id = Some(p.to_string());
            }
        }
        2 => {
            let (a, b) = (date_parts[0], date_parts[1]);
            if looks_like_date(a) && (looks_like_date(b) || is_duration(b)) {
                start = TaskStart::Date(a.to_string());
                end = if is_duration(b) {
                    TaskEnd::Duration(b.to_string())
                } else {
                    TaskEnd::Date(b.to_string())
                };
            } else if !looks_like_date(a) && !is_duration(a) {
                id = Some(a.to_string());
                if is_duration(b) {
                    end = TaskEnd::Duration(b.to_string());
                } else if looks_like_date(b) {
                    if matches!(start, TaskStart::Auto) {
                        start = TaskStart::Date(b.to_string());
                    } else {
                        end = TaskEnd::Date(b.to_string());
                    }
                }
            }
        }
        _ => {
            id = Some(date_parts[0].to_string());
            start = TaskStart::Date(date_parts[1].to_string());
            end = if date_parts.len() >= 3 && is_duration(date_parts[2]) {
                TaskEnd::Duration(date_parts[2].to_string())
            } else if date_parts.len() >= 3 {
                TaskEnd::Date(date_parts[2].to_string())
            } else {
                TaskEnd::Auto
            };
        }
    }

    (id, start, end)
}

fn is_duration(s: &str) -> bool {
    let s = s.trim();
    s.len() >= 2
        && s[..s.len() - 1].chars().all(|c| c.is_ascii_digit())
        && matches!(s.chars().last(), Some('d' | 'w' | 'h' | 'm'))
}

fn looks_like_date(s: &str) -> bool {
    let s = s.trim();
    // YYYY-MM-DD or similar patterns
    s.len() >= 8
        && s.chars().any(|c| c == '-' || c == '/')
        && s.chars().filter(|c| c.is_ascii_digit()).count() >= 4
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
        let c = parse("gantt\n    title Project Plan\n    dateFormat YYYY-MM-DD\n    section Phase 1\n    Task A :a1, 2024-01-01, 2024-01-10\n    Task B :a2, 2024-01-05, 5d").unwrap();
        assert_eq!(c.title.as_deref(), Some("Project Plan"));
        assert_eq!(c.sections.len(), 1);
        assert_eq!(c.sections[0].tasks.len(), 2);
    }

    #[test]
    fn parse_task_with_tags() {
        let c = parse("gantt\n    Task :done, crit, a1, 2024-01-01, 2024-01-05").unwrap();
        let t = &c.sections[0].tasks[0];
        assert!(t.tags.contains(&TaskTag::Done));
        assert!(t.tags.contains(&TaskTag::Crit));
    }

    #[test]
    fn parse_task_after_dependency() {
        let c = parse("gantt\n    Task A :a1, 2024-01-01, 5d\n    Task B :after a1, 3d").unwrap();
        let t = &c.sections[0].tasks[1];
        assert!(matches!(&t.start, TaskStart::After(id) if id == "a1"));
        assert!(matches!(&t.end, TaskEnd::Duration(d) if d == "3d"));
    }

    #[test]
    fn parse_task_duration_only() {
        let c = parse("gantt\n    Task :5d").unwrap();
        assert!(matches!(&c.sections[0].tasks[0].end, TaskEnd::Duration(d) if d == "5d"));
    }

    #[test]
    fn parse_milestone() {
        let c = parse("gantt\n    Milestone :milestone, m1, 2024-01-15, 0d").unwrap();
        assert!(c.sections[0].tasks[0].tags.contains(&TaskTag::Milestone));
    }

    #[test]
    fn parse_multiple_sections() {
        let c = parse("gantt\n    section Design\n    Wireframes :2024-01-01, 5d\n    section Dev\n    Backend :2024-01-06, 10d").unwrap();
        assert_eq!(c.sections.len(), 2);
        assert_eq!(c.sections[0].name.as_deref(), Some("Design"));
        assert_eq!(c.sections[1].name.as_deref(), Some("Dev"));
    }

    #[test]
    fn parse_date_format() {
        let c = parse("gantt\n    dateFormat DD-MM-YYYY\n    Task :01-01-2024, 5d").unwrap();
        assert_eq!(c.date_format, "DD-MM-YYYY");
    }

    #[test]
    fn parse_axis_format() {
        let c = parse("gantt\n    axisFormat %Y-%m-%d\n    Task :2024-01-01, 5d").unwrap();
        assert_eq!(c.axis_format.as_deref(), Some("%Y-%m-%d"));
    }

    #[test]
    fn parse_comments() {
        let c = parse("gantt\n    %% comment\n    Task :2024-01-01, 5d").unwrap();
        assert_eq!(c.sections[0].tasks.len(), 1);
    }

    #[test]
    fn reject_empty() {
        assert!(parse("").is_err());
    }

    #[test]
    fn reject_wrong_header() {
        assert!(parse("pie\n    title X").is_err());
    }
}
