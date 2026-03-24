pub mod ir;
pub mod parser;

use rusty_mermaid_core::{
    BBox, Color, PathSegment, Point, Primitive, Scene, SimpleTextMeasure, Style, TextAnchor,
    TextStyle, Theme,
    force_layout::{ForceConfig, ForceGraph, ForceNode, layout as force_layout},
};

use ir::{MindmapDiagram, MindmapNode, MindmapShape};

const NODE_PAD_X: f64 = 16.0;
const NODE_PAD_Y: f64 = 8.0;
const MIN_NODE_W: f64 = 60.0;
const SCENE_MARGIN: f64 = 40.0;

/// Section palette: root's child index determines color.
const SECTION_COLORS: [Color; 8] = [
    Color::rgb(78, 121, 167),
    Color::rgb(242, 142, 44),
    Color::rgb(225, 87, 89),
    Color::rgb(118, 183, 178),
    Color::rgb(89, 161, 79),
    Color::rgb(237, 201, 73),
    Color::rgb(175, 122, 161),
    Color::rgb(255, 157, 167),
];

pub fn to_scene(diagram: &MindmapDiagram) -> Scene {
    to_scene_themed(diagram, &Theme::default())
}

pub fn to_scene_themed(diagram: &MindmapDiagram, theme: &Theme) -> Scene {
    // Flatten tree to nodes + edges
    let mut flat_nodes: Vec<FlatNode> = Vec::new();
    let mut edges: Vec<(usize, usize)> = Vec::new();
    flatten(&diagram.root, &mut flat_nodes, &mut edges, None, 0, 0);

    if flat_nodes.is_empty() {
        return Scene::new(100.0, 50.0);
    }

    // Measure node sizes
    let style = TextStyle { font_size: theme.font_size_node, ..Default::default() };
    for node in &mut flat_nodes {
        let ts = SimpleTextMeasure::measure_raw(&node.text, &style);
        node.width = (ts.width + NODE_PAD_X * 2.0).max(MIN_NODE_W);
        node.height = ts.height + NODE_PAD_Y * 2.0;
    }

    // Build ForceGraph with radial tree seeding
    let mut fg = ForceGraph::new();
    for (i, node) in flat_nodes.iter().enumerate() {
        fg.add_node(ForceNode::new(i).with_size(node.width, node.height));
    }
    for &(s, t) in &edges {
        fg.add_edge(s, t);
    }

    // Seed positions: root at center, children radially by depth
    seed_tree_positions(&mut fg, &edges, &flat_nodes);

    // Pin root at center
    fg.nodes[0].fixed = true;

    // Run force simulation
    force_layout(&mut fg, &ForceConfig::tree());

    // Copy positions back
    for (i, fnode) in fg.nodes.iter().enumerate() {
        flat_nodes[i].x = fnode.x;
        flat_nodes[i].y = fnode.y;
    }

    // Compute bounding box and normalize to positive coords
    let min_x = flat_nodes.iter().map(|n| n.x - n.width / 2.0).fold(f64::INFINITY, f64::min);
    let min_y = flat_nodes.iter().map(|n| n.y - n.height / 2.0).fold(f64::INFINITY, f64::min);
    let max_x = flat_nodes.iter().map(|n| n.x + n.width / 2.0).fold(f64::NEG_INFINITY, f64::max);
    let max_y = flat_nodes.iter().map(|n| n.y + n.height / 2.0).fold(f64::NEG_INFINITY, f64::max);

    let offset_x = -min_x + SCENE_MARGIN;
    let offset_y = -min_y + SCENE_MARGIN;
    for node in &mut flat_nodes {
        node.x += offset_x;
        node.y += offset_y;
    }

    let width = max_x - min_x + SCENE_MARGIN * 2.0;
    let height = max_y - min_y + SCENE_MARGIN * 2.0;
    let mut scene = Scene::new(width, height);

    // Render edges first (nodes drawn on top will occlude the center portions)
    for &(parent_idx, child_idx) in &edges {
        let p = &flat_nodes[parent_idx];
        let c = &flat_nodes[child_idx];
        let color = section_color(c.section);
        let alpha_color = Color::rgba(color.r, color.g, color.b, 150);

        let mid_x = (p.x + c.x) / 2.0;
        scene.push(Primitive::Path {
            segments: vec![
                PathSegment::MoveTo(Point::new(p.x, p.y)),
                PathSegment::CubicTo {
                    cp1: Point::new(mid_x, p.y),
                    cp2: Point::new(mid_x, c.y),
                    to: Point::new(c.x, c.y),
                },
            ],
            style: Style {
                stroke: Some(alpha_color),
                stroke_width: Some(2.0),
                ..Default::default()
            },
            marker_start: None,
            marker_end: None,
        });
    }

    // Render nodes (opaque fills cover the edges underneath)
    for node in &flat_nodes {
        let color = section_color(node.section);
        let is_root = node.depth == 0;
        let is_parent = node.has_children;

        let (fill, stroke, text_color, font_weight) = if is_root {
            (theme.node_stroke, theme.node_stroke, Color::WHITE, rusty_mermaid_core::FontWeight::Bold)
        } else if is_parent {
            (color, color, Color::WHITE, rusty_mermaid_core::FontWeight::Bold)
        } else {
            // Leaf nodes: light opaque tint (blended with white background)
            let t = 0.15 + (node.depth as f64).min(3.0) * 0.1;
            let fill = Color::rgb(
                (255.0 * (1.0 - t) + color.r as f64 * t) as u8,
                (255.0 * (1.0 - t) + color.g as f64 * t) as u8,
                (255.0 * (1.0 - t) + color.b as f64 * t) as u8,
            );
            (fill, color, theme.node_text, rusty_mermaid_core::FontWeight::Normal)
        };

        render_shape(&mut scene, node.x, node.y, node.width, node.height, node.shape, fill, stroke);

        scene.push(Primitive::Text {
            position: Point::new(node.x, node.y),
            content: node.text.clone(),
            anchor: TextAnchor::Middle,
            style: TextStyle {
                font_size: if is_root { theme.font_size_title } else { theme.font_size_node },
                fill: Some(text_color),
                font_weight,
                ..Default::default()
            },
        });
    }

    scene
}

