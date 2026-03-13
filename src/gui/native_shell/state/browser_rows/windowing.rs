//! Browser-row viewport, scrollbar, and rendered-window helpers.

use super::*;

pub(in crate::gui::native_shell::state) fn row_index_for_visible_rows(
    rows: &[CachedBrowserRow],
    point: Point,
    browser_rows: Rect,
) -> Option<usize> {
    if rows.is_empty() || !browser_rows.contains(point) {
        return None;
    }
    row_index_for_stacked_geometry(rows, point)
}

/// Resolve one browser-row index from stacked row geometry in constant time.
pub(in crate::gui::native_shell::state) fn row_index_for_stacked_geometry(
    rows: &[CachedBrowserRow],
    point: Point,
) -> Option<usize> {
    let first = rows.first()?;
    if point.x < first.rect.min.x || point.x > first.rect.max.x {
        return None;
    }
    let row_height = first.rect.height().max(0.0);
    let stride = if rows.len() > 1 {
        (rows[1].rect.min.y - first.rect.min.y).max(1.0)
    } else {
        row_height.max(1.0)
    };
    let relative_y = point.y - first.rect.min.y;
    if relative_y < 0.0 {
        return None;
    }
    let index = (relative_y / stride).floor() as usize;
    if index >= rows.len() {
        return None;
    }
    let row_start = first.rect.min.y + (index as f32 * stride);
    let row_end = row_start + row_height;
    if index > 0 {
        let previous_end = row_start - stride + row_height;
        if point.y <= previous_end {
            return Some(index - 1);
        }
    }
    ((point.y >= row_start) && (point.y <= row_end)).then_some(index)
}

pub(in crate::gui::native_shell::state) fn browser_rows_cache_key(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    window_start: usize,
) -> BrowserRowsCacheKey {
    let sizing = style.sizing;
    let rows = model.browser.rows.as_slice();
    let row_capacity = browser_rows_capacity(layout.browser_rows, sizing) as u32;
    let content_rect =
        browser_rows_content_rect(layout.browser_rows, model.browser.visible_count, sizing);
    let selected_visible_row = model.browser.selected_visible_row.unwrap_or(usize::MAX);
    let anchor_visible_row = model.browser.anchor_visible_row.unwrap_or(usize::MAX);
    let focused_visible_row = rows
        .iter()
        .find(|row| row.focused)
        .map(|row| row.visible_row as u32)
        .unwrap_or(u32::MAX);
    let selected_visible_hint = rows
        .iter()
        .find(|row| row.selected)
        .map(|row| row.visible_row as u32)
        .unwrap_or(u32::MAX);
    let window_end = (window_start + browser_rows_capacity(layout.browser_rows, sizing))
        .min(model.browser.rows.len());
    let row_text_revision = browser_row_text_revision(&rows[window_start..window_end]);
    BrowserRowsCacheKey {
        root_min_x: f32_to_bits(layout.root.rect.min.x),
        root_min_y: f32_to_bits(layout.root.rect.min.y),
        root_max_x: f32_to_bits(layout.root.rect.max.x),
        root_max_y: f32_to_bits(layout.root.rect.max.y),
        browser_rows_min_x: f32_to_bits(content_rect.min.x),
        browser_rows_min_y: f32_to_bits(content_rect.min.y),
        browser_rows_max_x: f32_to_bits(content_rect.max.x),
        browser_rows_max_y: f32_to_bits(content_rect.max.y),
        browser_row_height: f32_to_bits(sizing.browser_row_height),
        browser_row_gap: f32_to_bits(sizing.browser_row_gap),
        browser_rows_max_per_column: usize_to_u32(sizing.browser_rows_max_per_column),
        row_capacity,
        browser_row_count: rows.len() as u32,
        selected_visible_row: usize_to_u32(selected_visible_row),
        anchor_visible_row: usize_to_u32(anchor_visible_row),
        focused_visible_row,
        selected_visible_hint,
        map_active: model.map.active as u32,
        visible_count: model.browser.visible_count as u32,
        window_start: usize_to_u32(window_start),
        row_text_revision,
        ui_scale: f32_to_bits(layout.ui_scale),
    }
}

pub(in crate::gui::native_shell::state) fn usize_to_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

pub(in crate::gui::native_shell::state) fn f32_to_bits(value: f32) -> u32 {
    value.to_bits()
}

