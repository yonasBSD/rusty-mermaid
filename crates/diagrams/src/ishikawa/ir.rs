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
        self.subcauses.len() + self.subcauses.iter().map(|c| c.descendant_count()).sum::<usize>()
    }
}

impl Category {
    pub fn total_causes(&self) -> usize {
        self.causes.len() + self.causes.iter().map(|c| c.descendant_count()).sum::<usize>()
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
                Cause { name: "B".into(), subcauses: vec![] },
                Cause { name: "C".into(), subcauses: vec![
                    Cause { name: "D".into(), subcauses: vec![] },
                ] },
            ],
        };
        assert_eq!(c.descendant_count(), 3);
    }
}
