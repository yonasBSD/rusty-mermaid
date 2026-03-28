use crate::common::error::{ParseError, ParseErrorKind};
use crate::common::tokens::skip;
use winnow::prelude::*;

use super::ir::*;

pub fn parse(input: &str) -> Result<PieChart, ParseError> {
    let mut rest = input;
    parse_pie(&mut rest).map_err(|_| {
        let offset = input.len() - rest.len();
        ParseError::new(ParseErrorKind::UnexpectedToken, offset..offset, input)
    })
}

fn parse_pie(input: &mut &str) -> ModalResult<PieChart> {
    skip.parse_next(input)?;
    "pie".parse_next(input)?;

    let mut chart = PieChart::new();

    // Optional showData flag
    skip_horizontal_ws(input);
    if input.starts_with("showData") {
        *input = &input[8..];
        chart.show_data = true;
    }

    // Optional title on same line or next line
    skip_horizontal_ws(input);
    if input.starts_with("title") {
        *input = &input[5..];
        skip_horizontal_ws(input);
        let title = take_to_eol(input);
        if !title.is_empty() {
            chart.title = Some(title.to_string());
        }
    }

    // Parse slices
    loop {
        skip.parse_next(input)?;
        if input.is_empty() {
            break;
        }

        // title on its own line
        if input.starts_with("title") {
            *input = &input[5..];
            skip_horizontal_ws(input);
            let title = take_to_eol(input);
            if !title.is_empty() {
                chart.title = Some(title.to_string());
            }
            continue;
        }

        // "label" : value
        if input.starts_with('"') {
            let cp = *input;
            if let Some(slice) = parse_slice(input) {
                chart.slices.push(slice);
                continue;
            }
            *input = cp;
        }

        // Skip unrecognized
        if !input.is_empty() {
            skip_to_eol(input);
        }
    }

    Ok(chart)
}

fn parse_slice(input: &mut &str) -> Option<PieSlice> {
    if !input.starts_with('"') {
        return None;
    }
    *input = &input[1..];
    let end = input.find('"')?;
    let label = input[..end].to_string();
    *input = &input[end + 1..];

    skip_horizontal_ws(input);
    if !input.starts_with(':') {
        return None;
    }
    *input = &input[1..];
    skip_horizontal_ws(input);

    let val_end = input
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(input.len());
    let val_str = &input[..val_end];
    let value: f64 = val_str.parse().ok()?;
    *input = &input[val_end..];

    if value < 0.0 {
        return None;
    }
    Some(PieSlice { label, value })
}

fn skip_horizontal_ws(input: &mut &str) {
    *input = input.trim_start_matches([' ', '\t']);
}

fn take_to_eol<'i>(input: &mut &'i str) -> &'i str {
    let end = input.find('\n').unwrap_or(input.len());
    let line = &input[..end];
    *input = &input[end..];
    line.trim()
}

fn skip_to_eol(input: &mut &str) {
    let end = input.find('\n').unwrap_or(input.len());
    *input = if end < input.len() {
        &input[end + 1..]
    } else {
        ""
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        let d = parse("pie\n    \"Dogs\" : 40\n    \"Cats\" : 30\n    \"Birds\" : 20").unwrap();
        assert_eq!(d.slices.len(), 3);
        assert!((d.slices[0].value - 40.0).abs() < f64::EPSILON);
        assert_eq!(d.slices[0].label, "Dogs");
    }

    #[test]
    fn parse_with_title() {
        let d = parse("pie title Pets\n    \"Dogs\" : 40\n    \"Cats\" : 60").unwrap();
        assert_eq!(d.title.as_deref(), Some("Pets"));
    }

    #[test]
    fn parse_title_next_line() {
        let d = parse("pie\n    title My Chart\n    \"A\" : 50\n    \"B\" : 50").unwrap();
        assert_eq!(d.title.as_deref(), Some("My Chart"));
    }

    #[test]
    fn parse_show_data() {
        let d = parse("pie showData\n    \"A\" : 30\n    \"B\" : 70").unwrap();
        assert!(d.show_data);
    }

    #[test]
    fn parse_decimal_values() {
        let d = parse("pie\n    \"A\" : 33.3\n    \"B\" : 66.7").unwrap();
        assert!((d.slices[0].value - 33.3).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_comments_ignored() {
        let d = parse("pie\n    %% comment\n    \"A\" : 100").unwrap();
        assert_eq!(d.slices.len(), 1);
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
    fn single_slice() {
        let d = parse("pie\n    \"Only\" : 100").unwrap();
        assert_eq!(d.slices.len(), 1);
        assert_eq!(d.slices[0].label, "Only");
        assert!((d.slices[0].value - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn zero_value_slice() {
        let d = parse("pie\n    \"Nothing\" : 0\n    \"All\" : 100").unwrap();
        assert_eq!(d.slices.len(), 2);
        assert!((d.slices[0].value - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn title_with_spaces() {
        let d = parse("pie title My Fancy Pie Chart\n    \"A\" : 50").unwrap();
        assert_eq!(d.title.as_deref(), Some("My Fancy Pie Chart"));
    }

    #[test]
    fn whitespace_around_colon() {
        let d = parse("pie\n    \"X\"  :  42").unwrap();
        assert_eq!(d.slices.len(), 1);
        assert!((d.slices[0].value - 42.0).abs() < f64::EPSILON);
    }

    #[test]
    fn show_data_with_title() {
        let d = parse("pie showData title Metrics\n    \"A\" : 10").unwrap();
        assert!(d.show_data);
        assert_eq!(d.title.as_deref(), Some("Metrics"));
    }

    #[test]
    fn no_slices_ok() {
        let d = parse("pie\n    title Empty Chart").unwrap();
        assert!(d.slices.is_empty());
        assert_eq!(d.title.as_deref(), Some("Empty Chart"));
    }
}
