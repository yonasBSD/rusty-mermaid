use crate::common::error::{ParseError, ParseErrorKind};
use super::ir::{Graticule, RadarAxis, RadarChart, RadarCurve};

pub fn parse(input: &str) -> Result<RadarChart, ParseError> {
    let mut chart = RadarChart::default();
    let mut header_found = false;

    for (line_no, raw_line) in input.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        if !header_found {
            if line.starts_with("radar") {
                header_found = true;
                continue;
            }
            return Err(make_err(input, line_no));
        }

        if let Some(rest) = line.strip_prefix("title") {
            chart.title = Some(rest.trim().trim_matches('"').to_string());
            continue;
        }

        if let Some(rest) = line.strip_prefix("ticks") {
            if let Ok(n) = rest.trim().parse::<usize>() {
                chart.ticks = n.max(1);
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix("min") {
            if let Ok(v) = rest.trim().parse::<f64>() {
                chart.min = v;
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix("max") {
            if let Ok(v) = rest.trim().parse::<f64>() {
                chart.max = Some(v);
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix("graticule") {
            chart.graticule = match rest.trim() {
                "circle" => Graticule::Circle,
                _ => Graticule::Polygon,
            };
            continue;
        }

        if line.starts_with("showLegend") {
            continue; // legend always shown for now
        }

        if let Some(rest) = line.strip_prefix("axis") {
            chart.axes = parse_axes(rest.trim());
            continue;
        }

        if let Some(rest) = line.strip_prefix("curve") {
            if let Some(curve) = parse_curve(rest.trim(), &chart.axes) {
                chart.curves.push(curve);
            }
            continue;
        }
    }

    if !header_found {
        return Err(ParseError::new(ParseErrorKind::UnexpectedToken, 0..input.len().min(10), input));
    }

    Ok(chart)
}

/// Parse "A,B,C" or "A[\"Label A\"], B[\"Label B\"]"
fn parse_axes(s: &str) -> Vec<RadarAxis> {
    s.split(',')
        .map(|part| {
            let part = part.trim();
            if let Some(bracket) = part.find('[') {
                let id = part[..bracket].trim().to_string();
                let label = part[bracket..]
                    .trim_start_matches('[')
                    .trim_end_matches(']')
                    .trim()
                    .trim_matches('"')
                    .to_string();
                RadarAxis { id, label }
            } else {
                RadarAxis { id: part.to_string(), label: part.to_string() }
            }
        })
        .filter(|a| !a.id.is_empty())
        .collect()
}

/// Parse "name{1,2,3}" or "name[\"Label\"]{A: 1, B: 2}"
fn parse_curve(s: &str, axes: &[RadarAxis]) -> Option<RadarCurve> {
    let brace_start = s.find('{')?;
    let brace_end = s.rfind('}')?;

    let header = s[..brace_start].trim();
    let data = &s[brace_start + 1..brace_end];

    let (id, label) = if let Some(bracket) = header.find('[') {
        let id = header[..bracket].trim().to_string();
        let label = header[bracket..]
            .trim_start_matches('[')
            .trim_end_matches(']')
            .trim()
            .trim_matches('"')
            .to_string();
        (id, label)
    } else {
        (header.to_string(), header.to_string())
    };

    // Try named values first: "A: 1, B: 2"
    let values = if data.contains(':') {
        let mut vals = vec![0.0f64; axes.len()];
        for pair in data.split(',') {
            let (name, val) = pair.split_once(':')?;
            let name = name.trim();
            let val: f64 = val.trim().parse().ok()?;
            if let Some(idx) = axes.iter().position(|a| a.id == name) {
                vals[idx] = val;
            }
        }
        vals
    } else {
        // Positional: "1,2,3"
        data.split(',')
            .map(|v| v.trim().parse::<f64>().unwrap_or(0.0))
            .collect()
    };

    Some(RadarCurve { id, label, values })
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
        let c = parse("radar-beta\naxis A,B,C\ncurve x{1,2,3}").unwrap();
        assert_eq!(c.axes.len(), 3);
        assert_eq!(c.curves.len(), 1);
        assert_eq!(c.curves[0].values, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn parse_labeled_axes() {
        let c = parse("radar-beta\naxis A[\"Speed\"], B[\"Power\"]\ncurve x{5,3}").unwrap();
        assert_eq!(c.axes[0].label, "Speed");
        assert_eq!(c.axes[1].label, "Power");
    }

    #[test]
    fn parse_named_values() {
        let c = parse("radar-beta\naxis A,B,C\ncurve x{C: 3, A: 1, B: 2}").unwrap();
        assert_eq!(c.curves[0].values, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn parse_title() {
        let c = parse("radar-beta\ntitle Skills\naxis A\ncurve x{5}").unwrap();
        assert_eq!(c.title.as_deref(), Some("Skills"));
    }

    #[test]
    fn parse_options() {
        let c = parse("radar-beta\nticks 3\nmin 1\nmax 10\ngraticule circle\naxis A\ncurve x{5}").unwrap();
        assert_eq!(c.ticks, 3);
        assert!((c.min - 1.0).abs() < f64::EPSILON);
        assert!((c.max.unwrap() - 10.0).abs() < f64::EPSILON);
        assert_eq!(c.graticule, Graticule::Circle);
    }

    #[test]
    fn parse_multiple_curves() {
        let c = parse("radar-beta\naxis A,B,C\ncurve a{1,2,3}\ncurve b{4,5,6}").unwrap();
        assert_eq!(c.curves.len(), 2);
    }

    #[test]
    fn reject_no_header() {
        assert!(parse("axis A,B\ncurve x{1,2}").is_err());
    }
}
