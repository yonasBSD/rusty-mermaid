use rusty_mermaid_core::Theme;
use rusty_mermaid_diagrams::render_to_scene;
use rusty_mermaid_wgpu::render_to_png;

#[test]
fn render_state_diagram_with_text() {
    let scene = render_to_scene(
        "stateDiagram-v2\n    [*] --> Active\n    Active --> Paused : pause\n    Paused --> Active : resume\n    Active --> [*] : done",
    ).unwrap();
    let theme = Theme::light();
    let png = render_to_png(&scene, &theme, 2.0);
    let path = std::env::temp_dir().join("rusty_mermaid_gpu_state.png");
    std::fs::write(&path, &png).unwrap();
    eprintln!("GPU PNG: {}", path.display());
    assert!(png.len() > 1000);
}
