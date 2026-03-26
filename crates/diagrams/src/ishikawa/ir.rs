/// Ishikawa (fishbone / cause-effect) diagram.
///
/// First item = effect (the "head"). Remaining top-level items = categories,
/// each with nested causes.

#[derive(Debug, Clone)]
pub struct IshikawaDiagram {
    pub effect: String,
    pub categories: Vec<Category>,
}

#[derive(Debug, Clone)]
pub struct Category {
    pub name: String,
    pub causes: Vec<Cause>,
}

#[derive(Debug, Clone)]
pub struct Cause {
    pub name: String,
    pub subcauses: Vec<Cause>,
}

impl Cause {
    pub fn descendant_count(&self) -> usize {
        self.subcauses.len()
            + self
                .subcauses
                .iter()
                .map(|c| c.descendant_count())
                .sum::<usize>()
    }
}

impl Category {
    pub fn total_causes(&self) -> usize {
        self.causes.len()
            + self
                .causes
                .iter()
                .map(|c| c.descendant_count())
                .sum::<usize>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descendant_count() {
        let c = Cause {
            name: "A".into(),
            subcauses: vec![
                Cause {
                    name: "B".into(),
                    subcauses: vec![],
                },
                Cause {
                    name: "C".into(),
                    subcauses: vec![Cause {
                        name: "D".into(),
                        subcauses: vec![],
                    }],
                },
            ],
        };
        assert_eq!(c.descendant_count(), 3);
    }

    #[test]
    fn descendant_count_leaf() {
        let c = Cause {
            name: "leaf".into(),
            subcauses: vec![],
        };
        assert_eq!(c.descendant_count(), 0);
    }

    #[test]
    fn category_total_causes_empty() {
        let cat = Category {
            name: "People".into(),
            causes: vec![],
        };
        assert_eq!(cat.total_causes(), 0);
    }

    #[test]
    fn category_total_causes_with_nesting() {
        let cat = Category {
            name: "Methods".into(),
            causes: vec![
                Cause {
                    name: "A".into(),
                    subcauses: vec![
                        Cause {
                            name: "A1".into(),
                            subcauses: vec![],
                        },
                        Cause {
                            name: "A2".into(),
                            subcauses: vec![],
                        },
                    ],
                },
                Cause {
                    name: "B".into(),
                    subcauses: vec![],
                },
            ],
        };
        // 2 direct + 2 descendants under A = 4
        assert_eq!(cat.total_causes(), 4);
    }

    #[test]
    fn diagram_construction() {
        let d = IshikawaDiagram {
            effect: "Defect".into(),
            categories: vec![
                Category {
                    name: "People".into(),
                    causes: vec![],
                },
                Category {
                    name: "Process".into(),
                    causes: vec![],
                },
            ],
        };
        assert_eq!(d.effect, "Defect");
        assert_eq!(d.categories.len(), 2);
    }
}
