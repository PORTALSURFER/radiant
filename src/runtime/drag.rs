//! Runtime-owned pointer drag preview sessions.

use crate::{
    gui::types::{Point, Vector2},
    runtime::PaintText,
};

/// Default offset from the pointer to the top-left of a drag preview.
pub const DRAG_PREVIEW_OFFSET: Vector2 = Vector2 { x: 14.0, y: 18.0 };

/// Declarative preview metadata for an active runtime drag.
#[derive(Clone, Debug, PartialEq)]
pub struct DragPreview {
    /// Human-readable label painted inside the preview.
    pub label: PaintText,
    /// Logical preview size.
    pub size: Vector2,
}

impl DragPreview {
    /// Build a drag preview from a label and explicit size.
    pub fn sized(label: impl Into<String>, size: Vector2) -> Self {
        Self {
            label: PaintText::from(label.into()),
            size: Vector2::new(size.x.max(1.0), size.y.max(1.0)),
        }
    }
}

/// Request to start a runtime-owned drag preview.
#[derive(Clone, Debug, PartialEq)]
pub struct DragRequest {
    /// Preview metadata to paint while the drag is active.
    pub preview: DragPreview,
    /// Initial pointer position in surface logical coordinates.
    pub pointer: Point,
}

impl DragRequest {
    /// Build a request from preview metadata and the initial pointer.
    pub fn new(preview: DragPreview, pointer: Point) -> Self {
        Self { preview, pointer }
    }
}

/// Active runtime drag state.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DragSession {
    pub(crate) preview: DragPreview,
    pub(crate) pointer: Point,
    pub(crate) visible: bool,
}

impl DragSession {
    pub(crate) fn new(request: DragRequest) -> Self {
        Self {
            preview: request.preview,
            pointer: request.pointer,
            visible: true,
        }
    }
}
