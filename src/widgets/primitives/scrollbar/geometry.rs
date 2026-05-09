use crate::gui::types::{Point, Rect, Vector2};

use super::ScrollbarAxis;

pub(super) fn axis_length(axis: ScrollbarAxis, rect: Rect) -> f32 {
    match axis {
        ScrollbarAxis::Horizontal => rect.width(),
        ScrollbarAxis::Vertical => rect.height(),
    }
}

pub(super) fn axis_start(axis: ScrollbarAxis, rect: Rect) -> f32 {
    match axis {
        ScrollbarAxis::Horizontal => rect.min.x,
        ScrollbarAxis::Vertical => rect.min.y,
    }
}

pub(super) fn axis_position(axis: ScrollbarAxis, point: Point) -> f32 {
    match axis {
        ScrollbarAxis::Horizontal => point.x,
        ScrollbarAxis::Vertical => point.y,
    }
}

pub(super) fn axis_rect(axis: ScrollbarAxis, bounds: Rect, start: f32, length: f32) -> Rect {
    match axis {
        ScrollbarAxis::Horizontal => Rect::from_min_size(
            Point::new(start, bounds.min.y),
            Vector2::new(length, bounds.height()),
        ),
        ScrollbarAxis::Vertical => Rect::from_min_size(
            Point::new(bounds.min.x, start),
            Vector2::new(bounds.width(), length),
        ),
    }
}
