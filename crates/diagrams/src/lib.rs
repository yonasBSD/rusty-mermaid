pub mod common;

#[cfg(feature = "flowchart")]
pub mod flowchart;

#[cfg(feature = "state")]
pub mod state;

#[cfg(feature = "sequence")]
pub mod sequence;

#[cfg(feature = "class")]
pub mod class;

#[cfg(feature = "er")]
pub mod er;

#[cfg(feature = "requirement")]
pub mod requirement;

#[cfg(feature = "pie")]
pub mod pie;

#[cfg(feature = "timeline")]
pub mod timeline;

#[cfg(feature = "kanban")]
pub mod kanban;

use common::error::ParseError;

/// Supported diagram types.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagramKind {
    Flowchart,
    State,
    Sequence,
    Class,
    Er,
    Requirement,
    Pie,
    Timeline,
    Kanban,
}

/// Detect the diagram type from the first non-empty, non-comment line.
pub fn detect(input: &str) -> Option<DiagramKind> {
    let line = input
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with("%%"))?;

    if line.starts_with("graph") || line.starts_with("flowchart") {
        return Some(DiagramKind::Flowchart);
    }
    if line.starts_with("stateDiagram") {
        return Some(DiagramKind::State);
    }
    if line.starts_with("sequenceDiagram") {
        return Some(DiagramKind::Sequence);
    }
    if line.starts_with("classDiagram") {
        return Some(DiagramKind::Class);
    }
    if line.starts_with("erDiagram") {
        return Some(DiagramKind::Er);
    }
    if line.starts_with("requirementDiagram") {
        return Some(DiagramKind::Requirement);
    }
    if line.starts_with("pie") {
        return Some(DiagramKind::Pie);
    }
    if line.starts_with("timeline") {
        return Some(DiagramKind::Timeline);
    }
    if line.starts_with("kanban") {
        return Some(DiagramKind::Kanban);
    }
    None
}

/// Unified entry: parse + layout → Scene.
#[cfg(any(feature = "flowchart", feature = "state", feature = "sequence", feature = "class", feature = "er", feature = "requirement", feature = "pie", feature = "timeline", feature = "kanban"))]
pub fn render_to_scene(input: &str) -> Result<rusty_mermaid_core::Scene, ParseError> {
    render_to_scene_themed(input, &rusty_mermaid_core::Theme::default())
}

/// Strip mermaid directives, accessibility metadata, and leading comments.
///
/// Handles:
/// - `%%{init: ...}%%` configuration directives
/// - `accTitle: ...` accessibility title
/// - `accDescr: ...` or `accDescr { ... }` accessibility description
/// - `%%` comment lines before the diagram keyword
fn preprocess(input: &str) -> String {
    let mut lines: Vec<&str> = Vec::new();
    let mut in_acc_block = false;

    for line in input.lines() {
        let trimmed = line.trim();

        // Skip %%{...}%% directives
        if trimmed.starts_with("%%{") && trimmed.ends_with("}%%") {
            continue;
        }

        // Skip accTitle / accDescr single-line
        if trimmed.starts_with("accTitle:") || trimmed.starts_with("accDescr:") {
            continue;
        }

        // Handle accDescr { ... } multi-line block
        if trimmed.starts_with("accDescr") && trimmed.contains('{') {
            in_acc_block = true;
            continue;
        }
        if in_acc_block {
            if trimmed.contains('}') {
                in_acc_block = false;
            }
            continue;
        }

        lines.push(line);
    }

    lines.join("\n")
}

