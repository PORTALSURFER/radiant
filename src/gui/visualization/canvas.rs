//! Generic retained canvas interaction primitives.

use std::sync::Arc;

use crate::gui::types::{Point, Rect};

/// Paint and input order for a generic layered canvas.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CanvasLayerOrder {
    /// Background or guide layer.
    Background,
    /// Primary content layer.
    Content,
    /// Selection, hover, or edit affordance layer.
    Interaction,
    /// Transient feedback layer.
    Feedback,
    /// Topmost focus or capture layer.
    Focus,
}

/// One retained canvas layer with optional input participation.
#[derive(Clone, Debug, PartialEq)]
pub struct CanvasLayer {
    /// Stable layer identifier.
    pub id: Arc<str>,
    /// Paint and hit-test order.
    pub order: CanvasLayerOrder,
    /// Layer bounds in canvas coordinates.
    pub bounds: Rect,
    /// Whether this layer participates in pointer hit testing.
    pub interactive: bool,
}

impl CanvasLayer {
    /// Build one retained canvas layer.
    pub fn new(
        id: impl Into<String>,
        order: CanvasLayerOrder,
        bounds: Rect,
        interactive: bool,
    ) -> Self {
        Self {
            id: Arc::<str>::from(id.into()),
            order,
            bounds,
            interactive,
        }
    }
}

/// Return the topmost interactive canvas layer containing `point`.
pub fn canvas_layer_at_point(layers: &[CanvasLayer], point: Point) -> Option<&str> {
    layers
        .iter()
        .enumerate()
        .filter(|(_, layer)| layer.interactive && layer.bounds.contains(point))
        .max_by_key(|(index, layer)| (layer.order, *index))
        .map(|(_, layer)| layer.id.as_ref())
}

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
