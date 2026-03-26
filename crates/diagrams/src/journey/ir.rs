/// User journey diagram: horizontal task flow with scored sections.

#[derive(Debug, Clone)]
pub struct JourneyDiagram {
    pub title: Option<String>,
    pub sections: Vec<JourneySection>,
}

#[derive(Debug, Clone)]
pub struct JourneySection {
    pub name: String,
    pub tasks: Vec<JourneyTask>,
}

#[derive(Debug, Clone)]
pub struct JourneyTask {
    pub name: String,
    pub score: u8, // 0–5
    pub actors: Vec<String>,
}

impl Default for JourneyDiagram {
    fn default() -> Self {
        Self { title: None, sections: Vec::new() }
    }
}

impl JourneyDiagram {
    pub fn all_actors(&self) -> Vec<String> {
        let mut actors = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for section in &self.sections {
            for task in &section.tasks {
                for actor in &task.actors {
                    if seen.insert(actor.clone()) {
                        actors.push(actor.clone());
                    }
                }
            }
        }
        actors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_actors_dedup() {
        let d = JourneyDiagram {
            title: None,
            sections: vec![JourneySection {
                name: "S".into(),
                tasks: vec![
                    JourneyTask { name: "T1".into(), score: 5, actors: vec!["A".into(), "B".into()] },
                    JourneyTask { name: "T2".into(), score: 3, actors: vec!["B".into(), "C".into()] },
                ],
            }],
        };
        assert_eq!(d.all_actors(), vec!["A", "B", "C"]);
    }

    #[test]
    fn default_empty() {
        let d = JourneyDiagram::default();
        assert!(d.title.is_none());
        assert!(d.sections.is_empty());
    }

    #[test]
    fn all_actors_empty_diagram() {
        let d = JourneyDiagram::default();
        assert!(d.all_actors().is_empty());
    }

    #[test]
    fn all_actors_across_sections() {
        let d = JourneyDiagram {
            title: Some("Test".into()),
            sections: vec![
                JourneySection {
                    name: "S1".into(),
                    tasks: vec![JourneyTask { name: "T1".into(), score: 4, actors: vec!["Alice".into()] }],
                },
                JourneySection {
                    name: "S2".into(),
                    tasks: vec![JourneyTask { name: "T2".into(), score: 2, actors: vec!["Bob".into(), "Alice".into()] }],
                },
            ],
        };
        assert_eq!(d.all_actors(), vec!["Alice", "Bob"]);
    }

    #[test]
    fn task_score_range() {
        let task = JourneyTask { name: "happy".into(), score: 5, actors: vec![] };
        assert_eq!(task.score, 5);
        let task_low = JourneyTask { name: "sad".into(), score: 0, actors: vec![] };
        assert_eq!(task_low.score, 0);
    }
}
