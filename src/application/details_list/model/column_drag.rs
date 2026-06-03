use crate::gui::types::Point;
use crate::widgets::DragHandleMessage;

use super::{DetailsColumnPlacement, details_column_reorder_index, reorder_details_columns_by_id};

#[cfg(test)]
#[path = "column_drag/tests.rs"]
mod tests;

/// Active pointer-driven details-column resize state.
#[derive(Clone, Debug, PartialEq)]
pub struct DetailsColumnResizeDrag {
    /// Stable caller-owned column id being resized.
    pub column_id: String,
    /// Pointer x-coordinate when the resize started.
    pub start_x: f32,
    /// Column width when the resize started.
    pub start_width: f32,
}

/// Width update produced by details-column resize drag state.
#[derive(Clone, Debug, PartialEq)]
pub struct DetailsColumnWidthUpdate {
    /// Stable caller-owned column id being resized.
    pub column_id: String,
    /// Clamped column width at the current pointer position.
    pub width: f32,
}

impl DetailsColumnResizeDrag {
    /// Start a details-column resize drag.
    pub fn new(column_id: impl ToString, start_x: f32, start_width: f32) -> Self {
        Self {
            column_id: column_id.to_string(),
            start_x,
            start_width,
        }
    }

    /// Resolve the resized width at the current pointer position.
    pub fn width_at(&self, pointer_x: f32, min_width: f32, max_width: f32) -> f32 {
        let min_width = min_width.max(0.0);
        let max_width = max_width.max(min_width);
        (self.start_width + pointer_x - self.start_x).clamp(min_width, max_width)
    }
}

/// Apply one drag-handle message to details-column resize state.
///
/// Hosts keep the durable column collection and optional active drag state.
/// This helper centralizes the generic resize lifecycle while leaving column
/// lookup and min/max policy in the caller. Pass `Some(current_width)` when
/// the start message references a known column; pass `None` to ignore invalid
/// starts.
pub fn update_details_column_resize_drag(
    active_drag: &mut Option<DetailsColumnResizeDrag>,
    column_id: impl ToString,
    message: DragHandleMessage,
    current_width: Option<f32>,
    min_width: f32,
    max_width: f32,
) -> Option<DetailsColumnWidthUpdate> {
    match message {
        DragHandleMessage::Started { position } => {
            let current_width = current_width?;
            *active_drag = Some(DetailsColumnResizeDrag::new(
                column_id,
                position.x,
                current_width,
            ));
            None
        }
        DragHandleMessage::Moved { position } | DragHandleMessage::Ended { position } => {
            let update = active_drag.as_ref().map(|drag| DetailsColumnWidthUpdate {
                column_id: drag.column_id.clone(),
                width: drag.width_at(position.x, min_width, max_width),
            });
            if message.is_ended() {
                *active_drag = None;
            }
            update
        }
        DragHandleMessage::Cancelled { .. } => {
            *active_drag = None;
            None
        }
        DragHandleMessage::DoubleActivate { .. } => None,
    }
}

/// Active pointer-driven details-column reorder state.
#[derive(Clone, Debug, PartialEq)]
pub struct DetailsColumnReorderDrag {
    /// Stable caller-owned column id being reordered.
    pub column_id: String,
    /// X-coordinate where the first column starts for reorder threshold math.
    pub content_left: f32,
    /// Current pointer position for host-rendered drag feedback.
    pub pointer: Point,
}

impl DetailsColumnReorderDrag {
    /// Start a details-column reorder drag.
    pub fn new(column_id: impl ToString, content_left: f32) -> Self {
        Self {
            column_id: column_id.to_string(),
            content_left,
            pointer: Point::new(0.0, 0.0),
        }
    }

    /// Start a details-column reorder drag with the current pointer position.
    pub fn from_start(column_id: impl ToString, content_left: f32, pointer: Point) -> Self {
        Self {
            column_id: column_id.to_string(),
            content_left,
            pointer,
        }
    }

    /// Resolve the insertion index for the current pointer position.
    pub fn target_index(
        &self,
        placements: &[DetailsColumnPlacement],
        pointer_x: f32,
        column_gap: f32,
    ) -> Option<usize> {
        details_column_reorder_index(
            placements,
            &self.column_id,
            pointer_x,
            self.content_left,
            column_gap,
        )
    }

