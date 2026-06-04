use super::sanitize::{finite_nonnegative, normalized_fraction};
use crate::{
    gui::types::{Point, Rect, Rgba8},
    runtime::{PaintPrimitive, WidgetPaint, push_visible_fill_rect},
    widgets::WidgetId,
};

#[cfg(test)]
#[path = "progress/tests.rs"]
mod tests;

/// Return the filled leading segment for a horizontal progress track.
///
/// The returned rect is clamped to `track` and omitted when either the track or
/// the clamped progress fraction has no visible area.
pub fn horizontal_progress_fill_rect(track: Rect, progress_fraction: f32) -> Option<Rect> {
    if !track.has_finite_positive_area() {
        return None;
    }
    let width = track.width() * normalized_fraction(progress_fraction);
    if width <= 0.0 {
        return None;
    }
    Some(Rect::from_min_max(
        track.min,
        Point::new(track.min.x + width.min(track.width()), track.max.y),
    ))
}

/// Return the moving segment used for an indeterminate horizontal progress track.
///
/// `position_fraction` is the normalized travel position for the segment.
/// `segment_fraction` controls the preferred width relative to the track, and
/// `min_segment_width` keeps the activity segment visible on wider tracks.
pub fn horizontal_progress_activity_rect(
    track: Rect,
    position_fraction: f32,
    segment_fraction: f32,
    min_segment_width: f32,
) -> Option<Rect> {
    if !track.has_finite_positive_area() {
        return None;
    }
    let preferred_width = track.width() * normalized_fraction(segment_fraction);
    let segment_width = preferred_width.clamp(
        finite_nonnegative(min_segment_width).min(track.width()),
        track.width(),
    );
    if segment_width <= 0.0 {
        return None;
    }
    let travel = (track.width() - segment_width).max(0.0);
    let min_x = track.min.x + (travel * normalized_fraction(position_fraction));
    Some(Rect::from_min_max(
        Point::new(min_x, track.min.y),
        Point::new((min_x + segment_width).min(track.max.x), track.max.y),
    ))
}

/// Return the visible segment for determinate or indeterminate progress.
///
/// When `total` is zero, the returned segment uses indeterminate activity
/// geometry. Otherwise, `completed / total` resolves the determinate fill.
pub fn horizontal_progress_track_rect(
    track: Rect,
    completed: usize,
    total: usize,
    activity_position_fraction: f32,
    activity_segment_fraction: f32,
    min_activity_segment_width: f32,
) -> Option<Rect> {
    if total == 0 {
        horizontal_progress_activity_rect(
            track,
            activity_position_fraction,
            activity_segment_fraction,
            min_activity_segment_width,
        )
    } else {
        let fraction = (completed as f32 / total as f32).clamp(0.0, 1.0);
        horizontal_progress_fill_rect(track, fraction)
    }
}

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

/// Push a normalized horizontal range segment into a paint primitive buffer.
///
/// Returns `true` when a fill primitive was appended. This combines
/// [`horizontal_value_range_rect`] with Radiant's visible-rect paint guard so
/// custom timeline, waveform, meter, and range widgets do not need local
/// `Option<Rect>` paint boilerplate.
pub fn push_horizontal_value_range_fill(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    track: Rect,
    start_fraction: f32,
    end_fraction: f32,
    height_fraction: f32,
    color: Rgba8,
) -> bool {
    let Some(rect) =
        horizontal_value_range_rect(track, start_fraction, end_fraction, height_fraction)
    else {
        return false;
    };
    push_visible_fill_rect(primitives, widget_id, rect, color)
}

impl WidgetPaint<'_> {
    /// Push a normalized horizontal range segment for this widget.
    ///
    /// This is the [`WidgetPaint`] counterpart to
    /// [`push_horizontal_value_range_fill`].
    pub fn push_horizontal_value_range_fill(
        &mut self,
        track: Rect,
        start_fraction: f32,
        end_fraction: f32,
        height_fraction: f32,
        color: Rgba8,
    ) -> bool {
        let widget_id = self.widget_id();
        push_horizontal_value_range_fill(
            self.primitives_mut(),
            widget_id,
            track,
            start_fraction,
            end_fraction,
            height_fraction,
            color,
        )
    }
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

