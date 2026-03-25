/// Architecture diagram: services, groups, and directional edges.

#[derive(Debug, Clone)]
pub struct ArchDiagram {
    pub groups: Vec<ArchGroup>,
    pub services: Vec<ArchService>,
    pub junctions: Vec<ArchJunction>,
    pub edges: Vec<ArchEdge>,
}

#[derive(Debug, Clone)]
pub struct ArchGroup {
    pub id: String,
    pub icon: String,
    pub label: String,
    pub parent: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ArchService {
    pub id: String,
    pub icon: String,
    pub label: String,
    pub group: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ArchJunction {
    pub id: String,
    pub group: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ArchEdge {
    pub from: String,
    pub to: String,
    pub from_dir: Dir,
    pub to_dir: Dir,
    pub arrow_left: bool,
    pub arrow_right: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir { T, B, L, R }

impl Default for ArchDiagram {
    fn default() -> Self {
        Self { groups: Vec::new(), services: Vec::new(), junctions: Vec::new(), edges: Vec::new() }
    }
}

impl ArchDiagram {
    /// All node IDs (services + junctions) in order.
    pub fn node_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.services.iter().map(|s| s.id.clone()).collect();
        ids.extend(self.junctions.iter().map(|j| j.id.clone()));
        ids
    }

    /// Find which group a node belongs to.
    pub fn node_group(&self, id: &str) -> Option<&str> {
        self.services.iter().find(|s| s.id == id).and_then(|s| s.group.as_deref())
            .or_else(|| self.junctions.iter().find(|j| j.id == id).and_then(|j| j.group.as_deref()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_empty() {
        let d = ArchDiagram::default();
        assert!(d.services.is_empty());
    }
}
