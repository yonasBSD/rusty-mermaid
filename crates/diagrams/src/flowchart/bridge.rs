use std::collections::HashMap;

use rusty_mermaid_core::{
    BBox, Color, Point, SimpleTextMeasure, Style, TextMeasure, TextStyle,
    intersect_circle, intersect_polygon, intersect_rect,
};
use rusty_mermaid_dagre::{DagreConfig, EdgeLabel, NodeLabel};
use rusty_mermaid_graph::{Graph, NodeId};

use rusty_mermaid_core::Shape;

use super::ir::{ArrowEnd, FlowDiagram, StrokeType};
use crate::common::styling::StyleProperty;
use crate::common::tokens::strip_html_tags;

const PADDING_X: f64 = 16.0;
const PADDING_Y: f64 = 8.0;

/// Layout result: node positions and edge points.
#[derive(Debug)]
pub struct LayoutResult {
    pub nodes: Vec<NodeLayout>,
    pub edges: Vec<EdgeLayout>,
    pub subgraphs: Vec<SubgraphLayout>,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug)]
pub struct NodeLayout {
    pub id: String,
    pub label: String,
    pub shape: Shape,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    /// Resolved style from classDef, class, style, and :::class.
    pub custom_style: Option<Style>,
}

#[derive(Debug)]
pub struct EdgeLayout {
    pub src: String,
    pub dst: String,
    pub points: Vec<(f64, f64)>,
    pub label: Option<String>,
    pub stroke: StrokeType,
    pub start_arrow: ArrowEnd,
    pub end_arrow: ArrowEnd,
}

