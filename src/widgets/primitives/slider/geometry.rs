//! Slider layout geometry helpers.

use crate::gui::types::{Point, Rect, Vector2};
use crate::widgets::primitives::support::clamp_fraction;

pub(super) const THUMB_WIDTH: f32 = 12.0;

pub(super) fn value_for_position(bounds: Rect, position: Point, track_height: f32) -> f32 {
    let track = track_rect(bounds, track_height);
    if track.width() <= f32::EPSILON {
        return 0.0;
    }
    clamp_fraction((position.x - track.min.x) / track.width())
}

pub(super) fn track_rect(bounds: Rect, track_height: f32) -> Rect {
    let track_height = if track_height.is_finite() {
        track_height.max(0.0).min(bounds.height())
    } else {
        0.0
    };
    let y = bounds.min.y + (bounds.height() - track_height) * 0.5;
    Rect::from_min_max(
        Point::new(bounds.min.x, y),
        Point::new(bounds.max.x, y + track_height),
    )
}

pub(super) fn thumb_rect(bounds: Rect, value: f32, track_height: f32) -> Rect {
    let track = track_rect(bounds, track_height);
    let x = track.min.x + value * track.width();
    Rect::from_min_size(
        Point::new(x - THUMB_WIDTH * 0.5, bounds.min.y),
        Vector2::new(THUMB_WIDTH, bounds.height()),
    )
}
