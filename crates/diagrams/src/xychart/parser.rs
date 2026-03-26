use crate::common::error::{ParseError, ParseErrorKind};
use crate::common::tokens::skip;
use winnow::prelude::*;

use super::ir::*;

pub fn parse(input: &str) -> Result<XyChart, ParseError> {
    let mut rest = input;
    parse_xychart(&mut rest).map_err(|_| {
        let offset = input.len() - rest.len();
        ParseError::new(ParseErrorKind::UnexpectedToken, offset..offset, input)
    })
}

fn parse_xychart(input: &mut &str) -> ModalResult<XyChart> {
    skip.parse_next(input)?;
    "xychart-beta".parse_next(input).or_else(
        |_: winnow::error::ErrMode<winnow::error::ContextError>| "xychart".parse_next(input),
    )?;

    let mut chart = XyChart::new();

    // Optional orientation
    skip_horizontal_ws(input);
    if input.starts_with("horizontal") {
        *input = &input[10..];
        chart.horizontal = true;
    } else if input.starts_with("vertical") {
        *input = &input[8..];
    }

    loop {
        skip.parse_next(input)?;
        if input.is_empty() {
            break;
        }

        let line = take_line(input);
        if line.is_empty() {
            continue;
        }

        if let Some(rest) = line.strip_prefix("title") {
            chart.title = Some(rest.trim().trim_matches('"').to_string());
            continue;
        }

        if line.starts_with("x-axis") {
            chart.x_axis = parse_axis_def(&line[6..]);
            continue;
        }

        if line.starts_with("y-axis") {
            chart.y_axis = parse_axis_def(&line[6..]);
            continue;
        }

        if line.starts_with("line") {
            if let Some(plot) = parse_plot_data(&line[4..], PlotType::Line) {
                chart.plots.push(plot);
            }
            continue;
        }

        if line.starts_with("bar") {
            if let Some(plot) = parse_plot_data(&line[3..], PlotType::Bar) {
                chart.plots.push(plot);
            }
            continue;
        }
    }

    Ok(chart)
}

fn parse_axis_def(rest: &str) -> AxisDef {
    let rest = rest.trim();

    // Check for categories: [cat1, cat2, ...]
    if let Some(bracket_start) = rest.find('[') {
        let title = if bracket_start > 0 {
            let t = rest[..bracket_start].trim().trim_matches('"');
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        } else {
            None
        };
        if let Some(bracket_end) = rest.rfind(']') {
            let cats: Vec<String> = rest[bracket_start + 1..bracket_end]
                .split(',')
                .map(|s| s.trim().trim_matches('"').to_string())
                .filter(|s| !s.is_empty())
                .collect();
            return AxisDef::Band {
                title,
                categories: cats,
            };
        }
    }

    // Check for range: title min --> max or just min --> max
    if let Some(arrow_pos) = rest.find("-->") {
        let before = rest[..arrow_pos].trim();
        let after = rest[arrow_pos + 3..].trim();

        // Try to parse min from end of before, title from beginning
        let (title, min_str) = split_title_and_number(before);
        let min: Option<f64> = min_str.and_then(|s| s.parse().ok());
        let max: Option<f64> = after.parse().ok();

        return AxisDef::Linear { title, min, max };
    }

    // Just a title
    let title = rest.trim_matches('"');
    if title.is_empty() {
        AxisDef::Linear {
            title: None,
            min: None,
            max: None,
        }
    } else {
        AxisDef::Linear {
            title: Some(title.to_string()),
            min: None,
            max: None,
        }
    }
}

fn split_title_and_number(s: &str) -> (Option<String>, Option<&str>) {
    let s = s.trim();
    // Try: "Title" number or just number
    if let Some(q_end) = s.rfind('"') {
        let title = s[..=q_end].trim().trim_matches('"');
        let num = s[q_end + 1..].trim();
        let title = if title.is_empty() {
            None
        } else {
            Some(title.to_string())
        };
        let num = if num.is_empty() { None } else { Some(num) };
        (title, num)
    } else {
        // Could be "number" or "title number"
        let parts: Vec<&str> = s.rsplitn(2, ' ').collect();
        if parts.len() == 2 && parts[0].parse::<f64>().is_ok() {
            (Some(parts[1].trim_matches('"').to_string()), Some(parts[0]))
        } else if s.parse::<f64>().is_ok() {
            (None, Some(s))
        } else {
            (Some(s.trim_matches('"').to_string()), None)
        }
    }
}

fn parse_plot_data(rest: &str, plot_type: PlotType) -> Option<PlotData> {
    let rest = rest.trim();

    let (label, values_str) = if let Some(bracket_start) = rest.find('[') {
        let label_part = rest[..bracket_start].trim().trim_matches('"');
        let label = if label_part.is_empty() {
            None
        } else {
            Some(label_part.to_string())
        };
        if let Some(bracket_end) = rest.rfind(']') {
            (label, &rest[bracket_start + 1..bracket_end])
        } else {
            return None;
        }
    } else {
        return None;
    };

    let values: Vec<f64> = values_str
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    Some(PlotData {
        plot_type,
        label,
        values,
    })
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
        let c = parse("xychart-beta\n    title \"Sales\"\n    x-axis [Jan, Feb, Mar]\n    y-axis 0 --> 100\n    bar [30, 60, 45]").unwrap();
        assert_eq!(c.title.as_deref(), Some("Sales"));
        assert_eq!(c.plots.len(), 1);
        assert_eq!(c.plots[0].values, vec![30.0, 60.0, 45.0]);
    }

    #[test]
    fn parse_categories() {
        let c = parse("xychart-beta\n    x-axis [A, B, C]").unwrap();
        if let AxisDef::Band { categories, .. } = &c.x_axis {
            assert_eq!(categories, &["A", "B", "C"]);
        } else {
            panic!("expected band axis");
        }
    }

    #[test]
    fn parse_y_axis_range() {
        let c = parse("xychart-beta\n    y-axis \"Revenue\" 0 --> 500").unwrap();
        if let AxisDef::Linear { title, min, max } = &c.y_axis {
            assert_eq!(title.as_deref(), Some("Revenue"));
            assert_eq!(*min, Some(0.0));
            assert_eq!(*max, Some(500.0));
        } else {
            panic!("expected linear axis");
        }
    }

    #[test]
    fn parse_line_and_bar() {
        let c = parse(
            "xychart-beta\n    x-axis [A, B]\n    line \"L1\" [10, 20]\n    bar \"B1\" [5, 15]",
        )
        .unwrap();
        assert_eq!(c.plots.len(), 2);
        assert_eq!(c.plots[0].plot_type, PlotType::Line);
        assert_eq!(c.plots[1].plot_type, PlotType::Bar);
    }

    #[test]
    fn parse_horizontal() {
        let c = parse("xychart-beta horizontal\n    bar [1, 2, 3]").unwrap();
        assert!(c.horizontal);
    }

    #[test]
    fn parse_xychart_without_beta() {
        let c = parse("xychart\n    bar [1, 2]").unwrap();
        assert_eq!(c.plots.len(), 1);
    }

    #[test]
    fn parse_comments() {
        let c = parse("xychart-beta\n    %% comment\n    bar [10, 20]").unwrap();
        assert_eq!(c.plots.len(), 1);
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
