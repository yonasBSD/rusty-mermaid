// Internal APIs are used by tests and the pipeline (Phase 1d).
#![allow(dead_code)]

pub mod config;
pub mod labels;

pub mod acyclic;
pub mod rank;
pub(crate) mod border_segments;
pub(crate) mod coord_system;
pub mod pipeline;
pub(crate) mod nesting;
pub(crate) mod normalize;
pub mod order;
pub(crate) mod parent_dummy_chains;
pub(crate) mod position;
pub(crate) mod self_edges;
pub mod util;

pub use config::{Acyclicer, Align, DagreConfig, RankAlign, Ranker};
pub use labels::{EdgeLabel, LabelPos, NodeLabel};
