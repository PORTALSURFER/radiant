use super::super::sanitize::{finite_nonnegative, normalized_fraction};
use crate::gui::types::{Point, Rect};

/// Return a normalized horizontal range segment centered vertically in a track.
///
/// `start_fraction` and `end_fraction` are clamped to the track. `height_fraction`
/// controls how much of the track height the returned segment occupies, centered
/// around the track's vertical midpoint.
pub fn horizontal_value_range_rect(
    track: Rect,
    start_fraction: f32,
    end_fraction: f32,
    height_fraction: f32,
) -> Option<Rect> {
    if !track.has_finite_positive_area() {
        return None;
    }
    let start = normalized_fraction(start_fraction);
    let end = normalized_fraction(end_fraction);
    let height = track.height() * normalized_fraction(height_fraction);
    if end <= start || height <= 0.0 {
        return None;
    }
    let center_y = track.center().y;
    Some(Rect::from_min_max(
        Point::new(track.x_for_ratio(start), center_y - height * 0.5),
        Point::new(track.x_for_ratio(end), center_y + height * 0.5),
    ))
}

/// Return top and bottom edge strips for a normalized horizontal range.
///
/// This is useful for timeline, waveform, and scrubber widgets that need to
/// outline a selected or annotated range without drawing a full rectangle
/// stroke. `edge_height` is clamped to the track height and never less than one
/// logical pixel when the range is visible.
pub fn horizontal_value_range_edge_rects(
    track: Rect,
    start_fraction: f32,
    end_fraction: f32,
    edge_height: f32,
) -> [Option<Rect>; 2] {
    let Some(range) = horizontal_value_range_rect(track, start_fraction, end_fraction, 1.0) else {
        return [None, None];
    };
    let height = finite_nonnegative(edge_height)
        .clamp(1.0, range.height().max(1.0))
        .min(range.height());
    if height <= 0.0 {
        return [None, None];
    }
    [
        Some(range.top_edge_strip(height)),
        Some(range.bottom_edge_strip(height)),
    ]
}

/// Return up to two normalized horizontal segments centered on `center_fraction`.
///
/// The returned array contains the visible segment in index `0` when it does not
/// wrap. Wrapped ranges split into tail and head segments in paint order.
pub fn horizontal_wrapped_value_range_rects(
    track: Rect,
    center_fraction: f32,
    width_fraction: f32,
    height_fraction: f32,
) -> [Option<Rect>; 2] {
    if !track.has_finite_positive_area() {
        return [None, None];
    }
    let width = finite_nonnegative(width_fraction).min(1.0);
    if width <= 0.0 {
        return [None, None];
    }
    let center = wrapped_fraction(center_fraction);
    let start = center - width * 0.5;
    let end = center + width * 0.5;
    if start < 0.0 {
        return [
            horizontal_value_range_rect(track, start + 1.0, 1.0, height_fraction),
            horizontal_value_range_rect(track, 0.0, end, height_fraction),
        ];
    }
    if end > 1.0 {
        return [
            horizontal_value_range_rect(track, start, 1.0, height_fraction),
            horizontal_value_range_rect(track, 0.0, end - 1.0, height_fraction),
        ];
    }
    [
        horizontal_value_range_rect(track, start, end, height_fraction),
        None,
    ]
}

fn wrapped_fraction(value: f32) -> f32 {
    if value.is_finite() {
        value.rem_euclid(1.0)
    } else {
        0.0
    }
}
