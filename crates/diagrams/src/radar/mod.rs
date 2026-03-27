pub mod ir;
pub mod parser;

use std::f64::consts::{FRAC_PI_2, TAU};

use rusty_mermaid_core::{
    BBox, Color, PathSegment, Point, Primitive, Scene, Style, TextAnchor, TextStyle, Theme,
};

use ir::{Graticule, RadarChart};

const RADIUS: f64 = 180.0;
const SCENE_PAD: f64 = 50.0;
const LABEL_PAD: f64 = 14.0;
const LEGEND_SWATCH: f64 = 12.0;
const LEGEND_LINE_H: f64 = 20.0;
const LEGEND_GAP: f64 = 30.0;

const RADAR_COLORS: [Color; 8] = [
    Color::rgb(78, 121, 167),
    Color::rgb(242, 142, 44),
    Color::rgb(225, 87, 89),
    Color::rgb(118, 183, 178),
    Color::rgb(89, 161, 79),
    Color::rgb(237, 201, 73),
    Color::rgb(175, 122, 161),
    Color::rgb(255, 157, 167),
];

/// Precomputed radar geometry shared across drawing helpers.
struct RadarArea {
    cx: f64,
    cy: f64,
    n_axes: usize,
    min_val: f64,
    max_val: f64,
}

impl RadarArea {
    fn angle(&self, i: usize) -> f64 {
        TAU * i as f64 / self.n_axes as f64 - FRAC_PI_2
    }

    fn polar(&self, a: f64, r: f64) -> Point {
        Point::new(self.cx + r * a.cos(), self.cy + r * a.sin())
    }
}