pub(in crate::gui::native_shell::state) fn rendered_browser_rows(
    layout: &ShellLayout,
    model: &AppModel,
    style: &StyleTokens,
) -> Vec<CachedBrowserRow> {
    let mut truncation_cache = BrowserRowTruncationCache::default();
    let mut frame_counts = BrowserRowTruncationFrameCounts::default();
    rendered_browser_rows_cached(
        layout,
        model,
        style,
        &mut truncation_cache,
        &mut frame_counts,
    )
}

/// Build rendered browser rows while reusing a retained truncation cache.
pub(in crate::gui::native_shell::state) fn rendered_browser_rows_cached(
    layout: &ShellLayout,
    model: &AppModel,
    style: &StyleTokens,
    truncation_cache: &mut BrowserRowTruncationCache,
    frame_counts: &mut BrowserRowTruncationFrameCounts,
) -> Vec<CachedBrowserRow> {
    rendered_browser_rows_cached_with_window_start(
        layout,
        model,
        style,
        truncation_cache,
        frame_counts,
    )
    .0
}

/// Build rendered browser rows and return the resolved viewport start used.
pub(in crate::gui::native_shell::state) fn rendered_browser_rows_cached_with_window_start(
    layout: &ShellLayout,
    model: &AppModel,
    style: &StyleTokens,
    truncation_cache: &mut BrowserRowTruncationCache,
    frame_counts: &mut BrowserRowTruncationFrameCounts,
) -> (Vec<CachedBrowserRow>, usize) {
    rendered_browser_rows_cached_with_window_start_and_previous(
        layout,
        model,
        style,
        truncation_cache,
        frame_counts,
        None,
    )
}

/// Build rendered browser rows while preserving a prior visible viewport start.
pub(in crate::gui::native_shell::state) fn rendered_browser_rows_cached_with_window_start_and_previous(
    layout: &ShellLayout,
    model: &AppModel,
    style: &StyleTokens,
    truncation_cache: &mut BrowserRowTruncationCache,
    frame_counts: &mut BrowserRowTruncationFrameCounts,
    previous_visible_start: Option<usize>,
) -> (Vec<CachedBrowserRow>, usize) {
    let sizing = style.sizing;
    if model.map.active || model.browser.rows.is_empty() {
        return (Vec::new(), 0);
    }

    let (window_start, window_end) =
        browser_rows_window_bounds_with_previous(layout, model, sizing, previous_visible_start);
    let window = &model.browser.rows[window_start..window_end];
    let content_rect =
        browser_rows_content_rect(layout.browser_rows, model.browser.visible_count, sizing);
    let row_rects = build_stacked_rows(
        content_rect,
        window.len(),
        sizing.browser_row_gap,
        sizing.browser_row_height,
    );

    let mut rendered = Vec::with_capacity(window.len());
    for (row, rect) in window.iter().zip(row_rects) {
        let row_text_layout = compute_browser_row_text_layout(rect, sizing);
        let rating_reserved_width =
            browser_rating_indicator_reserved_width(row.rating_level, row.locked, sizing);
        let bucket_label_width = browser_inline_tag_max_width(
            row_text_layout.sample_label.width(),
            rating_reserved_width,
        );
        let bucket_label_source = row.bucket_label.clone().unwrap_or_default();
        let bucket_label = if bucket_label_width > 0.0 {
            truncate_browser_row_text_cached(
                truncation_cache,
                frame_counts,
                row.visible_row,
                BrowserRowTextKind::Bucket,
                &bucket_label_source,
                bucket_label_width,
                sizing.font_meta,
            )
        } else {
            String::new()
        };
        let label_width = (row_text_layout.sample_label.width()
            - rating_reserved_width
            - browser_inline_tag_reserved_width(&bucket_label, sizing))
        .max(20.0);
        rendered.push(CachedBrowserRow {
            visible_row: row.visible_row,
            label: truncate_browser_row_text_cached(
                truncation_cache,
                frame_counts,
                row.visible_row,
                BrowserRowTextKind::Sample,
                &row.label,
                label_width,
                sizing.font_body,
            ),
            bucket_label,
            column: row.column.min(2),
            rating_level: row.rating_level.clamp(-3, 3),
            selected: row.selected,
            focused: row.focused,
            missing: row.missing,
            locked: row.locked,
            rect,
        });
    }
    (rendered, window_start)
}

/// Cap inline browser metadata width so the primary sample label keeps most of the row.
fn browser_inline_tag_max_width(sample_width: f32, rating_reserved_width: f32) -> f32 {
    let sample_width = sample_width.max(0.0);
    let available = (sample_width - rating_reserved_width - 24.0).max(0.0);
    available.min((sample_width * 0.38).clamp(44.0, 120.0))
}