// ── Tree flattening ──

struct FlatNode {
    text: String,
    shape: MindmapShape,
    depth: usize,
    section: usize,
    has_children: bool,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

fn flatten(
    node: &MindmapNode,
    nodes: &mut Vec<FlatNode>,
    edges: &mut Vec<(usize, usize)>,
    parent_idx: Option<usize>,
    depth: usize,
    section: usize,
) {
    let idx = nodes.len();
    nodes.push(FlatNode {
        text: node.text.clone(),
        shape: node.shape,
        depth,
        section,
        has_children: !node.children.is_empty(),
        x: 0.0, y: 0.0,
        width: 0.0, height: 0.0,
    });
    if let Some(pi) = parent_idx {
        edges.push((pi, idx));
    }
    for (ci, child) in node.children.iter().enumerate() {
        let child_section = if depth == 0 { ci } else { section };
        flatten(child, nodes, edges, Some(idx), depth + 1, child_section);
    }
}

/// Seed ForceGraph positions using radial tree layout.
/// Root at (0,0), children spread at angles proportional to subtree size.
fn seed_tree_positions(fg: &mut ForceGraph, edges: &[(usize, usize)], nodes: &[FlatNode]) {
    use std::f64::consts::TAU;

    if fg.nodes.is_empty() { return; }
    fg.nodes[0].x = 0.0;
    fg.nodes[0].y = 0.0;

    // Build children map
    let mut children_of: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
    for &(parent, child) in edges {
        children_of[parent].push(child);
    }

    // Count subtree sizes for proportional angle allocation
    let mut subtree_size: Vec<usize> = vec![1; nodes.len()];
    // Bottom-up: process deepest first
    let mut order: Vec<usize> = (0..nodes.len()).collect();
    order.sort_by(|a, b| nodes[*b].depth.cmp(&nodes[*a].depth));
    for &i in &order {
        let child_sum: usize = children_of[i].iter().map(|&c| subtree_size[c]).sum();
        subtree_size[i] = 1 + child_sum;
    }

    // BFS from root, assigning positions
    let radius_per_depth = 180.0;
    let mut queue = std::collections::VecDeque::new();
    queue.push_back((0usize, 0.0f64, TAU)); // (node_idx, angle_start, angle_span)

    while let Some((idx, angle_start, angle_span)) = queue.pop_front() {
        let children = &children_of[idx];
        if children.is_empty() { continue; }

        let total_weight: usize = children.iter().map(|&c| subtree_size[c]).sum();
        let mut current_angle = angle_start;

        for &child in children {
            let weight = subtree_size[child] as f64 / total_weight.max(1) as f64;
            let child_span = angle_span * weight;
            let mid_angle = current_angle + child_span / 2.0;
            let depth = nodes[child].depth as f64;
            let r = depth * radius_per_depth;

            fg.nodes[child].x = r * mid_angle.cos();
            fg.nodes[child].y = r * mid_angle.sin();

            queue.push_back((child, current_angle, child_span));
            current_angle += child_span;
        }
    }
}

fn section_color(section: usize) -> Color {
    SECTION_COLORS[section % SECTION_COLORS.len()]
}


// ── Shape rendering ──

fn render_shape(scene: &mut Scene, x: f64, y: f64, w: f64, h: f64, shape: MindmapShape, fill: Color, stroke: Color) {
    match shape {
        MindmapShape::Default | MindmapShape::RoundedRect => {
            scene.push(Primitive::Rect {
                bbox: BBox::new(x, y, w, h),
                rx: h / 2.0, ry: h / 2.0, // pill shape
                style: Style { fill: Some(fill), stroke: Some(stroke), stroke_width: Some(1.5), ..Default::default() },
            });
        }
        MindmapShape::Rect => {
            scene.push(Primitive::Rect {
                bbox: BBox::new(x, y, w, h),
                rx: 3.0, ry: 3.0,
                style: Style { fill: Some(fill), stroke: Some(stroke), stroke_width: Some(1.5), ..Default::default() },
            });
        }
        MindmapShape::Circle => {
            let r = w.max(h) / 2.0;
            scene.push(Primitive::Circle {
                center: Point::new(x, y),
                radius: r,
                style: Style { fill: Some(fill), stroke: Some(stroke), stroke_width: Some(1.5), ..Default::default() },
            });
        }
        MindmapShape::Hexagon => {
            let hw = w / 2.0;
            let hh = h / 2.0;
            let inset = hh * 0.6;
            scene.push(Primitive::Polygon {
                points: vec![
                    Point::new(x - hw + inset, y - hh),
                    Point::new(x + hw - inset, y - hh),
                    Point::new(x + hw, y),
                    Point::new(x + hw - inset, y + hh),
                    Point::new(x - hw + inset, y + hh),
                    Point::new(x - hw, y),
                ],
                style: Style { fill: Some(fill), stroke: Some(stroke), stroke_width: Some(1.5), ..Default::default() },
            });
        }
        MindmapShape::Cloud => {
            // Approximate cloud as rounded rect with very large radius
            scene.push(Primitive::Rect {
                bbox: BBox::new(x, y, w * 1.1, h * 1.2),
                rx: h, ry: h,
                style: Style { fill: Some(fill), stroke: Some(stroke), stroke_width: Some(1.5), ..Default::default() },
            });
        }
        MindmapShape::Bang => {
            // Starburst: jagged polygon
            let r_outer = w.max(h) / 2.0;
            let r_inner = r_outer * 0.65;
            let spikes = 8;
            let mut pts = Vec::with_capacity(spikes * 2);
            for i in 0..(spikes * 2) {
                let angle = std::f64::consts::TAU * i as f64 / (spikes * 2) as f64 - std::f64::consts::FRAC_PI_2;
                let r = if i % 2 == 0 { r_outer } else { r_inner };
                pts.push(Point::new(x + r * angle.cos(), y + r * angle.sin()));
            }
            scene.push(Primitive::Polygon {
                points: pts,
                style: Style { fill: Some(fill), stroke: Some(stroke), stroke_width: Some(1.5), ..Default::default() },
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(input: &str) -> Scene {
        let d = parser::parse(input).unwrap();
        to_scene(&d)
    }

    #[test]
    fn scene_has_primitives() {
        let scene = render("mindmap\n    Root\n        Child 1\n        Child 2");
        assert!(scene.len() >= 5, "3 nodes + 2 edges");
    }

    #[test]
    fn root_is_bold() {
        let scene = render("mindmap\n    Root\n        Child");
        let has_bold = scene.elements().iter().any(|e| {
            if let Primitive::Text { style, content, .. } = &e.primitive {
                content == "Root" && style.font_weight == rusty_mermaid_core::FontWeight::Bold
            } else { false }
        });
        assert!(has_bold, "root should be bold");
    }

    #[test]
    fn parent_nodes_are_bold() {
        let scene = render("mindmap\n    Root\n        Parent\n            Leaf");
        let parent_bold = scene.elements().iter().any(|e| {
            if let Primitive::Text { style, content, .. } = &e.primitive {
                content == "Parent" && style.font_weight == rusty_mermaid_core::FontWeight::Bold
            } else { false }
        });
        let leaf_normal = scene.elements().iter().any(|e| {
            if let Primitive::Text { style, content, .. } = &e.primitive {
                content == "Leaf" && style.font_weight == rusty_mermaid_core::FontWeight::Normal
            } else { false }
        });
        assert!(parent_bold, "parent nodes should be bold");
        assert!(leaf_normal, "leaf nodes should be normal weight");
    }

    #[test]
    fn shapes_render() {
        let scene = render("mindmap\n    Root\n        [Rect]\n        ((Circle))\n        {{Hex}}");
        assert!(scene.len() >= 8);
    }

    #[test]
    fn edges_are_curves() {
        let scene = render("mindmap\n    Root\n        A\n        B");
        let curves = scene.elements().iter().filter(|e| {
            if let Primitive::Path { segments, .. } = &e.primitive {
                segments.iter().any(|s| matches!(s, PathSegment::CubicTo { .. }))
            } else { false }
        }).count();
        assert_eq!(curves, 2, "2 curved edges from root to children");
    }

    #[test]
    fn all_nodes_have_finite_positions() {
        let scene = render("mindmap\n    Root\n        A\n            A1\n            A2\n        B\n            B1\n        C");
        for elem in scene.elements() {
            match &elem.primitive {
                Primitive::Text { position, .. } => {
                    assert!(position.x.is_finite() && position.y.is_finite());
                }
                Primitive::Rect { bbox, .. } => {
                    assert!(bbox.x.is_finite() && bbox.y.is_finite());
                }
                _ => {}
            }
        }
    }

    #[test]
    fn wide_tree_renders() {
        let scene = render("mindmap\n    Center\n        A\n        B\n        C\n        D\n        E\n        F");
        assert!(scene.len() >= 14, "7 nodes + 6 edges + text");
    }
}
