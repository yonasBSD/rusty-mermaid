/// Line height as a multiple of font size (CSS `line-height: 1.2`).
pub const LINE_HEIGHT_MULTIPLIER: f64 = 1.2;
pub const LINE_HEIGHT_MULTIPLIER_F32: f32 = LINE_HEIGHT_MULTIPLIER as f32;

/// Baseline offset as a fraction of font size (approximate ascent ratio).
pub const BASELINE_ASCENT_RATIO: f64 = 0.3;

/// Reference font size that MONOSPACE_CHAR_WIDTH_14PX is calibrated to.
pub const REFERENCE_FONT_SIZE: f64 = 14.0;

/// Number of line segments used to approximate arc paths.
pub const ARC_APPROXIMATION_STEPS: usize = 64;

/// Bezier approximation constant for quarter-circle arcs.
/// Exact value: 4 * (sqrt(2) - 1) / 3 ≈ 0.5522847498.
pub const KAPPA: f64 = 0.5522848;
pub const KAPPA_F32: f32 = KAPPA as f32;
