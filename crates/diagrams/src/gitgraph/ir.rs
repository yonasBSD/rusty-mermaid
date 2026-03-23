/// A parsed git graph.
#[derive(Debug, Clone)]
pub struct GitGraph {
    pub direction: GitDirection,
    pub statements: Vec<GitStatement>,
}

impl GitGraph {
    pub fn new() -> Self {
        Self { direction: GitDirection::LR, statements: Vec::new() }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitDirection {
    LR,
    TB,
    BT,
}

#[derive(Debug, Clone)]
pub enum GitStatement {
    Commit {
        id: Option<String>,
        tag: Option<String>,
        commit_type: CommitType,
    },
    Branch {
        name: String,
        order: Option<i32>,
    },
    Checkout(String),
    Merge {
        branch: String,
        id: Option<String>,
        tag: Option<String>,
        commit_type: CommitType,
    },
    CherryPick {
        id: String,
        tag: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CommitType {
    #[default]
    Normal,
    Reverse,
    Highlight,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_default() {
        let g = GitGraph::new();
        assert!(g.statements.is_empty());
        assert_eq!(g.direction, GitDirection::LR);
    }

    #[test]
    fn commit_type_default() {
        assert_eq!(CommitType::default(), CommitType::Normal);
    }
}
