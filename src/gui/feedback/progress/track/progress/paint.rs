use crate::{
    gui::types::{Rect, Rgba8},
    runtime::{PaintPrimitive, WidgetPaint, push_visible_fill_rect},
    widgets::WidgetId,
};

use super::{
    cursor::horizontal_value_cursor_rect,
    range::{horizontal_value_range_edge_rects, horizontal_value_range_rect},
    scalar::horizontal_progress_fill_rect,
};

/// Push the filled leading segment for a horizontal progress track.
///
/// Returns `true` when a fill primitive was appended. This combines
/// [`horizontal_progress_fill_rect`] with Radiant's visible-rect paint guard so
/// progress overlays, custom widgets, and feedback tracks do not need local
/// rect construction or primitive boilerplate.
pub fn push_horizontal_progress_fill(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    track: Rect,
    progress_fraction: f32,
    color: Rgba8,
) -> bool {
    let Some(rect) = horizontal_progress_fill_rect(track, progress_fraction) else {
        return false;
    };
    push_visible_fill_rect(primitives, widget_id, rect, color)
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

/// Push full-height cursor strips centered on normalized horizontal values.
///
/// Returns the number of fill primitives appended. Invalid values and empty
/// tracks are skipped using the same geometry and paint guard as
/// [`push_horizontal_value_cursor_fill`].
pub fn push_horizontal_value_cursor_fills(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    track: Rect,
    value_fractions: impl IntoIterator<Item = f32>,
    cursor_width: f32,
    color: Rgba8,
) -> usize {
    value_fractions
        .into_iter()
        .filter(|value_fraction| {
            push_horizontal_value_cursor_fill(
                primitives,
                widget_id,
                track,
                *value_fraction,
                cursor_width,
                color,
            )
        })
        .count()
}

impl WidgetPaint<'_> {
    /// Push a horizontal progress fill for this widget.
    ///
    /// This is the [`WidgetPaint`] counterpart to
    /// [`push_horizontal_progress_fill`].
    pub fn push_horizontal_progress_fill(
        &mut self,
        track: Rect,
        progress_fraction: f32,
        color: Rgba8,
    ) -> bool {
        let widget_id = self.widget_id();
        push_horizontal_progress_fill(
            self.primitives_mut(),
            widget_id,
            track,
            progress_fraction,
            color,
        )
    }

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

    /// Push full-height cursor strips centered on normalized horizontal values.
    ///
    /// This is the [`WidgetPaint`] counterpart to
    /// [`push_horizontal_value_cursor_fills`].
    pub fn push_horizontal_value_cursor_fills(
        &mut self,
        track: Rect,
        value_fractions: impl IntoIterator<Item = f32>,
        cursor_width: f32,
        color: Rgba8,
    ) -> usize {
        let widget_id = self.widget_id();
        push_horizontal_value_cursor_fills(
            self.primitives_mut(),
            widget_id,
            track,
            value_fractions,
            cursor_width,
            color,
        )
    }
}
