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
}
