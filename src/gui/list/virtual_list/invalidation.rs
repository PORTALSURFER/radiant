/// Retained-list invalidation summary for bounded rebuild decisions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VirtualListInvalidation {
    /// Logical item order, count, or identity changed.
    pub structure_changed: bool,
    /// The materialized viewport/window changed.
    pub window_changed: bool,
    /// One or more materialized item bounds changed.
    pub geometry_changed: bool,
    /// One or more item visual states changed.
    pub item_state_changed: bool,
}

impl VirtualListInvalidation {
    /// Return whether materialized item geometry must be rebuilt.
    pub fn requires_geometry_rebuild(self) -> bool {
        self.structure_changed || self.window_changed || self.geometry_changed
    }

    /// Return whether retained state overlays must be rebuilt.
    pub fn requires_overlay_rebuild(self) -> bool {
        self.requires_geometry_rebuild() || self.item_state_changed
    }
}