pub(in crate::gui::native_shell::state) fn browser_rows_capacity(
    table_rows_rect: Rect,
    sizing: SizingTokens,
) -> usize {
    let row_height = sizing.browser_row_height.max(1.0);
    let row_gap = sizing.browser_row_gap.max(0.0);
    let geometric_capacity = ((table_rows_rect.height() + row_gap) / (row_height + row_gap))
        .floor()
        .max(1.0) as usize;
    geometric_capacity
        .max(1)
        .min(sizing.browser_rows_max_per_column.max(1))
}

/// Resolve the track metrics used by the browser scrollbar lane.
fn browser_scrollbar_track_metrics(sizing: SizingTokens) -> (f32, f32, f32) {
    let track_inset_x = sizing.text_inset_x.clamp(2.0, 6.0);
    let track_inset_y = 0.0;
    let track_width = (sizing.border_width + 4.0).clamp(4.0, 8.0);
    (track_inset_x, track_inset_y, track_width)
}

/// Return the browser-row content rect after reserving the scrollbar lane.
pub(in crate::gui::native_shell::state) fn browser_rows_content_rect(
    browser_rows_rect: Rect,
    visible_count: usize,
    sizing: SizingTokens,
) -> Rect {
    let row_capacity = browser_rows_capacity(browser_rows_rect, sizing);
    if visible_count <= row_capacity {
        return browser_rows_rect;
    }
    let (track_inset_x, _, track_width) = browser_scrollbar_track_metrics(sizing);
    let reserved_width = track_inset_x + track_width + super::BROWSER_SCROLLBAR_CONTENT_GAP;
    let content_max_x = (browser_rows_rect.max.x - reserved_width)
        .round()
        .max(browser_rows_rect.min.x + 1.0);
    Rect::from_min_max(
        browser_rows_rect.min,
        Point::new(content_max_x, browser_rows_rect.max.y),
    )
}

/// Compute visual scrollbar geometry for one overflowing browser row viewport.
pub(in crate::gui::native_shell::state) fn browser_scrollbar_layout(
    browser_rows_rect: Rect,
    rows: &[CachedBrowserRow],
    visible_count: usize,
    sizing: SizingTokens,
) -> Option<BrowserScrollbarLayout> {
    if rows.is_empty() || visible_count <= rows.len() {
        return None;
    }
    let viewport_start = rows
        .first()?
        .visible_row
        .min(visible_count.saturating_sub(1));
    let viewport_len = rows.len().min(visible_count);
    let (track_inset_x, track_inset_y, track_width) = browser_scrollbar_track_metrics(sizing);
    let track_max_x = browser_rows_rect.max.x - track_inset_x;
    let track_min_x = (track_max_x - track_width).max(browser_rows_rect.min.x);
    let track_min_y = (browser_rows_rect.min.y + track_inset_y).min(browser_rows_rect.max.y);
    let track_max_y = (browser_rows_rect.max.y - track_inset_y).max(track_min_y + 1.0);
    let track = Rect::from_min_max(
        Point::new(track_min_x.round(), track_min_y.round()),
        Point::new(track_max_x.round(), track_max_y.round()),
    );
    if track.height() <= 1.0 {
        return None;
    }

    let min_thumb_height = (sizing.browser_row_height * 0.85).round().clamp(18.0, 32.0);
    let thumb_height = (track.height() * (viewport_len as f32 / visible_count as f32))
        .round()
        .clamp(min_thumb_height, track.height());
    let travel = (track.height() - thumb_height).max(0.0);
    let max_viewport_start = visible_count.saturating_sub(viewport_len);
    let start_ratio = if max_viewport_start == 0 {
        0.0
    } else {
        viewport_start.min(max_viewport_start) as f32 / max_viewport_start as f32
    };
    let thumb_min_y = (track.min.y + (travel * start_ratio)).round();
    let thumb_max_y = (thumb_min_y + thumb_height).min(track.max.y);
    let thumb = Rect::from_min_max(
        Point::new(track.min.x, thumb_min_y),
        Point::new(track.max.x, thumb_max_y.max(thumb_min_y + 1.0)),
    );

    Some(BrowserScrollbarLayout { track, thumb })
}

