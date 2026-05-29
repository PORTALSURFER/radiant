use super::{DetailsColumnPlacement, details_column_reorder_index};

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

/// Active pointer-driven details-column reorder state.
#[derive(Clone, Debug, PartialEq)]
pub struct DetailsColumnReorderDrag {
    /// Stable caller-owned column id being reordered.
    pub column_id: String,
    /// X-coordinate where the first column starts for reorder threshold math.
    pub content_left: f32,
}

impl DetailsColumnReorderDrag {
    /// Start a details-column reorder drag.
    pub fn new(column_id: impl ToString, content_left: f32) -> Self {
        Self {
            column_id: column_id.to_string(),
            content_left,
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
