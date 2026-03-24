/// Venn diagram: overlapping circles representing set relationships.

#[derive(Debug, Clone)]
pub struct VennDiagram {
    pub title: Option<String>,
    pub sets: Vec<VennSet>,
    pub unions: Vec<VennUnion>,
}

#[derive(Debug, Clone)]
pub struct VennSet {
    pub id: String,
    pub label: String,
    pub size: f64,
}

#[derive(Debug, Clone)]
pub struct VennUnion {
    pub set_ids: Vec<String>,
    pub label: Option<String>,
    pub size: f64,
}

impl Default for VennDiagram {
    fn default() -> Self {
        Self { title: None, sets: Vec::new(), unions: Vec::new() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_empty() {
        let d = VennDiagram::default();
        assert!(d.sets.is_empty());
        assert!(d.unions.is_empty());
    }
}
