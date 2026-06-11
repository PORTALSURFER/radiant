use crate::{
    gui::{
        range::{IndexViewportScope, NormalizedRange},
        types::{Point, Rect, Vector2},
    },
    runtime::{PaintPrimitive, push_visible_fill_rect},
    widgets::WidgetId,
};

use super::{
    drag_handle::{DragHandle, DragHandleRole},
    numeric::{finite_non_negative, normalized_fraction},
    selection_affordance::{
        CanvasSelectionAffordanceHitTestParts, CanvasSelectionBodyHandleHitTestParts,
        CanvasSelectionBodyHandlePaintParts, CanvasSelectionEdgeVisualPaintParts,
        CanvasSelectionTrailingControlHitTestParts, CanvasSelectionTrailingControlPaintParts,
    },
};
/// Return a normalized horizontal selection rectangle inside a canvas.
pub fn canvas_selection_rect(bounds: Rect, start_fraction: f32, end_fraction: f32) -> Option<Rect> {
    if !bounds.has_finite_positive_area() {
        return None;
    }
    let start = normalized_fraction(start_fraction);
    let end = normalized_fraction(end_fraction);
    if end <= start {
        return None;
    }
    Some(Rect::from_min_max(
        Point::new(bounds.x_for_ratio(start), bounds.min.y),
        Point::new(bounds.x_for_ratio(end), bounds.max.y),
    ))
}

/// Explicit parts for building reusable normalized canvas selection geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasSelectionGeometryParts {
    /// Canvas bounds containing the normalized selection.
    pub bounds: Rect,
    /// Normalized selection start.
    pub start_fraction: f32,
    /// Normalized selection end.
    pub end_fraction: f32,
}

/// Projected geometry for one horizontal normalized canvas selection.
///
/// The geometry is domain-neutral: hosts decide whether the range represents
/// audio, time, data, or graphics, then map the returned generic affordances to
/// their own actions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasSelectionGeometry {
    /// Canvas bounds containing the normalized selection.
    pub bounds: Rect,
    /// Normalized selection start.
    pub start_fraction: f32,
    /// Normalized selection end.
    pub end_fraction: f32,
    /// Projected selection rectangle.
    pub rect: Rect,
}

impl CanvasSelectionGeometry {
    /// Build projected selection geometry from named parts.
    pub fn from_parts(parts: CanvasSelectionGeometryParts) -> Option<Self> {
        let start_fraction = normalized_fraction(parts.start_fraction);
        let end_fraction = normalized_fraction(parts.end_fraction);
        let rect = canvas_selection_rect(parts.bounds, start_fraction, end_fraction)?;
        Some(Self {
            bounds: parts.bounds,
            start_fraction,
            end_fraction,
            rect,
        })
    }

    /// Build projected selection geometry.
    pub fn new(bounds: Rect, start_fraction: f32, end_fraction: f32) -> Option<Self> {
        Self::from_parts(CanvasSelectionGeometryParts {
            bounds,
            start_fraction,
            end_fraction,
        })
    }

    /// Project an absolute normalized range through an index viewport into
    /// visible canvas selection geometry.
    ///
    /// Returns `None` when the source range is outside the viewport or collapses
    /// after clipping. This is useful for timeline, waveform, strip, and
    /// document-like canvases backed by integer item ranges.
    pub fn from_viewport_range(
        bounds: Rect,
        viewport: IndexViewportScope,
        range: NormalizedRange,
    ) -> Option<Self> {
        let range = viewport.visible_normalized_range(range)?;
        Self::new(bounds, range.start_fraction(), range.end_fraction())
    }

    /// Return the top body-handle rectangle for moving this selection.
    pub fn body_handle_rect(
        self,
        height: f32,
        end_inset: f32,
        max_end_inset_fraction: f32,
        min_width_after_inset: f32,
    ) -> Option<Rect> {
        canvas_selection_body_handle_rect(CanvasSelectionBodyHandleParts {
            bounds: self.bounds,
            start_fraction: self.start_fraction,
            end_fraction: self.end_fraction,
            height,
            end_inset,
            max_end_inset_fraction,
            min_width_after_inset,
        })
    }

    /// Return whether `point` is inside the top body-handle rectangle.
    pub fn body_handle_at_point(
        self,
        height: f32,
        end_inset: f32,
        max_end_inset_fraction: f32,
        min_width_after_inset: f32,
        point: Point,
    ) -> bool {
        self.body_handle_rect(
            height,
            end_inset,
            max_end_inset_fraction,
            min_width_after_inset,
        )
        .is_some_and(|rect| rect.contains(point))
    }

    /// Return whether `point` is inside the configured body/move handle.
    pub fn body_affordance_at_point(self, parts: CanvasSelectionBodyHandleHitTestParts) -> bool {
        self.body_handle_at_point(
            parts.height,
            parts.end_inset,
            parts.max_end_inset_fraction,
            parts.min_width_after_inset,
            parts.point,
        )
    }

    /// Append the top body-handle fill for moving this selection.
    ///
    /// Returns `true` when a visible fill primitive was appended.
    pub fn push_body_handle_fill(
        self,
        primitives: &mut Vec<PaintPrimitive>,
        widget_id: WidgetId,
        parts: CanvasSelectionBodyHandlePaintParts,
    ) -> bool {
        let Some(rect) = self.body_handle_rect(
            parts.height,
            parts.end_inset,
            parts.max_end_inset_fraction,
            parts.min_width_after_inset,
        ) else {
            return false;
        };
        push_visible_fill_rect(primitives, widget_id, rect, parts.color)
    }