/// Unified entry with explicit theme: parse + layout → Scene.
#[cfg(any(feature = "flowchart", feature = "state", feature = "sequence", feature = "class", feature = "er", feature = "requirement", feature = "pie", feature = "timeline", feature = "kanban"))]
pub fn render_to_scene_themed(
    input: &str,
    theme: &rusty_mermaid_core::Theme,
) -> Result<rusty_mermaid_core::Scene, ParseError> {
    let cleaned = preprocess(input);
    let input = &cleaned;

    let kind = detect(input).ok_or_else(|| {
        ParseError::new(
            common::error::ParseErrorKind::UnexpectedToken,
            0..0,
            input,
        )
    })?;

    match kind {
        #[cfg(feature = "flowchart")]
        DiagramKind::Flowchart => {
            let diagram = flowchart::parser::parse(input)?;
            let layout = flowchart::bridge::layout(&diagram);
            Ok(flowchart::to_scene_themed(&layout, theme))
        }
        #[cfg(feature = "state")]
        DiagramKind::State => {
            let diagram = state::parser::parse(input)?;
            let layout = state::bridge::layout(&diagram);
            Ok(state::to_scene_themed(&layout, theme))
        }
        #[cfg(feature = "sequence")]
        DiagramKind::Sequence => {
            let diagram = sequence::parser::parse(input)?;
            let layout = sequence::layout::layout(
                &diagram,
                &rusty_mermaid_core::SimpleTextMeasure::default(),
            );
            Ok(sequence::to_scene_themed(&layout, theme))
        }
        #[cfg(feature = "class")]
        DiagramKind::Class => {
            let diagram = class::parser::parse(input)?;
            let layout = class::bridge::layout(&diagram);
            Ok(class::to_scene_themed(&layout, theme))
        }
        #[cfg(feature = "er")]
        DiagramKind::Er => {
            let diagram = er::parser::parse(input)?;
            let layout = er::bridge::layout(&diagram);
            Ok(er::to_scene_themed(&layout, theme))
        }
        #[cfg(feature = "requirement")]
        DiagramKind::Requirement => {
            let diagram = requirement::parser::parse(input)?;
            let layout = requirement::bridge::layout(&diagram);
            Ok(requirement::to_scene_themed(&layout, theme))
        }
        #[cfg(feature = "pie")]
        DiagramKind::Pie => {
            let chart = pie::parser::parse(input)?;
            Ok(pie::to_scene_themed(&chart, theme))
        }
        #[cfg(feature = "timeline")]
        DiagramKind::Timeline => {
            let diagram = timeline::parser::parse(input)?;
            Ok(timeline::to_scene_themed(&diagram, theme))
        }
        #[cfg(feature = "kanban")]
        DiagramKind::Kanban => {
            let board = kanban::parser::parse(input)?;
            Ok(kanban::to_scene_themed(&board, theme))
        }
        #[allow(unreachable_patterns)]
        _ => Err(ParseError::new(
            common::error::ParseErrorKind::UnexpectedToken,
            0..0,
            input,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_flowchart_graph() {
        assert_eq!(detect("graph TD\n    A --> B"), Some(DiagramKind::Flowchart));
    }

    #[test]
    fn detect_flowchart_keyword() {
        assert_eq!(detect("flowchart LR\n    A --> B"), Some(DiagramKind::Flowchart));
    }

    #[test]
    fn detect_state_v2() {
        assert_eq!(detect("stateDiagram-v2\n    A --> B"), Some(DiagramKind::State));
    }

    #[test]
    fn detect_state_v1() {
        assert_eq!(detect("stateDiagram\n    A --> B"), Some(DiagramKind::State));
    }

    #[test]
    fn detect_skips_comments() {
        assert_eq!(detect("%% comment\ngraph TD\n    A --> B"), Some(DiagramKind::Flowchart));
    }

    #[test]
    fn detect_sequence() {
        assert_eq!(
            detect("sequenceDiagram\n    Alice->>Bob: Hello"),
            Some(DiagramKind::Sequence)
        );
    }

    #[test]
    fn detect_sequence_with_comment() {
        assert_eq!(
            detect("%% comment\nsequenceDiagram\n    A->>B: hi"),
            Some(DiagramKind::Sequence)
        );
    }

    #[test]
    fn detect_class_diagram() {
        assert_eq!(detect("classDiagram\n    class Foo"), Some(DiagramKind::Class));
    }

    #[test]
    fn detect_class_diagram_v2() {
        assert_eq!(detect("classDiagram-v2\n    class Foo"), Some(DiagramKind::Class));
    }

    #[test]
    fn detect_pie() {
        assert_eq!(detect("pie\n    \"A\" : 50"), Some(DiagramKind::Pie));
    }

    #[test]
    fn detect_unknown() {
        assert_eq!(detect("unknownDiagram\n    stuff"), None);
    }

    #[cfg(feature = "flowchart")]
    #[test]
    fn render_flowchart_to_scene() {
        let scene = render_to_scene("graph TD\n    A[Start] --> B[End]").unwrap();
        assert!(scene.width > 0.0);
        assert!(!scene.is_empty());
    }

    #[cfg(feature = "state")]
    #[test]
    fn render_state_to_scene() {
        let scene = render_to_scene("stateDiagram-v2\n    [*] --> Still\n    Still --> [*]").unwrap();
        assert!(scene.width > 0.0);
        assert!(!scene.is_empty());
    }

    #[cfg(feature = "class")]
    #[test]
    fn render_class_to_scene() {
        let scene = render_to_scene("classDiagram\n    class Animal {\n        +String name\n        +makeSound()\n    }\n    Animal <|-- Dog").unwrap();
        assert!(scene.width > 0.0);
        assert!(!scene.is_empty());
    }

    #[cfg(feature = "sequence")]
    #[test]
    fn render_sequence_to_scene() {
        let scene =
            render_to_scene("sequenceDiagram\n    Alice->>Bob: Hello\n    Bob-->>Alice: Hi")
                .unwrap();
        assert!(scene.width > 0.0);
        assert!(!scene.is_empty());
    }
}
