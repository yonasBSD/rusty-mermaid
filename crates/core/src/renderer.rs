use crate::Scene;

/// Backend-agnostic rendering trait.
/// SVG, gpui, or any future backend implements this.
/// Static dispatch: callers use `impl Renderer` or generics.
pub trait Renderer {
    type Output;
    fn render(&self, scene: &Scene) -> Self::Output;
}
