use crate::{
    gui::types::Vector2,
    layout::NodeId,
    runtime::{Command, ScrollFixedRowIntoViewParts, ScrollIntoViewParts},
};

impl<Message> Command<Message> {
    /// Build a command that moves one scroll container to a logical offset.
    pub const fn scroll_to(node_id: NodeId, offset: Vector2) -> Self {
        Self::ScrollTo { node_id, offset }
    }

    /// Build a command that reveals a vertical span inside one scroll container.
    pub const fn scroll_into_view(
        node_id: NodeId,
        target_y: f32,
        target_height: f32,
        margin_top: f32,
        margin_bottom: f32,
    ) -> Self {
        Self::scroll_into_view_from_parts(ScrollIntoViewParts {
            node_id,
            target_y,
            target_height,
            margin_top,
            margin_bottom,
            snap_y: None,
        })
    }

    /// Build a command that reveals a vertical span from named parts.
    pub const fn scroll_into_view_from_parts(parts: ScrollIntoViewParts) -> Self {
        Self::ScrollIntoView {
            node_id: parts.node_id,
            target_y: parts.target_y,
            target_height: parts.target_height,
            margin_top: parts.margin_top,
            margin_bottom: parts.margin_bottom,
            snap_y: parts.snap_y,
        }
    }

    /// Build a command that reveals a vertical span and snaps movement to a fixed row height.
    pub const fn scroll_into_view_snapped(
        node_id: NodeId,
        target_y: f32,
        target_height: f32,
        margin_top: f32,
        margin_bottom: f32,
        snap_y: f32,
    ) -> Self {
        Self::scroll_into_view_from_parts(ScrollIntoViewParts {
            node_id,
            target_y,
            target_height,
            margin_top,
            margin_bottom,
            snap_y: Some(snap_y),
        })
    }

    /// Build a command that reveals a fixed-stride row with directional context rows.
    pub const fn scroll_fixed_row_into_view(
        node_id: NodeId,
        row_index: usize,
        row_stride: f32,
        leading_context_rows: usize,
        trailing_context_rows: usize,
        direction: i32,
    ) -> Self {
        Self::scroll_fixed_row_into_view_from_parts(ScrollFixedRowIntoViewParts {
            node_id,
            row_index,
            row_stride,
            leading_context_rows,
            trailing_context_rows,
            direction,
        })
    }

    /// Build a command that reveals a fixed-stride row from named parts.
    pub const fn scroll_fixed_row_into_view_from_parts(parts: ScrollFixedRowIntoViewParts) -> Self {
        Self::ScrollFixedRowIntoView {
            node_id: parts.node_id,
            row_index: parts.row_index,
            row_stride: parts.row_stride,
            leading_context_rows: parts.leading_context_rows,
            trailing_context_rows: parts.trailing_context_rows,
            direction: parts.direction,
        }
    }
}
