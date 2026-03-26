/// A parsed git graph.
#[derive(Debug, Clone)]
pub struct GitGraph {
    pub direction: GitDirection,
    pub statements: Vec<GitStatement>,
}

impl GitGraph {
    pub fn new() -> Self {
        Self {
            direction: GitDirection::LR,
            statements: Vec::new(),
        }
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

    #[test]
    fn direction_variants() {
        let dirs = [GitDirection::LR, GitDirection::TB, GitDirection::BT];
        for (i, a) in dirs.iter().enumerate() {
            for (j, b) in dirs.iter().enumerate() {
                assert_eq!(i == j, *a == *b);
            }
        }
    }

    #[test]
    fn commit_type_variants() {
        assert_ne!(CommitType::Normal, CommitType::Reverse);
        assert_ne!(CommitType::Reverse, CommitType::Highlight);
        assert_eq!(CommitType::Highlight, CommitType::Highlight);
    }

    #[test]
    fn statement_commit() {
        let stmt = GitStatement::Commit {
            id: Some("abc123".into()),
            tag: Some("v1.0".into()),
            commit_type: CommitType::Highlight,
        };
        assert!(
            matches!(stmt, GitStatement::Commit { id: Some(ref i), tag: Some(ref t), commit_type: CommitType::Highlight } if i == "abc123" && t == "v1.0")
        );
    }

    #[test]
    fn statement_branch_and_checkout() {
        let branch = GitStatement::Branch {
            name: "feature".into(),
            order: Some(2),
        };
        let checkout = GitStatement::Checkout("feature".into());
        assert!(
            matches!(branch, GitStatement::Branch { ref name, order: Some(2) } if name == "feature")
        );
        assert!(matches!(checkout, GitStatement::Checkout(ref n) if n == "feature"));
    }
}
