pub mod common;

#[cfg(feature = "flowchart")]
pub mod flowchart;

#[cfg(feature = "state")]
pub mod state;

#[cfg(feature = "sequence")]
pub mod sequence;

use common::error::ParseError;

/// Supported diagram types.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagramKind {
    Flowchart,
    State,
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
    None
}

/// Unified entry: parse + layout → Scene.
#[cfg(any(feature = "flowchart", feature = "state"))]
pub fn render_to_scene(input: &str) -> Result<rusty_mermaid_core::Scene, ParseError> {
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
            Ok(flowchart::to_scene(&layout))
        }
        #[cfg(feature = "state")]
        DiagramKind::State => {
            let diagram = state::parser::parse(input)?;
            let layout = state::bridge::layout(&diagram);
            Ok(state::to_scene(&layout))
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
    fn detect_unknown() {
        assert_eq!(detect("pie\n    title Chart"), None);
    }

    #[cfg(feature = "flowchart")]
    #[test]
    fn render_flowchart_to_scene() {
        let scene = render_to_scene("graph TD\n    A[Start] --> B[End]").unwrap();
        assert!(scene.width > 0.0);
        assert!(!scene.primitives().is_empty());
    }

    #[cfg(feature = "state")]
    #[test]
    fn render_state_to_scene() {
        let scene = render_to_scene("stateDiagram-v2\n    [*] --> Still\n    Still --> [*]").unwrap();
        assert!(scene.width > 0.0);
        assert!(!scene.primitives().is_empty());
    }
}
