pub mod ir;
pub mod parser;

use rusty_mermaid_core::{
    Color, PathSegment, Point, Primitive, Scene, Style, TextAnchor, TextStyle, Theme,
};

use ir::{TreeNode, TreeView};

const INDENT_W: f64 = 24.0;
const LINE_H: f64 = 22.0;
const CONNECTOR_GAP: f64 = 8.0;
const SCENE_PAD: f64 = 16.0;
const LINE_COLOR: Color = Color::rgb(160, 160, 160);

pub fn to_scene(tree: &TreeView) -> Scene {
    to_scene_themed(tree, &Theme::default())
}

pub fn to_scene_themed(tree: &TreeView, theme: &Theme) -> Scene {
    if tree.roots.is_empty() {
        return Scene::empty();
    }

    // Flatten tree: assign a row index to each node, track parent info
    let mut rows: Vec<FlatRow> = Vec::new();
    for (i, root) in tree.roots.iter().enumerate() {
        flatten(root, 0, i == tree.roots.len() - 1, &mut rows);
    }

    let max_depth = rows.iter().map(|r| r.depth).max().unwrap_or(0);
    let scene_w = SCENE_PAD * 2.0 + (max_depth + 1) as f64 * INDENT_W + 200.0;
    let scene_h = SCENE_PAD * 2.0 + rows.len() as f64 * LINE_H;
    let mut scene = Scene::new(scene_w, scene_h);

    let line_style = Style {
        stroke: Some(LINE_COLOR),
        stroke_width: Some(1.0),
        ..Default::default()
    };

    render_connectors(&mut scene, &rows, &line_style);
    render_stubs(&mut scene, &rows, &line_style);
    render_tree_labels(&mut scene, &rows, theme);

    scene
}

fn render_connectors(scene: &mut Scene, rows: &[FlatRow], line_style: &Style) {
    let mut i = 0;
    while i < rows.len() {
        let depth = rows[i].depth;
        let first_child = i + 1;
        if first_child < rows.len() && rows[first_child].depth > depth {
            let mut last_child = first_child;
            for j in (first_child + 1)..rows.len() {
                if rows[j].depth <= depth {
                    break;
                }
                if rows[j].depth == depth + 1 {
                    last_child = j;
                }
            }

            let vx = SCENE_PAD + depth as f64 * INDENT_W + CONNECTOR_GAP;
            let vy_top = SCENE_PAD + i as f64 * LINE_H + LINE_H;
            let vy_bot = SCENE_PAD + last_child as f64 * LINE_H + LINE_H / 2.0;

            scene.push(Primitive::Path {
                segments: vec![
                    PathSegment::MoveTo(Point::new(vx, vy_top)),
                    PathSegment::LineTo(Point::new(vx, vy_bot)),
                ],
                style: line_style.clone(),
                marker_start: None,
                marker_end: None,
            });
        }
        i += 1;
    }
}

fn render_stubs(scene: &mut Scene, rows: &[FlatRow], line_style: &Style) {
    for (row_idx, row) in rows.iter().enumerate() {
        if row.depth > 0 {
            let x = SCENE_PAD + row.depth as f64 * INDENT_W;
            let y = SCENE_PAD + row_idx as f64 * LINE_H + LINE_H / 2.0;
            let parent_vx = x - INDENT_W + CONNECTOR_GAP;

            scene.push(Primitive::Path {
                segments: vec![
                    PathSegment::MoveTo(Point::new(parent_vx, y)),
                    PathSegment::LineTo(Point::new(x - 2.0, y)),
                ],
                style: line_style.clone(),
                marker_start: None,
                marker_end: None,
            });
        }
    }
}

