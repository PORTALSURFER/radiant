use crate::{
    gui::types::Vector2,
    layout::NodeId,
    runtime::{Command, ScrollFixedRowIntoViewParts, ScrollIntoViewParts},
    widgets::WidgetId,
};

use super::UiUpdateContext;

impl<Message> UiUpdateContext<Message> {
    /// Move keyboard focus to a widget.
    pub fn focus(&mut self, widget_id: WidgetId) {
        self.queue_command(Command::focus(widget_id));
    }

    /// Clear keyboard focus from any focused widget.
    pub fn clear_focus(&mut self) {
        self.queue_command(Command::clear_focus());
    }

    /// Move one scroll container to a logical offset.
    pub fn scroll_to(&mut self, node_id: NodeId, offset: Vector2) {
        self.queue_command(Command::scroll_to(node_id, offset));
    }

    /// Reveal a vertical span inside one scroll container.
    pub fn scroll_into_view(
        &mut self,
        node_id: NodeId,
        target_y: f32,
        target_height: f32,
        margin_top: f32,
        margin_bottom: f32,
    ) {
        self.queue_command(Command::scroll_into_view(
            node_id,
            target_y,
            target_height,
            margin_top,
            margin_bottom,
        ));
    }

    /// Reveal a vertical span inside one scroll container from named parts.
    pub fn scroll_into_view_from_parts(&mut self, parts: ScrollIntoViewParts) {
        self.queue_command(Command::scroll_into_view_from_parts(parts));
    }

    /// Reveal a vertical span inside one scroll container and snap movement to a fixed row height.
    pub fn scroll_into_view_snapped(
        &mut self,
        node_id: NodeId,
        target_y: f32,
        target_height: f32,
        margin_top: f32,
        margin_bottom: f32,
        snap_y: f32,
    ) {
        self.queue_command(Command::scroll_into_view_snapped(
            node_id,
            target_y,
            target_height,
            margin_top,
            margin_bottom,
            snap_y,
        ));
    }

    /// Reveal a fixed-stride row inside one scroll container from named parts.
    pub fn scroll_fixed_row_into_view_from_parts(&mut self, parts: ScrollFixedRowIntoViewParts) {
        self.queue_command(Command::scroll_fixed_row_into_view_from_parts(parts));
    }

    /// Reveal a fixed-stride row inside one scroll container with directional context rows.
    pub fn scroll_fixed_row_into_view(
        &mut self,
        node_id: NodeId,
        row_index: usize,
        row_stride: f32,
        leading_context_rows: usize,
        trailing_context_rows: usize,
        direction: i32,
    ) {
        self.queue_command(Command::scroll_fixed_row_into_view(
            node_id,
            row_index,
            row_stride,
            leading_context_rows,
            trailing_context_rows,
            direction,
        ));
    }
}
