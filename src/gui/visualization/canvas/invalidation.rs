/// Retained canvas/timeline invalidation summary.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CanvasInvalidation {
    /// Primary retained content changed.
    pub content_changed: bool,
    /// Layer order, bounds, or hit-test participation changed.
    pub layers_changed: bool,
    /// Pointer capture or focused handle changed.
    pub interaction_changed: bool,
    /// Timeline projection or viewport changed.
    pub projection_changed: bool,
}

impl CanvasInvalidation {
    /// Return whether retained scene content must be rebuilt.
    pub fn requires_scene_rebuild(self) -> bool {
        self.content_changed || self.layers_changed || self.projection_changed
    }

    /// Return whether interaction overlays must be rebuilt.
    pub fn requires_interaction_overlay_rebuild(self) -> bool {
        self.requires_scene_rebuild() || self.interaction_changed
    }
}
