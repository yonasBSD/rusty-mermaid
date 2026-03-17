use rusty_mermaid_core::Direction;

/// Which algorithm to use for cycle removal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Acyclicer {
    #[default]
    Dfs,
    Greedy,
}

/// Which algorithm to use for rank assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Ranker {
    #[default]
    NetworkSimplex,
    TightTree,
    LongestPath,
}

/// Vertical alignment of nodes within a rank band.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RankAlign {
    #[default]
    Center,
    Top,
    Bottom,
}

/// Optional horizontal alignment override for Brandes-Köpf.
///
/// If set, uses a single alignment instead of the median of all four.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    UL,
    UR,
    DL,
    DR,
}

/// Configuration for the dagre layout algorithm.
#[derive(Debug, Clone)]
pub struct DagreConfig {
    pub rankdir: Direction,
    pub nodesep: f64,
    pub ranksep: f64,
    pub edgesep: f64,
    pub marginx: f64,
    pub marginy: f64,
    pub acyclicer: Acyclicer,
    pub ranker: Ranker,
    pub rankalign: RankAlign,
    pub align: Option<Align>,
}

impl Default for DagreConfig {
    fn default() -> Self {
        Self {
            rankdir: Direction::TB,
            nodesep: 50.0,
            ranksep: 50.0,
            edgesep: 20.0,
            marginx: 0.0,
            marginy: 0.0,
            acyclicer: Acyclicer::default(),
            ranker: Ranker::default(),
            rankalign: RankAlign::default(),
            align: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let c = DagreConfig::default();
        assert_eq!(c.rankdir, Direction::TB);
        assert!((c.nodesep - 50.0).abs() < f64::EPSILON);
        assert!((c.ranksep - 50.0).abs() < f64::EPSILON);
        assert_eq!(c.acyclicer, Acyclicer::Dfs);
        assert_eq!(c.ranker, Ranker::NetworkSimplex);
    }
}
