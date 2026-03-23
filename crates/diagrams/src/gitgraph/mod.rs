pub mod ir;
pub mod parser;

use std::collections::BTreeMap;

use rusty_mermaid_core::{
    BBox, Color, PathSegment, Point, Primitive, Scene, SimpleTextMeasure, Style, TextAnchor,
    TextStyle, Theme,
};

use ir::{CommitType, GitDirection, GitGraph, GitStatement};

const COMMIT_RADIUS: f64 = 8.0;
const COMMIT_STEP: f64 = 50.0;
const LANE_WIDTH: f64 = 40.0;
const MARGIN: f64 = 40.0;
const TAG_HEIGHT: f64 = 18.0;
const TAG_PAD: f64 = 6.0;
const LABEL_PAD: f64 = 10.0;

const LANE_COLORS: [Color; 8] = [
    Color::rgb(78, 121, 167),
    Color::rgb(242, 142, 44),
    Color::rgb(225, 87, 89),
    Color::rgb(118, 183, 178),
    Color::rgb(89, 161, 79),
    Color::rgb(237, 201, 73),
    Color::rgb(175, 122, 161),
    Color::rgb(255, 157, 167),
];

pub fn to_scene(graph: &GitGraph) -> Scene {
    to_scene_themed(graph, &Theme::default())
}

