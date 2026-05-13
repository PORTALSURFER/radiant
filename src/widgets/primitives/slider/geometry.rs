//! Slider layout geometry helpers.

use crate::gui::types::{Point, Rect, Vector2};
use crate::widgets::primitives::support::clamp_fraction;

pub(super) const TRACK_HEIGHT: f32 = 6.0;
pub(super) const THUMB_WIDTH: f32 = 12.0;

pub(super) fn value_for_position(bounds: Rect, position: Point) -> f32 {
    let track = track_rect(bounds);
    if track.width() <= f32::EPSILON {
        return 0.0;
    }
    clamp_fraction((position.x - track.min.x) / track.width())
}

pub(super) fn track_rect(bounds: Rect) -> Rect {
    let y = bounds.min.y + (bounds.height() - TRACK_HEIGHT) * 0.5;
    let inset = (THUMB_WIDTH * 0.5).min(bounds.width() * 0.5);
    Rect::from_min_max(
        Point::new(bounds.min.x + inset, y),
        Point::new(bounds.max.x - inset, y + TRACK_HEIGHT),
    )
}

pub(super) fn thumb_rect(bounds: Rect, value: f32) -> Rect {
    let track = track_rect(bounds);
    let x = track.min.x + value * track.width();
    Rect::from_min_size(
        Point::new(x - THUMB_WIDTH * 0.5, bounds.min.y),
        Vector2::new(THUMB_WIDTH, bounds.height()),
    )
}