#[derive(Debug)]
pub struct SubgraphLayout {
    pub id: String,
    pub label: Option<String>,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Build a dagre Graph from FlowDiagram IR, run layout, return positions.
pub fn layout(diagram: &FlowDiagram) -> LayoutResult {
    layout_with_measurer(diagram, &SimpleTextMeasure::default())
}

/// Layout with a custom text measurer.
pub fn layout_with_measurer(diagram: &FlowDiagram, measurer: &impl TextMeasure) -> LayoutResult {
    let mut g = Graph::new();
    let style = TextStyle::default();

    // Map vertex ID → NodeId
    let mut id_map: HashMap<&str, NodeId> = HashMap::new();

    for v in &diagram.vertices {
        let text = strip_html_tags(&v.label);
        let (tw, th) = measurer.measure(&text, &style);
        let text_w = tw + PADDING_X * 2.0;
        let text_h = th + PADDING_Y * 2.0;

        // Size the dagre node to match the visual shape bounds so edge
        // intersection clipping lands on the shape perimeter, not inside it.
        let (width, height) = match v.shape {
            Shape::Circle => {
                let d = text_w.max(text_h);
                (d, d)
            }
            Shape::DoubleCircle => {
                let d = text_w.max(text_h) + 10.0; // 5px gap on each side
                (d, d)
            }
            Shape::Diamond => {
                // Mermaid: diamond side = w + h, bounding box is square
                let s = text_w + text_h;
                (s, s)
            }
            Shape::Cylinder => {
                // Cylinder caps add ry above and below the body.
                // Mermaid sizes the node tall enough so the body between
                // the caps has room for the text, not a flat petri dish.
                let rx = text_w / 2.0;
                let ry = rx / (2.5 + text_w / 50.0);
                (text_w, text_h + ry * 2.0)
            }
            _ => (text_w, text_h),
        };

        let nid = g.add_node(NodeLabel::new(width, height));
        id_map.insert(&v.id, nid);
    }

    // Set up compound hierarchy for subgraphs (two passes: create all nodes
    // first so that parent→child references resolve regardless of order)
    for sg in &diagram.subgraphs {
        let sg_nid = g.add_node(NodeLabel::new(0.0, 0.0));
        id_map.insert(&sg.id, sg_nid);
    }
    for sg in &diagram.subgraphs {
        let &sg_nid = id_map.get(sg.id.as_str()).unwrap();
        for child_id in &sg.node_ids {
            if let Some(&child_nid) = id_map.get(child_id.as_str()) {
                // First-wins: don't reparent nodes already assigned to a subgraph.
                // Nodes referenced in edges across subgraph boundaries appear in
                // multiple subgraphs' node_ids; they belong to the first one.
                if g.parent(child_nid).is_none() {
                    g.set_parent(child_nid, sg_nid);
                }
            }
        }
        for child_sg_id in &sg.subgraph_ids {
            if let Some(&child_nid) = id_map.get(child_sg_id.as_str()) {
                if g.parent(child_nid).is_none() {
                    g.set_parent(child_nid, sg_nid);
                }
            }
        }
    }

    // Add edges
    for e in &diagram.edges {
        let Some(&src) = id_map.get(e.src.as_str()) else { continue };
        let Some(&dst) = id_map.get(e.dst.as_str()) else { continue };
        let mut label = EdgeLabel::default();
        label.minlen = e.minlen;
        if let Some(text) = &e.label {
            let (tw, th) = measurer.measure(text, &style);
            label.width = tw;
            label.height = th;
        }
        g.add_edge(src, dst, label);
    }

    // Configure
    let mut config = DagreConfig::default();
    config.rankdir = diagram.direction;

    // Run layout
    rusty_mermaid_dagre::pipeline::layout(&mut g, &config);

    // Recenter compound nodes on their content.  The BK position
    // algorithm can place left/right border nodes asymmetrically,
    // causing unequal left/right padding.  Redistribute padding
    // evenly by centering each compound on its children's bounding
    // box.  Process inner-to-outer so parent compounds see the
    // updated child positions.
    for sg in diagram.subgraphs.iter().rev() {
        let Some(&nid) = id_map.get(sg.id.as_str()) else {
            continue;
        };
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        for child in g.children(nid).collect::<Vec<_>>() {
            let c = g.node(child).unwrap();
            if c.width > 0.0 || c.height > 0.0 {
                min_x = min_x.min(c.x - c.width / 2.0);
                max_x = max_x.max(c.x + c.width / 2.0);
            }
        }
        if min_x.is_finite() && max_x.is_finite() {
            g.node_mut(nid).unwrap().x = (min_x + max_x) / 2.0;
        }
    }

    // Resolve per-node styles from classDef + class + style statements.
    let node_styles = resolve_node_styles(diagram);

    // Extract results
    let nid_to_id: HashMap<NodeId, &str> = id_map.iter().map(|(&id, &nid)| (nid, id)).collect();

    let mut nodes = Vec::new();
    let mut max_x: f64 = 0.0;
    let mut max_y: f64 = 0.0;

    for v in &diagram.vertices {
        if let Some(&nid) = id_map.get(v.id.as_str()) {
            let n = g.node(nid).unwrap();
            nodes.push(NodeLayout {
                id: v.id.clone(),
                label: strip_html_tags(&v.label),
                shape: v.shape,
                x: n.x,
                y: n.y,
                width: n.width,
                height: n.height,
                custom_style: node_styles.get(v.id.as_str()).cloned(),
            });
            max_x = max_x.max(n.x + n.width / 2.0);
            max_y = max_y.max(n.y + n.height / 2.0);
        }
    }

    // Build lookup: vertex id → (shape, NodeId) for edge endpoint clipping
    let vertex_shape: HashMap<&str, Shape> = diagram
        .vertices
        .iter()
        .map(|v| (v.id.as_str(), v.shape))
        .collect();

    let mut edges = Vec::new();
    for eid in g.edge_ids() {
        let (src, dst) = g.edge_endpoints(eid).unwrap();
        if let (Some(&src_id), Some(&dst_id)) = (nid_to_id.get(&src), nid_to_id.get(&dst)) {
            let e = g.edge(eid).unwrap();
            let mut points: Vec<(f64, f64)> = e.points.iter().map(|p| (p.x, p.y)).collect();

            // Re-clip edge endpoints for non-rect shapes.
            // Dagre uses intersect_rect for all nodes, which overshoots
            // for shapes inscribed within the bounding rect (diamond, circle, etc.).
            if points.len() >= 2 {
                let src_node = g.node(src).unwrap();
                let src_shape = vertex_shape.get(src_id).copied().unwrap_or(Shape::Rect);
                let adj = points[1];
                if let Some(p) = shape_intersect(
                    src_shape, src_node.x, src_node.y, src_node.width, src_node.height, adj,
                ) {
                    points[0] = p;
                }

                let last = points.len() - 1;
                let dst_node = g.node(dst).unwrap();
                let dst_shape = vertex_shape.get(dst_id).copied().unwrap_or(Shape::Rect);
                let adj = points[last - 1];
                if let Some(p) = shape_intersect(
                    dst_shape, dst_node.x, dst_node.y, dst_node.width, dst_node.height, adj,
                ) {
                    points[last] = p;
                }
            }

            let flow_edge = diagram
                .edges
                .iter()
                .find(|fe| fe.src == src_id && fe.dst == dst_id);
            let label = flow_edge.and_then(|fe| fe.label.clone());
            let stroke = flow_edge.map_or(StrokeType::Normal, |fe| fe.stroke);
            let start_arrow = flow_edge.map_or(ArrowEnd::None, |fe| fe.start_arrow);
            let end_arrow = flow_edge.map_or(ArrowEnd::Arrow, |fe| fe.end_arrow);
            edges.push(EdgeLayout {
                src: src_id.to_string(),
                dst: dst_id.to_string(),
                points,
                label,
                stroke,
                start_arrow,
                end_arrow,
            });
        }
    }

    // Extract subgraph positions from dagre's compound node bounds
    // (padding and label space are already included by remove_border_nodes).
    let mut subgraphs = Vec::new();
    for sg in &diagram.subgraphs {
        if let Some(&nid) = id_map.get(sg.id.as_str()) {
            let n = g.node(nid).unwrap();
            if n.width <= 0.0 || n.height <= 0.0 {
                continue;
            }
            subgraphs.push(SubgraphLayout {
                id: sg.id.clone(),
                label: sg.label.clone(),
                x: n.x,
                y: n.y,
                width: n.width,
                height: n.height,
            });
            max_x = max_x.max(n.x + n.width / 2.0);
            max_y = max_y.max(n.y + n.height / 2.0);
        }
    }

    LayoutResult {
        nodes,
        edges,
        subgraphs,
        width: max_x,
        height: max_y,
    }
}

/// Resolve all style sources into a single `Style` per node.
/// Priority (last wins): classDef "default" → classDef via class/:::class → style statement.
fn resolve_node_styles(diagram: &FlowDiagram) -> HashMap<&str, Style> {
    let class_map: HashMap<&str, &[StyleProperty]> = diagram
        .class_defs
        .iter()
        .map(|cd| (cd.name.as_str(), cd.styles.as_slice()))
        .collect();

    let mut result: HashMap<&str, Style> = HashMap::new();

    for v in &diagram.vertices {
        let mut style = Style::default();
        let mut has_custom = false;

        // 1. Apply "default" classDef to all nodes
        if let Some(props) = class_map.get("default") {
            apply_style_properties(&mut style, props);
            has_custom = true;
        }

        // 2. Apply classes (from `class` statement or `:::className`)
        for class_name in &v.classes {
            if let Some(props) = class_map.get(class_name.as_str()) {
                apply_style_properties(&mut style, props);
                has_custom = true;
            }
        }

        // 3. Apply inline `style` statement (highest priority)
        for stmt in &diagram.style_stmts {
            if stmt.ids.iter().any(|id| id == &v.id) {
                apply_style_properties(&mut style, &stmt.styles);
                has_custom = true;
            }
        }

        if has_custom {
            result.insert(&v.id, style);
        }
    }

    result
}

/// Apply CSS-like style properties onto a Style struct.
fn apply_style_properties(style: &mut Style, props: &[StyleProperty]) {
    for prop in props {
        match prop.key.as_str() {
            "fill" => {
                style.fill = Color::from_css(&prop.value);
            }
            "stroke" => {
                style.stroke = Color::from_css(&prop.value);
            }
            "stroke-width" => {
                let v = prop.value.trim_end_matches("px");
                if let Ok(w) = v.parse::<f64>() {
                    style.stroke_width = Some(w);
                }
            }
            "stroke-dasharray" => {
                let vals: Vec<f64> = prop
                    .value
                    .split_whitespace()
                    .flat_map(|s| s.split(','))
                    .filter_map(|s| s.trim().parse().ok())
                    .collect();
                if !vals.is_empty() {
                    style.stroke_dasharray = Some(vals);
                }
            }
            "opacity" => {
                if let Ok(o) = prop.value.parse::<f64>() {
                    style.opacity = Some(o);
                }
            }
            _ => {}
        }
    }
}

/// Compute shape-specific edge intersection point.
/// Returns `None` for rect-like shapes (dagre's default clipping is correct).
/// For inscribed shapes (diamond, circle, hexagon, etc.), returns the point
/// where the ray from node center toward `adj` crosses the shape perimeter.
fn shape_intersect(
    shape: Shape,
    cx: f64,
    cy: f64,
    w: f64,
    h: f64,
    adj: (f64, f64),
) -> Option<(f64, f64)> {
    let center = Point::new(cx, cy);
    let target = Point::new(adj.0, adj.1);
    let hw = w / 2.0;
    let hh = h / 2.0;

    let p = match shape {
        Shape::Diamond => {
            let verts = [
                Point::new(cx, cy - hh),
                Point::new(cx + hw, cy),
                Point::new(cx, cy + hh),
                Point::new(cx - hw, cy),
            ];
            intersect_polygon(&verts, center, target)
        }
        Shape::Circle | Shape::DoubleCircle => {
            let r = w.max(h) / 2.0;
            intersect_circle(center, r, target)
        }
        Shape::Hexagon => {
            let m = h / 4.0;
            let verts = [
                Point::new(cx - hw + m, cy - hh),
                Point::new(cx + hw - m, cy - hh),
                Point::new(cx + hw, cy),
                Point::new(cx + hw - m, cy + hh),
                Point::new(cx - hw + m, cy + hh),
                Point::new(cx - hw, cy),
            ];
            intersect_polygon(&verts, center, target)
        }
        Shape::Parallelogram => {
            let skew = h / 2.0;
            let verts = [
                Point::new(cx - hw + skew, cy - hh),
                Point::new(cx + hw + skew, cy - hh),
                Point::new(cx + hw - skew, cy + hh),
                Point::new(cx - hw - skew, cy + hh),
            ];
            intersect_polygon(&verts, center, target)
        }
        Shape::ParallelogramAlt => {
            let skew = h / 2.0;
            let verts = [
                Point::new(cx - hw - skew, cy - hh),
                Point::new(cx + hw - skew, cy - hh),
                Point::new(cx + hw + skew, cy + hh),
                Point::new(cx - hw + skew, cy + hh),
            ];
            intersect_polygon(&verts, center, target)
        }
        Shape::Trapezoid => {
            let offset = h / 2.0;
            let verts = [
                Point::new(cx - hw, cy - hh),
                Point::new(cx + hw, cy - hh),
                Point::new(cx + hw + offset, cy + hh),
                Point::new(cx - hw - offset, cy + hh),
            ];
            intersect_polygon(&verts, center, target)
        }
        Shape::TrapezoidAlt => {
            let offset = h / 2.0;
            let verts = [
                Point::new(cx - hw - offset, cy - hh),
                Point::new(cx + hw + offset, cy - hh),
                Point::new(cx + hw, cy + hh),
                Point::new(cx - hw, cy + hh),
            ];
            intersect_polygon(&verts, center, target)
        }
        Shape::Cylinder => {
            // The cylinder's elliptical caps extend beyond the dagre bounding
            // box by ry. Clip to a rect matching the full visual height.
            let rx = hw;
            let ry = rx / (2.5 + w / 50.0);
            let full_h = h + ry;
            let bbox = BBox::new(cx, cy, w, full_h);
            intersect_rect(&bbox, target)
        }
        Shape::Asymmetric => {
            let notch = h / 4.0;
            let verts = [
                Point::new(cx - hw, cy - hh),
                Point::new(cx + hw, cy - hh),
                Point::new(cx + hw, cy + hh),
                Point::new(cx - hw, cy + hh),
                Point::new(cx - hw + notch, cy),
            ];
            intersect_polygon(&verts, center, target)
        }
        // Rect-like shapes: dagre's intersect_rect is already correct
        _ => return None,
    };

    Some((p.x, p.y))
}

#[cfg(test)]
mod tests {
    use rusty_mermaid_core::{Direction, Shape};

