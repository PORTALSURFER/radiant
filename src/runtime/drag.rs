//! Runtime-owned pointer drag preview sessions.

use crate::{
    gui::{
        text_layout::{TextWidthEstimate, estimated_text_width_in_range},
        types::{Point, Vector2},
    },
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

/// Text-based sizing configuration for runtime drag previews.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragPreviewTextSizing {
    /// Approximate width assigned to each label character.
    pub character_width: f32,
    /// Horizontal space added around the label.
    pub horizontal_padding: f32,
    /// Minimum preview width.
    pub min_width: f32,
    /// Maximum preview width.
    pub max_width: f32,
    /// Preview height.
    pub height: f32,
}

impl DragPreview {
    /// Build a drag preview from a label and explicit size.
    pub fn sized(label: impl Into<String>, size: Vector2) -> Self {
        Self {
            label: PaintText::from(label.into()),
            size: Vector2::new(size.x.max(1.0), size.y.max(1.0)),
        }
    }

    /// Build a drag preview whose width is estimated from its label text.
    pub fn text_sized(label: impl Into<String>, sizing: DragPreviewTextSizing) -> Self {
        let label = label.into();
        let width = sizing.width_for_label(&label);
        Self::sized(label, Vector2::new(width, sizing.height))
    }
}

impl DragPreviewTextSizing {
    /// Build default text sizing for the given preview height.
    pub const fn new(height: f32) -> Self {
        Self {
            character_width: 7.0,
            horizontal_padding: 28.0,
            min_width: 96.0,
            max_width: 280.0,
            height,
        }
    }

    /// Set the approximate per-character width.
    pub const fn character_width(mut self, width: f32) -> Self {
        self.character_width = width;
        self
    }

    /// Set the horizontal padding.
    pub const fn horizontal_padding(mut self, padding: f32) -> Self {
        self.horizontal_padding = padding;
        self
    }

    /// Set the minimum width.
    pub const fn min_width(mut self, width: f32) -> Self {
        self.min_width = width;
        self
    }

    /// Set the maximum width.
    pub const fn max_width(mut self, width: f32) -> Self {
        self.max_width = width;
        self
    }

    /// Set the preview height.
    pub const fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Estimate the preview width for a label.
    pub fn width_for_label(self, label: &str) -> f32 {
        estimated_text_width_in_range(
            label,
            TextWidthEstimate::new(self.character_width, self.horizontal_padding),
            self.min_width.max(1.0),
            self.max_width,
        )
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
