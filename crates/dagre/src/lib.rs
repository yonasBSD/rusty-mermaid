// Internal APIs are used by tests and the pipeline (Phase 1d).
#![allow(dead_code)]

pub mod config;
pub mod labels;

pub mod acyclic;
pub mod rank;
pub(crate) mod border_segments;
pub(crate) mod nesting;
pub(crate) mod normalize;
pub(crate) mod parent_dummy_chains;
pub(crate) mod util;

pub use config::{Acyclicer, DagreConfig, Ranker};
pub use labels::{EdgeLabel, LabelPos, NodeLabel};
