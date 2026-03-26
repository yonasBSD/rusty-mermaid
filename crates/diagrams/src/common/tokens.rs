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
                if hex.len() == 4
                    && hex.chars().all(|h| h.is_ascii_hexdigit())
                    && let Some(decoded) = u32::from_str_radix(&hex, 16)
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
#[path = "tokens_tests.rs"]
mod tokens_tests;
