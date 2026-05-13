use super::{SurfaceNode, UiSurface};
use crate::layout::LayoutNode;

impl<Message> Clone for UiSurface<Message> {
    fn clone(&self) -> Self {
        Self {
            root: self.root.clone(),
        }
    }
}

impl<Message> UiSurface<Message> {
    /// Build a top-level UI surface from one declarative root node.
    pub fn new(root: SurfaceNode<Message>) -> Self {
        Self { root }
    }

    /// Return the root declarative node.
    pub fn root(&self) -> &SurfaceNode<Message> {
        &self.root
    }

    /// Consume the surface and return its root declarative node.
    pub fn into_root(self) -> SurfaceNode<Message> {
        self.root
    }

    /// Project the surface into the public layout tree consumed by layout engines.
    pub fn layout_node(&self) -> LayoutNode {
        self.root.layout_node()
    }
}
