/// A parsed gantt chart.
#[derive(Debug, Clone)]
pub struct GanttChart {
    pub title: Option<String>,
    pub date_format: String,
    pub axis_format: Option<String>,
    pub sections: Vec<GanttSection>,
}

impl GanttChart {
    pub fn new() -> Self {
        Self {
            title: None,
            date_format: "YYYY-MM-DD".to_string(),
            axis_format: None,
            sections: Vec::new(),
        }
    }
}

/// A section grouping tasks.
#[derive(Debug, Clone)]
pub struct GanttSection {
    pub name: Option<String>,
    pub tasks: Vec<GanttTask>,
}

/// A gantt task (raw, before date resolution).
#[derive(Debug, Clone)]
pub struct GanttTask {
    pub name: String,
    pub id: Option<String>,
    pub tags: Vec<TaskTag>,
    pub start: TaskStart,
    pub end: TaskEnd,
}

/// Task status/display tags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskTag {
    Done,
    Active,
    Crit,
    Milestone,
}

/// How a task's start is specified.
#[derive(Debug, Clone)]
pub enum TaskStart {
    /// Explicit date string.
    Date(String),
    /// After another task: `after taskId`.
    After(String),
    /// Immediately after previous task.
    Auto,
}

/// How a task's end is specified.
#[derive(Debug, Clone)]
pub enum TaskEnd {
    /// Explicit date string.
    Date(String),
    /// Duration: e.g. "5d", "2w", "3h".
    Duration(String),
    /// No end specified (defaults to 1 day).
    Auto,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chart_default() {
        let c = GanttChart::new();
        assert_eq!(c.date_format, "YYYY-MM-DD");
        assert!(c.sections.is_empty());
    }

    #[test]
    fn task_tags() {
        assert_ne!(TaskTag::Done, TaskTag::Active);
        assert_eq!(TaskTag::Crit, TaskTag::Crit);
    }

    #[test]
    fn new_no_title_no_axis_format() {
        let c = GanttChart::new();
        assert!(c.title.is_none());
        assert!(c.axis_format.is_none());
    }

    #[test]
    fn task_tag_all_variants_distinct() {
        let tags = [
            TaskTag::Done,
            TaskTag::Active,
            TaskTag::Crit,
            TaskTag::Milestone,
        ];
        for (i, a) in tags.iter().enumerate() {
            for (j, b) in tags.iter().enumerate() {
                assert_eq!(i == j, *a == *b);
            }
        }
    }

    #[test]
    fn task_start_variants() {
        let date = TaskStart::Date("2024-01-01".into());
        let after = TaskStart::After("task1".into());
        let auto = TaskStart::Auto;
        assert!(matches!(date, TaskStart::Date(ref s) if s == "2024-01-01"));
        assert!(matches!(after, TaskStart::After(ref s) if s == "task1"));
        assert!(matches!(auto, TaskStart::Auto));
    }

    #[test]
    fn task_end_variants() {
        let date = TaskEnd::Date("2024-06-30".into());
        let dur = TaskEnd::Duration("5d".into());
        let auto = TaskEnd::Auto;
        assert!(matches!(date, TaskEnd::Date(ref s) if s == "2024-06-30"));
        assert!(matches!(dur, TaskEnd::Duration(ref s) if s == "5d"));
        assert!(matches!(auto, TaskEnd::Auto));
    }
}
