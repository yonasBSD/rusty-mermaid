use super::*;

#[test]
fn detect_flowchart_graph() {
    assert_eq!(
        detect("graph TD\n    A --> B"),
        Some(DiagramKind::Flowchart)
    );
}

#[test]
fn detect_flowchart_keyword() {
    assert_eq!(
        detect("flowchart LR\n    A --> B"),
        Some(DiagramKind::Flowchart)
    );
}

#[test]
fn detect_state_v2() {
    assert_eq!(
        detect("stateDiagram-v2\n    A --> B"),
        Some(DiagramKind::State)
    );
}

#[test]
fn detect_state_v1() {
    assert_eq!(
        detect("stateDiagram\n    A --> B"),
        Some(DiagramKind::State)
    );
}

#[test]
fn detect_skips_comments() {
    assert_eq!(
        detect("%% comment\ngraph TD\n    A --> B"),
        Some(DiagramKind::Flowchart)
    );
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
    assert_eq!(
        detect("classDiagram\n    class Foo"),
        Some(DiagramKind::Class)
    );
}

#[test]
fn detect_class_diagram_v2() {
    assert_eq!(
        detect("classDiagram-v2\n    class Foo"),
        Some(DiagramKind::Class)
    );
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
        render_to_scene("sequenceDiagram\n    Alice->>Bob: Hello\n    Bob-->>Alice: Hi").unwrap();
    assert!(scene.width > 0.0);
    assert!(!scene.is_empty());
}
