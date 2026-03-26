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

    #[test]
    fn default_axes_and_points_empty() {
        let c = QuadrantChart::default();
        assert!(c.x_axis.is_none());
        assert!(c.y_axis.is_none());
        assert!(c.points.is_empty());
    }

    #[test]
    fn point_construction() {
        let p = QuadrantPoint { label: "Feature X".into(), x: 0.75, y: 0.25 };
        assert_eq!(p.label, "Feature X");
        assert!((p.x - 0.75).abs() < f64::EPSILON);
        assert!((p.y - 0.25).abs() < f64::EPSILON);
    }

    #[test]
    fn quadrant_labels() {
        let mut c = QuadrantChart::default();
        c.quadrants[0] = Some("High Impact, High Effort".into());
        c.quadrants[2] = Some("Low Impact, Low Effort".into());
        assert!(c.quadrants[0].is_some());
        assert!(c.quadrants[1].is_none());
        assert!(c.quadrants[2].is_some());
        assert!(c.quadrants[3].is_none());
    }

    #[test]
    fn axis_labels() {
        let mut c = QuadrantChart::default();
        c.x_axis = Some(("Low".into(), Some("High".into())));
        c.y_axis = Some(("Bottom".into(), None));
        let (left, right) = c.x_axis.as_ref().unwrap();
        assert_eq!(left, "Low");
        assert_eq!(right.as_deref(), Some("High"));
        let (bottom, top) = c.y_axis.as_ref().unwrap();
        assert_eq!(bottom, "Bottom");
        assert!(top.is_none());
    }
}
