pub mod ir;
pub mod parser;

use std::collections::HashMap;

use rusty_mermaid_core::{
    BBox, Color, PathSegment, Point, Primitive, Scene, Style, TextAnchor, TextStyle, Theme,
    force_layout::{ForceConfig, ForceGraph, ForceNode, layout as force_layout},
    intersect_rect,
};

use crate::common::palette::tint_color;
use ir::{ArchDiagram, ArchService};

const SERVICE_W: f64 = 100.0;
const SERVICE_H: f64 = 80.0;
const JUNCTION_SIZE: f64 = 16.0;
const SCENE_PAD: f64 = 40.0;
const GROUP_PAD: f64 = 30.0;
const GROUP_HEADER: f64 = 28.0;
const ICON_SIZE: f64 = 32.0;
const TINT: f64 = 0.12;

const COLORS: [Color; 8] = [
    Color::rgb(78, 121, 167),
    Color::rgb(242, 142, 44),
    Color::rgb(225, 87, 89),
    Color::rgb(118, 183, 178),
    Color::rgb(89, 161, 79),
    Color::rgb(237, 201, 73),
    Color::rgb(175, 122, 161),
    Color::rgb(255, 157, 167),
];

pub fn to_scene(diagram: &ArchDiagram, theme: &Theme) -> Scene {
    let node_ids = diagram.node_ids();
    if node_ids.is_empty() {
        return Scene::empty();
    }

    let mut positions = run_force_layout(diagram, &node_ids);
    normalize_positions(&mut positions);

    let min_x = positions
        .values()
        .map(|&(x, _, w, _)| x - w / 2.0)
        .fold(f64::INFINITY, f64::min);
    let min_y = positions
        .values()
        .map(|&(_, y, _, h)| y - h / 2.0)
        .fold(f64::INFINITY, f64::min);
    let max_x = positions
        .values()
        .map(|&(x, _, w, _)| x + w / 2.0)
        .fold(f64::NEG_INFINITY, f64::max);
    let max_y = positions
        .values()
        .map(|&(_, y, _, h)| y + h / 2.0)
        .fold(f64::NEG_INFINITY, f64::max);

    let scene_w = max_x - min_x + SCENE_PAD * 2.0;
    let scene_h = max_y - min_y + SCENE_PAD * 2.0;
    let mut scene = Scene::new(scene_w, scene_h);

    render_groups(&mut scene, diagram, &positions, theme);
    render_arch_edges(&mut scene, diagram, &positions, theme);
    render_services(&mut scene, diagram, &positions, theme);
    render_junctions(&mut scene, diagram, &positions, theme);

    scene
}

fn run_force_layout(
    diagram: &ArchDiagram,
    node_ids: &[String],
) -> HashMap<String, (f64, f64, f64, f64)> {
    let id_to_idx: HashMap<&str, usize> = node_ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id.as_str(), i))
        .collect();

    let mut fg = ForceGraph::new();

    let mut group_centers: HashMap<String, (f64, f64)> = HashMap::new();
    let group_spacing = 250.0;
    for (gi, group) in diagram.groups.iter().enumerate() {
        let angle = std::f64::consts::TAU * gi as f64 / diagram.groups.len().max(1) as f64;
        group_centers.insert(
            group.id.clone(),
            (group_spacing * angle.cos(), group_spacing * angle.sin()),
        );
    }

    for (i, id) in node_ids.iter().enumerate() {
        let is_junction = diagram.junctions.iter().any(|j| j.id == *id);
        let (w, h) = if is_junction {
            (JUNCTION_SIZE, JUNCTION_SIZE)
        } else {
            (SERVICE_W, SERVICE_H)
        };

        let group_id = diagram.node_group(id);
        let (seed_x, seed_y) = group_id
            .and_then(|g| group_centers.get(g))
            .copied()
            .unwrap_or((0.0, 0.0));
        let group_idx = group_id
            .map(|g| {
                node_ids[..i]
                    .iter()
                    .filter(|prev| diagram.node_group(prev) == Some(g))
                    .count()
            })
            .unwrap_or(i);
        let offset_x = (group_idx as f64 - 1.5) * (w + 30.0);
        let offset_y = ((group_idx % 2) as f64 - 0.5) * 20.0;
        fg.add_node(
            ForceNode::new(i)
                .with_size(w, h)
                .with_position(seed_x + offset_x, seed_y + offset_y),
        );
    }

    for edge in &diagram.edges {
        if let (Some(&s), Some(&t)) = (
            id_to_idx.get(edge.from.as_str()),
            id_to_idx.get(edge.to.as_str()),
        ) {
            fg.add_edge(s, t);
        }
    }

    force_layout(
        &mut fg,
        &ForceConfig {
            ideal_length: 120.0,
            repulsion: 6000.0,
            ..ForceConfig::default()
        },
    );

    let mut positions: HashMap<String, (f64, f64, f64, f64)> = HashMap::new();
    for (i, id) in node_ids.iter().enumerate() {
        let node = &fg.nodes[i];
        positions.insert(id.clone(), (node.x, node.y, node.width, node.height));
    }

    positions
}

