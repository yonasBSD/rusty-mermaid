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
        Self {
            x,
            y,
            width,
            height,
        }
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

    /// Relative luminance (0.0 = black, 1.0 = white) per WCAG 2.0 formula.
    pub fn luminance(self) -> f64 {
        // sRGB linearization constants (IEC 61966-2-1)
        const GAMMA_THRESHOLD: f64 = 0.04045;
        const GAMMA_LINEAR_SCALE: f64 = 12.92;
        const GAMMA_OFFSET: f64 = 0.055;
        const GAMMA_DIVISOR: f64 = 1.055;
        const GAMMA_EXPONENT: f64 = 2.4;
        // Luminance channel weights (ITU-R BT.709)
        const LUM_R: f64 = 0.2126;
        const LUM_G: f64 = 0.7152;
        const LUM_B: f64 = 0.0722;

        fn linearize(c: u8) -> f64 {
            let s = c as f64 / 255.0;
            if s <= GAMMA_THRESHOLD {
                s / GAMMA_LINEAR_SCALE
            } else {
                ((s + GAMMA_OFFSET) / GAMMA_DIVISOR).powf(GAMMA_EXPONENT)
            }
        }
        LUM_R * linearize(self.r) + LUM_G * linearize(self.g) + LUM_B * linearize(self.b)
    }

    /// Parse a CSS color string: `#rgb`, `#rrggbb`, or a named color.
    pub fn from_css(s: &str) -> Option<Self> {
        let s = s.trim();
        if let Some(hex) = s.strip_prefix('#') {
            return Self::from_hex(hex);
        }
        match s.to_ascii_lowercase().as_str() {
            "black" => Some(Self::BLACK),
            "white" => Some(Self::WHITE),
            "red" => Some(Self::rgb(255, 0, 0)),
            "green" => Some(Self::rgb(0, 128, 0)),
            "blue" => Some(Self::rgb(0, 0, 255)),
            "yellow" => Some(Self::rgb(255, 255, 0)),
            "orange" => Some(Self::rgb(255, 165, 0)),
            "purple" => Some(Self::rgb(128, 0, 128)),
            "pink" => Some(Self::rgb(255, 192, 203)),
            "gray" | "grey" => Some(Self::rgb(128, 128, 128)),
            "lightgray" | "lightgrey" => Some(Self::rgb(211, 211, 211)),
            "darkgray" | "darkgrey" => Some(Self::rgb(169, 169, 169)),
            "cyan" => Some(Self::rgb(0, 255, 255)),
            "magenta" => Some(Self::rgb(255, 0, 255)),
            "lime" => Some(Self::rgb(0, 255, 0)),
            "navy" => Some(Self::rgb(0, 0, 128)),
            "teal" => Some(Self::rgb(0, 128, 128)),
            "maroon" => Some(Self::rgb(128, 0, 0)),
            "olive" => Some(Self::rgb(128, 128, 0)),
            "aqua" => Some(Self::rgb(0, 255, 255)),
            "silver" => Some(Self::rgb(192, 192, 192)),
            "transparent" => Some(Self::TRANSPARENT),
            "none" => Some(Self::TRANSPARENT),
            _ => None,
        }
    }

    fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim();
        match hex.len() {
            3 => {
                let r = u8::from_str_radix(&hex[0..1], 16).ok()?;
                let g = u8::from_str_radix(&hex[1..2], 16).ok()?;
                let b = u8::from_str_radix(&hex[2..3], 16).ok()?;
                Some(Self::rgb(r * 17, g * 17, b * 17))
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                Some(Self::rgb(r, g, b))
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
                Some(Self::rgba(r, g, b, a))
            }
            _ => None,
        }
    }
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