/// Push top and bottom edge strips for a normalized horizontal range.
///
/// Returns the number of edge fill primitives appended. Degenerate or invalid
/// ranges append no primitives.
pub fn push_horizontal_value_range_edge_fills(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    track: Rect,
    start_fraction: f32,
    end_fraction: f32,
    edge_height: f32,
    color: Rgba8,
) -> usize {
    horizontal_value_range_edge_rects(track, start_fraction, end_fraction, edge_height)
        .into_iter()
        .flatten()
        .filter(|rect| push_visible_fill_rect(primitives, widget_id, *rect, color))
        .count()
}

impl WidgetPaint<'_> {
    /// Push top and bottom edge strips for a normalized horizontal range.
    ///
    /// This is the [`WidgetPaint`] counterpart to
    /// [`push_horizontal_value_range_edge_fills`].
    pub fn push_horizontal_value_range_edge_fills(
        &mut self,
        track: Rect,
        start_fraction: f32,
        end_fraction: f32,
        edge_height: f32,
        color: Rgba8,
    ) -> usize {
        let widget_id = self.widget_id();
        push_horizontal_value_range_edge_fills(
            self.primitives_mut(),
            widget_id,
            track,
            start_fraction,
            end_fraction,
            edge_height,
            color,
        )
    }
}

/// Return a full-height cursor strip centered on a normalized horizontal value.
///
/// The cursor center is snapped to the nearest logical pixel to keep narrow
/// realtime cursors visually stable while progress or playback values move in
/// sub-pixel increments. `cursor_width` is clamped to the track width and never
/// less than one logical pixel.
pub fn horizontal_value_cursor_rect(
    track: Rect,
    value_fraction: f32,
    cursor_width: f32,
) -> Option<Rect> {
    if !track.has_finite_positive_area() {
        return None;
    }
    let width = finite_nonnegative(cursor_width)
        .ceil()
        .clamp(1.0, track.width());
    if width <= 0.0 {
        return None;
    }
    let x = track
        .x_for_ratio(normalized_fraction(value_fraction))
        .round()
        .clamp(track.min.x, track.max.x);
    let left = (x - width * 0.5).clamp(track.min.x, (track.max.x - width).max(track.min.x));
    let right = (left + width).min(track.max.x);
    if right <= left {
        return None;
    }
    Some(Rect::from_min_max(
        Point::new(left, track.min.y),
        Point::new(right, track.max.y),
    ))
}

/// Push a full-height cursor strip centered on a normalized horizontal value.
///
/// Returns `true` when a fill primitive was appended. This is the paint-plan
/// counterpart to [`horizontal_value_cursor_rect`].
pub fn push_horizontal_value_cursor_fill(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    track: Rect,
    value_fraction: f32,
    cursor_width: f32,
    color: Rgba8,
) -> bool {
    let Some(rect) = horizontal_value_cursor_rect(track, value_fraction, cursor_width) else {
        return false;
    };
    push_visible_fill_rect(primitives, widget_id, rect, color)
}

impl WidgetPaint<'_> {
    /// Push a full-height cursor strip centered on a normalized horizontal value.
    ///
    /// This is the [`WidgetPaint`] counterpart to
    /// [`push_horizontal_value_cursor_fill`].
    pub fn push_horizontal_value_cursor_fill(
        &mut self,
        track: Rect,
        value_fraction: f32,
        cursor_width: f32,
        color: Rgba8,
    ) -> bool {
        let widget_id = self.widget_id();
        push_horizontal_value_cursor_fill(
            self.primitives_mut(),
            widget_id,
            track,
            value_fraction,
            cursor_width,
            color,
        )
    }
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
