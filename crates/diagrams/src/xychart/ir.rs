/// A parsed xy chart.
#[derive(Debug, Clone)]
pub struct XyChart {
    pub title: Option<String>,
    pub x_axis: AxisDef,
    pub y_axis: AxisDef,
    pub plots: Vec<PlotData>,
    pub horizontal: bool,
}

impl XyChart {
    pub fn new() -> Self {
        Self {
            title: None,
            x_axis: AxisDef::Band { title: None, categories: Vec::new() },
            y_axis: AxisDef::Linear { title: None, min: None, max: None },
            plots: Vec::new(),
            horizontal: false,
        }
    }
}

/// Axis definition.
#[derive(Debug, Clone)]
pub enum AxisDef {
    Band { title: Option<String>, categories: Vec<String> },
    Linear { title: Option<String>, min: Option<f64>, max: Option<f64> },
}

/// A data series.
#[derive(Debug, Clone)]
pub struct PlotData {
    pub plot_type: PlotType,
    pub label: Option<String>,
    pub values: Vec<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlotType {
    Bar,
    Line,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chart_default() {
        let c = XyChart::new();
        assert!(c.plots.is_empty());
        assert!(!c.horizontal);
    }

    #[test]
    fn new_axes_defaults() {
        let c = XyChart::new();
        assert!(c.title.is_none());
        assert!(matches!(c.x_axis, AxisDef::Band { ref categories, .. } if categories.is_empty()));
        assert!(matches!(c.y_axis, AxisDef::Linear { min: None, max: None, .. }));
    }

    #[test]
    fn plot_type_equality() {
        assert_eq!(PlotType::Bar, PlotType::Bar);
        assert_eq!(PlotType::Line, PlotType::Line);
        assert_ne!(PlotType::Bar, PlotType::Line);
    }

    #[test]
    fn plot_data_construction() {
        let p = PlotData {
            plot_type: PlotType::Bar,
            label: Some("Sales".into()),
            values: vec![10.0, 20.0, 30.0],
        };
        assert_eq!(p.values.len(), 3);
        assert_eq!(p.label.as_deref(), Some("Sales"));
    }

    #[test]
    fn axis_def_band_with_categories() {
        let axis = AxisDef::Band {
            title: Some("Months".into()),
            categories: vec!["Jan".into(), "Feb".into(), "Mar".into()],
        };
        if let AxisDef::Band { title, categories } = axis {
            assert_eq!(title.as_deref(), Some("Months"));
            assert_eq!(categories.len(), 3);
        } else {
            panic!("expected Band");
        }
    }
}
