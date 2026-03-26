use rusty_mermaid_core::{BBox, ElementId, Point};

/// Viewport state for interactive diagram backends.
///
/// Tracks pan offset, zoom level, and interaction state (hover/selection).
/// All backends share this state; each maps native events to [`ViewportAction`].
#[derive(Debug, Clone)]
pub struct ViewportState {
    /// Pan offset in screen pixels (scene origin relative to screen origin).
    pub offset: Point,
    /// Zoom factor (1.0 = 100%, 2.0 = 200%).
    pub zoom: f64,
    /// Currently hovered element, if any.
    pub hovered: Option<ElementId>,
    /// Currently selected elements.
    pub selected: Vec<ElementId>,
}

impl Default for ViewportState {
    fn default() -> Self {
        Self {
            offset: Point::new(0.0, 0.0),
            zoom: 1.0,
            hovered: None,
            selected: Vec::new(),
        }
    }
}

/// Convert a screen-space point to scene-space coordinates.
///
/// Inverts the viewport transform: `scene = (screen - offset) / zoom`.
pub fn screen_to_scene(screen: Point, viewport: &ViewportState) -> Point {
    Point::new(
        (screen.x - viewport.offset.x) / viewport.zoom,
        (screen.y - viewport.offset.y) / viewport.zoom,
    )
}

/// Convert a scene-space point to screen-space coordinates.
///
/// Applies the viewport transform: `screen = scene * zoom + offset`.
pub fn scene_to_screen(scene: Point, viewport: &ViewportState) -> Point {
    Point::new(
        scene.x * viewport.zoom + viewport.offset.x,
        scene.y * viewport.zoom + viewport.offset.y,
    )
}

/// Compute a viewport that fits the given scene bounding box within a screen size,
/// centered with a small margin.
pub fn zoom_to_fit(scene_bbox: BBox, screen_width: f64, screen_height: f64) -> ViewportState {
    let margin = 20.0;
    let available_w = (screen_width - margin * 2.0).max(1.0);
    let available_h = (screen_height - margin * 2.0).max(1.0);

    let scale_x = available_w / scene_bbox.width.max(1.0);
    let scale_y = available_h / scene_bbox.height.max(1.0);
    let zoom = scale_x.min(scale_y).min(4.0); // cap at 4x

    let scene_cx = scene_bbox.x;
    let scene_cy = scene_bbox.y;

    let offset_x = screen_width / 2.0 - scene_cx * zoom;
    let offset_y = screen_height / 2.0 - scene_cy * zoom;

    ViewportState {
        offset: Point::new(offset_x, offset_y),
        zoom,
        hovered: None,
        selected: Vec::new(),
    }
}

/// An event that modifies viewport state.
///
/// Each interactive backend maps its native events (mouse, touch, keyboard)
/// to these actions. The [`apply`] function is a pure state transition.
#[derive(Debug, Clone)]
pub enum ViewportAction {
    /// Pan by (dx, dy) screen pixels.
    Pan { dx: f64, dy: f64 },
    /// Zoom by `factor` around a screen-space `center` point.
    Zoom { factor: f64, center: Point },
    /// Hover at a screen-space point (backend resolves to element via hit-test).
    Hover(Option<ElementId>),
    /// Select an element (replaces current selection).
    Select(ElementId),
    /// Add an element to the current selection (e.g. shift-click).
    SelectAdd(ElementId),
    /// Clear all selection.
    ClearSelection,
}

