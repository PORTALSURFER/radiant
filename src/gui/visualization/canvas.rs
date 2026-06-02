//! Generic retained canvas interaction primitives.

use std::sync::Arc;

use crate::{
    gui::types::{Point, Rect, Rgba8, Vector2},
    runtime::{PaintPrimitive, push_visible_fill_rect},
    widgets::WidgetId,
};

/// Paint and input order for a generic layered canvas.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CanvasLayerOrder {
    /// Background or guide layer.
    Background,
    /// Primary content layer.
    Content,
    /// Selection, hover, or edit affordance layer.
    Interaction,
    /// Transient feedback layer.
    Feedback,
    /// Topmost focus or capture layer.
    Focus,
}

/// Explicit parts used to build one retained canvas layer.
#[derive(Clone, Debug, PartialEq)]
pub struct CanvasLayerParts {
    /// Stable layer identifier.
    pub id: String,
    /// Paint and hit-test order.
    pub order: CanvasLayerOrder,
    /// Layer bounds in canvas coordinates.
    pub bounds: Rect,
    /// Whether this layer participates in pointer hit testing.
    pub interactive: bool,
}

/// One retained canvas layer with optional input participation.
#[derive(Clone, Debug, PartialEq)]
pub struct CanvasLayer {
    /// Stable layer identifier.
    pub id: Arc<str>,
    /// Paint and hit-test order.
    pub order: CanvasLayerOrder,
    /// Layer bounds in canvas coordinates.
    pub bounds: Rect,
    /// Whether this layer participates in pointer hit testing.
    pub interactive: bool,
}

impl CanvasLayer {
    /// Build one retained canvas layer from named generic parts.
    pub fn from_parts(parts: CanvasLayerParts) -> Self {
        Self {
            id: Arc::<str>::from(parts.id),
            order: parts.order,
            bounds: parts.bounds,
            interactive: parts.interactive,
        }
    }

    /// Build one retained canvas layer.
    pub fn new(
        id: impl Into<String>,
        order: CanvasLayerOrder,
        bounds: Rect,
        interactive: bool,
    ) -> Self {
        Self::from_parts(CanvasLayerParts {
            id: id.into(),
            order,
            bounds,
            interactive,
        })
    }
}

/// Return the topmost interactive canvas layer containing `point`.
pub fn canvas_layer_at_point(layers: &[CanvasLayer], point: Point) -> Option<&str> {
    layers
        .iter()
        .enumerate()
        .filter(|(_, layer)| layer.interactive && layer.bounds.contains(point))
        .max_by_key(|(index, layer)| (layer.order, *index))
        .map(|(_, layer)| layer.id.as_ref())
}

/// Domain-neutral drag handle role for generic timeline and canvas editing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DragHandleRole {
    /// Leading edge of a selected range or shape.
    Start,
    /// Trailing edge of a selected range or shape.
    End,
    /// Interior move handle for an existing selection or shape.
    Body,
    /// Leading auxiliary control.
    LeadingControl,
    /// Trailing auxiliary control.
    TrailingControl,
}

/// One hit-testable drag handle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragHandle {
    /// Semantic role emitted to the host.
    pub role: DragHandleRole,
    /// Handle bounds in canvas coordinates.
    pub rect: Rect,
    /// Stable capture token for backends that keep drag ownership after press.
    pub capture_token: u64,
    /// Whether this handle currently accepts input.
    pub enabled: bool,
}

impl DragHandle {
    /// Build one enabled drag handle.
    pub fn new(role: DragHandleRole, rect: Rect, capture_token: u64) -> Self {
        Self {
            role,
            rect,
            capture_token,
            enabled: true,
        }
    }

