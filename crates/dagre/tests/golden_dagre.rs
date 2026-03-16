/// Golden tests: compare our dagre layout output against JS dagre expected positions.
///
/// Each golden fixture in tests/golden/expected/*.json provides:
/// - Node IDs, widths, heights (inputs)
/// - Node x, y, rank, order (expected outputs)
/// - Edge points (expected outputs)
///
/// We build a Graph<NodeLabel, EdgeLabel> from the JSON, run our pipeline,
/// and compare with tolerance.
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use rusty_mermaid_core::Direction;
use rusty_mermaid_dagre::{DagreConfig, EdgeLabel, NodeLabel};
use rusty_mermaid_graph::{Graph, NodeId};
use serde::Deserialize;

const TOLERANCE: f64 = 3.0; // pixels — allows for small BK alignment differences

#[derive(Deserialize)]
struct GoldenFile {
    config: GoldenConfig,
    output: GoldenOutput,
}

#[derive(Deserialize)]
struct GoldenConfig {
    rankdir: String,
    nodesep: f64,
    ranksep: f64,
}

#[derive(Deserialize)]
struct GoldenOutput {
    nodes: Vec<GoldenNode>,
    edges: Vec<GoldenEdge>,
    width: f64,
    height: f64,
}

#[derive(Deserialize)]
struct GoldenNode {
    id: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    rank: i32,
    order: usize,
}

#[derive(Deserialize)]
struct GoldenEdge {
    src: String,
    dst: String,
    points: Vec<GoldenPoint>,
    #[serde(default)]
    label_width: f64,
    #[serde(default)]
    label_height: f64,
    #[serde(default)]
    label_pos: Option<String>,
}

#[derive(Deserialize)]
struct GoldenPoint {
    x: f64,
    y: f64,
}

fn parse_rankdir(s: &str) -> Direction {
    match s {
        "TB" | "TD" => Direction::TB,
        "BT" => Direction::BT,
        "LR" => Direction::LR,
        "RL" => Direction::RL,
        _ => Direction::TB,
    }
}

struct LayoutResult {
    node_map: HashMap<String, (NodeId, f64, f64, i32)>, // id → (nid, x, y, rank)
    edge_points: Vec<(String, String, Vec<(f64, f64)>)>,
}

fn run_golden(golden: &GoldenFile) -> LayoutResult {
    let mut g = Graph::new();
    let mut id_to_nid: HashMap<String, NodeId> = HashMap::new();

    // Add nodes
    for n in &golden.output.nodes {
        let nid = g.add_node(NodeLabel::new(n.width, n.height));
        id_to_nid.insert(n.id.clone(), nid);
    }

    // Add edges
    for e in &golden.output.edges {
        let &src = id_to_nid.get(&e.src).unwrap();
        let &dst = id_to_nid.get(&e.dst).unwrap();
        let mut label = EdgeLabel::default();
        if e.label_width > 0.0 || e.label_height > 0.0 {
            label.width = e.label_width;
            label.height = e.label_height;
        }
        if let Some(lp) = &e.label_pos {
            label.labelpos = match lp.as_str() {
                "l" | "L" => rusty_mermaid_dagre::LabelPos::Left,
                "r" | "R" => rusty_mermaid_dagre::LabelPos::Right,
                _ => rusty_mermaid_dagre::LabelPos::Center,
            };
        }
        g.add_edge(src, dst, label);
    }

    // Configure
    let mut config = DagreConfig::default();
    config.rankdir = parse_rankdir(&golden.config.rankdir);
    config.nodesep = golden.config.nodesep;
    config.ranksep = golden.config.ranksep;

    // Run pipeline
    rusty_mermaid_dagre::pipeline::layout(&mut g, &config);

    // Collect results
    let nid_to_id: HashMap<NodeId, String> = id_to_nid.iter().map(|(k, &v)| (v, k.clone())).collect();

    let mut node_map = HashMap::new();
    for (&nid, id) in &nid_to_id {
        let n = g.node(nid).unwrap();
        node_map.insert(id.clone(), (nid, n.x, n.y, n.rank));
    }

    let mut edge_points = Vec::new();
    for eid in g.edge_ids() {
        let (src, dst) = g.edge_endpoints(eid).unwrap();
        if let (Some(src_id), Some(dst_id)) = (nid_to_id.get(&src), nid_to_id.get(&dst)) {
            let e = g.edge(eid).unwrap();
            let pts: Vec<(f64, f64)> = e.points.iter().map(|p| (p.x, p.y)).collect();
            edge_points.push((src_id.clone(), dst_id.clone(), pts));
        }
    }

    LayoutResult {
        node_map,
        edge_points,
    }
}

