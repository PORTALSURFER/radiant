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

/// Explicit parts used to build one retained canvas layer.
#[derive(Clone, Debug, PartialEq)]
pub struct CanvasLayerParts {
    /// Stable layer identifier.
    pub id: String,
    /// Paint and hit-test order.
    pub order: CanvasLayerOrder,
    /// Layer bounds in canvas coordinates.
    pub bounds: Rect,
    /// Whether this layer participates in pointer hit testing.
    pub interactive: bool,
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
    /// Build one retained canvas layer from named generic parts.
    pub fn from_parts(parts: CanvasLayerParts) -> Self {
        Self {
            id: Arc::<str>::from(parts.id),
            order: parts.order,
            bounds: parts.bounds,
            interactive: parts.interactive,
        }
    }

    /// Build one retained canvas layer.
    pub fn new(
        id: impl Into<String>,
        order: CanvasLayerOrder,
        bounds: Rect,
        interactive: bool,
    ) -> Self {
        Self::from_parts(CanvasLayerParts {
            id: id.into(),
            order,
            bounds,
            interactive,
        })
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
