/// Treemap diagram: squarified rectangle packing with nested labels.

#[derive(Debug, Clone)]
pub struct TreemapDiagram {
    pub roots: Vec<TreemapNode>,
}

#[derive(Debug, Clone)]
pub struct TreemapNode {
    pub name: String,
    pub value: Option<f64>, // leaf value (None = section)
    pub children: Vec<TreemapNode>,
}

impl TreemapNode {
    /// Total value: own value or sum of children.
    pub fn total_value(&self) -> f64 {
        self.value
            .unwrap_or_else(|| self.children.iter().map(|c| c.total_value()).sum())
    }

    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_value_leaf() {
        let n = TreemapNode {
            name: "A".into(),
            value: Some(100.0),
            children: vec![],
        };
        assert!((n.total_value() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn total_value_section() {
        let n = TreemapNode {
            name: "S".into(),
            value: None,
            children: vec![
                TreemapNode {
                    name: "A".into(),
                    value: Some(60.0),
                    children: vec![],
                },
                TreemapNode {
                    name: "B".into(),
                    value: Some(40.0),
                    children: vec![],
                },
            ],
        };
        assert!((n.total_value() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn is_leaf_true() {
        let n = TreemapNode {
            name: "L".into(),
            value: Some(10.0),
            children: vec![],
        };
        assert!(n.is_leaf());
    }

    #[test]
    fn is_leaf_false() {
        let n = TreemapNode {
            name: "P".into(),
            value: None,
            children: vec![TreemapNode {
                name: "C".into(),
                value: Some(5.0),
                children: vec![],
            }],
        };
        assert!(!n.is_leaf());
    }

    #[test]
    fn total_value_nested_sections() {
        let n = TreemapNode {
            name: "Root".into(),
            value: None,
            children: vec![
                TreemapNode {
                    name: "Sub".into(),
                    value: None,
                    children: vec![
                        TreemapNode {
                            name: "A".into(),
                            value: Some(30.0),
                            children: vec![],
                        },
                        TreemapNode {
                            name: "B".into(),
                            value: Some(20.0),
                            children: vec![],
                        },
                    ],
                },
                TreemapNode {
                    name: "C".into(),
                    value: Some(50.0),
                    children: vec![],
                },
            ],
        };
        assert!((n.total_value() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn diagram_empty_roots() {
        let d = TreemapDiagram { roots: vec![] };
        assert!(d.roots.is_empty());
    }
}
