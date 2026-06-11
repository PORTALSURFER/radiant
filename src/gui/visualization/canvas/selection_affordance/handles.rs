use crate::gui::types::{Point, Rect, Rgba8};

use crate::gui::visualization::canvas::DragHandleRole;
/// Hit-test policy for a normalized selection body/move handle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasSelectionBodyHandleHitTestParts {
    /// Pointer position in canvas coordinates.
    pub point: Point,
    /// Requested handle height from the selection's top edge.
    pub height: f32,
    /// Preferred inset from both horizontal selection edges.
    pub end_inset: f32,
    /// Maximum inset as a fraction of the projected selection width.
    pub max_end_inset_fraction: f32,
    /// Minimum width required after applying the horizontal inset.
    pub min_width_after_inset: f32,
}

impl CanvasSelectionBodyHandleHitTestParts {
    /// Build body/move handle hit-test parts.
    pub const fn new(
        point: Point,
        height: f32,
        end_inset: f32,
        max_end_inset_fraction: f32,
        min_width_after_inset: f32,
    ) -> Self {
        Self {
            point,
            height,
            end_inset,
            max_end_inset_fraction,
            min_width_after_inset,
        }
    }
}

/// Reusable body/move-handle dimensions for a normalized canvas selection.
///
/// Use this when hit testing and painting the same selection affordance should
/// share one dimension policy instead of duplicating constants in separate
/// input and paint paths.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasSelectionBodyHandleStyle {
    /// Requested handle height from the selection's top edge.
    pub height: f32,
    /// Preferred inset from both horizontal selection edges.
    pub end_inset: f32,
    /// Maximum inset as a fraction of the projected selection width.
    pub max_end_inset_fraction: f32,
    /// Minimum width required after applying the horizontal inset.
    pub min_width_after_inset: f32,
}

impl CanvasSelectionBodyHandleStyle {
    /// Build a body/move-handle style.
    pub const fn new(
        height: f32,
        end_inset: f32,
        max_end_inset_fraction: f32,
        min_width_after_inset: f32,
    ) -> Self {
        Self {
            height,
            end_inset,
            max_end_inset_fraction,
            min_width_after_inset,
        }
    }

    /// Build matching body/move-handle hit-test parts.
    pub const fn hit_test_parts(self, point: Point) -> CanvasSelectionBodyHandleHitTestParts {
        CanvasSelectionBodyHandleHitTestParts::new(
            point,
            self.height,
            self.end_inset,
            self.max_end_inset_fraction,
            self.min_width_after_inset,
        )
    }

    /// Build matching body/move-handle paint parts.
    pub const fn paint_parts(self, color: Rgba8) -> CanvasSelectionBodyHandlePaintParts {
        CanvasSelectionBodyHandlePaintParts::new(
            self.height,
            self.end_inset,
            self.max_end_inset_fraction,
            self.min_width_after_inset,
            color,
        )
    }
}

/// Hit-test policy for normalized selection resize edges.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasSelectionEdgeHitTestParts {
    /// Bounds containing the edge visuals.
    pub bounds: Rect,
    /// Pointer position in canvas coordinates.
    pub point: Point,
    /// Requested edge width.
    pub width: f32,
    /// Vertical inset from `bounds`.
    pub vertical_inset: f32,
}

impl CanvasSelectionEdgeHitTestParts {
    /// Build resize-edge hit-test parts.
    pub const fn new(bounds: Rect, point: Point, width: f32, vertical_inset: f32) -> Self {
        Self {
            bounds,
            point,
            width,
            vertical_inset,
        }
    }
}

/// Reusable resize-edge dimensions for a normalized canvas selection.
///
/// The style is domain-neutral; callers still provide the vertical strip bounds
/// used by their canvas and map the returned edge roles into app commands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasSelectionEdgeVisualStyle {
    /// Requested visual width.
    pub width: f32,
    /// Vertical inset from the supplied bounds.
    pub vertical_inset: f32,
}

impl CanvasSelectionEdgeVisualStyle {
    /// Build a resize-edge style.
    pub const fn new(width: f32, vertical_inset: f32) -> Self {
        Self {
            width,
            vertical_inset,
        }
    }

