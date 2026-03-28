/// Venn diagram: overlapping circles representing set relationships.

#[derive(Debug, Clone, Default)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_empty() {
        let d = VennDiagram::default();
        assert!(d.sets.is_empty());
        assert!(d.unions.is_empty());
    }

    #[test]
    fn default_no_title() {
        let d = VennDiagram::default();
        assert!(d.title.is_none());
    }

    #[test]
    fn set_construction() {
        let s = VennSet {
            id: "A".into(),
            label: "Set A".into(),
            size: 100.0,
        };
        assert_eq!(s.id, "A");
        assert_eq!(s.label, "Set A");
        assert!((s.size - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn union_with_label() {
        let u = VennUnion {
            set_ids: vec!["A".into(), "B".into()],
            label: Some("A and B".into()),
            size: 25.0,
        };
        assert_eq!(u.set_ids.len(), 2);
        assert_eq!(u.label.as_deref(), Some("A and B"));
    }

    #[test]
    fn union_without_label() {
        let u = VennUnion {
            set_ids: vec!["X".into(), "Y".into(), "Z".into()],
            label: None,
            size: 10.0,
        };
        assert_eq!(u.set_ids.len(), 3);
        assert!(u.label.is_none());
        assert!((u.size - 10.0).abs() < f64::EPSILON);
    }
}
