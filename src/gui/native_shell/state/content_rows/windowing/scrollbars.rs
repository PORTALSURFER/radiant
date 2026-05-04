use super::*;
use crate::gui::list::{
    VirtualListScrollbar, VirtualListScrollbarRequest, VirtualListStackMetrics,
    resolve_virtual_list_scrollbar, virtual_list_scrollbar_view_start_for_pointer,
    virtual_list_viewport_len_for_extent,
};

pub(in crate::gui::native_shell::state) fn content_list_row_capacity(
    list_rect: Rect,
    sizing: SizingTokens,
) -> usize {
    virtual_list_viewport_len_for_extent(
        list_rect.height(),
        VirtualListStackMetrics::new(sizing.browser_row_height, sizing.browser_row_gap)
            .with_max_viewport_len(sizing.browser_rows_max_per_column),
    )
}

/// Resolve the track metrics used by the content-list scrollbar lane.
fn content_list_scrollbar_track_metrics(sizing: SizingTokens) -> (f32, f32, f32) {
    let track_inset_x = sizing.text_inset_x.clamp(2.0, 6.0);
    let track_inset_y = 0.0;
    let track_width = (sizing.border_width + 4.0).clamp(4.0, 8.0);
    (track_inset_x, track_inset_y, track_width)
}

/// Return the content-list rect after reserving the scrollbar lane.
pub(in crate::gui::native_shell::state) fn content_list_content_rect(
    list_rect: Rect,
    visible_count: usize,
    sizing: SizingTokens,
) -> Rect {
    let row_capacity = content_list_row_capacity(list_rect, sizing);
    if visible_count <= row_capacity {
        return list_rect;
    }
    let (track_inset_x, _, track_width) = content_list_scrollbar_track_metrics(sizing);
    let reserved_width = track_inset_x + track_width + super::CONTENT_LIST_SCROLLBAR_CONTENT_GAP;
    let content_max_x = (list_rect.max.x - reserved_width)
        .round()
        .max(list_rect.min.x + 1.0);
    Rect::from_min_max(list_rect.min, Point::new(content_max_x, list_rect.max.y))
}

/// Compute visual scrollbar geometry for one overflowing content-list viewport.
pub(in crate::gui::native_shell::state) fn content_list_scrollbar_layout(
    content_rows_rect: Rect,
    rows: &[CachedContentRow],
    visible_count: usize,
    sizing: SizingTokens,
) -> Option<ContentListScrollbarLayout> {
    if rows.is_empty() || visible_count <= rows.len() {
        return None;
    }
    let viewport_start = rows
        .first()?
        .visible_row
        .min(visible_count.saturating_sub(1));
    let viewport_len = rows.len().min(visible_count);
    let (track_inset_x, track_inset_y, track_width) = content_list_scrollbar_track_metrics(sizing);
    let track_max_x = content_rows_rect.max.x - track_inset_x;
    let track_min_x = (track_max_x - track_width).max(content_rows_rect.min.x);
    let track_min_y = (content_rows_rect.min.y + track_inset_y).min(content_rows_rect.max.y);
    let track_max_y = (content_rows_rect.max.y - track_inset_y).max(track_min_y + 1.0);
    let track = Rect::from_min_max(
        Point::new(track_min_x.round(), track_min_y.round()),
        Point::new(track_max_x.round(), track_max_y.round()),
    );
    if track.height() <= 1.0 {
        return None;
    }

    resolve_virtual_list_scrollbar(VirtualListScrollbarRequest {
        track,
        total_items: visible_count,
        viewport_len,
        viewport_start,
        min_thumb_extent: (sizing.browser_row_height * 0.85).round().clamp(18.0, 32.0),
    })
    .map(|scrollbar| ContentListScrollbarLayout {
        track: scrollbar.track,
        thumb: scrollbar.thumb,
    })
}

/// Resolve the content-list viewport start row for a dragged scrollbar thumb position.
pub(in crate::gui::native_shell::state) fn content_list_scrollbar_view_start_for_pointer(
    scrollbar: ContentListScrollbarLayout,
    viewport_len: usize,
    visible_count: usize,
    pointer_y: f32,
    thumb_pointer_offset_y: f32,
) -> Option<usize> {
    virtual_list_scrollbar_view_start_for_pointer(
        VirtualListScrollbar {
            track: scrollbar.track,
            thumb: scrollbar.thumb,
        },
        viewport_len,
        visible_count,
        pointer_y,
        thumb_pointer_offset_y,
    )
}