pub fn to_scene(chart: &RadarChart, theme: &Theme) -> Scene {
    let n_axes = chart.axes.len();
    if n_axes < 3 {
        return Scene::empty();
    }

    let title_h = if chart.title.is_some() { 50.0 } else { 0.0 };
    let legend_w = if chart.curves.len() > 1 { 120.0 } else { 0.0 };
    let cx = SCENE_PAD + RADIUS;
    let cy = SCENE_PAD + RADIUS + title_h;
    let scene_w = cx + RADIUS + SCENE_PAD + legend_w;
    let scene_h = cy + RADIUS + SCENE_PAD;
    let mut scene = Scene::new(scene_w, scene_h);

    let area = RadarArea {
        cx,
        cy,
        n_axes,
        min_val: chart.min,
        max_val: chart.effective_max().max(chart.min + 1.0),
    };

    if let Some(title) = &chart.title {
        scene.push(Primitive::Text {
            position: Point::new(cx, SCENE_PAD + 10.0),
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

    render_graticule(&mut scene, chart, &area, theme);
    render_axes(&mut scene, chart, &area, theme);
    render_curves(&mut scene, chart, &area);
    render_legend(&mut scene, chart, &area, theme);

    scene
}

fn render_graticule(scene: &mut Scene, chart: &RadarChart, area: &RadarArea, theme: &Theme) {
    let grid_color = theme.grid_stroke;
    let grid_style = Style {
        stroke: Some(grid_color),
        stroke_width: Some(0.8),
        fill: Some(Color::rgba(0, 0, 0, 0)),
        ..Default::default()
    };

    for tick in 1..=chart.ticks {
        let r = RADIUS * tick as f64 / chart.ticks as f64;

        match chart.graticule {
            Graticule::Circle => {
                scene.push(Primitive::Circle {
                    center: Point::new(area.cx, area.cy),
                    radius: r,
                    style: grid_style.clone(),
                });
            }
            Graticule::Polygon => {
                let pts: Vec<Point> = (0..area.n_axes)
                    .map(|i| area.polar(area.angle(i), r))
                    .collect();
                scene.push(Primitive::Polygon {
                    points: pts,
                    style: grid_style.clone(),
                });
            }
        }
    }
}

fn render_axes(scene: &mut Scene, chart: &RadarChart, area: &RadarArea, theme: &Theme) {
    let grid_color = theme.grid_stroke;

    for (i, axis) in chart.axes.iter().enumerate() {
        let a = area.angle(i);
        let edge = area.polar(a, RADIUS);

        scene.push(Primitive::Path {
            segments: vec![
                PathSegment::MoveTo(Point::new(area.cx, area.cy)),
                PathSegment::LineTo(edge),
            ],
            style: Style {
                stroke: Some(grid_color),
                stroke_width: Some(0.8),
                ..Default::default()
            },
            marker_start: None,
            marker_end: None,
        });

        let label_pt = area.polar(a, RADIUS + LABEL_PAD);
        let anchor = if a.cos().abs() < 0.1 {
            TextAnchor::Middle
        } else if a.cos() > 0.0 {
            TextAnchor::Start
        } else {
            TextAnchor::End
        };

        scene.push(Primitive::Text {
            position: label_pt,
            content: axis.label.clone(),
            anchor,
            style: TextStyle {
                font_size: theme.font_size_node,
                fill: Some(theme.node_text),
                ..Default::default()
            },
        });
    }

    // Tick value labels — placed just right of north axis
    let tick_angle = area.angle(0) + 0.15;
    for tick in 1..=chart.ticks {
        let r = RADIUS * tick as f64 / chart.ticks as f64;
        let val = area.min_val + (area.max_val - area.min_val) * tick as f64 / chart.ticks as f64;
        let pt = area.polar(tick_angle, r);

        scene.push(Primitive::Text {
            position: Point::new(pt.x + 3.0, pt.y),
            content: format!("{:.0}", val),
            anchor: TextAnchor::Start,
            style: TextStyle {
                font_size: theme.font_size_tiny,
                fill: Some(theme.muted_text),
                ..Default::default()
            },
        });
    }
}

fn render_curves(scene: &mut Scene, chart: &RadarChart, area: &RadarArea) {
    for (ci, curve) in chart.curves.iter().enumerate() {
        let color = RADAR_COLORS[ci % RADAR_COLORS.len()];
        let fill = Color::rgba(color.r, color.g, color.b, 40);

        let pts: Vec<Point> = curve
            .values
            .iter()
            .enumerate()
            .map(|(i, &v)| {
                let ratio = ((v - area.min_val) / (area.max_val - area.min_val)).clamp(0.0, 1.0);
                area.polar(area.angle(i), RADIUS * ratio)
            })
            .collect();

        scene.push(Primitive::Polygon {
            points: pts.clone(),
            style: Style {
                fill: Some(fill),
                stroke: Some(color),
                stroke_width: Some(2.0),
                ..Default::default()
            },
        });

        for pt in &pts {
            scene.push(Primitive::Circle {
                center: *pt,
                radius: 3.0,
                style: Style {
                    fill: Some(color),
                    ..Default::default()
                },
            });
        }
    }
}

fn render_legend(scene: &mut Scene, chart: &RadarChart, area: &RadarArea, theme: &Theme) {
    if chart.curves.len() <= 1 {
        return;
    }

    let lx = area.cx + RADIUS + LEGEND_GAP;
    let ly = area.cy - RADIUS;

    for (ci, curve) in chart.curves.iter().enumerate() {
        let color = RADAR_COLORS[ci % RADAR_COLORS.len()];
        let y = ly + ci as f64 * LEGEND_LINE_H;

        scene.push(Primitive::Rect {
            bbox: BBox::new(
                lx + LEGEND_SWATCH / 2.0,
                y + LEGEND_SWATCH / 2.0,
                LEGEND_SWATCH,
                LEGEND_SWATCH,
            ),
            rx: 2.0,
            ry: 2.0,
            style: Style {
                fill: Some(color),
                ..Default::default()
            },
        });

        scene.push(Primitive::Text {
            position: Point::new(lx + LEGEND_SWATCH + 6.0, y + LEGEND_SWATCH / 2.0),
            content: curve.label.clone(),
            anchor: TextAnchor::Start,
            style: TextStyle {
                font_size: theme.font_size_small,
                fill: Some(theme.node_text),
                ..Default::default()
            },
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(input: &str) -> Scene {
        let c = parser::parse(input).unwrap();
        to_scene(&c, &Theme::default())
    }

    #[test]
    fn basic_renders() {
        let scene = render("radar-beta\naxis A,B,C\ncurve x{1,2,3}");
        assert!(!scene.is_empty());
    }

    #[test]
    fn has_polygon_for_curve() {
        let scene = render("radar-beta\naxis A,B,C\ncurve x{1,2,3}");
        let polygons = scene
            .elements()
            .iter()
            .filter(|e| {
                if let Primitive::Polygon { style, .. } = &e.primitive {
                    style.stroke_width == Some(2.0) // data polygon, not grid
                } else {
                    false
                }
            })
            .count();
        assert_eq!(polygons, 1);
    }

    #[test]
    fn grid_has_tick_rings() {
        let scene = render("radar-beta\nticks 3\naxis A,B,C,D\ncurve x{1,2,3,4}");
        let grids = scene
            .elements()
            .iter()
            .filter(|e| {
                if let Primitive::Polygon { style, .. } = &e.primitive {
                    style.stroke_width == Some(0.8)
                } else {
                    false
                }
            })
            .count();
        assert_eq!(grids, 3, "3 tick rings");
    }

    #[test]
    fn axis_labels_present() {
        let scene = render("radar-beta\naxis Speed,Power,Agility\ncurve x{5,3,4}");
        let labels: Vec<&str> = scene
            .elements()
            .iter()
            .filter_map(|e| {
                if let Primitive::Text { content, .. } = &e.primitive {
                    Some(content.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert!(labels.contains(&"Speed"));
        assert!(labels.contains(&"Power"));
        assert!(labels.contains(&"Agility"));
    }

    #[test]
    fn multiple_curves_with_legend() {
        let scene = render("radar-beta\naxis A,B,C\ncurve a{1,2,3}\ncurve b{3,2,1}");
        let data_polygons = scene
            .elements()
            .iter()
            .filter(|e| {
                if let Primitive::Polygon { style, .. } = &e.primitive {
                    style.stroke_width == Some(2.0)
                } else {
                    false
                }
            })
            .count();
        assert_eq!(data_polygons, 2);
    }

    #[test]
    fn vertex_dots() {
        let scene = render("radar-beta\naxis A,B,C,D\ncurve x{1,2,3,4}");
        let dots = scene
            .elements()
            .iter()
            .filter(|e| {
                if let Primitive::Circle { radius, style, .. } = &e.primitive {
                    *radius < 4.0 && style.fill.is_some()
                } else {
                    false
                }
            })
            .count();
        assert_eq!(dots, 4, "one dot per axis vertex");
    }

    #[test]
    fn all_positions_finite() {
        let scene = render("radar-beta\naxis A,B,C,D,E\ncurve x{1,2,3,4,5}\ncurve y{5,4,3,2,1}");
        for elem in scene.elements() {
            match &elem.primitive {
                Primitive::Circle { center, .. } => {
                    assert!(center.x.is_finite() && center.y.is_finite());
                }
                Primitive::Text { position, .. } => {
                    assert!(position.x.is_finite() && position.y.is_finite());
                }
                Primitive::Polygon { points, .. } => {
                    for p in points {
                        assert!(p.x.is_finite() && p.y.is_finite());
                    }
                }
                _ => {}
            }
        }
    }
}
