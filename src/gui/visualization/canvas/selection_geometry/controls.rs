use crate::gui::types::Rect;

use super::{CanvasSelectionGeometry, projection::canvas_selection_rect};
use crate::gui::visualization::canvas::numeric::{finite_non_negative, normalized_fraction};

/// Parameters for projecting an interior selection move/body handle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasSelectionBodyHandleParts {
    /// Canvas bounds containing the normalized selection.
    pub bounds: Rect,
    /// Normalized selection start.
    pub start_fraction: f32,
    /// Normalized selection end.
    pub end_fraction: f32,
    /// Requested handle height from the selection's top edge.
    pub height: f32,
    /// Preferred inset from both horizontal selection edges.
    pub end_inset: f32,
    /// Maximum inset as a fraction of the projected selection width.
    pub max_end_inset_fraction: f32,
    /// Minimum width required after applying the horizontal inset.
    pub min_width_after_inset: f32,
}

pub(super) fn body_handle_rect_for_geometry(
    geometry: CanvasSelectionGeometry,
    height: f32,
    end_inset: f32,
    max_end_inset_fraction: f32,
    min_width_after_inset: f32,
) -> Option<Rect> {
    canvas_selection_body_handle_rect(CanvasSelectionBodyHandleParts {
        bounds: geometry.bounds,
        start_fraction: geometry.start_fraction,
        end_fraction: geometry.end_fraction,
        height,
        end_inset,
        max_end_inset_fraction,
        min_width_after_inset,
    })
}

/// Return the top body-handle rectangle for moving a normalized canvas selection.
///
/// The handle is inset from the selection edges when the projected selection is
/// wide enough, otherwise it falls back to the full selection width. This keeps
/// resize-edge hit targets readable on wider selections without making narrow
/// selections impossible to move.
pub fn canvas_selection_body_handle_rect(parts: CanvasSelectionBodyHandleParts) -> Option<Rect> {
    let selection = canvas_selection_rect(parts.bounds, parts.start_fraction, parts.end_fraction)?;
    let height = finite_non_negative(parts.height).min(selection.height());
    if height <= 0.0 {
        return None;
    }

    let width = selection.width();
    let max_fraction = normalized_fraction(parts.max_end_inset_fraction);
    let inset = finite_non_negative(parts.end_inset).min(width * max_fraction);
    let min_width_after_inset = finite_non_negative(parts.min_width_after_inset);
    let handle = if width > inset * 2.0 + min_width_after_inset {
        selection.inset_horizontal_saturating(inset)
    } else {
        selection
    };
    Some(handle.top_edge_strip(height))
}

pub(super) fn trailing_control_rect_for_geometry(
    geometry: CanvasSelectionGeometry,
    side: f32,
    inset: f32,
) -> Option<Rect> {
    canvas_selection_trailing_control_rect(
        geometry.bounds,
        geometry.start_fraction,
        geometry.end_fraction,
        side,
        inset,
    )
}

/// Return a bottom-trailing control square for a normalized canvas selection.
///
/// Hosts can map this generic rectangle to domain-specific actions such as
/// dragging, exporting, duplicating, or opening selection options.
pub fn canvas_selection_trailing_control_rect(
    bounds: Rect,
    start_fraction: f32,
    end_fraction: f32,
    side: f32,
    inset: f32,
) -> Option<Rect> {
    let selection = canvas_selection_rect(bounds, start_fraction, end_fraction)?;
    let side = finite_non_negative(side);
    if side <= 0.0 {
        return None;
    }
    Some(selection.bottom_right_square(side, finite_non_negative(inset)))
}