    /// Set whether this handle accepts input.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Return the topmost enabled drag handle containing `point`.
pub fn drag_handle_at_point(handles: &[DragHandle], point: Point) -> Option<DragHandle> {
    handles
        .iter()
        .rev()
        .copied()
        .find(|handle| handle.enabled && handle.rect.contains(point))
}

fn finite_non_negative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn horizontal_resize_edge_width(rect: Rect, requested_width: f32) -> Option<f32> {
    if !rect.has_finite_positive_area() {
        return None;
    }
    let width = finite_non_negative(requested_width).min(rect.width() * 0.5);
    (width > 0.0).then_some(width)
}

/// Return hit-test handles for the horizontal resize edges of a rectangle.
pub fn horizontal_resize_edge_handles(
    rect: Rect,
    edge_width: f32,
    capture_token: u64,
) -> Option<[DragHandle; 2]> {
    let width = horizontal_resize_edge_width(rect, edge_width)?;
    Some([
        DragHandle::new(
            DragHandleRole::Start,
            Rect::from_min_max(rect.min, Point::new(rect.min.x + width, rect.max.y)),
            capture_token,
        ),
        DragHandle::new(
            DragHandleRole::End,
            Rect::from_min_max(Point::new(rect.max.x - width, rect.min.y), rect.max),
            capture_token,
        ),
    ])
}

/// Return body, start-edge, and end-edge handles for a horizontally resizable rectangle.
///
/// Handles are returned in paint-order priority: body first, then start, then
/// end. Passing the result to [`drag_handle_at_point`] gives edges priority
/// over the body when hit targets overlap.
pub fn horizontal_resize_handles(
    rect: Rect,
    edge_width: f32,
    capture_token: u64,
) -> Option<[DragHandle; 3]> {
    let [start, end] = horizontal_resize_edge_handles(rect, edge_width, capture_token)?;
    Some([
        DragHandle::new(DragHandleRole::Body, rect, capture_token),
        start,
        end,
    ])
}

/// Return the visible affordance rectangle for one horizontal resize edge.
pub fn horizontal_resize_edge_visual_rect(
    rect: Rect,
    role: DragHandleRole,
    width: f32,
    edge_inset: f32,
    vertical_inset: f32,
) -> Option<Rect> {
    if !rect.has_finite_positive_area() {
        return None;
    }
    let width = finite_non_negative(width);
    let edge_inset = finite_non_negative(edge_inset);
    let vertical_inset = finite_non_negative(vertical_inset).min(rect.height() * 0.5);
    let visual_height = rect.height() - vertical_inset * 2.0;
    if width <= 0.0 || visual_height <= 0.0 || width + edge_inset > rect.width() {
        return None;
    }
    let (min_x, max_x) = match role {
        DragHandleRole::Start => (rect.min.x + edge_inset, rect.min.x + edge_inset + width),
        DragHandleRole::End => (rect.max.x - edge_inset - width, rect.max.x - edge_inset),
        _ => return None,
    };
    Some(Rect::from_min_max(
        Point::new(min_x, rect.min.y + vertical_inset),
        Point::new(max_x, rect.max.y - vertical_inset),
    ))
}

/// Return a three-rect bracket affordance for one horizontal resize edge.
///
/// The rectangles are returned as the vertical edge stem, top tick, and bottom
/// tick. This shape is useful for editor-style timeline and canvas items where
/// the resize affordance should read as a bracket instead of a plain edge bar.
pub fn horizontal_resize_edge_bracket_rects(
    rect: Rect,
    role: DragHandleRole,
    stroke: f32,
    tick_length: f32,
) -> Option<[Rect; 3]> {
    if !rect.has_finite_positive_area() {
        return None;
    }
    let stroke = finite_non_negative(stroke)
        .min(rect.width())
        .min(rect.height());
    let tick_length = finite_non_negative(tick_length).min(rect.width());
    if stroke <= 0.0 || tick_length <= 0.0 {
        return None;
    }

    let (stem_min_x, tick_min_x, tick_max_x) = match role {
        DragHandleRole::Start => {
            let stem_min_x = rect.min.x;
            (stem_min_x, stem_min_x, stem_min_x + tick_length)
        }
        DragHandleRole::End => {
            let stem_min_x = rect.max.x - stroke;
            (
                stem_min_x,
                stem_min_x + stroke - tick_length,
                stem_min_x + stroke,
            )
        }
        _ => return None,
    };
    Some([
        Rect::from_min_max(
            Point::new(stem_min_x, rect.min.y),
            Point::new(stem_min_x + stroke, rect.max.y),
        ),
        Rect::from_min_max(
            Point::new(tick_min_x, rect.min.y),
            Point::new(tick_max_x, rect.min.y + stroke),
        ),
        Rect::from_min_max(
            Point::new(tick_min_x, rect.max.y - stroke),
            Point::new(tick_max_x, rect.max.y),
        ),
    ])
}

fn normalized_fraction(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

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

/// Retained canvas/timeline invalidation summary.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CanvasInvalidation {
    /// Primary retained content changed.
    pub content_changed: bool,
    /// Layer order, bounds, or hit-test participation changed.
    pub layers_changed: bool,
    /// Pointer capture or focused handle changed.
    pub interaction_changed: bool,
    /// Timeline projection or viewport changed.
    pub projection_changed: bool,
}

impl CanvasInvalidation {
    /// Return whether retained scene content must be rebuilt.
    pub fn requires_scene_rebuild(self) -> bool {
        self.content_changed || self.layers_changed || self.projection_changed
    }

    /// Return whether interaction overlays must be rebuilt.
    pub fn requires_interaction_overlay_rebuild(self) -> bool {
        self.requires_scene_rebuild() || self.interaction_changed
    }
}
