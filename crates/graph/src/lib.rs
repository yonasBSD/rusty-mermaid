//! Directed multigraph with compound (hierarchical) node support.
//!
//! This crate provides [`Graph<N, E>`], a generic directed multigraph where nodes
//! carry labels of type `N` and edges carry labels of type `E`. Multiple edges
//! between the same pair of nodes are allowed, and nodes can be organized into a
//! parent-child hierarchy for compound graphs (subgraphs).
//!
//! # Key types
//!
//! - [`Graph<N, E>`] -- the graph container
//! - [`NodeId`] / [`EdgeId`] -- opaque, stable identifiers
//!
//! # Traversal
//!
//! The [`traversal`] module provides [`dfs`], [`bfs`], [`topo_sort`], and
//! [`postorder`] iterators.
//!
//! # Examples
//!
//! ```
//! use rusty_mermaid_graph::{Graph, NodeId};
//!
//! let mut g: Graph<&str, &str> = Graph::new();
//!
//! let a = g.add_node("A");
//! let b = g.add_node("B");
//! let c = g.add_node("C");
//!
//! g.add_edge(a, b, "a->b");
//! g.add_edge(b, c, "b->c");
//!
//! // Iterate successors
//! let successors: Vec<NodeId> = g.successors(a).collect();
//! assert_eq!(successors, vec![b]);
//!
//! // Compound hierarchy
//! let parent = g.add_node("Parent");
//! g.set_parent(a, parent);
//! g.set_parent(b, parent);
//! assert_eq!(g.children(parent).count(), 2);
//! ```

pub mod graph;
pub mod id;
pub mod traversal;

pub use graph::Graph;
pub use id::{EdgeId, IdGen, NodeId};
pub use traversal::{bfs, dfs, dfs_all, postorder, topo_sort};
