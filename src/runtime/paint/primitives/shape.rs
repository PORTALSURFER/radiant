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
