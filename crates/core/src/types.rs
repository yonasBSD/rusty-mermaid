/// 2D point in layout coordinate space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn distance_to(self, other: Point) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

/// Axis-aligned bounding box.
/// `x` and `y` are the center coordinates (matching dagre's convention).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl BBox {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        debug_assert!(width >= 0.0, "BBox width must be non-negative: {width}");
        debug_assert!(height >= 0.0, "BBox height must be non-negative: {height}");
        Self { x, y, width, height }
    }

    pub fn left(&self) -> f64 {
        self.x - self.width / 2.0
    }

    pub fn right(&self) -> f64 {
        self.x + self.width / 2.0
    }

    pub fn top(&self) -> f64 {
        self.y - self.height / 2.0
    }

    pub fn bottom(&self) -> f64 {
        self.y + self.height / 2.0
    }

    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.left() && p.x <= self.right() && p.y >= self.top() && p.y <= self.bottom()
    }

    /// Smallest bounding box enclosing both `self` and `other`.
    pub fn union(&self, other: &BBox) -> BBox {
        let left = self.left().min(other.left());
        let top = self.top().min(other.top());
        let right = self.right().max(other.right());
        let bottom = self.bottom().max(other.bottom());
        let width = right - left;
        let height = bottom - top;
        BBox {
            x: left + width / 2.0,
            y: top + height / 2.0,
            width,
            height,
        }
    }
}

/// RGBA color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.a == 255 {
            write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            write!(
                f,
                "#{:02x}{:02x}{:02x}{:02x}",
                self.r, self.g, self.b, self.a
            )
        }
    }
}

/// Layout direction for ranked graphs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Direction {
    #[default]
    TB,
    BT,
    LR,
    RL,
}

impl Direction {
    pub fn is_horizontal(self) -> bool {
        matches!(self, Direction::LR | Direction::RL)
    }

    pub fn is_vertical(self) -> bool {
        !self.is_horizontal()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_distance() {
        let a = Point::new(0.0, 0.0);
        let b = Point::new(3.0, 4.0);
        assert!((a.distance_to(b) - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn point_distance_to_self() {
        let p = Point::new(42.0, -7.0);
        assert!((p.distance_to(p)).abs() < f64::EPSILON);
    }

    #[test]
    fn bbox_edges() {
        let b = BBox::new(10.0, 20.0, 6.0, 4.0);
        assert!((b.left() - 7.0).abs() < f64::EPSILON);
        assert!((b.right() - 13.0).abs() < f64::EPSILON);
        assert!((b.top() - 18.0).abs() < f64::EPSILON);
        assert!((b.bottom() - 22.0).abs() < f64::EPSILON);
    }

    #[test]
    fn bbox_contains() {
        let b = BBox::new(0.0, 0.0, 10.0, 10.0);
        assert!(b.contains(Point::new(0.0, 0.0)));
        assert!(b.contains(Point::new(5.0, 5.0)));
        assert!(b.contains(Point::new(-5.0, -5.0)));
        assert!(!b.contains(Point::new(6.0, 0.0)));
        assert!(!b.contains(Point::new(0.0, -6.0)));
    }

    #[test]
    fn bbox_union() {
        let a = BBox::new(0.0, 0.0, 4.0, 4.0);
        let b = BBox::new(10.0, 10.0, 4.0, 4.0);
        let u = a.union(&b);
        assert!((u.left() - (-2.0)).abs() < f64::EPSILON);
        assert!((u.right() - 12.0).abs() < f64::EPSILON);
        assert!((u.top() - (-2.0)).abs() < f64::EPSILON);
        assert!((u.bottom() - 12.0).abs() < f64::EPSILON);
    }

    #[test]
    fn color_display_rgb() {
        assert_eq!(Color::rgb(255, 0, 128).to_string(), "#ff0080");
    }

    #[test]
    fn color_display_rgba() {
        assert_eq!(Color::rgba(255, 0, 128, 200).to_string(), "#ff0080c8");
    }

    #[test]
    fn color_constants() {
        assert_eq!(Color::BLACK, Color::rgb(0, 0, 0));
        assert_eq!(Color::WHITE, Color::rgb(255, 255, 255));
        assert_eq!(Color::TRANSPARENT.a, 0);
    }

    #[test]
    fn direction_orientation() {
        assert!(Direction::LR.is_horizontal());
        assert!(Direction::RL.is_horizontal());
        assert!(Direction::TB.is_vertical());
        assert!(Direction::BT.is_vertical());
    }

    #[test]
    fn direction_default() {
        assert_eq!(Direction::default(), Direction::TB);
    }
}
