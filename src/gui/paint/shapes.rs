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
    let mut fills = Vec::with_capacity(4);
    push_border_fill_rects(&mut fills, rect, color, stroke, sides);
    fills
}

/// Push filled rectangles for the requested border edges into an existing buffer.
pub fn push_border_fill_rects(
    fills: &mut Vec<FillRect>,
    rect: Rect,
    color: Rgba8,
    stroke: f32,
    sides: BorderSides,
) {
    for_each_border_fill_rect(rect, color, stroke, sides, |fill| fills.push(fill));
}

pub(super) fn for_each_border_fill_rect(
    rect: Rect,
    color: Rgba8,
    stroke: f32,
    sides: BorderSides,
    mut emit: impl FnMut(FillRect),
) {
    let stroke = stroke.max(1.0);
    if rect.width() <= stroke * 2.0 || rect.height() <= stroke * 2.0 {
        return;
    }

    if sides.top {
        emit(FillRect {
            rect: rect.top_edge_strip(stroke),
            color,
        });
    }
    if sides.bottom {
        emit(FillRect {
            rect: rect.bottom_edge_strip(stroke),
            color,
        });
    }
    if sides.left {
        emit(FillRect {
            rect: rect.left_edge_strip(stroke),
            color,
        });
    }
    if sides.right {
        emit(FillRect {
            rect: rect.right_edge_strip(stroke),
            color,
        });
    }
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