    /// Resolve the insertion index for the retained current pointer position.
    pub fn current_target_index(
        &self,
        placements: &[DetailsColumnPlacement],
        column_gap: f32,
    ) -> Option<usize> {
        self.target_index(placements, self.pointer.x, column_gap)
    }

    /// Resolve the insertion marker x-coordinate for the current pointer target.
    pub fn current_marker_x(
        &self,
        placements: &[DetailsColumnPlacement],
        column_gap: f32,
    ) -> Option<f32> {
        let target_index = self.current_target_index(placements, column_gap)?;
        Some(details_column_reorder_marker_x(
            placements,
            &self.column_id,
            target_index,
            self.content_left,
            column_gap,
        ))
    }
}

/// Resolve the marker x-coordinate for a details-column insertion target.
pub fn details_column_reorder_marker_x(
    placements: &[DetailsColumnPlacement],
    dragged_id: &str,
    target_index: usize,
    content_left: f32,
    column_gap: f32,
) -> f32 {
    let column_gap = column_gap.max(0.0);
    let target_index = target_index.min(placements.len().saturating_sub(1));
    let mut x = content_left;
    let mut non_dragged_index = 0usize;
    let mut last_column_end = content_left;

    for placement in placements {
        let column_start = x;
        let column_end = column_start + placement.width;
        last_column_end = column_end;
        if placement.id != dragged_id {
            if non_dragged_index == target_index {
                return column_start;
            }
            non_dragged_index += 1;
        }
        x = column_end + column_gap;
    }

    last_column_end
}

/// Estimate the content-left x-coordinate for a details-column drag start.
///
/// Header drag callbacks often expose only pointer coordinates, not full row
/// bounds. Treating the press as if it began from the dragged column midpoint
/// gives stable reorder thresholds without application-local geometry math.
pub fn details_column_drag_content_left(
    placements: &[DetailsColumnPlacement],
    dragged_id: &str,
    start_x: f32,
    column_gap: f32,
) -> Option<f32> {
    let index = placements
        .iter()
        .position(|placement| placement.id == dragged_id)?;
    let prior_width = placements
        .iter()
        .take(index)
        .map(|placement| placement.width + column_gap.max(0.0))
        .sum::<f32>();
    let width = placements.get(index)?.width;
    Some(start_x - prior_width - width * 0.5)
}

/// Apply one drag-handle message to details-column reorder state.
///
/// Hosts pass current placements and the mutable column collection. Radiant
/// owns the generic reorder threshold math and active-drag lifecycle, while
/// the caller decides how app column records expose stable ids.
pub fn update_details_column_reorder_drag<T>(
    active_drag: &mut Option<DetailsColumnReorderDrag>,
    columns: &mut Vec<T>,
    column_id: impl ToString,
    message: DragHandleMessage,
    placements: &[DetailsColumnPlacement],
    column_gap: f32,
    id: impl Fn(&T) -> &str,
) -> bool {
    match message {
        DragHandleMessage::Started { position } => {
            let column_id = column_id.to_string();
            let Some(content_left) =
                details_column_drag_content_left(placements, &column_id, position.x, column_gap)
            else {
                return false;
            };
            *active_drag = Some(DetailsColumnReorderDrag::from_start(
                column_id,
                content_left,
                position,
            ));
            false
        }
        DragHandleMessage::Moved { position } => active_drag.as_mut().is_some_and(|drag| {
            drag.pointer = position;
            false
        }),
        DragHandleMessage::Ended { position } => {
            let changed = active_drag.as_mut().is_some_and(|drag| {
                drag.pointer = position;
                drag.current_target_index(placements, column_gap)
                    .is_some_and(|target_index| {
                        reorder_details_columns_by_id(columns, &drag.column_id, target_index, id)
                    })
            });
            *active_drag = None;
            changed
        }
        DragHandleMessage::Cancelled { .. } => {
            *active_drag = None;
            false
        }
        DragHandleMessage::DoubleActivate { .. } => false,
    }
}
