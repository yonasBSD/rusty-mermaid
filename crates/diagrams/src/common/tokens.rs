use rusty_mermaid_core::Direction;
use winnow::combinator::{alt, opt, repeat};
use winnow::prelude::*;
use winnow::token::{any, take_while};

/// Skip horizontal whitespace (spaces and tabs, NOT newlines).
pub fn ws<'i>(input: &mut &'i str) -> ModalResult<&'i str> {
    take_while(0.., |c: char| c == ' ' || c == '\t').parse_next(input)
}

/// Skip whitespace including newlines.
pub fn ws_nl<'i>(input: &mut &'i str) -> ModalResult<&'i str> {
    take_while(0.., |c: char| c.is_ascii_whitespace()).parse_next(input)
}

/// Skip a `%%` line comment (consumes through end of line, not the newline itself).
pub fn line_comment<'i>(input: &mut &'i str) -> ModalResult<&'i str> {
    ("%%", take_while(0.., |c: char| c != '\n')).take().parse_next(input)
}

/// Skip any combination of whitespace, newlines, and `%%` comments.
pub fn skip<'i>(input: &mut &'i str) -> ModalResult<&'i str> {
    repeat::<_, _, (), _, _>(
        0..,
        alt((
            take_while(1.., |c: char| c.is_ascii_whitespace()),
            line_comment,
        )),
    )
    .take()
    .parse_next(input)
}

/// Statement separator: semicolon or newline (with surrounding whitespace/comments).
pub fn separator(input: &mut &str) -> ModalResult<()> {
    (
        ws,
        opt(line_comment),
        alt((
            ";".void(),
            winnow::combinator::peek(winnow::token::one_of(['\n', '\r'])).void(),
            winnow::combinator::eof.void(),
        )),
    )
        .void()
        .parse_next(input)
}

/// Parse an identifier: `[a-zA-Z_][a-zA-Z0-9_]*`.
pub fn identifier<'i>(input: &mut &'i str) -> ModalResult<&'i str> {
    (
        any.verify(|c: &char| c.is_ascii_alphabetic() || *c == '_'),
        take_while(0.., |c: char| c.is_ascii_alphanumeric() || c == '_'),
    )
        .take()
        .parse_next(input)
}

/// Parse a mermaid node ID: alphanumeric, underscore, hyphen (not before `>`/`.`).
/// More permissive than `identifier` to match mermaid's node ID rules.
pub fn node_id<'i>(input: &mut &'i str) -> ModalResult<&'i str> {
    (
        any.verify(|c: &char| c.is_ascii_alphanumeric() || *c == '_'),
        take_while(0.., |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-'),
    )
        .take()
        .parse_next(input)
}

/// Parse a double-quoted string, returning the content between quotes.
pub fn quoted_string<'i>(input: &mut &'i str) -> ModalResult<&'i str> {
    ('"', take_while(0.., |c: char| c != '"'), '"')
        .map(|(_, content, _): (_, &str, _)| content)
        .parse_next(input)
}

/// Convert `\uXXXX` escape sequences to actual Unicode characters.
/// Leaves other backslash sequences (e.g. `\n`, `\\`) untouched.
pub fn unescape_unicode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            // Peek at next char
            let mut peek = chars.clone();
            if peek.next() == Some('u') {
                let hex: String = peek.clone().take(4).collect();
                if hex.len() == 4 && hex.chars().all(|h| h.is_ascii_hexdigit()) {
                    if let Some(decoded) = u32::from_str_radix(&hex, 16)
                        .ok()
                        .and_then(char::from_u32)
                    {
                        result.push(decoded);
                        // Advance past 'u' + 4 hex digits
                        chars.next(); // 'u'
                        for _ in 0..4 {
                            chars.next();
                        }
                        continue;
                    }
                }
            }
            result.push(c);
        } else {
            result.push(c);
        }
    }
    result
}

/// Parse a direction keyword: TB, TD, BT, LR, RL.
pub fn direction(input: &mut &str) -> ModalResult<Direction> {
    alt((
        "TB".value(Direction::TB),
        "TD".value(Direction::TB),
        "BT".value(Direction::BT),
        "LR".value(Direction::LR),
        "RL".value(Direction::RL),
    ))
    .parse_next(input)
}

/// Consume text until hitting `close_delim`, handling nested quotes.
/// Returns the content (not including the delimiter).
pub fn text_until<'i>(close_delim: char, input: &mut &'i str) -> ModalResult<&'i str> {
    let mut depth = 0usize;
    let mut in_quotes = false;
    let start = *input;

    loop {
        if input.is_empty() {
            return Err(winnow::error::ErrMode::Backtrack(
                winnow::error::ContextError::new(),
            ));
        }
        let Some(c) = input.chars().next() else {
            return Err(winnow::error::ErrMode::Backtrack(
                winnow::error::ContextError::new(),
            ));
        };
        if c == '"' {
            in_quotes = !in_quotes;
            *input = &input[1..];
        } else if in_quotes {
            *input = &input[c.len_utf8()..];
        } else if c == close_delim && depth == 0 {
            let consumed = &start[..start.len() - input.len()];
            return Ok(consumed);
        } else {
            if c == '[' || c == '(' || c == '{' {
                depth += 1;
            } else if (c == ']' || c == ')' || c == '}') && depth > 0 {
                depth -= 1;
            }
            *input = &input[c.len_utf8()..];
        }
    }
}

