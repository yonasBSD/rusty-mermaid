use rusty_mermaid_core::Renderer;
use rusty_mermaid_core::Theme;
use rusty_mermaid_diagrams::render_to_scene;
use rusty_mermaid_svg::SvgRenderer;

#[test]
fn flowchart_to_svg() {
    let mmd = "graph TD\n    A[Start] --> B{Decision}\n    B -->|Yes| C[OK]\n    B -->|No| D[Fail]";
    let scene = render_to_scene(mmd, &Theme::default()).unwrap();
    let svg = SvgRenderer::new().render(&scene);

    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("<rect"));
    assert!(svg.contains("<text"));
    assert!(svg.contains("<path"));
    assert!(svg.contains("arrow-point"));
    assert!(svg.trim_end().ends_with("</svg>"));
}

#[test]
fn state_diagram_to_svg() {
    let mmd = "stateDiagram-v2\n    [*] --> Still\n    Still --> Moving\n    Moving --> Crash\n    Crash --> [*]";
    let scene = render_to_scene(mmd, &Theme::default()).unwrap();
    let svg = SvgRenderer::new().render(&scene);

    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("<rect"));
    assert!(svg.contains("<path"));
    assert!(svg.trim_end().ends_with("</svg>"));
}

#[test]
fn complex_flowchart_to_svg() {
    let mmd = r#"graph LR
    A[Input] --> B[Process]
    B --> C{Check}
    C -->|Pass| D[Output]
    C -->|Fail| E[Error]
    E --> B"#;
    let scene = render_to_scene(mmd, &Theme::default()).unwrap();
    let svg = SvgRenderer::new().render(&scene);

    assert!(svg.contains("viewBox"));
    // Should have nodes and edges
    let rect_count = svg.matches("<rect").count();
    let path_count = svg.matches("<path").count();
    assert!(rect_count >= 4, "expected >= 4 rects, got {rect_count}");
    assert!(path_count >= 5, "expected >= 5 paths, got {path_count}");
}

#[test]
fn sequence_diagram_to_svg() {
    let mmd = "sequenceDiagram\n    Alice->>Bob: Hello\n    Bob-->>Alice: Hi there\n    Note right of Bob: Thinking";
    let scene = render_to_scene(mmd, &Theme::default()).unwrap();
    let svg = SvgRenderer::new().render(&scene);

    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("<rect"));
    assert!(svg.contains("<text"));
    assert!(svg.contains("<path"));
    assert!(svg.trim_end().ends_with("</svg>"));

    // Should have participant names and message labels.
    assert!(svg.contains("Alice"));
    assert!(svg.contains("Bob"));
    assert!(svg.contains("Hello"));
}

#[test]
fn sequence_self_message_to_svg() {
    let mmd = "sequenceDiagram\n    Alice->>Alice: Think\n    Alice->>Bob: Done";
    let scene = render_to_scene(mmd, &Theme::default()).unwrap();
    let svg = SvgRenderer::new().render(&scene);

    assert!(svg.contains("Think"));
    assert!(svg.contains("Done"));
}

#[test]
fn svg_output_is_valid_xml_structure() {
    let mmd = "graph TD\n    A --> B";
    let scene = render_to_scene(mmd, &Theme::default()).unwrap();
    let svg = SvgRenderer::new().render(&scene);

    // Basic well-formedness: matching open/close tags
    let open_svg = svg.matches("<svg").count();
    let close_svg = svg.matches("</svg>").count();
    assert_eq!(open_svg, 1);
    assert_eq!(close_svg, 1);

    let open_g = svg.matches("<g").count();
    let close_g = svg.matches("</g>").count();
    assert_eq!(open_g, close_g);
}
