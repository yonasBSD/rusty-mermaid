/// C4 model diagram: system context, containers, components.

#[derive(Debug, Clone)]
pub struct C4Diagram {
    pub title: Option<String>,
    pub level: C4Level,
    pub elements: Vec<C4Element>,
    pub boundaries: Vec<C4Boundary>,
    pub relationships: Vec<C4Rel>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum C4Level {
    Context,
    Container,
    Component,
    Dynamic,
}

#[derive(Debug, Clone)]
pub struct C4Element {
    pub alias: String,
    pub label: String,
    pub technology: Option<String>,
    pub description: Option<String>,
    pub shape: C4Shape,
    pub external: bool,
    pub boundary: Option<String>, // parent boundary alias
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum C4Shape {
    Person,
    System,
    Container,
    Component,
    Database,
    Queue,
}

#[derive(Debug, Clone)]
pub struct C4Boundary {
    pub alias: String,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct C4Rel {
    pub from: String,
    pub to: String,
    pub label: String,
    pub technology: Option<String>,
}

impl Default for C4Diagram {
    fn default() -> Self {
        Self {
            title: None,
            level: C4Level::Context,
            elements: Vec::new(),
            boundaries: Vec::new(),
            relationships: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_empty() {
        let d = C4Diagram::default();
        assert!(d.elements.is_empty());
        assert_eq!(d.level, C4Level::Context);
    }

    #[test]
    fn default_no_title_and_empty_collections() {
        let d = C4Diagram::default();
        assert!(d.title.is_none());
        assert!(d.boundaries.is_empty());
        assert!(d.relationships.is_empty());
    }

    #[test]
    fn c4_level_variants() {
        let levels = [C4Level::Context, C4Level::Container, C4Level::Component, C4Level::Dynamic];
        for (i, a) in levels.iter().enumerate() {
            for (j, b) in levels.iter().enumerate() {
                assert_eq!(i == j, *a == *b);
            }
        }
    }

    #[test]
    fn c4_shape_variants() {
        assert_ne!(C4Shape::Person, C4Shape::System);
        assert_ne!(C4Shape::Container, C4Shape::Database);
        assert_eq!(C4Shape::Queue, C4Shape::Queue);
    }

    #[test]
    fn element_construction() {
        let elem = C4Element {
            alias: "web".into(), label: "Web App".into(),
            technology: Some("React".into()), description: Some("Frontend".into()),
            shape: C4Shape::Container, external: false, boundary: Some("sys1".into()),
        };
        assert_eq!(elem.alias, "web");
        assert!(!elem.external);
        assert_eq!(elem.shape, C4Shape::Container);
        assert_eq!(elem.boundary.as_deref(), Some("sys1"));
    }
}
