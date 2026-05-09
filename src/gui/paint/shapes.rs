use crate::gui::types::{Point, Rect, Rgba8};

/// Filled rectangle draw primitive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FillRect {
    /// Destination rectangle in logical surface coordinates.
    pub rect: Rect,
    /// Fill color.
    pub color: Rgba8,
}

/// Per-edge border ownership for rectangle border emission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BorderSides {
    /// Draw the top edge.
    pub top: bool,
    /// Draw the bottom edge.
    pub bottom: bool,
    /// Draw the left edge.
    pub left: bool,
    /// Draw the right edge.
    pub right: bool,
}

impl BorderSides {
    /// Draw all four edges.
    pub const ALL: Self = Self {
        top: true,
        bottom: true,
        left: true,
        right: true,
    };
}

/// Return filled rectangles that draw the requested border edges.
pub fn border_fill_rects(
    rect: Rect,
    color: Rgba8,
    stroke: f32,
    sides: BorderSides,
) -> Vec<FillRect> {
    let stroke = stroke.max(1.0);
    if rect.width() <= stroke * 2.0 || rect.height() <= stroke * 2.0 {
        return Vec::new();
    }

    let mut fills = Vec::with_capacity(4);
    if sides.top {
        fills.push(FillRect {
            rect: Rect::from_min_max(rect.min, Point::new(rect.max.x, rect.min.y + stroke)),
            color,
        });
    }
    if sides.bottom {
        fills.push(FillRect {
            rect: Rect::from_min_max(Point::new(rect.min.x, rect.max.y - stroke), rect.max),
            color,
        });
    }
    if sides.left {
        fills.push(FillRect {
            rect: Rect::from_min_max(rect.min, Point::new(rect.min.x + stroke, rect.max.y)),
            color,
        });
    }
    if sides.right {
        fills.push(FillRect {
            rect: Rect::from_min_max(Point::new(rect.max.x - stroke, rect.min.y), rect.max),
            color,
        });
    }
    fills
}

/// Filled circle draw primitive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FillCircle {
    /// Circle center in logical surface coordinates.
    pub center: Point,
    /// Circle radius in logical pixels.
    pub radius: f32,
    /// Fill color.
    pub color: Rgba8,
}

/// Filled rectangle draw primitive using a linear gradient in logical surface coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FillLinearGradient {
    /// Destination rectangle for the gradient fill.
    pub rect: Rect,
    /// Gradient start point.
    pub start: Point,
    /// Gradient end point.
    pub end: Point,
    /// Gradient color at `start`.
    pub start_color: Rgba8,
    /// Gradient color at `end`.
    pub end_color: Rgba8,
}