    /// Return a bottom-trailing control square for this selection.
    pub fn trailing_control_rect(self, side: f32, inset: f32) -> Option<Rect> {
        canvas_selection_trailing_control_rect(
            self.bounds,
            self.start_fraction,
            self.end_fraction,
            side,
            inset,
        )
    }

    /// Return whether `point` is inside the bottom-trailing control rectangle.
    pub fn trailing_control_at_point(self, side: f32, inset: f32, point: Point) -> bool {
        self.trailing_control_rect(side, inset)
            .is_some_and(|rect| rect.contains(point))
    }

    /// Return whether `point` is inside the configured trailing control.
    pub fn trailing_control_affordance_at_point(
        self,
        parts: CanvasSelectionTrailingControlHitTestParts,
    ) -> bool {
        self.trailing_control_at_point(parts.side, parts.inset, parts.point)
    }

    /// Append the bottom-trailing control fill for this selection.
    ///
    /// Returns `true` when a visible fill primitive was appended.
    pub fn push_trailing_control_fill(
        self,
        primitives: &mut Vec<PaintPrimitive>,
        widget_id: WidgetId,
        parts: CanvasSelectionTrailingControlPaintParts,
    ) -> bool {
        let Some(rect) = self.trailing_control_rect(parts.side, parts.inset) else {
            return false;
        };
        push_visible_fill_rect(primitives, widget_id, rect, parts.color)
    }

    /// Return the visible edge handle for this selection in `bounds`.
    ///
    /// `bounds` may be the full canvas or a vertical strip reserved for edge
    /// affordances.
    pub fn edge_visual_rect(
        self,
        bounds: Rect,
        role: DragHandleRole,
        width: f32,
        vertical_inset: f32,
    ) -> Option<Rect> {
        let fraction = match role {
            DragHandleRole::Start => self.start_fraction,
            DragHandleRole::End => self.end_fraction,
            _ => return None,
        };
        canvas_selection_edge_visual_rect(bounds, fraction, width, vertical_inset)
    }

    /// Append one visible edge-handle fill for this selection.
    ///
    /// Returns `true` when a visible fill primitive was appended.
    pub fn push_edge_visual_fill(
        self,
        primitives: &mut Vec<PaintPrimitive>,
        widget_id: WidgetId,
        parts: CanvasSelectionEdgeVisualPaintParts,
    ) -> bool {
        let Some(rect) =
            self.edge_visual_rect(parts.bounds, parts.role, parts.width, parts.vertical_inset)
        else {
            return false;
        };
        push_visible_fill_rect(primitives, widget_id, rect, parts.color)
    }

    /// Return the first resize edge role containing `point`.
    pub fn edge_at_point(
        self,
        bounds: Rect,
        point: Point,
        width: f32,
        vertical_inset: f32,
    ) -> Option<DragHandleRole> {
        [DragHandleRole::Start, DragHandleRole::End]
            .into_iter()
            .find(|role| {
                self.edge_visual_rect(bounds, *role, width, vertical_inset)
                    .is_some_and(|rect| rect.contains(point))
            })
    }

    /// Return the first configured selection affordance containing `point`.
    ///
    /// Priority matches common editor interaction paint order: trailing
    /// controls first, then resize edges, then the body/move handle. Callers
    /// can omit any affordance group when host-specific controls need to be
    /// checked between these generic layers.
    pub fn affordance_at_point(
        self,
        parts: CanvasSelectionAffordanceHitTestParts,
    ) -> Option<DragHandleRole> {
        if parts
            .trailing_control
            .is_some_and(|trailing| self.trailing_control_affordance_at_point(trailing))
        {
            return Some(DragHandleRole::TrailingControl);
        }
        if let Some(edge) = parts.edge
            && let Some(role) =
                self.edge_at_point(edge.bounds, edge.point, edge.width, edge.vertical_inset)
        {
            return Some(role);
        }
        if parts
            .body
            .is_some_and(|body| self.body_affordance_at_point(body))
        {
            return Some(DragHandleRole::Body);
        }
        None
    }
}

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

/// Return hit-test handles for the start and end edges of a normalized canvas selection.
pub fn canvas_selection_edge_handles(
    bounds: Rect,
    start_fraction: f32,
    end_fraction: f32,
    hit_width: f32,
    capture_token: u64,
) -> Option<[DragHandle; 2]> {
    let selection = canvas_selection_rect(bounds, start_fraction, end_fraction)?;
    let width = finite_non_negative(hit_width);
    if width <= 0.0 {
        return None;
    }
    Some([
        DragHandle::new(
            DragHandleRole::Start,
            Rect::from_min_size(
                Point::new(selection.min.x - width * 0.5, bounds.min.y),
                Vector2::new(width, bounds.height()),
            ),
            capture_token,
        ),
        DragHandle::new(
            DragHandleRole::End,
            Rect::from_min_size(
                Point::new(selection.max.x - width * 0.5, bounds.min.y),
                Vector2::new(width, bounds.height()),
            ),
            capture_token,
        ),
    ])
}

/// Return the visible edge handle for a normalized canvas selection.
pub fn canvas_selection_edge_visual_rect(
    bounds: Rect,
    edge_fraction: f32,
    width: f32,
    vertical_inset: f32,
) -> Option<Rect> {
    if !bounds.has_finite_positive_area() {
        return None;
    }
    let width = finite_non_negative(width);
    let inset = finite_non_negative(vertical_inset).min(bounds.height() * 0.5);
    if width <= 0.0 || bounds.height() - inset * 2.0 <= 0.0 {
        return None;
    }
    let center_x = bounds.x_for_ratio(normalized_fraction(edge_fraction));
    Some(Rect::from_min_size(
        Point::new(center_x - width * 0.5, bounds.min.y + inset),
        Vector2::new(width, bounds.height() - inset * 2.0),
    ))
}
