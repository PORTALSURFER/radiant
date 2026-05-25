use crate::{
    gui::types::{Point, Rect, Rgba8},
    widgets::WidgetId,
};
use std::sync::Arc;

use super::path::{PaintFillRule, PaintPath, PaintTransform};

/// Shared immutable point list used by polygon and polyline paint primitives.
pub type PaintPointList = Arc<[Point]>;

/// Filled rectangle primitive in logical surface coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintFillRect {
    /// Widget that produced this primitive.
    pub widget_id: WidgetId,
    /// Rectangle to fill.
    pub rect: Rect,
    /// Fill color.
    pub color: Rgba8,
}

/// Shared immutable rectangle list used by batched rectangle paint primitives.
pub type PaintRectList = Arc<[Rect]>;

/// Batched filled rectangle primitive in logical surface coordinates.
///
/// Use this when a widget needs to paint many independent rectangles with the
/// same color. Native renderers can encode the batch as one backend shape
/// instead of one scene operation per rectangle.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintFillRectBatch {
    /// Widget that produced this primitive.
    pub widget_id: WidgetId,
    /// Rectangles to fill, in local paint order.
    pub rects: PaintRectList,
    /// Fill color applied to every rectangle.
    pub color: Rgba8,
}

/// Filled bezier path primitive in logical surface coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintFillPath {
    /// Widget or node that produced this primitive.
    pub widget_id: WidgetId,
    /// Path in logical surface coordinates.
    pub path: PaintPath,
    /// Transform applied to the path during rendering.
    pub transform: PaintTransform,
    /// Fill rule used for self-intersecting or nested path regions.
    pub fill_rule: PaintFillRule,
    /// Fill color.
    pub color: Rgba8,
}

/// Stroked rectangle primitive in logical surface coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintStrokeRect {
    /// Widget that produced this primitive.
    pub widget_id: WidgetId,
    /// Rectangle to stroke.
    pub rect: Rect,
    /// Stroke color.
    pub color: Rgba8,
    /// Stroke width in logical pixels.
    pub width: f32,
}

/// Batched stroked rectangle primitive in logical surface coordinates.
///
/// Use this when a widget needs to stroke many independent rectangles with the
/// same color and width. Native renderers can encode the batch as one backend
/// path instead of one scene operation per rectangle.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintStrokeRectBatch {
    /// Widget that produced this primitive.
    pub widget_id: WidgetId,
    /// Rectangles to stroke, in local paint order.
    pub rects: PaintRectList,
    /// Stroke color applied to every rectangle.
    pub color: Rgba8,
    /// Stroke width in logical pixels.
    pub width: f32,
}

/// Filled polygon primitive in logical surface coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintFillPolygon {
    /// Widget or node that produced this primitive.
    pub widget_id: WidgetId,
    /// Polygon points in clockwise or counter-clockwise order.
    pub points: PaintPointList,
    /// Fill color.
    pub color: Rgba8,
}

/// Stroked polygon primitive in logical surface coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintStrokePolygon {
    /// Widget or node that produced this primitive.
    pub widget_id: WidgetId,
    /// Polygon points in clockwise or counter-clockwise order.
    pub points: PaintPointList,
    /// Stroke color.
    pub color: Rgba8,
    /// Stroke width in logical pixels.
    pub width: f32,
}

/// Stroked open polyline primitive in logical surface coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintStrokePolyline {
    /// Widget or node that produced this primitive.
    pub widget_id: WidgetId,
    /// Connected line points in paint order.
    pub points: PaintPointList,
    /// Stroke color.
    pub color: Rgba8,
    /// Stroke width in logical pixels.
    pub width: f32,
}