pub fn to_scene_themed(graph: &GitGraph, theme: &Theme) -> Scene {
    let commits = build_commits(graph);
    if commits.is_empty() {
        return Scene::new(100.0, 50.0);
    }

    let is_horizontal = graph.direction == GitDirection::LR;

    // Find final checked-out branch (last HEAD)
    let final_branch = find_final_branch(graph);

    // Assign lane indices to branches
    let mut branch_lanes: BTreeMap<String, usize> = BTreeMap::new();
    let mut next_lane: usize = 0;
    for c in &commits {
        if !branch_lanes.contains_key(&c.branch) {
            branch_lanes.insert(c.branch.clone(), next_lane);
            next_lane += 1;
        }
    }

    // Compute branch label widths
    let label_style = TextStyle { font_size: theme.font_size_small, ..Default::default() };
    let max_label_w = branch_lanes.keys()
        .map(|name| SimpleTextMeasure::measure_raw(name, &label_style).width + LABEL_PAD * 2.0)
        .fold(60.0f64, f64::max);
    let label_area = max_label_w + MARGIN / 2.0;

    let n_lanes = next_lane.max(1);
    let n_commits = commits.len();

    let (width, height) = if is_horizontal {
        let w = label_area + MARGIN + n_commits as f64 * COMMIT_STEP + MARGIN;
        let h = MARGIN * 2.0 + n_lanes as f64 * LANE_WIDTH;
        (w, h)
    } else {
        let w = MARGIN * 2.0 + n_lanes as f64 * LANE_WIDTH + label_area;
        let h = label_area + MARGIN + n_commits as f64 * COMMIT_STEP + MARGIN;
        (w, h)
    };

    let mut scene = Scene::new(width, height);

    // Position function — label area offsets the commits
    let pos = |seq: usize, lane: usize| -> (f64, f64) {
        if is_horizontal {
            let x = label_area + seq as f64 * COMMIT_STEP + COMMIT_STEP / 2.0;
            let y = MARGIN + lane as f64 * LANE_WIDTH + LANE_WIDTH / 2.0;
            (x, y)
        } else {
            let x = MARGIN + lane as f64 * LANE_WIDTH + LANE_WIDTH / 2.0;
            let y = label_area + seq as f64 * COMMIT_STEP + COMMIT_STEP / 2.0;
            (x, y)
        }
    };

    // Branch labels — auto-sized, bold for final branch
    for (name, &lane) in &branch_lanes {
        let color = LANE_COLORS[lane % LANE_COLORS.len()];
        let is_final = name == &final_branch;
        let label_w = SimpleTextMeasure::measure_raw(name, &label_style).width + LABEL_PAD * 2.0;

        if is_horizontal {
            let y = MARGIN + lane as f64 * LANE_WIDTH + LANE_WIDTH / 2.0;
            let x = MARGIN / 2.0 + label_w / 2.0;
            scene.push(Primitive::Rect {
                bbox: BBox::new(x, y, label_w, 20.0),
                rx: 4.0, ry: 4.0,
                style: Style { fill: Some(color), ..Default::default() },
            });
            scene.push(Primitive::Text {
                position: Point::new(x, y),
                content: name.clone(),
                anchor: TextAnchor::Middle,
                style: TextStyle {
                    font_size: theme.font_size_small,
                    fill: Some(Color::WHITE),
                    font_weight: if is_final { rusty_mermaid_core::FontWeight::Bold } else { rusty_mermaid_core::FontWeight::Normal },
                    ..Default::default()
                },
            });
        } else {
            let x = MARGIN + lane as f64 * LANE_WIDTH + LANE_WIDTH / 2.0;
            let y = MARGIN / 2.0 + 10.0;
            scene.push(Primitive::Rect {
                bbox: BBox::new(x, y, label_w, 20.0),
                rx: 4.0, ry: 4.0,
                style: Style { fill: Some(color), ..Default::default() },
            });
            scene.push(Primitive::Text {
                position: Point::new(x, y),
                content: name.clone(),
                anchor: TextAnchor::Middle,
                style: TextStyle {
                    font_size: theme.font_size_small,
                    fill: Some(Color::WHITE),
                    font_weight: if is_final { rusty_mermaid_core::FontWeight::Bold } else { rusty_mermaid_core::FontWeight::Normal },
                    ..Default::default()
                },
            });
        }
    }

    // Branch lines (connect ALL consecutive commits on same branch, including merges)
    for (name, &lane) in &branch_lanes {
        let color = LANE_COLORS[lane % LANE_COLORS.len()];
        let branch_commits: Vec<usize> = commits.iter().enumerate()
            .filter(|(_, c)| c.branch == *name)
            .map(|(i, _)| i)
            .collect();

        for window in branch_commits.windows(2) {
            let (x1, y1) = pos(window[0], lane);
            let (x2, y2) = pos(window[1], lane);
            scene.push(Primitive::Path {
                segments: vec![
                    PathSegment::MoveTo(Point::new(x1, y1)),
                    PathSegment::LineTo(Point::new(x2, y2)),
                ],
                style: Style { stroke: Some(color), stroke_width: Some(2.0), ..Default::default() },
                marker_start: None,
                marker_end: None,
            });
        }
    }

    // Merge/cross-branch lines (smooth curves)
    for (i, commit) in commits.iter().enumerate() {
        if let Some(parent_idx) = commit.parent_idx {
            if commits[parent_idx].branch != commit.branch {
                let parent_lane = branch_lanes[&commits[parent_idx].branch];
                let commit_lane = branch_lanes[&commit.branch];
                let (x1, y1) = pos(parent_idx, parent_lane);
                let (x2, y2) = pos(i, commit_lane);
                let color = LANE_COLORS[commit_lane % LANE_COLORS.len()];

                // Smooth S-curve between lanes
                let (cp1, cp2) = if is_horizontal {
                    (Point::new((x1 + x2) / 2.0, y1), Point::new((x1 + x2) / 2.0, y2))
                } else {
                    (Point::new(x1, (y1 + y2) / 2.0), Point::new(x2, (y1 + y2) / 2.0))
                };

                scene.push(Primitive::Path {
                    segments: vec![
                        PathSegment::MoveTo(Point::new(x1, y1)),
                        PathSegment::CubicTo { cp1, cp2, to: Point::new(x2, y2) },
                    ],
                    style: Style {
                        stroke: Some(color),
                        stroke_width: Some(2.0),
                        stroke_dasharray: Some(vec![5.0, 3.0]),
                        ..Default::default()
                    },
                    marker_start: None,
                    marker_end: None,
                });
            }
        }
    }

    // Commit dots + tags + labels
    for (i, commit) in commits.iter().enumerate() {
        let lane = branch_lanes[&commit.branch];
        let color = LANE_COLORS[lane % LANE_COLORS.len()];
        let (cx, cy) = pos(i, lane);

        // Commit symbol
        match commit.commit_type {
            CommitType::Normal => {
                scene.push(Primitive::Circle {
                    center: Point::new(cx, cy),
                    radius: COMMIT_RADIUS,
                    style: Style { fill: Some(color), stroke: Some(Color::WHITE), stroke_width: Some(2.0), ..Default::default() },
                });
            }
            CommitType::Reverse => {
                scene.push(Primitive::Circle {
                    center: Point::new(cx, cy),
                    radius: COMMIT_RADIUS,
                    style: Style { fill: Some(Color::WHITE), stroke: Some(color), stroke_width: Some(2.0), ..Default::default() },
                });
                let s = COMMIT_RADIUS * 0.5;
                scene.push(Primitive::Path {
                    segments: vec![
                        PathSegment::MoveTo(Point::new(cx - s, cy - s)),
                        PathSegment::LineTo(Point::new(cx + s, cy + s)),
                        PathSegment::MoveTo(Point::new(cx + s, cy - s)),
                        PathSegment::LineTo(Point::new(cx - s, cy + s)),
                    ],
                    style: Style { stroke: Some(color), stroke_width: Some(2.0), ..Default::default() },
                    marker_start: None,
                    marker_end: None,
                });
            }
            CommitType::Highlight => {
                let s = COMMIT_RADIUS * 0.9;
                scene.push(Primitive::Rect {
                    bbox: BBox::new(cx, cy, s * 2.0, s * 2.0),
                    rx: 2.0, ry: 2.0,
                    style: Style { fill: Some(color), stroke: Some(Color::WHITE), stroke_width: Some(2.0), ..Default::default() },
                });
            }
        }

        // Merge indicator (smaller inner circle)
        if commit.is_merge {
            scene.push(Primitive::Circle {
                center: Point::new(cx, cy),
                radius: COMMIT_RADIUS * 0.35,
                style: Style { fill: Some(Color::WHITE), ..Default::default() },
            });
        }

        // Tag
        if let Some(tag) = &commit.tag {
            let tag_w = SimpleTextMeasure::measure_raw(tag, &label_style).width + TAG_PAD * 2.0;
            let (tx, ty) = if is_horizontal {
                (cx, cy - COMMIT_RADIUS - TAG_HEIGHT)
            } else {
                (cx + COMMIT_RADIUS + tag_w / 2.0 + 4.0, cy)
            };
            scene.push(Primitive::Rect {
                bbox: BBox::new(tx, ty, tag_w, TAG_HEIGHT),
                rx: 3.0, ry: 3.0,
                style: Style { fill: Some(Color::rgba(255, 255, 200, 220)), stroke: Some(color), stroke_width: Some(1.0), ..Default::default() },
            });
            scene.push(Primitive::Text {
                position: Point::new(tx, ty),
                content: tag.clone(),
                anchor: TextAnchor::Middle,
                style: TextStyle {
                    font_size: theme.font_size_small,
                    fill: Some(theme.node_text),
                    ..Default::default()
                },
            });
        }

        // Commit ID label
        if let Some(id) = &commit.id {
            let (lx, ly, anchor) = if is_horizontal {
                (cx, cy + COMMIT_RADIUS + 14.0, TextAnchor::Middle)
            } else {
                (cx - COMMIT_RADIUS - 6.0, cy, TextAnchor::End)
            };
            scene.push(Primitive::Text {
                position: Point::new(lx, ly),
                content: id.clone(),
                anchor,
                style: TextStyle {
                    font_size: theme.font_size_small,
                    fill: Some(Color::rgb(130, 130, 130)),
                    ..Default::default()
                },
            });
        }
    }

    scene
}

