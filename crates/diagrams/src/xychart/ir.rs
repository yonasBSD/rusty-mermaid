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
}
