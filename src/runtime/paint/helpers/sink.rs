use super::{push_fill_rect, push_stroke_rect, push_text, push_visible_fill_rect};
use crate::{
    gui::types::{Rect, Rgba8},
    runtime::{PaintPrimitive, PaintTextAlign},
    widgets::WidgetId,
};

/// Paint primitive sink bound to one widget id.
///
/// Custom widgets can use this to append several primitives without passing the
/// same `Vec<PaintPrimitive>` and `WidgetId` through every helper call.
pub struct WidgetPaint<'a> {
    primitives: &'a mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
}

impl<'a> WidgetPaint<'a> {
    /// Create a paint sink for primitives emitted by `widget_id`.
    pub fn new(primitives: &'a mut Vec<PaintPrimitive>, widget_id: WidgetId) -> Self {
        Self {
            primitives,
            widget_id,
        }
    }

    /// Return the widget id attached to primitives emitted by this sink.
    pub const fn widget_id(&self) -> WidgetId {
        self.widget_id
    }

    /// Return the underlying primitive buffer for specialized helpers.
    pub fn primitives_mut(&mut self) -> &mut Vec<PaintPrimitive> {
        self.primitives
    }

    /// Push a filled rectangle into this widget's paint primitive buffer.
    pub fn push_fill_rect(&mut self, rect: Rect, color: Rgba8) {
        push_fill_rect(self.primitives, self.widget_id, rect, color);
    }

    /// Push a filled rectangle when it has finite positive area.
    pub fn push_visible_fill_rect(&mut self, rect: Rect, color: Rgba8) -> bool {
        push_visible_fill_rect(self.primitives, self.widget_id, rect, color)
    }

    /// Push a stroked rectangle into this widget's paint primitive buffer.
    pub fn push_stroke_rect(&mut self, rect: Rect, color: Rgba8, width: f32) {
        push_stroke_rect(self.primitives, self.widget_id, rect, color, width);
    }

    /// Push a compact default single-line text run for this widget.
    pub fn push_text(
        &mut self,
        text: impl Into<String>,
        rect: Rect,
        color: Rgba8,
        align: PaintTextAlign,
    ) {
        push_text(self.primitives, self.widget_id, text, rect, color, align);
    }
}