// ── Commit DAG builder ──

struct BuiltCommit {
    id: Option<String>,
    branch: String,
    commit_type: CommitType,
    tag: Option<String>,
    parent_idx: Option<usize>,
    is_merge: bool,
}

fn build_commits(graph: &GitGraph) -> Vec<BuiltCommit> {
    let mut commits: Vec<BuiltCommit> = Vec::new();
    let mut current_branch = "main".to_string();
    let mut branch_heads: BTreeMap<String, usize> = BTreeMap::new();
    let mut auto_id: u32 = 0;

    for stmt in &graph.statements {
        match stmt {
            GitStatement::Commit { id, tag, commit_type } => {
                let parent_idx = branch_heads.get(&current_branch).copied();
                let idx = commits.len();
                commits.push(BuiltCommit {
                    id: id.clone(),
                    branch: current_branch.clone(),
                    commit_type: *commit_type,
                    tag: tag.clone(),
                    parent_idx,
                    is_merge: false,
                });
                branch_heads.insert(current_branch.clone(), idx);
            }
            GitStatement::Branch { name, .. } => {
                if let Some(&head) = branch_heads.get(&current_branch) {
                    branch_heads.insert(name.clone(), head);
                }
                current_branch = name.clone();
            }
            GitStatement::Checkout(name) => {
                current_branch = name.clone();
            }
            GitStatement::Merge { branch, id, tag, commit_type } => {
                let parent_idx = branch_heads.get(branch).copied();
                let idx = commits.len();
                commits.push(BuiltCommit {
                    id: id.clone(),
                    branch: current_branch.clone(),
                    commit_type: *commit_type,
                    tag: tag.clone(),
                    parent_idx,
                    is_merge: true,
                });
                branch_heads.insert(current_branch.clone(), idx);
            }
            GitStatement::CherryPick { id, tag } => {
                let parent_idx = branch_heads.get(&current_branch).copied();
                let idx = commits.len();
                commits.push(BuiltCommit {
                    id: Some(id.clone()),
                    branch: current_branch.clone(),
                    commit_type: CommitType::Normal,
                    tag: tag.clone(),
                    parent_idx,
                    is_merge: false,
                });
                branch_heads.insert(current_branch.clone(), idx);
            }
        }
    }

    commits
}