fn render_tree_labels(scene: &mut Scene, rows: &[FlatRow], theme: &Theme) {
    for (row_idx, row) in rows.iter().enumerate() {
        let x = SCENE_PAD + row.depth as f64 * INDENT_W;
        let y = SCENE_PAD + row_idx as f64 * LINE_H + LINE_H / 2.0;

        let font_weight = if row.has_children {
            rusty_mermaid_core::FontWeight::Bold
        } else {
            rusty_mermaid_core::FontWeight::Normal
        };

        scene.push(Primitive::Text {
            position: Point::new(x, y),
            content: row.name.clone(),
            anchor: TextAnchor::Start,
            style: TextStyle {
                font_size: theme.font_size_node,
                fill: Some(theme.node_text),
                font_weight,
                ..Default::default()
            },
        });
    }
}

struct FlatRow {
    name: String,
    depth: usize,
    has_children: bool,
}

fn flatten(node: &TreeNode, depth: usize, _is_last: bool, rows: &mut Vec<FlatRow>) {
    rows.push(FlatRow {
        name: node.name.clone(),
        depth,
        has_children: !node.children.is_empty(),
    });
    for (i, child) in node.children.iter().enumerate() {
        flatten(child, depth + 1, i == node.children.len() - 1, rows);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(input: &str) -> Scene {
        let t = parser::parse(input).unwrap();
        to_scene(&t)
    }

    #[test]
    fn basic_renders() {
        let scene = render("treeView-beta\n    root\n        child");
        assert!(!scene.is_empty());
    }

    #[test]
    fn has_text_labels() {
        let scene = render("treeView-beta\n    src\n        main.rs\n        lib.rs");
        let labels: Vec<&str> = scene.elements().iter().filter_map(|e| {
            if let Primitive::Text { content, .. } = &e.primitive { Some(content.as_str()) } else { None }
        }).collect();
        assert!(labels.contains(&"src"));
        assert!(labels.contains(&"main.rs"));
        assert!(labels.contains(&"lib.rs"));
    }

    #[test]
    fn has_connector_lines() {
        let scene = render("treeView-beta\n    root\n        child");
        let lines = scene.elements().iter().filter(|e| {
            matches!(&e.primitive, Primitive::Path { .. })
        }).count();
        assert!(lines >= 2, "should have horizontal + vertical connectors");
    }

    #[test]
    fn deeper_nodes_indented_right() {
        let scene = render("treeView-beta\n    a\n        b\n            c");
        let positions: Vec<f64> = scene.elements().iter().filter_map(|e| {
            if let Primitive::Text { position, .. } = &e.primitive { Some(position.x) } else { None }
        }).collect();
        assert_eq!(positions.len(), 3);
        assert!(positions[1] > positions[0], "b should be right of a");
        assert!(positions[2] > positions[1], "c should be right of b");
    }

    #[test]
    fn parent_nodes_bold() {
        let scene = render("treeView-beta\n    folder\n        file.rs");
        let folder_bold = scene.elements().iter().any(|e| {
            if let Primitive::Text { content, style, .. } = &e.primitive {
                content == "folder" && style.font_weight == rusty_mermaid_core::FontWeight::Bold
            } else { false }
        });
        let file_normal = scene.elements().iter().any(|e| {
            if let Primitive::Text { content, style, .. } = &e.primitive {
                content == "file.rs" && style.font_weight == rusty_mermaid_core::FontWeight::Normal
            } else { false }
        });
        assert!(folder_bold, "folder should be bold");
        assert!(file_normal, "file should be normal weight");
    }

    #[test]
    fn all_positions_finite() {
        let scene = render("treeView-beta\n    a\n        b\n            c\n        d\n    e");
        for elem in scene.elements() {
            match &elem.primitive {
                Primitive::Text { position, .. } => {
                    assert!(position.x.is_finite() && position.y.is_finite());
                }
                Primitive::Path { segments, .. } => {
                    for seg in segments {
                        match seg {
                            PathSegment::MoveTo(p) | PathSegment::LineTo(p) => {
                                assert!(p.x.is_finite() && p.y.is_finite());
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