    /// Build matching resize-edge hit-test parts.
    pub const fn hit_test_parts(
        self,
        bounds: Rect,
        point: Point,
    ) -> CanvasSelectionEdgeHitTestParts {
        CanvasSelectionEdgeHitTestParts::new(bounds, point, self.width, self.vertical_inset)
    }

    /// Build matching resize-edge paint parts.
    pub const fn paint_parts(
        self,
        bounds: Rect,
        role: DragHandleRole,
        color: Rgba8,
    ) -> CanvasSelectionEdgeVisualPaintParts {
        CanvasSelectionEdgeVisualPaintParts::new(
            bounds,
            role,
            self.width,
            self.vertical_inset,
            color,
        )
    }
}

/// Hit-test policy for a normalized selection trailing control.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasSelectionTrailingControlHitTestParts {
    /// Pointer position in canvas coordinates.
    pub point: Point,
    /// Requested square side length.
    pub side: f32,
    /// Inset from the selection's bottom-right corner.
    pub inset: f32,
}

impl CanvasSelectionTrailingControlHitTestParts {
    /// Build trailing-control hit-test parts.
    pub const fn new(point: Point, side: f32, inset: f32) -> Self {
        Self { point, side, inset }
    }
}

/// Reusable trailing-control dimensions for a normalized canvas selection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasSelectionTrailingControlStyle {
    /// Requested square side length.
    pub side: f32,
    /// Inset from the selection's bottom-right corner.
    pub inset: f32,
}

impl CanvasSelectionTrailingControlStyle {
    /// Build a trailing-control style.
    pub const fn new(side: f32, inset: f32) -> Self {
        Self { side, inset }
    }

    /// Build matching trailing-control hit-test parts.
    pub const fn hit_test_parts(self, point: Point) -> CanvasSelectionTrailingControlHitTestParts {
        CanvasSelectionTrailingControlHitTestParts::new(point, self.side, self.inset)
    }

    /// Build matching trailing-control paint parts.
    pub const fn paint_parts(self, color: Rgba8) -> CanvasSelectionTrailingControlPaintParts {
        CanvasSelectionTrailingControlPaintParts::new(self.side, self.inset, color)
    }
}

/// Paint policy for a normalized selection body/move handle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasSelectionBodyHandlePaintParts {
    /// Requested handle height from the selection's top edge.
    pub height: f32,
    /// Preferred inset from both horizontal selection edges.
    pub end_inset: f32,
    /// Maximum inset as a fraction of the projected selection width.
    pub max_end_inset_fraction: f32,
    /// Minimum width required after applying the horizontal inset.
    pub min_width_after_inset: f32,
    /// Fill color.
    pub color: Rgba8,
}

impl CanvasSelectionBodyHandlePaintParts {
    /// Build selection body-handle paint parts.
    pub const fn new(
        height: f32,
        end_inset: f32,
        max_end_inset_fraction: f32,
        min_width_after_inset: f32,
        color: Rgba8,
    ) -> Self {
        Self {
            height,
            end_inset,
            max_end_inset_fraction,
            min_width_after_inset,
            color,
        }
    }
}

/// Paint policy for a normalized selection trailing control.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasSelectionTrailingControlPaintParts {
    /// Requested square side length.
    pub side: f32,
    /// Inset from the selection's bottom-right corner.
    pub inset: f32,
    /// Fill color.
    pub color: Rgba8,
}

impl CanvasSelectionTrailingControlPaintParts {
    /// Build selection trailing-control paint parts.
    pub const fn new(side: f32, inset: f32, color: Rgba8) -> Self {
        Self { side, inset, color }
    }
}

/// Paint policy for a normalized selection edge visual.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasSelectionEdgeVisualPaintParts {
    /// Bounds containing the edge visual.
    pub bounds: Rect,
    /// Edge role to paint.
    pub role: DragHandleRole,
    /// Requested visual width.
    pub width: f32,
    /// Vertical inset from the supplied bounds.
    pub vertical_inset: f32,
    /// Fill color.
    pub color: Rgba8,
}

impl CanvasSelectionEdgeVisualPaintParts {
    /// Build selection edge-visual paint parts.
    pub const fn new(
        bounds: Rect,
        role: DragHandleRole,
        width: f32,
        vertical_inset: f32,
        color: Rgba8,
    ) -> Self {
        Self {
            bounds,
            role,
            width,
            vertical_inset,
            color,
        }
    }
}
