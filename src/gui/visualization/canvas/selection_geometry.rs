mod controls;
mod edges;
mod hit_test;
mod paint;
mod projection;

use crate::{
    gui::{
        range::{IndexViewportScope, NormalizedRange},
        types::{Point, Rect},
    },
    runtime::PaintPrimitive,
    widgets::WidgetId,
};

use super::{
    drag_handle::DragHandleRole,
    selection_affordance::{
        CanvasSelectionAffordanceHitTestParts, CanvasSelectionBodyHandleHitTestParts,
        CanvasSelectionBodyHandlePaintParts, CanvasSelectionEdgeVisualPaintParts,
        CanvasSelectionTrailingControlHitTestParts, CanvasSelectionTrailingControlPaintParts,
    },
};

pub use controls::{
    CanvasSelectionBodyHandleParts, canvas_selection_body_handle_rect,
    canvas_selection_trailing_control_rect,
};
pub use edges::{canvas_selection_edge_handles, canvas_selection_edge_visual_rect};
pub use projection::{CanvasSelectionGeometryParts, canvas_selection_rect};

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
        let projection = projection::project_canvas_selection(parts)?;
        Some(Self {
            bounds: projection.bounds,
            start_fraction: projection.start_fraction,
            end_fraction: projection.end_fraction,
            rect: projection.rect,
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
        controls::body_handle_rect_for_geometry(
            self,
            height,
            end_inset,
            max_end_inset_fraction,
            min_width_after_inset,
        )
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
        hit_test::body_handle_at_point(
            self,
            height,
            end_inset,
            max_end_inset_fraction,
            min_width_after_inset,
            point,
        )
    }

    /// Return whether `point` is inside the configured body/move handle.
    pub fn body_affordance_at_point(self, parts: CanvasSelectionBodyHandleHitTestParts) -> bool {
        hit_test::body_affordance_at_point(self, parts)
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
        paint::push_body_handle_fill(self, primitives, widget_id, parts)
    }

    /// Return a bottom-trailing control square for this selection.
    pub fn trailing_control_rect(self, side: f32, inset: f32) -> Option<Rect> {
        controls::trailing_control_rect_for_geometry(self, side, inset)
    }

    /// Return whether `point` is inside the bottom-trailing control rectangle.
    pub fn trailing_control_at_point(self, side: f32, inset: f32, point: Point) -> bool {
        hit_test::trailing_control_at_point(self, side, inset, point)
    }

    /// Return whether `point` is inside the configured trailing control.
    pub fn trailing_control_affordance_at_point(
        self,
        parts: CanvasSelectionTrailingControlHitTestParts,
    ) -> bool {
        hit_test::trailing_control_affordance_at_point(self, parts)
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
        paint::push_trailing_control_fill(self, primitives, widget_id, parts)
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
        edges::edge_visual_rect_for_geometry(self, bounds, role, width, vertical_inset)
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
        paint::push_edge_visual_fill(self, primitives, widget_id, parts)
    }

    /// Return the first resize edge role containing `point`.
    pub fn edge_at_point(
        self,
        bounds: Rect,
        point: Point,
        width: f32,
        vertical_inset: f32,
    ) -> Option<DragHandleRole> {
        hit_test::edge_at_point(self, bounds, point, width, vertical_inset)
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
        hit_test::affordance_at_point(self, parts)
    }
}
