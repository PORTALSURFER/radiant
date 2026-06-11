use crate::gui::types::{Point, Rect};
/// Domain-neutral drag handle role for generic timeline and canvas editing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DragHandleRole {
    /// Leading edge of a selected range or shape.
    Start,
    /// Trailing edge of a selected range or shape.
    End,
    /// Interior move handle for an existing selection or shape.
    Body,
    /// Leading auxiliary control.
    LeadingControl,
    /// Trailing auxiliary control.
    TrailingControl,
}

/// One hit-testable drag handle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragHandle {
    /// Semantic role emitted to the host.
    pub role: DragHandleRole,
    /// Handle bounds in canvas coordinates.
    pub rect: Rect,
    /// Stable capture token for backends that keep drag ownership after press.
    pub capture_token: u64,
    /// Whether this handle currently accepts input.
    pub enabled: bool,
}

impl DragHandle {
    /// Build one enabled drag handle.
    pub fn new(role: DragHandleRole, rect: Rect, capture_token: u64) -> Self {
        Self {
            role,
            rect,
            capture_token,
            enabled: true,
        }
    }

    /// Set whether this handle accepts input.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Return the topmost enabled drag handle containing `point`.
pub fn drag_handle_at_point(handles: &[DragHandle], point: Point) -> Option<DragHandle> {
    handles
        .iter()
        .rev()
        .copied()
        .find(|handle| handle.enabled && handle.rect.contains(point))
}