fn normalize_positions(positions: &mut HashMap<String, (f64, f64, f64, f64)>) {
    let min_x = positions
        .values()
        .map(|&(x, _, w, _)| x - w / 2.0)
        .fold(f64::INFINITY, f64::min);
    let min_y = positions
        .values()
        .map(|&(_, y, _, h)| y - h / 2.0)
        .fold(f64::INFINITY, f64::min);

    let ox = -min_x + SCENE_PAD;
    let oy = -min_y + SCENE_PAD;
    for pos in positions.values_mut() {
        pos.0 += ox;
        pos.1 += oy;
    }
}

fn render_groups(
    scene: &mut Scene,
    diagram: &ArchDiagram,
    positions: &HashMap<String, (f64, f64, f64, f64)>,
    theme: &Theme,
) {
    for (gi, group) in diagram.groups.iter().enumerate() {
        let members: Vec<&str> = diagram
            .services
            .iter()
            .filter(|s| s.group.as_deref() == Some(&group.id))
            .map(|s| s.id.as_str())
            .chain(
                diagram
                    .junctions
                    .iter()
                    .filter(|j| j.group.as_deref() == Some(&group.id))
                    .map(|j| j.id.as_str()),
            )
            .collect();

        if members.is_empty() {
            continue;
        }

        let gmin_x = members
            .iter()
            .filter_map(|id| positions.get(*id))
            .map(|p| p.0 - p.2 / 2.0)
            .fold(f64::INFINITY, f64::min);
        let gmin_y = members
            .iter()
            .filter_map(|id| positions.get(*id))
            .map(|p| p.1 - p.3 / 2.0)
            .fold(f64::INFINITY, f64::min);
        let gmax_x = members
            .iter()
            .filter_map(|id| positions.get(*id))
            .map(|p| p.0 + p.2 / 2.0)
            .fold(f64::NEG_INFINITY, f64::max);
        let gmax_y = members
            .iter()
            .filter_map(|id| positions.get(*id))
            .map(|p| p.1 + p.3 / 2.0)
            .fold(f64::NEG_INFINITY, f64::max);

        let gx = (gmin_x + gmax_x) / 2.0;
        let gy = (gmin_y + gmax_y) / 2.0 + GROUP_HEADER / 2.0;
        let gw = gmax_x - gmin_x + GROUP_PAD * 2.0;
        let gh = gmax_y - gmin_y + GROUP_PAD * 2.0 + GROUP_HEADER;
        let color = COLORS[gi % COLORS.len()];

        scene.push(Primitive::Rect {
            bbox: BBox::new(gx, gy, gw, gh),
            rx: 6.0,
            ry: 6.0,
            style: Style {
                fill: Some(Color::rgba(0, 0, 0, 0)),
                stroke: Some(color),
                stroke_width: Some(1.5),
                stroke_dasharray: Some(vec![7.0, 5.0]),
                ..Default::default()
            },
        });

        scene.push(Primitive::Text {
            position: Point::new(gx - gw / 2.0 + 10.0, gy - gh / 2.0 + 14.0),
            content: group.label.clone(),
            anchor: TextAnchor::Start,
            style: TextStyle {
                font_size: theme.font_size_edge_label,
                fill: Some(color),
                font_weight: rusty_mermaid_core::FontWeight::Bold,
                ..Default::default()
            },
        });
    }
}