/// Strip HTML tags from label text for text measurement.
/// `<b>Bold</b>` → `Bold`, `Line 1<br/>Line 2` → `Line 1\nLine 2`.
pub fn strip_html_tags(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find('<') {
        result.push_str(&rest[..start]);
        if let Some(end) = rest[start..].find('>') {
            let tag = &rest[start..start + end + 1];
            // Convert line-break tags to newlines for multi-line rendering
            if tag.eq_ignore_ascii_case("<br>") || tag.eq_ignore_ascii_case("<br/>") || tag.eq_ignore_ascii_case("<br />") {
                result.push('\n');
            }
            rest = &rest[start + end + 1..];
        } else {
            // No closing '>' — treat rest as plain text
            result.push_str(&rest[start..]);
            rest = "";
        }
    }
    result.push_str(rest);
    result
}

/// Parse `:::className`, returning the class name.
pub fn style_class<'i>(input: &mut &'i str) -> ModalResult<&'i str> {
    (":::", identifier).map(|(_, name)| name).parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_identifier() {
        assert_eq!(identifier.parse_peek("hello world"), Ok((" world", "hello")));
        assert_eq!(identifier.parse_peek("_foo"), Ok(("", "_foo")));
        assert!(identifier.parse_peek("123").is_err());
    }

    #[test]
    fn parse_node_id() {
        assert_eq!(node_id.parse_peek("my-node rest"), Ok((" rest", "my-node")));
        assert_eq!(node_id.parse_peek("A123"), Ok(("", "A123")));
        assert!(node_id.parse_peek("-bad").is_err());
    }

    #[test]
    fn parse_quoted_string() {
        assert_eq!(quoted_string.parse_peek("\"hello world\""), Ok(("", "hello world")));
        assert_eq!(
            quoted_string.parse_peek("\"<b>Bold</b>\" rest"),
            Ok((" rest", "<b>Bold</b>"))
        );
    }

    #[test]
    fn parse_direction_keywords() {
        assert_eq!(direction.parse_peek("TB"), Ok(("", Direction::TB)));
        assert_eq!(direction.parse_peek("TD"), Ok(("", Direction::TB)));
        assert_eq!(direction.parse_peek("LR"), Ok(("", Direction::LR)));
        assert_eq!(direction.parse_peek("RL"), Ok(("", Direction::RL)));
        assert_eq!(direction.parse_peek("BT"), Ok(("", Direction::BT)));
    }

    #[test]
    fn parse_text_until_delimiter() {
        let mut input = "hello world]rest";
        let result = text_until(']', &mut input).unwrap();
        assert_eq!(result, "hello world");
        assert_eq!(input, "]rest");
    }

    #[test]
    fn text_until_handles_nested_brackets() {
        let mut input = "a [nested] end]rest";
        let result = text_until(']', &mut input).unwrap();
        assert_eq!(result, "a [nested] end");
    }

    #[test]
    fn strip_tags() {
        assert_eq!(strip_html_tags("<b>Bold</b>"), "Bold");
        assert_eq!(strip_html_tags("Line 1<br/>Line 2"), "Line 1\nLine 2");
        assert_eq!(
            strip_html_tags("<i>italic</i> and <b>bold</b>"),
            "italic and bold"
        );
        assert_eq!(strip_html_tags("no tags"), "no tags");
    }

    #[test]
    fn parse_style_class() {
        assert_eq!(
            style_class.parse_peek(":::myClass rest"),
            Ok((" rest", "myClass"))
        );
    }

    #[test]
    fn skip_whitespace_and_comments() {
        let mut input = "  \n  %% comment\n  rest";
        skip.parse_next(&mut input).unwrap();
        assert_eq!(input, "rest");
    }

    #[test]
    fn line_comment_stops_at_newline() {
        let mut input = "%% this is a comment\nnext";
        line_comment.parse_next(&mut input).unwrap();
        assert_eq!(input, "\nnext");
    }

    #[test]
    fn unescape_basic() {
        assert_eq!(unescape_unicode(r"\u00e9"), "é");
        assert_eq!(unescape_unicode(r"\u2615"), "☕");
        assert_eq!(unescape_unicode(r"Caf\u00e9 \u2615"), "Café ☕");
    }

    #[test]
    fn unescape_no_escapes() {
        assert_eq!(unescape_unicode("hello"), "hello");
        assert_eq!(unescape_unicode("Café ☕"), "Café ☕");
    }

    #[test]
    fn unescape_partial_sequence() {
        // Incomplete \u sequences left as-is
        assert_eq!(unescape_unicode(r"\u00"), r"\u00");
        assert_eq!(unescape_unicode(r"\u"), r"\u");
        assert_eq!(unescape_unicode(r"\uzzz"), r"\uzzz");
    }

    #[test]
    fn unescape_cjk_and_symbols() {
        assert_eq!(unescape_unicode(r"\u4f60\u597d"), "你好");
        assert_eq!(unescape_unicode(r"\u03b1 + \u03b2"), "α + β");
    }
}