/// Apply a viewport action, returning the new state.
///
/// Pure function — no side effects.
pub fn apply(state: &ViewportState, action: &ViewportAction) -> ViewportState {
    let mut next = state.clone();
    match action {
        ViewportAction::Pan { dx, dy } => {
            next.offset.x += dx;
            next.offset.y += dy;
        }
        ViewportAction::Zoom { factor, center } => {
            let scene_before = screen_to_scene(*center, &next);
            next.zoom = (next.zoom * factor).clamp(0.1, 10.0);
            // Adjust offset so the scene point under the cursor stays fixed.
            next.offset.x = center.x - scene_before.x * next.zoom;
            next.offset.y = center.y - scene_before.y * next.zoom;
        }
        ViewportAction::Hover(id) => {
            next.hovered = id.clone();
        }
        ViewportAction::Select(id) => {
            next.selected = vec![id.clone()];
        }
        ViewportAction::SelectAdd(id) => {
            if !next.selected.contains(id) {
                next.selected.push(id.clone());
            }
        }
        ViewportAction::ClearSelection => {
            next.selected.clear();
            next.hovered = None;
        }
    }
    next
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_vp() -> ViewportState {
        ViewportState::default()
    }

    #[test]
    fn default_state() {
        let vp = default_vp();
        assert!((vp.zoom - 1.0).abs() < f64::EPSILON);
        assert!((vp.offset.x).abs() < f64::EPSILON);
        assert!((vp.offset.y).abs() < f64::EPSILON);
        assert!(vp.hovered.is_none());
        assert!(vp.selected.is_empty());
    }

    #[test]
    fn screen_to_scene_identity() {
        let vp = default_vp();
        let p = screen_to_scene(Point::new(100.0, 200.0), &vp);
        assert!((p.x - 100.0).abs() < f64::EPSILON);
        assert!((p.y - 200.0).abs() < f64::EPSILON);
    }

    #[test]
    fn scene_to_screen_identity() {
        let vp = default_vp();
        let p = scene_to_screen(Point::new(100.0, 200.0), &vp);
        assert!((p.x - 100.0).abs() < f64::EPSILON);
        assert!((p.y - 200.0).abs() < f64::EPSILON);
    }

    #[test]
    fn roundtrip_screen_scene() {
        let vp = ViewportState {
            offset: Point::new(50.0, -30.0),
            zoom: 2.5,
            ..Default::default()
        };
        let screen = Point::new(300.0, 400.0);
        let scene = screen_to_scene(screen, &vp);
        let back = scene_to_screen(scene, &vp);
        assert!((back.x - screen.x).abs() < 1e-10);
        assert!((back.y - screen.y).abs() < 1e-10);
    }

    #[test]
    fn screen_to_scene_with_zoom_and_offset() {
        let vp = ViewportState {
            offset: Point::new(100.0, 50.0),
            zoom: 2.0,
            ..Default::default()
        };
        // screen(100, 50) → scene((100-100)/2, (50-50)/2) = (0, 0)
        let p = screen_to_scene(Point::new(100.0, 50.0), &vp);
        assert!((p.x).abs() < f64::EPSILON);
        assert!((p.y).abs() < f64::EPSILON);
    }

    #[test]
    fn zoom_to_fit_centers_scene() {
        let bbox = BBox::new(500.0, 300.0, 1000.0, 600.0);
        let vp = zoom_to_fit(bbox, 800.0, 600.0);

        // Scene center should map to screen center
        let screen_center = scene_to_screen(Point::new(500.0, 300.0), &vp);
        assert!((screen_center.x - 400.0).abs() < 1.0);
        assert!((screen_center.y - 300.0).abs() < 1.0);
    }

    #[test]
    fn zoom_to_fit_respects_aspect_ratio() {
        // Wide scene in a square screen → zoom limited by width
        let bbox = BBox::new(500.0, 100.0, 1000.0, 200.0);
        let vp = zoom_to_fit(bbox, 400.0, 400.0);
        assert!(vp.zoom < 1.0); // must shrink to fit
    }

    #[test]
    fn zoom_to_fit_caps_at_4x() {
        // Tiny scene in a large screen
        let bbox = BBox::new(5.0, 5.0, 10.0, 10.0);
        let vp = zoom_to_fit(bbox, 2000.0, 2000.0);
        assert!((vp.zoom - 4.0).abs() < f64::EPSILON);
    }

    #[test]
    fn apply_pan() {
        let vp = default_vp();
        let next = apply(&vp, &ViewportAction::Pan { dx: 10.0, dy: -5.0 });
        assert!((next.offset.x - 10.0).abs() < f64::EPSILON);
        assert!((next.offset.y - -5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn apply_zoom_preserves_point_under_cursor() {
        let vp = ViewportState {
            offset: Point::new(50.0, 50.0),
            zoom: 1.0,
            ..Default::default()
        };
        let center = Point::new(200.0, 200.0);
        let scene_before = screen_to_scene(center, &vp);

        let next = apply(
            &vp,
            &ViewportAction::Zoom {
                factor: 2.0,
                center,
            },
        );
        let scene_after = screen_to_scene(center, &next);

        assert!((scene_before.x - scene_after.x).abs() < 1e-10);
        assert!((scene_before.y - scene_after.y).abs() < 1e-10);
    }

    #[test]
    fn apply_zoom_clamps() {
        let vp = ViewportState {
            zoom: 0.15,
            ..Default::default()
        };
        // Zoom down to below 0.1
        let next = apply(
            &vp,
            &ViewportAction::Zoom {
                factor: 0.1,
                center: Point::new(0.0, 0.0),
            },
        );
        assert!((next.zoom - 0.1).abs() < f64::EPSILON);

        // Zoom up past 10.0
        let vp2 = ViewportState {
            zoom: 9.0,
            ..Default::default()
        };
        let next2 = apply(
            &vp2,
            &ViewportAction::Zoom {
                factor: 2.0,
                center: Point::new(0.0, 0.0),
            },
        );
        assert!((next2.zoom - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn apply_hover() {
        let vp = default_vp();
        let next = apply(&vp, &ViewportAction::Hover(Some(ElementId::node("A"))));
        assert_eq!(next.hovered, Some(ElementId::node("A")));

        let next2 = apply(&next, &ViewportAction::Hover(None));
        assert!(next2.hovered.is_none());
    }

    #[test]
    fn apply_select_replaces() {
        let vp = default_vp();
        let next = apply(&vp, &ViewportAction::Select(ElementId::node("A")));
        assert_eq!(next.selected, vec![ElementId::node("A")]);

        let next2 = apply(&next, &ViewportAction::Select(ElementId::node("B")));
        assert_eq!(next2.selected, vec![ElementId::node("B")]);
    }

    #[test]
    fn apply_select_add() {
        let vp = default_vp();
        let next = apply(&vp, &ViewportAction::Select(ElementId::node("A")));
        let next2 = apply(&next, &ViewportAction::SelectAdd(ElementId::node("B")));
        assert_eq!(next2.selected.len(), 2);

        // Adding duplicate is no-op
        let next3 = apply(&next2, &ViewportAction::SelectAdd(ElementId::node("A")));
        assert_eq!(next3.selected.len(), 2);
    }

    #[test]
    fn apply_clear_selection() {
        let mut vp = default_vp();
        vp.selected = vec![ElementId::node("A"), ElementId::node("B")];
        vp.hovered = Some(ElementId::node("A"));

        let next = apply(&vp, &ViewportAction::ClearSelection);
        assert!(next.selected.is_empty());
        assert!(next.hovered.is_none());
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    prop_compose! {
        fn arb_point()(x in -1e4..1e4, y in -1e4..1e4) -> Point {
            Point::new(x, y)
        }
    }

    prop_compose! {
        fn arb_viewport()(
            ox in -1e3..1e3,
            oy in -1e3..1e3,
            zoom in 0.1..10.0_f64,
        ) -> ViewportState {
            ViewportState {
                offset: Point::new(ox, oy),
                zoom,
                ..Default::default()
            }
        }
    }

    proptest! {
        #[test]
        fn roundtrip_transforms(vp in arb_viewport(), pt in arb_point()) {
            let scene = screen_to_scene(pt, &vp);
            let back = scene_to_screen(scene, &vp);
            prop_assert!((back.x - pt.x).abs() < 1e-6, "x: {} vs {}", back.x, pt.x);
            prop_assert!((back.y - pt.y).abs() < 1e-6, "y: {} vs {}", back.y, pt.y);
        }

        #[test]
        fn zoom_preserves_cursor_point(
            vp in arb_viewport(),
            center in arb_point(),
            factor in 0.5..2.0_f64,
        ) {
            let before = screen_to_scene(center, &vp);
            let next = apply(&vp, &ViewportAction::Zoom { factor, center });
            let after = screen_to_scene(center, &next);
            prop_assert!((before.x - after.x).abs() < 1e-6);
            prop_assert!((before.y - after.y).abs() < 1e-6);
        }

        #[test]
        fn pan_then_unpan_is_identity(
            vp in arb_viewport(),
            dx in -500.0..500.0_f64,
            dy in -500.0..500.0_f64,
        ) {
            let panned = apply(&vp, &ViewportAction::Pan { dx, dy });
            let back = apply(&panned, &ViewportAction::Pan { dx: -dx, dy: -dy });
            prop_assert!((back.offset.x - vp.offset.x).abs() < 1e-10);
            prop_assert!((back.offset.y - vp.offset.y).abs() < 1e-10);
        }
    }
}
