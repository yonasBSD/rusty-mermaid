use crate::common::error::{ParseError, ParseErrorKind};
use super::ir::{PacketDiagram, PacketField};

/// Parse a packet diagram.
///
/// ```text
/// packet-beta
/// 0-15: "Source Port"
/// 16-31: "Destination Port"
/// ```
///
/// Also supports relative bit counts: `+16: "Field"`
pub fn parse(input: &str) -> Result<PacketDiagram, ParseError> {
    let mut diagram = PacketDiagram::default();
    let mut header_found = false;
    let mut next_bit: usize = 0;

    for (line_no, raw_line) in input.lines().enumerate() {
        let line = raw_line.trim();

        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        if !header_found {
            if line.starts_with("packet") {
                header_found = true;
                continue;
            }
            return Err(make_err(input, line_no, raw_line.len()));
        }

        // Title
        if let Some(rest) = line.strip_prefix("title") {
            let title = rest.trim().trim_matches('"').to_string();
            if !title.is_empty() {
                diagram.title = Some(title);
            }
            continue;
        }

        // Field: "start-end: label" or "+bits: label" or "bit: label"
        let Some((range_part, label)) = line.split_once(':') else {
            return Err(make_err(input, line_no, raw_line.len()));
        };

        let range_part = range_part.trim();
        let label = label.trim().trim_matches('"').to_string();

        let (start, end) = if let Some(bits_str) = range_part.strip_prefix('+') {
            // Relative: +N
            let bits: usize = bits_str.trim().parse().map_err(|_| make_err(input, line_no, raw_line.len()))?;
            if bits == 0 {
                return Err(make_err(input, line_no, raw_line.len()));
            }
            let s = next_bit;
            (s, s + bits - 1)
        } else if let Some((s_str, e_str)) = range_part.split_once('-') {
            // Absolute range: start-end
            let s: usize = s_str.trim().parse().map_err(|_| make_err(input, line_no, raw_line.len()))?;
            let e: usize = e_str.trim().parse().map_err(|_| make_err(input, line_no, raw_line.len()))?;
            if e < s {
                return Err(make_err(input, line_no, raw_line.len()));
            }
            (s, e)
        } else {
            // Single bit: N
            let b: usize = range_part.parse().map_err(|_| make_err(input, line_no, raw_line.len()))?;
            (b, b)
        };

        // Contiguity check
        if start != next_bit && !diagram.fields.is_empty() {
            return Err(make_err(input, line_no, raw_line.len()));
        }

        next_bit = end + 1;
        diagram.fields.push(PacketField { start, end, label });
    }

    if !header_found {
        return Err(ParseError::new(ParseErrorKind::UnexpectedToken, 0..input.len().min(10), input));
    }
    if diagram.fields.is_empty() {
        return Err(ParseError::new(ParseErrorKind::UnexpectedEof, input.len()..input.len(), input));
    }

    Ok(diagram)
}

fn make_err(input: &str, line_no: usize, _line_len: usize) -> ParseError {
    let offset: usize = input.lines().take(line_no).map(|l| l.len() + 1).sum();
    ParseError::new(ParseErrorKind::UnexpectedToken, offset..offset + 1, input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_absolute_ranges() {
        let d = parse("packet-beta\n0-15: \"Source Port\"\n16-31: \"Dest Port\"").unwrap();
        assert_eq!(d.fields.len(), 2);
        assert_eq!(d.fields[0].start, 0);
        assert_eq!(d.fields[0].end, 15);
        assert_eq!(d.fields[1].start, 16);
        assert_eq!(d.fields[1].end, 31);
    }

    #[test]
    fn parse_relative_bits() {
        let d = parse("packet-beta\n+16: \"A\"\n+16: \"B\"").unwrap();
        assert_eq!(d.fields[0].start, 0);
        assert_eq!(d.fields[0].end, 15);
        assert_eq!(d.fields[1].start, 16);
        assert_eq!(d.fields[1].end, 31);
    }

    #[test]
    fn parse_single_bit() {
        let d = parse("packet-beta\n0-15: \"Port\"\n16: \"Flag\"").unwrap();
        assert_eq!(d.fields[1].bits(), 1);
    }

    #[test]
    fn parse_mixed_syntax() {
        let d = parse("packet-beta\n+8: \"A\"\n8-15: \"B\"\n+4: \"C\"").unwrap();
        assert_eq!(d.fields[2].start, 16);
        assert_eq!(d.fields[2].end, 19);
    }

    #[test]
    fn parse_title() {
        let d = parse("packet-beta\ntitle \"TCP\"\n0-7: \"A\"").unwrap();
        assert_eq!(d.title.as_deref(), Some("TCP"));
    }

    #[test]
    fn parse_comments_and_blanks() {
        let d = parse("packet\n%% comment\n\n0-7: \"A\"\n\n8-15: \"B\"").unwrap();
        assert_eq!(d.fields.len(), 2);
    }

    #[test]
    fn parse_packet_header() {
        assert!(parse("packet\n0-7: \"A\"").is_ok());
    }

    #[test]
    fn reject_no_header() {
        assert!(parse("0-7: \"A\"").is_err());
    }

    #[test]
    fn reject_no_fields() {
        assert!(parse("packet-beta\n%% empty").is_err());
    }

    #[test]
    fn reject_end_before_start() {
        assert!(parse("packet-beta\n15-0: \"Bad\"").is_err());
    }

    #[test]
    fn reject_gap() {
        assert!(parse("packet-beta\n0-7: \"A\"\n10-15: \"B\"").is_err());
    }

    #[test]
    fn reject_zero_bits() {
        assert!(parse("packet-beta\n+0: \"Bad\"").is_err());
    }
}