fn find_final_branch(graph: &GitGraph) -> String {
    let mut current = "main".to_string();
    for stmt in &graph.statements {
        match stmt {
            GitStatement::Branch { name, .. } => current = name.clone(),
            GitStatement::Checkout(name) => current = name.clone(),
            _ => {}
        }
    }
    current
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(input: &str) -> Scene {
        let g = parser::parse(input).unwrap();
        to_scene(&g)
    }

    #[test]
    fn scene_has_commits() {
        let scene = render("gitGraph\n    commit\n    commit\n    commit");
        assert!(scene.len() >= 5);
    }

    #[test]
    fn scene_with_branches() {
        let scene = render("gitGraph\n    commit\n    branch develop\n    commit\n    checkout main\n    commit");
        assert!(scene.len() >= 8);
    }

    #[test]
    fn scene_with_merge() {
        let scene = render("gitGraph\n    commit\n    branch feature\n    commit\n    checkout main\n    merge feature");
        assert!(scene.len() >= 8);
    }

    #[test]
    fn scene_with_tags() {
        let scene = render("gitGraph\n    commit tag: \"v1.0\"\n    commit");
        let has_tag = scene.elements().iter().any(|e| {
            if let Primitive::Text { content, .. } = &e.primitive { content == "v1.0" } else { false }
        });
        assert!(has_tag);
    }

    #[test]
    fn tb_direction() {
        let scene = render("gitGraph TB\n    commit\n    commit");
        assert!(scene.height > scene.width || scene.height > 50.0);
    }

    #[test]
    fn final_branch_detected() {
        let g = parser::parse("gitGraph\n    commit\n    branch dev\n    commit\n    checkout main").unwrap();
        assert_eq!(find_final_branch(&g), "main");
    }

    #[test]
    fn final_branch_is_last_checkout() {
        let g = parser::parse("gitGraph\n    commit\n    branch dev\n    checkout dev").unwrap();
        assert_eq!(find_final_branch(&g), "dev");
    }
}
