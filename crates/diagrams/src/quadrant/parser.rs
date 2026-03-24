use crate::common::error::{ParseError, ParseErrorKind};
use super::ir::{QuadrantChart, QuadrantPoint};

pub fn parse(input: &str) -> Result<QuadrantChart, ParseError> {
    let mut chart = QuadrantChart::default();
    let mut header_found = false;

    for (line_no, raw_line) in input.lines().enumerate() {
        let line = raw_line.trim();

        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        if !header_found {
            if line.starts_with("quadrantChart") {
                header_found = true;
                continue;
            }
            return Err(make_err(input, line_no));
        }

        // Title
        if let Some(rest) = line.strip_prefix("title") {
            chart.title = Some(rest.trim().trim_matches('"').to_string());
            continue;
        }

        // X-axis: "x-axis Left --> Right" or "x-axis Label"
        if let Some(rest) = line.strip_prefix("x-axis") {
            let rest = rest.trim();
            if let Some((left, right)) = rest.split_once("-->") {
                chart.x_axis = Some((
                    left.trim().trim_matches('"').to_string(),
                    Some(right.trim().trim_matches('"').to_string()),
                ));
            } else {
                chart.x_axis = Some((rest.trim_matches('"').to_string(), None));
            }
            continue;
        }

        // Y-axis
        if let Some(rest) = line.strip_prefix("y-axis") {
            let rest = rest.trim();
            if let Some((bottom, top)) = rest.split_once("-->") {
                chart.y_axis = Some((
                    bottom.trim().trim_matches('"').to_string(),
                    Some(top.trim().trim_matches('"').to_string()),
                ));
            } else {
                chart.y_axis = Some((rest.trim_matches('"').to_string(), None));
            }
            continue;
        }

        // Quadrant labels: "quadrant-1 Label"
        if let Some(rest) = line.strip_prefix("quadrant-") {
            if let Some((num_str, label)) = rest.split_once(' ') {
                if let Ok(num) = num_str.trim().parse::<usize>() {
                    if (1..=4).contains(&num) {
                        chart.quadrants[num - 1] = Some(label.trim().trim_matches('"').to_string());
                        continue;
                    }
                }
            }
            return Err(make_err(input, line_no));
        }

        // classDef — skip (styling not implemented yet)
        if line.starts_with("classDef") {
            continue;
        }

        // Point: "Label: [x, y]" or "Label:::class: [x, y]"
        if let Some((label_part, coord_part)) = line.split_once(':') {
            // Strip :::className if present
            let label = label_part
                .split(":::")
                .next()
                .unwrap_or(label_part)
                .trim()
                .trim_matches('"')
                .to_string();

            // Find [x, y] in the rest (skip second colon for :::class)
            let coord_str = if coord_part.starts_with("::") {
                // "::className: [x, y]" — find the actual coordinates after the second colon
                coord_part.split_once(':').map(|(_, c)| c).unwrap_or(coord_part)
            } else {
                coord_part
            };

            if let Some((x, y)) = parse_coords(coord_str.trim()) {
                chart.points.push(QuadrantPoint { label, x, y });
                continue;
            }
        }

        // Unknown line — skip silently for forward compatibility
    }

    if !header_found {
        return Err(ParseError::new(ParseErrorKind::UnexpectedToken, 0..input.len().min(10), input));
    }

    Ok(chart)
}

fn parse_coords(s: &str) -> Option<(f64, f64)> {
    let s = s.trim();
    let inner = s.strip_prefix('[')?.strip_suffix(']')?;
    let (x_str, y_str) = inner.split_once(',')?;
    let x: f64 = x_str.trim().parse().ok()?;
    let y: f64 = y_str.trim().parse().ok()?;
    if (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y) {
        Some((x, y))
    } else {
        None
    }
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
        let c = parse("quadrantChart\n  A: [0.5, 0.5]\n  B: [0.2, 0.8]").unwrap();
        assert_eq!(c.points.len(), 2);
        assert!((c.points[0].x - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_title() {
        let c = parse("quadrantChart\n  title Priority Matrix\n  A: [0.1, 0.1]").unwrap();
        assert_eq!(c.title.as_deref(), Some("Priority Matrix"));
    }

    #[test]
    fn parse_axes() {
        let c = parse("quadrantChart\n  x-axis Low --> High\n  y-axis Bad --> Good\n  A: [0.5, 0.5]").unwrap();
        let (xl, xr) = c.x_axis.unwrap();
        assert_eq!(xl, "Low");
        assert_eq!(xr.unwrap(), "High");
    }

    #[test]
    fn parse_quadrant_labels() {
        let c = parse("quadrantChart\n  quadrant-1 Leaders\n  quadrant-2 Challengers\n  quadrant-3 Niche\n  quadrant-4 Visionaries\n  A: [0.5, 0.5]").unwrap();
        assert_eq!(c.quadrants[0].as_deref(), Some("Leaders"));
        assert_eq!(c.quadrants[2].as_deref(), Some("Niche"));
    }

    #[test]
    fn parse_single_axis_label() {
        let c = parse("quadrantChart\n  x-axis Urgency\n  A: [0.5, 0.5]").unwrap();
        let (label, right) = c.x_axis.unwrap();
        assert_eq!(label, "Urgency");
        assert!(right.is_none());
    }

    #[test]
    fn parse_comments_and_blanks() {
        let c = parse("quadrantChart\n  %% comment\n\n  A: [0.1, 0.9]").unwrap();
        assert_eq!(c.points.len(), 1);
    }

    #[test]
    fn reject_no_header() {
        assert!(parse("A: [0.5, 0.5]").is_err());
    }

    #[test]
    fn coords_out_of_range() {
        let c = parse("quadrantChart\n  A: [1.5, 0.5]").unwrap();
        assert!(c.points.is_empty(), "out of range coords should be skipped");
    }

    #[test]
    fn empty_chart_ok() {
        let c = parse("quadrantChart").unwrap();
        assert!(c.points.is_empty());
    }
}
