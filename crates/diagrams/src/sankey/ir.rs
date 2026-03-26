/// Sankey diagram intermediate representation.
///
/// CSV-like input: `source,target,value` triples defining flow between nodes.

#[derive(Debug, Clone)]
pub struct SankeyDiagram {
    pub links: Vec<SankeyLink>,
}

#[derive(Debug, Clone)]
pub struct SankeyLink {
    pub source: String,
    pub target: String,
    pub value: f64,
}

impl SankeyDiagram {
    /// Collect unique node names in order of first appearance.
    pub fn node_names(&self) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for link in &self.links {
            if seen.insert(link.source.clone()) {
                names.push(link.source.clone());
            }
            if seen.insert(link.target.clone()) {
                names.push(link.target.clone());
            }
        }
        names
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_names_preserves_order() {
        let d = SankeyDiagram {
            links: vec![
                SankeyLink { source: "A".into(), target: "B".into(), value: 10.0 },
                SankeyLink { source: "A".into(), target: "C".into(), value: 5.0 },
                SankeyLink { source: "B".into(), target: "D".into(), value: 8.0 },
            ],
        };
        assert_eq!(d.node_names(), vec!["A", "B", "C", "D"]);
    }

    #[test]
    fn node_names_deduplicates() {
        let d = SankeyDiagram {
            links: vec![
                SankeyLink { source: "X".into(), target: "Y".into(), value: 1.0 },
                SankeyLink { source: "X".into(), target: "Y".into(), value: 2.0 },
            ],
        };
        assert_eq!(d.node_names(), vec!["X", "Y"]);
    }

    #[test]
    fn node_names_empty() {
        let d = SankeyDiagram { links: vec![] };
        assert!(d.node_names().is_empty());
    }

    #[test]
    fn link_construction() {
        let link = SankeyLink { source: "Power".into(), target: "Heat".into(), value: 42.5 };
        assert_eq!(link.source, "Power");
        assert_eq!(link.target, "Heat");
        assert!((link.value - 42.5).abs() < f64::EPSILON);
    }

    #[test]
    fn node_names_single_link() {
        let d = SankeyDiagram {
            links: vec![SankeyLink { source: "In".into(), target: "Out".into(), value: 100.0 }],
        };
        assert_eq!(d.node_names(), vec!["In", "Out"]);
    }
}
