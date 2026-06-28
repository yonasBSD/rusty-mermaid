//! Sugiyama-style layered graph layout engine, ported from dagre.js.
//!
//! Converts a directed graph into a layered layout with positioned nodes and
//! routed edges. The main entry point is [`pipeline::layout()`], which runs
//! the full pipeline:
//!
//! 1. **Rank assignment** -- assign each node to a horizontal layer
//! 2. **Normalize** -- insert dummy nodes for long edges
//! 3. **Order** -- minimize edge crossings within each layer
//! 4. **Position** -- assign x/y coordinates (Brandes-Kopf algorithm)
//!
//! # Key types
//!
//! - [`DagreConfig`] -- layout parameters (direction, spacing, algorithm choices)
//! - [`NodeLabel`] -- node dimensions (input) and position (output)
//! - [`EdgeLabel`] -- edge label dimensions (input) and routed points (output)
//! - [`RankDir`](rusty_mermaid_core::Direction) -- layout flow direction (TB, LR, etc.)
//!
//! # Examples
//!
//! ```
//! use rusty_mermaid_graph::Graph;
//! use rusty_mermaid_dagre::config::DagreConfig;
//! use rusty_mermaid_dagre::labels::{NodeLabel, EdgeLabel};
//! use rusty_mermaid_dagre::pipeline::layout;
//!
//! let mut g = Graph::new();
//! let a = g.add_node(NodeLabel::new(60.0, 30.0));
//! let b = g.add_node(NodeLabel::new(60.0, 30.0));
//! let c = g.add_node(NodeLabel::new(60.0, 30.0));
//! g.add_edge(a, b, EdgeLabel::default());
//! g.add_edge(b, c, EdgeLabel::default());
//!
//! let config = DagreConfig::default();
//! layout(&mut g, &config);
//!
//! // After layout, every node has x/y coordinates
//! let a_pos = g.node(a).unwrap();
//! assert!(a_pos.x > 0.0 && a_pos.y > 0.0);
//!
//! // Edges have routed waypoints
//! for eid in g.edge_ids() {
//!     let e = g.edge(eid).unwrap();
//!     assert!(!e.points.is_empty());
//! }
//! ```

pub mod config;
pub mod labels;

pub mod acyclic;
pub(crate) mod border_segments;
pub(crate) mod coord_system;
pub(crate) mod nesting;
pub(crate) mod normalize;
pub mod order;
pub(crate) mod parent_dummy_chains;
pub mod pipeline;
pub(crate) mod position;
pub mod rank;
pub(crate) mod self_edges;
pub mod util;

pub use config::{Acyclicer, Align, DagreConfig, RankAlign, Ranker};
pub use labels::{EdgeLabel, LabelPos, NodeLabel};

/// Everything needed to build a graph, run the layout, and read results, in one
/// import: `use rusty_mermaid_dagre::prelude::*;`. Pulls the graph + geometry
/// types from the sibling crates so a consumer needn't depend on their paths.
///
/// ```
/// use rusty_mermaid_dagre::prelude::*;
///
/// let mut g: Graph<NodeLabel, EdgeLabel> = Graph::new();
/// let a = g.add_node(NodeLabel::new(60.0, 30.0));
/// let b = g.add_node(NodeLabel::new(60.0, 30.0));
/// g.add_edge(a, b, EdgeLabel::default());
/// layout(&mut g, &DagreConfig::default());
/// assert!(g.node(b).unwrap().y > g.node(a).unwrap().y); // TB: b ranks below a
/// ```
pub mod prelude {
    pub use crate::config::DagreConfig;
    pub use crate::labels::{EdgeLabel, NodeLabel};
    pub use crate::pipeline::layout;
    pub use rusty_mermaid_core::{Direction, Point};
    pub use rusty_mermaid_graph::{EdgeId, Graph, NodeId};
}
