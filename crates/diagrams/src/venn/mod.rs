pub mod ir;
pub mod parser;

use std::f64::consts::TAU;

use rusty_mermaid_core::{
    Color, Point, Primitive, Scene, Style, TextAnchor, TextStyle, Theme,
};

use ir::VennDiagram;

const SCENE_PAD: f64 = 30.0;
const BASE_RADIUS: f64 = 140.0;
const OVERLAP_RATIO: f64 = 0.35;

const VENN_COLORS: [Color; 8] = [
    Color::rgb(78, 121, 167),
    Color::rgb(242, 142, 44),
    Color::rgb(225, 87, 89),
    Color::rgb(118, 183, 178),
    Color::rgb(89, 161, 79),
    Color::rgb(237, 201, 73),
    Color::rgb(175, 122, 161),
    Color::rgb(255, 157, 167),
];

const FILL_ALPHA: u8 = 50; // real transparency so overlaps blend

pub fn to_scene(diagram: &VennDiagram) -> Scene {
    to_scene_themed(diagram, &Theme::default())
}

pub fn to_scene_themed(diagram: &VennDiagram, theme: &Theme) -> Scene {
    let n = diagram.sets.len();
    if n == 0 {
        return Scene::new(100.0, 50.0);
    }

    // Compute radii proportional to sqrt(size)
    let max_size = diagram.sets.iter().map(|s| s.size).fold(0.0f64, f64::max);
    let radii: Vec<f64> = diagram
        .sets
        .iter()
        .map(|s| BASE_RADIUS * (s.size / max_size.max(1.0)).sqrt())
        .collect();

    // Position circles
    let centers = compute_centers(n, &radii);

    // Bounding box
    let min_x = centers.iter().zip(&radii).map(|(&(cx, _), &r)| cx - r).fold(f64::INFINITY, f64::min);
    let max_x = centers.iter().zip(&radii).map(|(&(cx, _), &r)| cx + r).fold(f64::NEG_INFINITY, f64::max);
    let min_y = centers.iter().zip(&radii).map(|(&(_, cy), &r)| cy - r).fold(f64::INFINITY, f64::min);
    let max_y = centers.iter().zip(&radii).map(|(&(_, cy), &r)| cy + r).fold(f64::NEG_INFINITY, f64::max);

    let title_h = if diagram.title.is_some() { 30.0 } else { 0.0 };
    let ox = -min_x + SCENE_PAD;
    let oy = -min_y + SCENE_PAD + title_h;

    let scene_w = max_x - min_x + SCENE_PAD * 2.0;
    let scene_h = max_y - min_y + SCENE_PAD * 2.0 + title_h;
    let mut scene = Scene::new(scene_w, scene_h);

    // Title
    if let Some(title) = &diagram.title {
        scene.push(Primitive::Text {
            position: Point::new(scene_w / 2.0, SCENE_PAD + 10.0),
            content: title.clone(),
            anchor: TextAnchor::Middle,
            style: TextStyle {
                font_size: theme.font_size_title,
                fill: Some(theme.node_text),
                font_weight: rusty_mermaid_core::FontWeight::Bold,
                ..Default::default()
            },
        });
    }

    // Draw circles (translucent fills)
    for (i, set) in diagram.sets.iter().enumerate() {
        let (cx, cy) = centers[i];
        let r = radii[i];
        let color = VENN_COLORS[i % VENN_COLORS.len()];

        let fill = Color::rgba(color.r, color.g, color.b, FILL_ALPHA);

        scene.push(Primitive::Circle {
            center: Point::new(cx + ox, cy + oy),
            radius: r,
            style: Style {
                fill: Some(fill),
                stroke: Some(color),
                stroke_width: Some(2.0),
                ..Default::default()
            },
        });

        // Label: position outside if 2+ sets, center if single
        let (lx, ly) = if n == 1 {
            (cx, cy)
        } else {
            label_position(cx, cy, r, &centers, i)
        };

        scene.push(Primitive::Text {
            position: Point::new(lx + ox, ly + oy),
            content: set.label.clone(),
            anchor: TextAnchor::Middle,
            style: TextStyle {
                font_size: theme.font_size_node,
                fill: Some(color),
                font_weight: rusty_mermaid_core::FontWeight::Bold,
                ..Default::default()
            },
        });
    }

    // Union labels at intersection midpoints, offset vertically to avoid overlap
    let union_line_h = theme.font_size_node + 2.0;
    let mut union_count_at: std::collections::HashMap<(i32, i32), usize> = std::collections::HashMap::new();

    for union in &diagram.unions {
        if let Some(label) = &union.label {
            let indices: Vec<usize> = union
                .set_ids
                .iter()
                .filter_map(|id| diagram.sets.iter().position(|s| s.id == *id))
                .collect();

            if indices.is_empty() {
                continue;
            }

            let mid_x: f64 = indices.iter().map(|&i| centers[i].0).sum::<f64>() / indices.len() as f64;
            let mid_y: f64 = indices.iter().map(|&i| centers[i].1).sum::<f64>() / indices.len() as f64;

            // Offset to avoid stacking at same position
            let key = ((mid_x * 10.0) as i32, (mid_y * 10.0) as i32);
            let slot = union_count_at.entry(key).or_insert(0);
            let y_offset = *slot as f64 * union_line_h;
            *slot += 1;

            scene.push(Primitive::Text {
                position: Point::new(mid_x + ox, mid_y + oy + y_offset),
                content: label.clone(),
                anchor: TextAnchor::Middle,
                style: TextStyle {
                    font_size: theme.font_size_node - 1.0,
                    fill: Some(Color::rgb(80, 80, 80)),
                    ..Default::default()
                },
            });
        }
    }

    scene
}

