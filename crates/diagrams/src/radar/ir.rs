/// Radar (spider) chart: polar axes with polygon overlays per data series.

#[derive(Debug, Clone)]
pub struct RadarChart {
    pub title: Option<String>,
    pub axes: Vec<RadarAxis>,
    pub curves: Vec<RadarCurve>,
    pub ticks: usize,
    pub min: f64,
    pub max: Option<f64>, // None = auto from data
    pub graticule: Graticule,
}

#[derive(Debug, Clone)]
pub struct RadarAxis {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct RadarCurve {
    pub id: String,
    pub label: String,
    pub values: Vec<f64>, // one per axis, in axis order
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Graticule {
    Circle,
    Polygon,
}

impl Default for RadarChart {
    fn default() -> Self {
        Self {
            title: None,
            axes: Vec::new(),
            curves: Vec::new(),
            ticks: 5,
            min: 0.0,
            max: None,
            graticule: Graticule::Polygon,
        }
    }
}

impl RadarChart {
    pub fn effective_max(&self) -> f64 {
        self.max.unwrap_or_else(|| {
            self.curves
                .iter()
                .flat_map(|c| &c.values)
                .copied()
                .fold(0.0f64, f64::max)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effective_max_auto() {
        let mut c = RadarChart::default();
        c.curves.push(RadarCurve {
            id: "a".into(),
            label: "A".into(),
            values: vec![3.0, 7.0, 5.0],
        });
        assert!((c.effective_max() - 7.0).abs() < f64::EPSILON);
    }

    #[test]
    fn effective_max_explicit() {
        let mut c = RadarChart::default();
        c.max = Some(10.0);
        c.curves.push(RadarCurve {
            id: "a".into(),
            label: "A".into(),
            values: vec![3.0, 7.0],
        });
        assert!((c.effective_max() - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn default_values() {
        let c = RadarChart::default();
        assert!(c.title.is_none());
        assert!(c.axes.is_empty());
        assert!(c.curves.is_empty());
        assert_eq!(c.ticks, 5);
        assert!((c.min - 0.0).abs() < f64::EPSILON);
        assert!(c.max.is_none());
        assert_eq!(c.graticule, Graticule::Polygon);
    }

    #[test]
    fn graticule_variants() {
        assert_ne!(Graticule::Circle, Graticule::Polygon);
        assert_eq!(Graticule::Circle, Graticule::Circle);
    }

    #[test]
    fn effective_max_no_curves() {
        let c = RadarChart::default();
        assert!((c.effective_max() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn effective_max_multiple_curves() {
        let mut c = RadarChart::default();
        c.curves.push(RadarCurve {
            id: "a".into(),
            label: "A".into(),
            values: vec![1.0, 2.0],
        });
        c.curves.push(RadarCurve {
            id: "b".into(),
            label: "B".into(),
            values: vec![5.0, 3.0],
        });
        assert!((c.effective_max() - 5.0).abs() < f64::EPSILON);
    }
}
