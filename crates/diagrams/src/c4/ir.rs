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
}
