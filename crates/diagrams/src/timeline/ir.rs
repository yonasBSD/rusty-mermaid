use rusty_mermaid_core::Direction;

/// A parsed timeline diagram.
#[derive(Debug, Clone)]
pub struct TimelineDiagram {
    pub title: Option<String>,
    pub direction: Direction,
    pub sections: Vec<TimelineSection>,
}

impl Default for TimelineDiagram {
    fn default() -> Self {
        Self {
            title: None,
            direction: Direction::LR,
            sections: Vec::new(),
        }
    }
}

impl TimelineDiagram {
    pub fn new() -> Self {
        Self::default()
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

    #[test]
    fn new_has_no_title() {
        let d = TimelineDiagram::new();
        assert!(d.title.is_none());
    }

    #[test]
    fn section_with_tasks() {
        let section = TimelineSection {
            name: Some("Phase 1".into()),
            tasks: vec![
                TimelineTask {
                    name: "Start".into(),
                    events: vec!["Kickoff".into()],
                },
                TimelineTask {
                    name: "Plan".into(),
                    events: vec!["Draft".into(), "Review".into()],
                },
            ],
        };
        assert_eq!(section.tasks.len(), 2);
        assert_eq!(section.tasks[1].events.len(), 2);
    }

    #[test]
    fn section_unnamed() {
        let section = TimelineSection {
            name: None,
            tasks: vec![],
        };
        assert!(section.name.is_none());
        assert!(section.tasks.is_empty());
    }

    #[test]
    fn task_with_no_events() {
        let task = TimelineTask {
            name: "Milestone".into(),
            events: vec![],
        };
        assert_eq!(task.name, "Milestone");
        assert!(task.events.is_empty());
    }
}