    use super::*;
    use crate::flowchart::ir::*;

    #[test]
    fn layout_simple_chain() {
        let mut d = FlowDiagram::new(Direction::TB);
        d.vertices.push(FlowVertex::new("A", "Start", Shape::Rect));
        d.vertices.push(FlowVertex::new("B", "End", Shape::Rect));
        d.edges.push(FlowEdge::new("A", "B"));

        let result = layout(&d);
        assert_eq!(result.nodes.len(), 2);
        assert_eq!(result.edges.len(), 1);

        let a = result.nodes.iter().find(|n| n.id == "A").unwrap();
        let b = result.nodes.iter().find(|n| n.id == "B").unwrap();
        assert!(a.y < b.y, "A should be above B in TB layout");
    }

    #[test]
    fn layout_lr_direction() {
        let mut d = FlowDiagram::new(Direction::LR);
        d.vertices.push(FlowVertex::new("A", "Left", Shape::Rect));
        d.vertices.push(FlowVertex::new("B", "Right", Shape::Rect));
        d.edges.push(FlowEdge::new("A", "B"));

        let result = layout(&d);
        let a = result.nodes.iter().find(|n| n.id == "A").unwrap();
        let b = result.nodes.iter().find(|n| n.id == "B").unwrap();
        assert!(a.x < b.x, "A should be left of B in LR layout");
    }

