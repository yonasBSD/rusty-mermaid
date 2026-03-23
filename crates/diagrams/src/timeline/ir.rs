use rusty_mermaid_core::Direction;

/// A parsed timeline diagram.
#[derive(Debug, Clone)]
pub struct TimelineDiagram {
    pub title: Option<String>,
    pub direction: Direction,
    pub sections: Vec<TimelineSection>,
}

impl TimelineDiagram {
    pub fn new() -> Self {
        Self { title: None, direction: Direction::LR, sections: Vec::new() }
    }
}

/// A section grouping tasks.
#[derive(Debug, Clone)]
pub struct TimelineSection {
    pub name: Option<String>,
    pub tasks: Vec<TimelineTask>,
}

/// A task/period with associated events.
#[derive(Debug, Clone)]
pub struct TimelineTask {
    pub name: String,
    pub events: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagram_default() {
        let d = TimelineDiagram::new();
        assert!(d.sections.is_empty());
        assert_eq!(d.direction, Direction::LR);
    }
}
