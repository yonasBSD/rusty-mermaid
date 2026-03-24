/// Quadrant chart: 2×2 labeled grid with scatter points.

#[derive(Debug, Clone)]
pub struct QuadrantChart {
    pub title: Option<String>,
    pub x_axis: Option<(String, Option<String>)>, // (left, right)
    pub y_axis: Option<(String, Option<String>)>, // (bottom, top)
    pub quadrants: [Option<String>; 4],           // 1=TR, 2=TL, 3=BL, 4=BR
    pub points: Vec<QuadrantPoint>,
}

#[derive(Debug, Clone)]
pub struct QuadrantPoint {
    pub label: String,
    pub x: f64, // 0.0–1.0
    pub y: f64, // 0.0–1.0
}

impl Default for QuadrantChart {
    fn default() -> Self {
        Self {
            title: None,
            x_axis: None,
            y_axis: None,
            quadrants: [None, None, None, None],
            points: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_no_labels() {
        let c = QuadrantChart::default();
        assert!(c.title.is_none());
        assert!(c.quadrants.iter().all(|q| q.is_none()));
    }
}