    #[test]
    fn layout_from_parsed_mmd() {
        let d = crate::flowchart::parser::parse("graph TD\n    A[Start] --> B[End]").unwrap();
        let result = layout(&d);
        assert_eq!(result.nodes.len(), 2);
        assert_eq!(result.edges.len(), 1);
    }

    #[test]
    fn subgraph_contains_children() {
        let mmd = "graph TD\n    subgraph outer[Outer]\n        subgraph inner[Inner]\n            A[Node A] --> B[Node B]\n        end\n        C[Node C]\n    end\n    C --> D[Node D]";
        let d = crate::flowchart::parser::parse(mmd).unwrap();
        let result = layout(&d);

        let inner_sg = result.subgraphs.iter().find(|sg| sg.id == "inner").unwrap();
        let a = result.nodes.iter().find(|n| n.id == "A").unwrap();
        let b = result.nodes.iter().find(|n| n.id == "B").unwrap();

        let sg_left = inner_sg.x - inner_sg.width / 2.0;
        let sg_right = inner_sg.x + inner_sg.width / 2.0;
        let a_left = a.x - a.width / 2.0;
        let a_right = a.x + a.width / 2.0;
        let b_left = b.x - b.width / 2.0;
        let b_right = b.x + b.width / 2.0;

        eprintln!("inner sg: x={:.1} w={:.1} [{:.1}, {:.1}]", inner_sg.x, inner_sg.width, sg_left, sg_right);
        eprintln!("A: x={:.1} w={:.1} [{:.1}, {:.1}]", a.x, a.width, a_left, a_right);
        eprintln!("B: x={:.1} w={:.1} [{:.1}, {:.1}]", b.x, b.width, b_left, b_right);

        assert!(sg_left <= a_left, "inner should contain A horizontally");
        assert!(sg_right >= a_right, "inner should contain A horizontally");
        assert!(sg_left <= b_left, "inner should contain B horizontally");
        assert!(sg_right >= b_right, "inner should contain B horizontally");
    }

