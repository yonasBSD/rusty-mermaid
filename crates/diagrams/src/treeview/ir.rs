/// Treeview diagram: indented file/folder tree with line connectors.

#[derive(Debug, Clone)]
pub struct TreeView {
    pub roots: Vec<TreeNode>,
}

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub name: String,
    pub children: Vec<TreeNode>,
}

impl TreeView {
    pub fn node_count(&self) -> usize {
        self.roots.iter().map(|r| r.count()).sum()
    }
}

impl TreeNode {
    fn count(&self) -> usize {
        1 + self.children.iter().map(|c| c.count()).sum::<usize>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count() {
        let t = TreeView {
            roots: vec![TreeNode {
                name: "a".into(),
                children: vec![
                    TreeNode { name: "b".into(), children: vec![] },
                    TreeNode { name: "c".into(), children: vec![] },
                ],
            }],
        };
        assert_eq!(t.node_count(), 3);
    }

    #[test]
    fn count_empty() {
        let t = TreeView { roots: vec![] };
        assert_eq!(t.node_count(), 0);
    }

    #[test]
    fn count_single_root() {
        let t = TreeView {
            roots: vec![TreeNode { name: "root".into(), children: vec![] }],
        };
        assert_eq!(t.node_count(), 1);
    }

    #[test]
    fn count_multiple_roots() {
        let t = TreeView {
            roots: vec![
                TreeNode { name: "a".into(), children: vec![] },
                TreeNode { name: "b".into(), children: vec![
                    TreeNode { name: "b1".into(), children: vec![] },
                ]},
            ],
        };
        assert_eq!(t.node_count(), 3);
    }

    #[test]
    fn count_deep_nesting() {
        let t = TreeView {
            roots: vec![TreeNode {
                name: "1".into(),
                children: vec![TreeNode {
                    name: "2".into(),
                    children: vec![TreeNode {
                        name: "3".into(),
                        children: vec![TreeNode { name: "4".into(), children: vec![] }],
                    }],
                }],
            }],
        };
        assert_eq!(t.node_count(), 4);
    }
}
