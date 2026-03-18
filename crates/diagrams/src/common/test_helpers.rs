#[cfg(test)]
pub mod test_helpers {
    use rusty_mermaid_core::{Primitive, Scene};

    pub fn count_rects(scene: &Scene) -> usize {
        scene
            .primitives()
            .iter()
            .filter(|p| matches!(p, Primitive::Rect { .. }))
            .count()
    }

    pub fn count_circles(scene: &Scene) -> usize {
        scene
            .primitives()
            .iter()
            .filter(|p| matches!(p, Primitive::Circle { .. }))
            .count()
    }

    pub fn count_polygons(scene: &Scene) -> usize {
        scene
            .primitives()
            .iter()
            .filter(|p| matches!(p, Primitive::Polygon { .. }))
            .count()
    }

    pub fn count_paths(scene: &Scene) -> usize {
        scene
            .primitives()
            .iter()
            .filter(|p| matches!(p, Primitive::Path { .. }))
            .count()
    }

    pub fn count_texts(scene: &Scene) -> usize {
        scene
            .primitives()
            .iter()
            .filter(|p| matches!(p, Primitive::Text { .. }))
            .count()
    }

    pub fn has_text(scene: &Scene, expected: &str) -> bool {
        scene
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Text { content, .. } if content == expected))
    }

    pub fn find_texts(scene: &Scene) -> Vec<&str> {
        scene
            .primitives()
            .iter()
            .filter_map(|p| {
                if let Primitive::Text { content, .. } = p {
                    Some(content.as_str())
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn assert_scene_valid(scene: &Scene) {
        assert!(scene.width > 0.0, "scene width must be positive");
        assert!(scene.height > 0.0, "scene height must be positive");
        assert!(
            !scene.primitives().is_empty(),
            "scene must have primitives"
        );
    }

    pub fn has_rect(scene: &Scene) -> bool {
        scene
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Rect { .. }))
    }

    pub fn has_path(scene: &Scene) -> bool {
        scene
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Path { .. }))
    }

    pub fn has_circle(scene: &Scene) -> bool {
        scene
            .primitives()
            .iter()
            .any(|p| matches!(p, Primitive::Circle { .. }))
    }
}