    #[test]
    fn layout_edge_has_points() {
        let d = crate::flowchart::parser::parse("graph TD\n    A --> B --> C").unwrap();
        let result = layout(&d);
        for e in &result.edges {
            assert!(!e.points.is_empty(), "edge {}->{} should have points", e.src, e.dst);
        }
    }

    #[test]
    fn subgraph_centered_on_children() {
        let mmd = "flowchart TD\n    subgraph outer[Level 1]\n        subgraph inner[Level 2]\n            A --> B\n        end\n        C --> A\n    end\n    D --> C";
        let d = crate::flowchart::parser::parse(mmd).unwrap();
        let result = layout(&d);

        for sg in &result.subgraphs {
            let sg_left = sg.x - sg.width / 2.0;
            let sg_right = sg.x + sg.width / 2.0;

            // Collect direct children bounds
            let children: Vec<_> = result
                .nodes
                .iter()
                .filter(|n| {
                    let ir_sg = d.subgraphs.iter().find(|s| s.id == sg.id).unwrap();
                    ir_sg.node_ids.contains(&n.id)
                })
                .collect();

            if children.is_empty() {
                continue;
            }

            let content_min = children
                .iter()
                .map(|n| n.x - n.width / 2.0)
                .fold(f64::INFINITY, f64::min);
            let content_max = children
                .iter()
                .map(|n| n.x + n.width / 2.0)
                .fold(f64::NEG_INFINITY, f64::max);

            let left_pad = content_min - sg_left;
            let right_pad = sg_right - content_max;

            eprintln!(
                "{}: center={:.1} left_pad={:.1} right_pad={:.1}",
                sg.id, sg.x, left_pad, right_pad
            );
            assert!(
                (left_pad - right_pad).abs() < 1.0,
                "{}: padding asymmetry {:.1} vs {:.1}",
                sg.id,
                left_pad,
                right_pad
            );
        }
    }

