/// A parsed kanban board.
#[derive(Debug, Clone)]
pub struct KanbanBoard {
    pub columns: Vec<KanbanColumn>,
}

impl KanbanBoard {
    pub fn new() -> Self {
        Self { columns: Vec::new() }
    }
}

/// A kanban column (section).
#[derive(Debug, Clone)]
pub struct KanbanColumn {
    pub id: String,
    pub label: String,
    pub cards: Vec<KanbanCard>,
}

/// A kanban card (task/item).
#[derive(Debug, Clone)]
pub struct KanbanCard {
    pub id: String,
    pub label: String,
    pub priority: Option<Priority>,
    pub assigned: Option<String>,
    pub ticket: Option<String>,
}

impl KanbanCard {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            priority: None,
            assigned: None,
            ticket: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    VeryHigh,
    High,
    Medium,
    Low,
    VeryLow,
}

impl Priority {
    pub fn label(self) -> &'static str {
        match self {
            Self::VeryHigh => "Very High",
            Self::High => "High",
            Self::Medium => "Medium",
            Self::Low => "Low",
            Self::VeryLow => "Very Low",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn board_default() {
        let b = KanbanBoard::new();
        assert!(b.columns.is_empty());
    }

    #[test]
    fn card_default() {
        let c = KanbanCard::new("task1", "Do something");
        assert_eq!(c.id, "task1");
        assert_eq!(c.label, "Do something");
        assert!(c.priority.is_none());
    }

    #[test]
    fn card_optional_fields_none() {
        let c = KanbanCard::new("x", "Y");
        assert!(c.assigned.is_none());
        assert!(c.ticket.is_none());
    }

    #[test]
    fn priority_labels() {
        assert_eq!(Priority::VeryHigh.label(), "Very High");
        assert_eq!(Priority::High.label(), "High");
        assert_eq!(Priority::Medium.label(), "Medium");
        assert_eq!(Priority::Low.label(), "Low");
        assert_eq!(Priority::VeryLow.label(), "Very Low");
    }

    #[test]
    fn priority_equality() {
        let all = [Priority::VeryHigh, Priority::High, Priority::Medium, Priority::Low, Priority::VeryLow];
        for (i, a) in all.iter().enumerate() {
            for (j, b) in all.iter().enumerate() {
                assert_eq!(i == j, *a == *b);
            }
        }
    }

    #[test]
    fn column_with_cards() {
        let col = KanbanColumn {
            id: "todo".into(),
            label: "To Do".into(),
            cards: vec![
                KanbanCard::new("t1", "First"),
                KanbanCard::new("t2", "Second"),
            ],
        };
        assert_eq!(col.cards.len(), 2);
        assert_eq!(col.label, "To Do");
    }
}