/// Resolve the browser viewport start row for a dragged scrollbar thumb position.
pub(in crate::gui::native_shell::state) fn browser_scrollbar_view_start_for_pointer(
    scrollbar: BrowserScrollbarLayout,
    viewport_len: usize,
    visible_count: usize,
    pointer_y: f32,
    thumb_pointer_offset_y: f32,
) -> Option<usize> {
    if viewport_len == 0 || visible_count <= viewport_len {
        return None;
    }
    let max_viewport_start = visible_count.saturating_sub(viewport_len);
    let thumb_height = scrollbar.thumb.height().max(1.0);
    let travel = (scrollbar.track.height() - thumb_height).max(0.0);
    if travel <= f32::EPSILON || max_viewport_start == 0 {
        return Some(0);
    }
    let thumb_min_y = (pointer_y - thumb_pointer_offset_y)
        .clamp(scrollbar.track.min.y, scrollbar.track.max.y - thumb_height);
    let start_ratio = ((thumb_min_y - scrollbar.track.min.y) / travel).clamp(0.0, 1.0);
    Some(((start_ratio * max_viewport_start as f32).round() as usize).min(max_viewport_start))
}

/// Resolve browser row-window bounds while preserving a prior visible viewport start.
///
/// The host projects a much larger retained browser row slice than the native
/// shell can show at once. When autoscroll is active inside that prewindowed
/// slice, callers can pass the previous visible-row start so edge guards stay
/// anchored to the rows the user is actually looking at instead of snapping
/// back to the host slice start on every focus change.
pub(in crate::gui::native_shell::state) fn browser_rows_window_bounds_with_previous(
    layout: &ShellLayout,
    model: &AppModel,
    sizing: SizingTokens,
    previous_visible_start: Option<usize>,
) -> (usize, usize) {
    if model.map.active || model.browser.rows.is_empty() {
        return (0, 0);
    }
    let window_len = browser_rows_capacity(layout.browser_rows, sizing);
    let window_start = browser_window_start_with_previous(
        &model.browser.rows,
        window_len,
        model.browser.visible_count,
        model.browser.selected_visible_row,
        model.browser.anchor_visible_row,
        model.browser.autoscroll,
        model.browser.view_start_row,
        previous_visible_start,
    );
    let window_end = (window_start + window_len).min(model.browser.rows.len());
    (window_start, window_end)
}

/// Resolve one browser viewport start while preserving a prior visible start.
pub(in crate::gui::native_shell::state) fn browser_window_start_with_previous(
    rows: &[BrowserRowModel],
    window_len: usize,
    _visible_count: usize,
    selected_visible_row: Option<usize>,
    anchor_visible_row: Option<usize>,
    autoscroll: bool,
    view_start_row: usize,
    previous_visible_start: Option<usize>,
) -> usize {
    if rows.len() <= window_len {
        return 0;
    }
    let max_start = rows.len() - window_len;
    let slice_start = rows.first().map(|row| row.visible_row).unwrap_or(0);
    let focus_index = selected_visible_row
        .and_then(|target| rows.iter().position(|row| row.visible_row == target))
        .or_else(|| {
            anchor_visible_row
                .and_then(|target| rows.iter().position(|row| row.visible_row == target))
        })
        .or_else(|| rows.iter().position(|row| row.focused))
        .or_else(|| rows.iter().position(|row| row.selected))
        .unwrap_or(0);
    let projected_window_start = if autoscroll {
        previous_visible_start
            .map(|visible_row| prewindowed_relative_view_start(slice_start, visible_row, max_start))
            .unwrap_or_else(|| {
                prewindowed_relative_view_start(slice_start, view_start_row, max_start)
            })
    } else {
        prewindowed_relative_view_start(slice_start, view_start_row, max_start)
    };
    if !autoscroll {
        return projected_window_start;
    }
    browser_prewindowed_start(focus_index, window_len, max_start, projected_window_start)
}

/// Resolve the authoritative viewport start inside a host-prewindowed browser slice.
fn prewindowed_relative_view_start(
    slice_start: usize,
    view_start_row: usize,
    max_start: usize,
) -> usize {
    view_start_row.saturating_sub(slice_start).min(max_start)
}

/// Resolve one viewport start inside a host-prewindowed browser slice.
fn browser_prewindowed_start(
    focus_index: usize,
    window_len: usize,
    max_start: usize,
    projected_window_start: usize,
) -> usize {
    let edge_margin = super::BROWSER_VIEW_EDGE_MARGIN_ROWS.min(window_len.saturating_sub(1) / 2);
    let mut window_start = projected_window_start.min(max_start);
    let window_end = window_start + window_len;
    let top_guard = window_start + edge_margin;
    let bottom_guard = window_end.saturating_sub(edge_margin);
    if focus_index < top_guard {
        window_start = focus_index.saturating_sub(edge_margin);
    } else if focus_index >= bottom_guard {
        window_start = focus_index
            .saturating_add(edge_margin + 1)
            .saturating_sub(window_len);
    }
    window_start.min(max_start)
}
