use std::ops::Range;

/// What went wrong during parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    UnexpectedToken,
    UnexpectedEof,
    InvalidDirection,
    InvalidShape,
    InvalidEdge,
    UnclosedString,
    UnclosedDelimiter { open: char },
    InvalidStyle,
}

impl std::fmt::Display for ParseErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnexpectedToken => write!(f, "unexpected token"),
            Self::UnexpectedEof => write!(f, "unexpected end of input"),
            Self::InvalidDirection => {
                write!(f, "invalid direction (expected TB, BT, LR, RL, or TD)")
            }
            Self::InvalidShape => write!(f, "unrecognized node shape"),
            Self::InvalidEdge => write!(f, "malformed edge syntax"),
            Self::UnclosedString => write!(f, "unclosed string"),
            Self::UnclosedDelimiter { open } => write!(f, "unclosed delimiter '{open}'"),
            Self::InvalidStyle => write!(f, "malformed style property"),
        }
    }
}

/// A parse error with location information.
#[derive(Debug, Clone)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub span: Range<usize>,
    snippet: String,
}

impl ParseError {
    pub fn new(kind: ParseErrorKind, span: Range<usize>, input: &str) -> Self {
        let start = snap_to_char_boundary(input, span.start.min(input.len()));
        let end = snap_to_char_boundary(input, (start + 40).min(input.len()));
        let snippet = input[start..end].to_string();
        Self {
            kind,
            span,
            snippet,
        }
    }

    /// Build from a winnow error and the full original input.
    pub fn from_winnow(
        err: &winnow::error::ParseError<&str, winnow::error::ContextError>,
        full_input: &str,
    ) -> Self {
        let offset = err.offset();
        let kind = ParseErrorKind::UnexpectedToken;
        Self::new(kind, offset..offset, full_input)
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at byte {}", self.kind, self.span.start)?;
        if !self.snippet.is_empty() {
            write!(f, ": \"{}\"", self.snippet)?;
        }
        Ok(())
    }
}

impl std::error::Error for ParseError {}

/// Snap a byte index to the nearest char boundary (rounding down).
fn snap_to_char_boundary(s: &str, idx: usize) -> usize {
    if idx >= s.len() {
        return s.len();
    }
    let mut i = idx;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_with_snippet() {
        let err = ParseError::new(ParseErrorKind::UnclosedString, 5..10, "hello\"world");
        assert!(err.to_string().contains("unclosed string"));
        assert!(err.to_string().contains("at byte 5"));
    }

    #[test]
    fn display_unexpected_eof() {
        let err = ParseError::new(ParseErrorKind::UnexpectedEof, 3..3, "abc");
        assert!(err.to_string().contains("unexpected end of input"));
    }

    #[test]
    fn snippet_truncated_to_40_chars() {
        let long_input = "x".repeat(100);
        let err = ParseError::new(ParseErrorKind::UnexpectedToken, 0..0, &long_input);
        assert!(err.snippet.len() <= 40);
    }

    #[test]
    fn unclosed_delimiter_display() {
        let err = ParseError::new(
            ParseErrorKind::UnclosedDelimiter { open: '[' },
            0..1,
            "[hello",
        );
        assert!(err.to_string().contains("unclosed delimiter '['"));
    }

    // ── snap_to_char_boundary ──

    #[test]
    fn snap_ascii_already_on_boundary() {
        assert_eq!(snap_to_char_boundary("hello", 0), 0);
        assert_eq!(snap_to_char_boundary("hello", 3), 3);
        assert_eq!(snap_to_char_boundary("hello", 5), 5);
    }

    #[test]
    fn snap_beyond_len_clamps() {
        assert_eq!(snap_to_char_boundary("hi", 99), 2);
        assert_eq!(snap_to_char_boundary("", 1), 0);
    }

    #[test]
    fn snap_mid_2byte_char() {
        // 'é' is 2 bytes: [0xC3, 0xA9]
        let s = "aé"; // bytes: [a=0x61, 0xC3, 0xA9]
        assert_eq!(snap_to_char_boundary(s, 2), 1); // byte 2 is mid-é → snaps to 1
    }

    #[test]
    fn snap_mid_3byte_char() {
        // '你' is 3 bytes: [0xE4, 0xBD, 0xA0]
        let s = "a你b"; // bytes: a(1) + 你(3) + b(1) = 5
        assert_eq!(snap_to_char_boundary(s, 2), 1); // mid-你, byte 2 → snaps to 1
        assert_eq!(snap_to_char_boundary(s, 3), 1); // mid-你, byte 3 → snaps to 1
        assert_eq!(snap_to_char_boundary(s, 4), 4); // 'b' start, already valid
    }

    #[test]
    fn snap_mid_4byte_char() {
        // '😀' is 4 bytes: [0xF0, 0x9F, 0x98, 0x80]
        let s = "😀x"; // bytes: 😀(4) + x(1) = 5
        assert_eq!(snap_to_char_boundary(s, 1), 0); // mid-emoji → snaps to 0
        assert_eq!(snap_to_char_boundary(s, 2), 0);
        assert_eq!(snap_to_char_boundary(s, 3), 0);
        assert_eq!(snap_to_char_boundary(s, 4), 4); // 'x' start, valid
    }

    #[test]
    fn snap_empty_string() {
        assert_eq!(snap_to_char_boundary("", 0), 0);
    }

    #[test]
    fn parse_error_with_multibyte_no_panic() {
        // The original crash case: span pointing into a multi-byte char.
        let input = "%%\n你好\n%%";
        let _ = ParseError::new(ParseErrorKind::UnexpectedToken, 4..4, input);
        // byte 4 is mid-'你' — must not panic
    }
}