fn render_arch_edges(
    scene: &mut Scene,
    diagram: &ArchDiagram,
    positions: &HashMap<String, (f64, f64, f64, f64)>,
    theme: &Theme,
) {
    for edge in &diagram.edges {
        let Some(&(x1, y1, w1, h1)) = positions.get(&edge.from) else {
            continue;
        };
        let Some(&(x2, y2, w2, h2)) = positions.get(&edge.to) else {
            continue;
        };

        let start = intersect_rect(&BBox::new(x1, y1, w1, h1), Point::new(x2, y2));
        let end = intersect_rect(&BBox::new(x2, y2, w2, h2), Point::new(x1, y1));

        let marker_start = if edge.arrow_left {
            Some(rusty_mermaid_core::MarkerType::ArrowPoint)
        } else {
            None
        };
        let marker_end = if edge.arrow_right {
            Some(rusty_mermaid_core::MarkerType::ArrowPoint)
        } else {
            None
        };

        scene.push(Primitive::Path {
            segments: vec![PathSegment::MoveTo(start), PathSegment::LineTo(end)],
            style: Style {
                stroke: Some(theme.grid_stroke),
                stroke_width: Some(1.5),
                ..Default::default()
            },
            marker_start,
            marker_end,
        });
    }
}

fn render_services(
    scene: &mut Scene,
    diagram: &ArchDiagram,
    positions: &HashMap<String, (f64, f64, f64, f64)>,
    theme: &Theme,
) {
    for (si, svc) in diagram.services.iter().enumerate() {
        let Some(&(cx, cy, _, _)) = positions.get(&svc.id) else {
            continue;
        };
        let color = COLORS[si % COLORS.len()];
        render_service(scene, svc, cx, cy, color, theme);
    }
}

fn render_junctions(
    scene: &mut Scene,
    diagram: &ArchDiagram,
    positions: &HashMap<String, (f64, f64, f64, f64)>,
    theme: &Theme,
) {
    for junc in &diagram.junctions {
        let Some(&(cx, cy, _, _)) = positions.get(&junc.id) else {
            continue;
        };
        scene.push(Primitive::Rect {
            bbox: BBox::new(cx, cy, JUNCTION_SIZE, JUNCTION_SIZE),
            rx: 2.0,
            ry: 2.0,
            style: Style {
                fill: Some(theme.muted_text),
                ..Default::default()
            },
        });
    }
}

fn render_service(
    scene: &mut Scene,
    svc: &ArchService,
    cx: f64,
    cy: f64,
    color: Color,
    theme: &Theme,
) {
    let fill = tint_color(color, TINT);

    // Service box
    scene.push(Primitive::Rect {
        bbox: BBox::new(cx, cy, SERVICE_W, SERVICE_H),
        rx: 6.0,
        ry: 6.0,
        style: Style {
            fill: Some(fill),
            stroke: Some(color),
            stroke_width: Some(1.5),
            ..Default::default()
        },
    });

    // Icon placeholder (small colored circle)
    scene.push(Primitive::Circle {
        center: Point::new(cx, cy - 12.0),
        radius: ICON_SIZE / 2.0 - 4.0,
        style: Style {
            fill: Some(Color::rgba(color.r, color.g, color.b, 60)),
            stroke: Some(color),
            stroke_width: Some(1.0),
            ..Default::default()
        },
    });

    // Icon type label (small text in circle)
    scene.push(Primitive::Text {
        position: Point::new(cx, cy - 12.0),
        content: abbreviate_icon(&svc.icon),
        anchor: TextAnchor::Middle,
        style: TextStyle {
            font_size: theme.font_size_tiny,
            fill: Some(color),
            ..Default::default()
        },
    });

    // Service label
    scene.push(Primitive::Text {
        position: Point::new(cx, cy + 20.0),
        content: svc.label.clone(),
        anchor: TextAnchor::Middle,
        style: TextStyle {
            font_size: theme.font_size_small,
            fill: Some(theme.node_text),
            font_weight: rusty_mermaid_core::FontWeight::Bold,
            ..Default::default()
        },
    });
}

