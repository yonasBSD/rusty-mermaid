/// A parsed pie chart.
#[derive(Debug, Clone)]
pub struct PieChart {
    pub title: Option<String>,
    pub show_data: bool,
    pub slices: Vec<PieSlice>,
}

impl PieChart {
    pub fn new() -> Self {
        Self {
            title: None,
            show_data: false,
            slices: Vec::new(),
        }
    }

    pub fn total(&self) -> f64 {
        self.slices.iter().map(|s| s.value).sum()
    }
}

/// A single pie slice.
#[derive(Debug, Clone)]
pub struct PieSlice {
    pub label: String,
    pub value: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total() {
        let mut p = PieChart::new();
        p.slices.push(PieSlice {
            label: "A".into(),
            value: 30.0,
        });
        p.slices.push(PieSlice {
            label: "B".into(),
            value: 70.0,
        });
        assert!((p.total() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn new_defaults() {
        let p = PieChart::new();
        assert!(p.title.is_none());
        assert!(!p.show_data);
        assert!(p.slices.is_empty());
    }

    #[test]
    fn total_empty() {
        let p = PieChart::new();
        assert!((p.total() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn total_single_slice() {
        let mut p = PieChart::new();
        p.slices.push(PieSlice {
            label: "Only".into(),
            value: 42.5,
        });
        assert!((p.total() - 42.5).abs() < f64::EPSILON);
    }

    #[test]
    fn show_data_flag() {
        let mut p = PieChart::new();
        p.show_data = true;
        p.title = Some("Revenue".into());
        assert!(p.show_data);
        assert_eq!(p.title.as_deref(), Some("Revenue"));
    }
}
