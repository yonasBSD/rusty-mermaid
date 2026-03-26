pub mod ir;
pub mod parser;

use std::collections::HashMap;

use rusty_mermaid_core::{
    BBox, Color, PathSegment, Point, Primitive, Scene, SimpleTextMeasure, Style, TextAnchor,
    TextStyle, Theme,
};

use ir::SankeyDiagram;

const NODE_WIDTH: f64 = 10.0;
const NODE_PAD: f64 = 12.0;
const SCENE_PAD: f64 = 20.0;
const LABEL_GAP: f64 = 6.0;
const DIAGRAM_W: f64 = 600.0;
const DIAGRAM_H: f64 = 400.0;

const SANKEY_COLORS: [Color; 10] = [
    Color::rgb(78, 121, 167),
    Color::rgb(242, 142, 44),
    Color::rgb(225, 87, 89),
    Color::rgb(118, 183, 178),
    Color::rgb(89, 161, 79),
    Color::rgb(237, 201, 73),
    Color::rgb(175, 122, 161),
    Color::rgb(255, 157, 167),
    Color::rgb(186, 176, 172),
    Color::rgb(140, 86, 75),
];

struct LinkLayout {
    source: usize,
    target: usize,
    y0: f64,
    y1: f64,
    width: f64,
}

struct SankeyLayout {
    node_x0: Vec<f64>,
    node_y0: Vec<f64>,
    node_y1: Vec<f64>,
    node_value: Vec<f64>,
    depth: Vec<usize>,
    link_layouts: Vec<LinkLayout>,
}

struct Adjacency {
    outgoing: Vec<Vec<(usize, f64)>>,
    incoming: Vec<Vec<(usize, f64)>>,
    value_out: Vec<f64>,
    value_in: Vec<f64>,
    node_value: Vec<f64>,
}

fn build_adjacency(diagram: &SankeyDiagram, n: usize, name_to_idx: &HashMap<&str, usize>) -> Adjacency {
    let mut outgoing: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
    let mut incoming: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
    let mut value_out = vec![0.0f64; n];
    let mut value_in = vec![0.0f64; n];

    for link in &diagram.links {
        let s = name_to_idx[link.source.as_str()];
        let t = name_to_idx[link.target.as_str()];
        outgoing[s].push((t, link.value));
        incoming[t].push((s, link.value));
        value_out[s] += link.value;
        value_in[t] += link.value;
    }

    let node_value: Vec<f64> = (0..n).map(|i| value_out[i].max(value_in[i])).collect();
    Adjacency { outgoing, incoming, value_out, value_in, node_value }
}