fn abbreviate_icon(icon: &str) -> String {
    match icon {
        "database" => "DB".into(),
        "server" => "SRV".into(),
        "disk" => "DSK".into(),
        "cloud" => "CLD".into(),
        "internet" => "NET".into(),
        _ => icon.chars().take(3).collect::<String>().to_uppercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(input: &str) -> Scene {
        let d = parser::parse(input).unwrap();
        to_scene(&d, &Theme::default())
    }

    #[test]
    fn basic_renders() {
        let scene = render(
            "architecture-beta\n  service db(database)[Database]\n  service srv(server)[Server]\n  db:R -- L:srv",
        );
        assert!(!scene.is_empty());
    }

    #[test]
    fn has_service_rects() {
        let scene = render("architecture-beta\n  service a(server)[A]\n  service b(server)[B]");
        let rects = scene
            .elements()
            .iter()
            .filter(|e| {
                if let Primitive::Rect { bbox, .. } = &e.primitive {
                    (bbox.width - SERVICE_W).abs() < 1.0
                } else {
                    false
                }
            })
            .count();
        assert_eq!(rects, 2);
    }

    #[test]
    fn group_renders_dashed() {
        let scene =
            render("architecture-beta\n  group g(cloud)[Cloud]\n  service a(server)[A] in g");
        let dashed = scene.elements().iter().any(|e| {
            if let Primitive::Rect { style, .. } = &e.primitive {
                style.stroke_dasharray.is_some()
            } else {
                false
            }
        });
        assert!(dashed);
    }

    #[test]
    fn edges_render() {
        let scene = render(
            "architecture-beta\n  service a(server)[A]\n  service b(server)[B]\n  a:R -- L:b",
        );
        let paths = scene
            .elements()
            .iter()
            .filter(|e| matches!(&e.primitive, Primitive::Path { .. }))
            .count();
        assert!(paths >= 1);
    }

    #[test]
    fn arrow_markers() {
        let scene = render(
            "architecture-beta\n  service a(server)[A]\n  service b(server)[B]\n  a:R --> L:b",
        );
        let has_marker = scene.elements().iter().any(|e| {
            if let Primitive::Path { marker_end, .. } = &e.primitive {
                marker_end.is_some()
            } else {
                false
            }
        });
        assert!(has_marker);
    }

    #[test]
    fn junction_renders() {
        let scene =
            render("architecture-beta\n  junction mid\n  service a(server)[A]\n  a:R -- L:mid");
        let small_rects = scene
            .elements()
            .iter()
            .filter(|e| {
                if let Primitive::Rect { bbox, .. } = &e.primitive {
                    (bbox.width - JUNCTION_SIZE).abs() < 1.0
                } else {
                    false
                }
            })
            .count();
        assert_eq!(small_rects, 1);
    }

    #[test]
    fn all_positions_finite() {
        let scene = render(
            "architecture-beta\n  group api(cloud)[API]\n  service db(database)[DB] in api\n  service srv(server)[Srv] in api\n  db:R -- L:srv",
        );
        for elem in scene.elements() {
            match &elem.primitive {
                Primitive::Rect { bbox, .. } => assert!(bbox.x.is_finite() && bbox.y.is_finite()),
                Primitive::Text { position, .. } => {
                    assert!(position.x.is_finite() && position.y.is_finite())
                }
                Primitive::Circle { center, .. } => {
                    assert!(center.x.is_finite() && center.y.is_finite())
                }
                _ => {}
            }
        }
    }
}
