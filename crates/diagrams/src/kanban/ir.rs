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
}
