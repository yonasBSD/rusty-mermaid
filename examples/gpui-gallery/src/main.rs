use gpui::*;
use rusty_mermaid_core::Theme;
use rusty_mermaid_viewport::ViewportState;

static DIAGRAMS: &[(&str, &str)] = &[
    ("hello", "flowchart TD\n    A[Hello] --> B[World]"),
    ("decision", "flowchart TD\n    A[Start] --> B{Decision}\n    B -->|Yes| C[OK]\n    B -->|No| D[Fail]\n    C --> E[End]\n    D --> E"),
    ("state", "stateDiagram-v2\n    [*] --> Active\n    Active --> Paused : pause\n    Paused --> Active : resume\n    Active --> [*] : done"),
    ("subgraph", "flowchart TD\n    subgraph Frontend\n        A[React] --> B[Redux]\n    end\n    subgraph Backend\n        C[API] --> D[DB]\n    end\n    B --> C"),
    ("linear", "flowchart LR\n    A --> B --> C --> D --> E"),
    ("shapes", "flowchart TD\n    A[Rectangle] --> B(Rounded)\n    B --> C{Diamond}\n    C --> D([Stadium])\n    D --> E[[Subroutine]]"),
    ("sequence", "sequenceDiagram\n    Alice->>Bob: Hello\n    Bob-->>Alice: Hi\n    Alice->>Bob: How are you?\n    Bob-->>Alice: Fine!"),
    ("composite", "stateDiagram-v2\n    [*] --> Active\n    state Active {\n        [*] --> Running\n        Running --> Stopped : stop\n        Stopped --> Running : start\n    }\n    Active --> [*]"),
    ("arrows", "flowchart LR\n    A --> B\n    C --- D\n    E -.-> F\n    G ==> H"),
    ("diamond", "flowchart TD\n    Start --> IsValid{Valid?}\n    IsValid -->|Yes| Process\n    IsValid -->|No| Error\n    Process --> End\n    Error --> End"),
];

struct GalleryApp {
    current: usize,
}

impl Render for GalleryApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (name, mmd) = DIAGRAMS[self.current];
        let scene = rusty_mermaid_diagrams::render_to_scene(mmd).unwrap();
        let theme = Theme::light();
        let viewport = ViewportState::default();
        let total = DIAGRAMS.len();
        let idx = self.current;

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0xf5f5f5))
            // Header
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .p_4()
                    .bg(rgb(0xffffff))
                    .border_b_1()
                    .border_color(rgb(0xeeeeee))
                    .child(
                        div().flex().items_center().gap_3()
                            .child(
                                div().id("prev").px_3().py_1()
                                    .bg(rgb(0xf0f0f0)).rounded_md().cursor_pointer()
                                    .child("◀ Prev")
                                    .on_click(cx.listener(|this, _, _, _| {
                                        this.current = if this.current > 0 { this.current - 1 } else { DIAGRAMS.len() - 1 };
                                    }))
                            )
                            .child(format!("{} / {}  —  {}", idx + 1, total, name))
                            .child(
                                div().id("next").px_3().py_1()
                                    .bg(rgb(0xf0f0f0)).rounded_md().cursor_pointer()
                                    .child("Next ▶")
                                    .on_click(cx.listener(|this, _, _, _| {
                                        this.current = (this.current + 1) % DIAGRAMS.len();
                                    }))
                            )
                    )
                    .child("rusty-mermaid gpui gallery")
            )
            // Diagram
            .child(
                div().flex_1().p_4().child(
                    div().bg(rgb(0xffffff)).rounded_lg().p_4()
                        .child(
                            rusty_mermaid_gpui::render_element(scene, theme, viewport)
                        )
                )
            )
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1200.0), px(800.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("rusty-mermaid gpui gallery".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| cx.new(|_| GalleryApp { current: 0 }),
        )
        .unwrap();
    });
}