fn assign_columns(
    n: usize,
    adj: &Adjacency,
) -> (Vec<usize>, Vec<Vec<usize>>) {
    let depth = compute_depths(n, &adj.outgoing, &adj.incoming);
    let max_depth = depth.iter().copied().max().unwrap_or(0);

    let mut columns: Vec<Vec<usize>> = vec![Vec::new(); max_depth + 1];
    for (i, &d) in depth.iter().enumerate() {
        columns[d].push(i);
    }

    // Relaxation: reorder each column by average position of neighbors
    let mut node_y_center = vec![0.0f64; n];
    for col in &columns {
        for (rank, &node_idx) in col.iter().enumerate() {
            node_y_center[node_idx] = rank as f64;
        }
    }
    for _pass in 0..6 {
        for d in 0..=max_depth {
            for &node_idx in &columns[d] {
                let neighbors: Vec<f64> = adj.incoming[node_idx]
                    .iter()
                    .map(|&(src, _)| node_y_center[src])
                    .chain(adj.outgoing[node_idx].iter().map(|&(tgt, _)| node_y_center[tgt]))
                    .collect();
                if !neighbors.is_empty() {
                    node_y_center[node_idx] =
                        neighbors.iter().sum::<f64>() / neighbors.len() as f64;
                }
            }
            columns[d].sort_by(|&a, &b| {
                node_y_center[a]
                    .partial_cmp(&node_y_center[b])
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            for (rank, &node_idx) in columns[d].iter().enumerate() {
                node_y_center[node_idx] = rank as f64;
            }
        }
    }

    (depth, columns)
}

fn assign_node_positions(
    n: usize,
    columns: &[Vec<usize>],
    node_value: &[f64],
) -> (Vec<f64>, Vec<f64>, Vec<f64>, f64) {
    let max_depth = columns.len().saturating_sub(1);
    let available_w = DIAGRAM_W - SCENE_PAD * 2.0;
    let available_h = DIAGRAM_H - SCENE_PAD * 2.0;
    let col_spacing = if max_depth > 0 {
        (available_w - NODE_WIDTH) / max_depth as f64
    } else {
        0.0
    };

    let max_col_value = columns
        .iter()
        .map(|col| {
            let sum: f64 = col.iter().map(|&i| node_value[i]).sum();
            let pad = (col.len().saturating_sub(1)) as f64 * NODE_PAD;
            (sum, pad)
        })
        .map(|(sum, pad)| sum / (available_h - pad).max(1.0))
        .fold(0.0f64, f64::max);

    let scale = if max_col_value > 0.0 { 1.0 / max_col_value } else { 1.0 };

    let mut node_x0 = vec![0.0f64; n];
    let mut node_y0 = vec![0.0f64; n];
    let mut node_y1 = vec![0.0f64; n];

    for (d, col) in columns.iter().enumerate() {
        let x0 = SCENE_PAD + d as f64 * col_spacing;
        let col_height: f64 =
            col.iter().map(|&i| node_value[i] * scale).sum::<f64>()
                + (col.len().saturating_sub(1)) as f64 * NODE_PAD;
        let y_start = SCENE_PAD + (available_h - col_height) / 2.0;

        let mut y = y_start;
        for &node_idx in col {
            let h = node_value[node_idx] * scale;
            node_x0[node_idx] = x0;
            node_y0[node_idx] = y;
            node_y1[node_idx] = y + h;
            y += h + NODE_PAD;
        }
    }

    (node_x0, node_y0, node_y1, scale)
}

fn compute_link_layouts(
    diagram: &SankeyDiagram,
    name_to_idx: &HashMap<&str, usize>,
    adj: &Adjacency,
    depth: &[usize],
    node_y0: &[f64],
    node_y1: &[f64],
) -> Vec<LinkLayout> {
    let mut out_y = node_y0.to_vec();
    let mut in_y = node_y0.to_vec();

    let mut sorted_links: Vec<(usize, usize, f64)> = diagram
        .links
        .iter()
        .map(|l| {
            (
                name_to_idx[l.source.as_str()],
                name_to_idx[l.target.as_str()],
                l.value,
            )
        })
        .collect();
    sorted_links.sort_by(|a, b| {
        depth[a.0]
            .cmp(&depth[b.0])
            .then_with(|| node_y0[a.1].partial_cmp(&node_y0[b.1]).unwrap_or(std::cmp::Ordering::Equal))
    });

    let mut link_layouts = Vec::new();
    for &(s, t, val) in &sorted_links {
        let src_range = node_y1[s] - node_y0[s];
        let tgt_range = node_y1[t] - node_y0[t];

        let w_src = if adj.value_out[s] > 0.0 { val / adj.value_out[s] * src_range } else { 0.0 };
        let w_tgt = if adj.value_in[t] > 0.0 { val / adj.value_in[t] * tgt_range } else { 0.0 };
        let width = w_src.max(w_tgt).max(1.0);

        let y0 = out_y[s] + w_src / 2.0;
        let y1 = in_y[t] + w_tgt / 2.0;

        out_y[s] += w_src;
        in_y[t] += w_tgt;

        link_layouts.push(LinkLayout { source: s, target: t, y0, y1, width });
    }

    link_layouts
}

/// Build adjacency, assign columns, vertical positions, and link y-stacking.
fn compute_layout(diagram: &SankeyDiagram, names: &[String], name_to_idx: &HashMap<&str, usize>) -> SankeyLayout {
    let n = names.len();
    let adj = build_adjacency(diagram, n, name_to_idx);
    let (depth, columns) = assign_columns(n, &adj);
    let (node_x0, node_y0, node_y1, _scale) = assign_node_positions(n, &columns, &adj.node_value);
    let link_layouts = compute_link_layouts(diagram, name_to_idx, &adj, &depth, &node_y0, &node_y1);

    SankeyLayout { node_x0, node_y0, node_y1, node_value: adj.node_value, depth, link_layouts }
}

fn render_links(scene: &mut Scene, link_layouts: &[LinkLayout], node_x0: &[f64]) {
    for ll in link_layouts {
        let color = node_color(ll.source);
        let link_color = Color::rgba(color.r, color.g, color.b, 100);

        let sx = node_x0[ll.source] + NODE_WIDTH;
        let tx = node_x0[ll.target];
        let mid_x = (sx + tx) / 2.0;

        scene.push(Primitive::Path {
            segments: vec![
                PathSegment::MoveTo(Point::new(sx, ll.y0)),
                PathSegment::CubicTo {
                    cp1: Point::new(mid_x, ll.y0),
                    cp2: Point::new(mid_x, ll.y1),
                    to: Point::new(tx, ll.y1),
                },
            ],
            style: Style {
                stroke: Some(link_color),
                stroke_width: Some(ll.width.max(1.0)),
                ..Default::default()
            },
            marker_start: None,
            marker_end: None,
        });
    }
}

fn render_nodes(
    scene: &mut Scene,
    names: &[String],
    node_x0: &[f64],
    node_y0: &[f64],
    node_y1: &[f64],
    node_value: &[f64],
    depth: &[usize],
    theme: &Theme,
) {
    for (i, name) in names.iter().enumerate() {
        let color = node_color(i);
        let x0 = node_x0[i];
        let y0 = node_y0[i];
        let h = node_y1[i] - node_y0[i];

        scene.push(Primitive::Rect {
            bbox: BBox::new(x0 + NODE_WIDTH / 2.0, y0 + h / 2.0, NODE_WIDTH, h),
            rx: 0.0,
            ry: 0.0,
            style: Style {
                fill: Some(color),
                ..Default::default()
            },
        });

        let label_text = format!("{} {:.0}", name, node_value[i]);
        let (label_x, anchor) = if depth[i] == 0 {
            (x0 - LABEL_GAP, TextAnchor::End)
        } else {
            (x0 + NODE_WIDTH + LABEL_GAP, TextAnchor::Start)
        };

        scene.push(Primitive::Text {
            position: Point::new(label_x, y0 + h / 2.0),
            content: label_text,
            anchor,
            style: TextStyle {
                font_size: theme.font_size_node,
                fill: Some(theme.node_text),
                ..Default::default()
            },
        });
    }
}

pub fn to_scene(diagram: &SankeyDiagram) -> Scene {
    to_scene_themed(diagram, &Theme::default())
}

pub fn to_scene_themed(diagram: &SankeyDiagram, theme: &Theme) -> Scene {
    if diagram.links.is_empty() {
        return Scene::empty();
    }

    let names = diagram.node_names();
    let name_to_idx: HashMap<&str, usize> =
        names.iter().enumerate().map(|(i, n)| (n.as_str(), i)).collect();

    let mut layout = compute_layout(diagram, &names, &name_to_idx);

    // Measure labels to compute margins
    let max_depth = layout.depth.iter().copied().max().unwrap_or(0);
    let label_style = TextStyle { font_size: theme.font_size_node, ..Default::default() };

    let mut left_label_w = 0.0f64;
    let mut right_label_w = 0.0f64;
    for (i, name) in names.iter().enumerate() {
        let val_str = format!("{} {:.0}", name, layout.node_value[i]);
        let w = SimpleTextMeasure::measure_raw(&val_str, &label_style).width;
        if layout.depth[i] == 0 {
            left_label_w = left_label_w.max(w);
        }
        if layout.depth[i] == max_depth {
            right_label_w = right_label_w.max(w);
        }
    }

    // Shift node x positions right for left labels
    let left_margin = left_label_w + LABEL_GAP;
    for x in &mut layout.node_x0 {
        *x += left_margin;
    }

    let scene_w = DIAGRAM_W + left_margin + right_label_w + LABEL_GAP + SCENE_PAD;
    let mut scene = Scene::new(scene_w, DIAGRAM_H);

    render_links(&mut scene, &layout.link_layouts, &layout.node_x0);
    render_nodes(
        &mut scene,
        &names,
        &layout.node_x0,
        &layout.node_y0,
        &layout.node_y1,
        &layout.node_value,
        &layout.depth,
        theme,
    );

    scene
}

/// Compute topological depth for each node. Sources get depth 0.
fn compute_depths(
    n: usize,
    outgoing: &[Vec<(usize, f64)>],
    incoming: &[Vec<(usize, f64)>],
) -> Vec<usize> {
    let mut depth = vec![0usize; n];
    let mut in_degree: Vec<usize> = incoming.iter().map(|v| v.len()).collect();
    let mut queue: std::collections::VecDeque<usize> =
        (0..n).filter(|&i| in_degree[i] == 0).collect();

    while let Some(node) = queue.pop_front() {
        for &(target, _) in &outgoing[node] {
            depth[target] = depth[target].max(depth[node] + 1);
            in_degree[target] -= 1;
            if in_degree[target] == 0 {
                queue.push_back(target);
            }
        }
    }

    depth
}

fn node_color(idx: usize) -> Color {
    SANKEY_COLORS[idx % SANKEY_COLORS.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(input: &str) -> Scene {
        let d = parser::parse(input).unwrap();
        to_scene(&d)
    }

    #[test]
    fn basic_renders() {
        let scene = render("sankey-beta\nA,B,10\nA,C,5\nB,D,8\nC,D,4");
        assert!(!scene.is_empty());
        assert!(scene.width > 0.0);
    }

    #[test]
    fn has_nodes_and_links() {
        let scene = render("sankey-beta\nA,B,10\nA,C,5");
        // 2 links + 3 node rects + 3 labels = 8
        assert!(scene.len() >= 8, "expected >= 8 elements, got {}", scene.len());
    }

    #[test]
    fn node_rects_have_fixed_width() {
        let scene = render("sankey-beta\nA,B,100");
        let rects: Vec<_> = scene.elements().iter().filter(|e| {
            matches!(&e.primitive, Primitive::Rect { .. })
        }).collect();
        assert_eq!(rects.len(), 2, "should have 2 node rects");
        for r in &rects {
            if let Primitive::Rect { bbox, .. } = &r.primitive {
                assert!((bbox.width - NODE_WIDTH).abs() < 0.01);
            }
        }
    }

    #[test]
    fn links_are_curves() {
        let scene = render("sankey-beta\nA,B,10\nA,C,5");
        let curves = scene.elements().iter().filter(|e| {
            if let Primitive::Path { segments, .. } = &e.primitive {
                segments.iter().any(|s| matches!(s, PathSegment::CubicTo { .. }))
            } else {
                false
            }
        }).count();
        assert_eq!(curves, 2, "should have 2 curved links");
    }

    #[test]
    fn multi_column_layout() {
        let scene = render("sankey-beta\nA,B,10\nB,C,10\nC,D,10");
        // 4 columns: A→B→C→D
        let rects: Vec<_> = scene.elements().iter().filter_map(|e| {
            if let Primitive::Rect { bbox, .. } = &e.primitive { Some(bbox.x) } else { None }
        }).collect();
        assert_eq!(rects.len(), 4);
        // Each subsequent node should be to the right
        for w in rects.windows(2) {
            assert!(w[1] > w[0], "nodes should progress left to right");
        }
    }

    #[test]
    fn node_heights_proportional() {
        let scene = render("sankey-beta\nA,B,100\nA,C,50");
        let heights: Vec<_> = scene.elements().iter().filter_map(|e| {
            if let Primitive::Rect { bbox, .. } = &e.primitive { Some(bbox.height) } else { None }
        }).collect();
        // A has value 150, B has 100, C has 50
        // A should be tallest
        assert!(heights.len() == 3);
        assert!(heights[0] > heights[1], "A should be taller than B");
        assert!(heights[1] > heights[2], "B should be taller than C");
    }

    #[test]
    fn all_positions_finite() {
        let scene = render("sankey-beta\nA,B,10\nB,C,5\nA,C,3\nC,D,8");
        for elem in scene.elements() {
            match &elem.primitive {
                Primitive::Rect { bbox, .. } => {
                    assert!(bbox.x.is_finite() && bbox.y.is_finite());
                    assert!(bbox.width.is_finite() && bbox.height.is_finite());
                }
                Primitive::Text { position, .. } => {
                    assert!(position.x.is_finite() && position.y.is_finite());
                }
                Primitive::Path { segments, .. } => {
                    for seg in segments {
                        match seg {
                            PathSegment::MoveTo(p) => {
                                assert!(p.x.is_finite() && p.y.is_finite());
                            }
                            PathSegment::CubicTo { cp1, cp2, to } => {
                                assert!(cp1.x.is_finite() && cp1.y.is_finite());
                                assert!(cp2.x.is_finite() && cp2.y.is_finite());
                                assert!(to.x.is_finite() && to.y.is_finite());
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