    #[test]
    fn shape_propagated_to_layout() {
        let d = crate::flowchart::parser::parse(
            "flowchart TD\n    A[Rect] --> B(Rounded) --> C{Diamond} --> D((Circle))",
        )
        .unwrap();
        let result = layout(&d);

        let a = result.nodes.iter().find(|n| n.id == "A").unwrap();
        let b = result.nodes.iter().find(|n| n.id == "B").unwrap();
        let c = result.nodes.iter().find(|n| n.id == "C").unwrap();
        let d = result.nodes.iter().find(|n| n.id == "D").unwrap();

        assert_eq!(a.shape, Shape::Rect);
        assert_eq!(b.shape, Shape::RoundedRect);
        assert_eq!(c.shape, Shape::Diamond);
        assert_eq!(d.shape, Shape::Circle);
    }

    #[test]
    fn arrow_types_propagated() {
        let d = crate::flowchart::parser::parse(
            "flowchart TD\n    A --o B\n    A --x C\n    A --- D",
        )
        .unwrap();
        let result = layout(&d);

        let ab = result.edges.iter().find(|e| e.dst == "B").unwrap();
        let ac = result.edges.iter().find(|e| e.dst == "C").unwrap();
        let ad = result.edges.iter().find(|e| e.dst == "D").unwrap();

        assert_eq!(ab.end_arrow, ArrowEnd::Circle);
        assert_eq!(ac.end_arrow, ArrowEnd::Cross);
        assert_eq!(ad.end_arrow, ArrowEnd::None);
    }

    #[test]
    fn bidirectional_arrows() {
        let d = crate::flowchart::parser::parse(
            "flowchart TD\n    A <--> B\n    C <-.-> D\n    E o--o F\n    G x--x H",
        )
        .unwrap();
        let result = layout(&d);

        let ab = result.edges.iter().find(|e| e.src == "A" && e.dst == "B").unwrap();
        assert_eq!(ab.start_arrow, ArrowEnd::Arrow);
        assert_eq!(ab.end_arrow, ArrowEnd::Arrow);

        let cd = result.edges.iter().find(|e| e.src == "C" && e.dst == "D").unwrap();
        assert_eq!(cd.start_arrow, ArrowEnd::Arrow);
        assert_eq!(cd.end_arrow, ArrowEnd::Arrow);

        let ef = result.edges.iter().find(|e| e.src == "E" && e.dst == "F").unwrap();
        assert_eq!(ef.start_arrow, ArrowEnd::Circle);
        assert_eq!(ef.end_arrow, ArrowEnd::Circle);

        let gh = result.edges.iter().find(|e| e.src == "G" && e.dst == "H").unwrap();
        assert_eq!(gh.start_arrow, ArrowEnd::Cross);
        assert_eq!(gh.end_arrow, ArrowEnd::Cross);
    }
}
