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
            Self::InvalidDirection => write!(f, "invalid direction (expected TB, BT, LR, RL, or TD)"),
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
        let start = span.start.min(input.len());
        let end = (start + 40).min(input.len());
        let snippet = input[start..end].to_string();
        Self { kind, span, snippet }
    }

    /// Build from a winnow error and the full original input.
    pub fn from_winnow(err: &winnow::error::ParseError<&str, winnow::error::ContextError>, full_input: &str) -> Self {
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
}
