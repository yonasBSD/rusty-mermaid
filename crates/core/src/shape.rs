/// Node shape variants across all diagram types.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Shape {
    // Flowchart
    Rect,
    RoundedRect,
    Stadium,
    Subroutine,
    Cylinder,
    Circle,
    DoubleCircle,
    Diamond,
    Hexagon,
    Parallelogram,
    ParallelogramAlt,
    Trapezoid,
    TrapezoidAlt,
    Asymmetric,
    // State
    StateStart,
    StateEnd,
    ForkJoin,
    Choice,
    // Class/ER
    ClassBox,
    ErEntity,
    // Generic
    Note,
    Cloud,
    Document,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn shapes_are_distinct() {
        let shapes = [
            Shape::Rect,
            Shape::RoundedRect,
            Shape::Stadium,
            Shape::Diamond,
            Shape::Circle,
            Shape::Hexagon,
            Shape::StateStart,
            Shape::StateEnd,
            Shape::ClassBox,
            Shape::Note,
        ];
        let set: HashSet<_> = shapes.iter().collect();
        assert_eq!(set.len(), shapes.len());
    }

    #[test]
    fn shape_is_copy() {
        let s = Shape::Diamond;
        let s2 = s;
        assert_eq!(s, s2);
    }
}