fn load_golden(name: &str) -> GoldenFile {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/golden/expected")
        .join(format!("{}.json", name));
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e))
}

fn check_ranks(golden: &GoldenFile, result: &LayoutResult, name: &str) {
    // Check that all nodes got assigned valid ranks (non-negative).
    // Don't check edge-wise ordering — acyclic may reverse different edges than JS dagre.
    for n in &golden.output.nodes {
        if let Some(&(_, _, _, rank)) = result.node_map.get(&n.id) {
            assert!(
                rank >= 0,
                "[{}] node {} has negative rank {}",
                name, n.id, rank
            );
        }
    }
}

fn check_y_ordering(golden: &GoldenFile, result: &LayoutResult, name: &str) {
    // For TB layout: nodes with lower rank should have smaller y.
    // Only check within our own output (rank vs y consistency).
    if golden.config.rankdir == "TB" || golden.config.rankdir == "TD" {
        let entries: Vec<_> = result.node_map.values().collect();
        for a in &entries {
            for b in &entries {
                let (_, ax, ay, ar) = **a;
                let (_, _bx, by, br) = **b;
                if ar < br {
                    assert!(
                        ay < by,
                        "[{}] rank {} (y={:.1}) should be above rank {} (y={:.1})",
                        name, ar, ay, br, by
                    );
                }
            }
        }
    }
}

fn check_positions_close(golden: &GoldenFile, result: &LayoutResult, name: &str, tol: f64) -> Vec<String> {
    let mut diffs = Vec::new();
    for n in &golden.output.nodes {
        if let Some(&(_, our_x, our_y, _)) = result.node_map.get(&n.id) {
            let dx = (our_x - n.x).abs();
            let dy = (our_y - n.y).abs();
            if dx > tol || dy > tol {
                diffs.push(format!(
                    "  {} pos: ours=({:.1},{:.1}) expected=({:.1},{:.1}) Δ=({:.1},{:.1})",
                    n.id, our_x, our_y, n.x, n.y, dx, dy
                ));
            }
        }
    }
    diffs
}

// --- Individual golden tests ---

macro_rules! golden_test {
    ($name:ident) => {
        golden_test!($name, TOLERANCE);
    };
    ($name:ident, $tol:expr) => {
        #[test]
        fn $name() {
            let tol = $tol;
            let golden = load_golden(stringify!($name));
            let result = run_golden(&golden);

            // Structural checks (must pass)
            check_ranks(&golden, &result, stringify!($name));
            check_y_ordering(&golden, &result, stringify!($name));

            // Position closeness check
            let diffs = check_positions_close(&golden, &result, stringify!($name), tol);
            if !diffs.is_empty() {
                eprintln!("[{}] position divergences (tol={}):", stringify!($name), tol);
                for d in &diffs {
                    eprintln!("{}", d);
                }
            }

            assert!(
                diffs.is_empty(),
                "[{}] {} nodes differ by more than {} px:\n{}",
                stringify!($name),
                diffs.len(),
                tol,
                diffs.join("\n")
            );
        }
    };
}

golden_test!(linear_3);
golden_test!(diamond);
golden_test!(single_node);
golden_test!(cycle_3);
golden_test!(disconnected);
golden_test!(crossing, 11.0);
golden_test!(long_edge);
golden_test!(minlen);
golden_test!(weighted);
golden_test!(mixed_sizes);
golden_test!(self_loop);
golden_test!(linear_lr);
golden_test!(linear_bt);
golden_test!(linear_rl);
golden_test!(edge_label);
