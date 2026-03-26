use crate::common::error::{ParseError, ParseErrorKind};
use crate::common::tokens::skip;
use winnow::prelude::*;

use super::ir::*;

pub fn parse(input: &str) -> Result<GitGraph, ParseError> {
    let mut rest = input;
    parse_gitgraph(&mut rest).map_err(|_| {
        let offset = input.len() - rest.len();
        ParseError::new(ParseErrorKind::UnexpectedToken, offset..offset, input)
    })
}

fn parse_gitgraph(input: &mut &str) -> ModalResult<GitGraph> {
    skip.parse_next(input)?;
    "gitGraph".parse_next(input)?;

    let mut graph = GitGraph::new();

    // Optional direction
    skip_horizontal_ws(input);
    if input.starts_with("LR") { *input = &input[2..]; graph.direction = GitDirection::LR; }
    else if input.starts_with("TB") { *input = &input[2..]; graph.direction = GitDirection::TB; }
    else if input.starts_with("BT") { *input = &input[2..]; graph.direction = GitDirection::BT; }

    loop {
        skip.parse_next(input)?;
        if input.is_empty() { break; }

        let line = take_line(input);
        if line.is_empty() { continue; }

        if let Some(stmt) = parse_statement(line) {
            graph.statements.push(stmt);
        }
    }

    Ok(graph)
}

fn parse_statement(line: &str) -> Option<GitStatement> {
    let line = line.trim();

    if line.starts_with("commit") {
        let rest = line.strip_prefix("commit").unwrap_or("").trim();
        let id = extract_option(rest, "id:");
        let tag = extract_option(rest, "tag:");
        let commit_type = extract_commit_type(rest);
        return Some(GitStatement::Commit { id, tag, commit_type });
    }

    if line.starts_with("branch") {
        let rest = line.strip_prefix("branch").expect("guarded by starts_with").trim();
        let parts: Vec<&str> = rest.splitn(2, |c: char| c == ' ' || c == '\t').collect();
        let name = parts[0].to_string();
        let order = parts.get(1).and_then(|r| {
            extract_option(r, "order:").and_then(|o| o.parse().ok())
        });
        return Some(GitStatement::Branch { name, order });
    }

    if line.starts_with("checkout") || line.starts_with("switch") {
        let keyword_len = if line.starts_with("checkout") { 8 } else { 6 };
        let name = line[keyword_len..].trim().to_string();
        return Some(GitStatement::Checkout(name));
    }

    if line.starts_with("merge") {
        let rest = line.strip_prefix("merge").expect("guarded by starts_with").trim();
        let parts: Vec<&str> = rest.splitn(2, |c: char| c == ' ' || c == '\t').collect();
        let branch = parts[0].to_string();
        let opts = parts.get(1).copied().unwrap_or("");
        let id = extract_option(opts, "id:");
        let tag = extract_option(opts, "tag:");
        let commit_type = extract_commit_type(opts);
        return Some(GitStatement::Merge { branch, id, tag, commit_type });
    }

    if line.starts_with("cherry-pick") {
        let rest = line.strip_prefix("cherry-pick").expect("guarded by starts_with").trim();
        let id = extract_option(rest, "id:").unwrap_or_default();
        let tag = extract_option(rest, "tag:");
        return Some(GitStatement::CherryPick { id, tag });
    }

    None
}

fn extract_option(text: &str, key: &str) -> Option<String> {
    let idx = text.find(key)?;
    let rest = text[idx + key.len()..].trim();
    if rest.starts_with('"') {
        let end = rest[1..].find('"')?;
        Some(rest[1..1 + end].to_string())
    } else {
        let end = rest.find(|c: char| c == ' ' || c == '\t').unwrap_or(rest.len());
        Some(rest[..end].to_string())
    }
}

fn extract_commit_type(text: &str) -> CommitType {
    if text.contains("type: REVERSE") || text.contains("type:REVERSE") {
        CommitType::Reverse
    } else if text.contains("type: HIGHLIGHT") || text.contains("type:HIGHLIGHT") {
        CommitType::Highlight
    } else {
        CommitType::Normal
    }
}

fn skip_horizontal_ws(input: &mut &str) {
    *input = input.trim_start_matches(|c: char| c == ' ' || c == '\t');
}

fn take_line<'i>(input: &mut &'i str) -> &'i str {
    let end = input.find('\n').unwrap_or(input.len());
    let line = input[..end].trim();
    *input = if end < input.len() { &input[end + 1..] } else { "" };
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        let g = parse("gitGraph\n    commit\n    commit\n    commit").unwrap();
        assert_eq!(g.statements.len(), 3);
        assert!(matches!(g.statements[0], GitStatement::Commit { .. }));
    }

    #[test]
    fn parse_commit_with_id_and_tag() {
        let g = parse("gitGraph\n    commit id: \"abc\" tag: \"v1.0\"").unwrap();
        if let GitStatement::Commit { id, tag, .. } = &g.statements[0] {
            assert_eq!(id.as_deref(), Some("abc"));
            assert_eq!(tag.as_deref(), Some("v1.0"));
        } else { panic!("expected commit"); }
    }

    #[test]
    fn parse_commit_type() {
        let g = parse("gitGraph\n    commit type: REVERSE\n    commit type: HIGHLIGHT").unwrap();
        if let GitStatement::Commit { commit_type, .. } = &g.statements[0] {
            assert_eq!(*commit_type, CommitType::Reverse);
        }
        if let GitStatement::Commit { commit_type, .. } = &g.statements[1] {
            assert_eq!(*commit_type, CommitType::Highlight);
        }
    }

    #[test]
    fn parse_branch_and_checkout() {
        let g = parse("gitGraph\n    commit\n    branch develop\n    checkout develop\n    commit").unwrap();
        assert_eq!(g.statements.len(), 4);
        assert!(matches!(&g.statements[1], GitStatement::Branch { name, .. } if name == "develop"));
        assert!(matches!(&g.statements[2], GitStatement::Checkout(name) if name == "develop"));
    }

    #[test]
    fn parse_merge() {
        let g = parse("gitGraph\n    commit\n    branch feature\n    commit\n    checkout main\n    merge feature tag: \"v2.0\"").unwrap();
        if let GitStatement::Merge { branch, tag, .. } = &g.statements[4] {
            assert_eq!(branch, "feature");
            assert_eq!(tag.as_deref(), Some("v2.0"));
        } else { panic!("expected merge"); }
    }

    #[test]
    fn parse_cherry_pick() {
        let g = parse("gitGraph\n    commit id: \"abc\"\n    branch dev\n    cherry-pick id: \"abc\"").unwrap();
        if let GitStatement::CherryPick { id, .. } = &g.statements[2] {
            assert_eq!(id, "abc");
        } else { panic!("expected cherry-pick"); }
    }

    #[test]
    fn parse_switch_alias() {
        let g = parse("gitGraph\n    branch dev\n    switch main").unwrap();
        assert!(matches!(&g.statements[1], GitStatement::Checkout(name) if name == "main"));
    }

    #[test]
    fn parse_direction_tb() {
        let g = parse("gitGraph TB\n    commit").unwrap();
        assert_eq!(g.direction, GitDirection::TB);
    }

    #[test]
    fn parse_comments() {
        let g = parse("gitGraph\n    %% comment\n    commit").unwrap();
        assert_eq!(g.statements.len(), 1);
    }

    #[test]
    fn reject_empty() {
        assert!(parse("").is_err());
    }

    #[test]
    fn reject_wrong_header() {
        assert!(parse("gantt\n    title X").is_err());
    }
}