/// Compute circle centers based on count.
fn compute_centers(n: usize, radii: &[f64]) -> Vec<(f64, f64)> {
    match n {
        1 => vec![(0.0, 0.0)],
        2 => {
            let overlap = (radii[0] + radii[1]) * OVERLAP_RATIO;
            let dist = radii[0] + radii[1] - overlap;
            vec![(-dist / 2.0, 0.0), (dist / 2.0, 0.0)]
        }
        _ => {
            // Arrange in a regular polygon, with overlap
            let avg_r = radii.iter().sum::<f64>() / n as f64;
            let ring_r = avg_r * (1.0 - OVERLAP_RATIO) * 1.2;
            (0..n)
                .map(|i| {
                    let angle = TAU * i as f64 / n as f64 - std::f64::consts::FRAC_PI_2;
                    (ring_r * angle.cos(), ring_r * angle.sin())
                })
                .collect()
        }
    }
}

/// Position label away from other circles (toward the outer edge).
fn label_position(cx: f64, cy: f64, r: f64, centers: &[(f64, f64)], idx: usize) -> (f64, f64) {
    // Direction: away from centroid of other circles
    let other_cx: f64 = centers.iter().enumerate()
        .filter(|&(i, _)| i != idx)
        .map(|(_, &(x, _))| x)
        .sum::<f64>() / (centers.len() - 1).max(1) as f64;
    let other_cy: f64 = centers.iter().enumerate()
        .filter(|&(i, _)| i != idx)
        .map(|(_, &(_, y))| y)
        .sum::<f64>() / (centers.len() - 1).max(1) as f64;

    let dx = cx - other_cx;
    let dy = cy - other_cy;
    let dist = (dx * dx + dy * dy).sqrt().max(1.0);

    let label_r = r * 0.55;
    (cx + label_r * dx / dist, cy + label_r * dy / dist)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(input: &str) -> Scene {
        let d = parser::parse(input).unwrap();
        to_scene(&d)
    }

    #[test]
    fn basic_two_sets() {
        let scene = render("venn-beta\n  set A:20\n  set B:15\n  union A,B[\"Both\"]:5");
        assert!(!scene.is_empty());
    }

    #[test]
    fn has_circles() {
        let scene = render("venn-beta\n  set A\n  set B");
        let circles = scene.elements().iter().filter(|e| {
            matches!(&e.primitive, Primitive::Circle { .. })
        }).count();
        assert_eq!(circles, 2);
    }

    #[test]
    fn three_set_renders() {
        let scene = render("venn-beta\n  set A:30\n  set B:20\n  set C:15");
        let circles = scene.elements().iter().filter(|e| {
            matches!(&e.primitive, Primitive::Circle { .. })
        }).count();
        assert_eq!(circles, 3);
    }

    #[test]
    fn union_label_renders() {
        let scene = render("venn-beta\n  set A\n  set B\n  union A,B[\"Overlap\"]");
        let has_overlap = scene.elements().iter().any(|e| {
            if let Primitive::Text { content, .. } = &e.primitive { content == "Overlap" } else { false }
        });
        assert!(has_overlap);
    }

    #[test]
    fn circles_overlap() {
        let scene = render("venn-beta\n  set A:20\n  set B:20");
        let circles: Vec<_> = scene.elements().iter().filter_map(|e| {
            if let Primitive::Circle { center, radius, .. } = &e.primitive {
                Some((center.x, center.y, *radius))
            } else { None }
        }).collect();
        assert_eq!(circles.len(), 2);
        let dist = ((circles[1].0 - circles[0].0).powi(2) + (circles[1].1 - circles[0].1).powi(2)).sqrt();
        let sum_r = circles[0].2 + circles[1].2;
        assert!(dist < sum_r, "circles should overlap: dist={dist} sum_r={sum_r}");
    }

    #[test]
    fn single_set() {
        let scene = render("venn-beta\n  set A[\"Solo\"]:10");
        assert!(!scene.is_empty());
    }

    #[test]
    fn all_positions_finite() {
        let scene = render("venn-beta\n  set A:30\n  set B:20\n  set C:15\n  union A,B:8\n  union B,C:5");
        for elem in scene.elements() {
            match &elem.primitive {
                Primitive::Circle { center, .. } => {
                    assert!(center.x.is_finite() && center.y.is_finite());
                }
                Primitive::Text { position, .. } => {
                    assert!(position.x.is_finite() && position.y.is_finite());
                }
                _ => {}
            }
        }
    }
}
