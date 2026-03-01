//! Browser-row caches, windowing, truncation, and row-hit helpers for native shell state.

use super::*;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct CachedBrowserRow {
    pub(super) visible_row: usize,
    pub(super) label: String,
    pub(super) bucket_label: String,
    pub(super) column: usize,
    pub(super) selected: bool,
    pub(super) focused: bool,
    pub(super) rect: Rect,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct SidebarRowsCacheKey {
    pub(super) root_min_x: u32,
    pub(super) root_min_y: u32,
    pub(super) root_max_x: u32,
    pub(super) root_max_y: u32,
    pub(super) sidebar_rows_min_x: u32,
    pub(super) sidebar_rows_min_y: u32,
    pub(super) sidebar_rows_max_x: u32,
    pub(super) sidebar_rows_max_y: u32,
    pub(super) sidebar_section_gap: u32,
    pub(super) panel_section_padding_top: u32,
    pub(super) panel_section_padding_bottom: u32,
    pub(super) source_rows_min_when_split: u32,
    pub(super) folder_rows_min: u32,
    pub(super) source_rows: u32,
    pub(super) folder_rows: u32,
    pub(super) source_row_height: u32,
    pub(super) source_row_gap: u32,
    pub(super) folder_row_height: u32,
    pub(super) folder_row_gap: u32,
    pub(super) folder_header_block_height: u32,
    pub(super) ui_scale: u32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct BrowserRowsCacheKey {
    pub(super) root_min_x: u32,
    pub(super) root_min_y: u32,
    pub(super) root_max_x: u32,
    pub(super) root_max_y: u32,
    pub(super) browser_rows_min_x: u32,
    pub(super) browser_rows_min_y: u32,
    pub(super) browser_rows_max_x: u32,
    pub(super) browser_rows_max_y: u32,
    pub(super) browser_row_height: u32,
    pub(super) browser_row_gap: u32,
    pub(super) browser_rows_max_per_column: u32,
    pub(super) row_capacity: u32,
    pub(super) browser_row_count: u32,
    pub(super) selected_visible_row: u32,
    pub(super) anchor_visible_row: u32,
    pub(super) focused_visible_row: u32,
    pub(super) selected_visible_hint: u32,
    pub(super) map_active: u32,
    pub(super) visible_count: u32,
    pub(super) window_start: u32,
    pub(super) row_text_revision: u64,
    pub(super) ui_scale: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ActionButton {
    pub(super) rect: Rect,
    pub(super) label: &'static str,
    pub(super) enabled: bool,
    pub(super) action: UiAction,
    pub(super) text_color: Rgba8,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct BrowserColumnChip {
    pub(super) rect: Rect,
    pub(super) column: usize,
    pub(super) label: String,
    pub(super) item_count: usize,
    pub(super) selected: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct WaveformToolbarButton {
    pub(super) rect: Rect,
    pub(super) label: &'static str,
    pub(super) enabled: bool,
    pub(super) active: bool,
    pub(super) action: Option<UiAction>,
    pub(super) text_color: Rgba8,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct SidebarSections {
    pub(super) source_rows: Rect,
    pub(super) folder_header: Rect,
    pub(super) folder_rows: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct BrowserToolbarLayout {
    pub(super) search_field: Rect,
    pub(super) activity_chip: Rect,
    pub(super) sort_chip: Rect,
    pub(super) triage_chips: [Rect; 3],
}

#[derive(Clone, Copy, Debug)]
pub(super) struct TopBarControlsLayout {
    pub(super) active: bool,
    pub(super) options_label: Rect,
    pub(super) volume_meter: Rect,
    pub(super) volume_value: Rect,
    pub(super) volume_label: Rect,
}

pub(super) fn format_milli_value(value: u16) -> String {
    format!("{:.3}", f32::from(value.min(1000)) / 1000.0)
}

pub(super) fn volume_action_for_meter(volume_meter: Rect, point: Point) -> UiAction {
    let width = volume_meter.width().max(1.0);
    let clamped_x = point.x.clamp(volume_meter.min.x, volume_meter.max.x);
    let ratio = ((clamped_x - volume_meter.min.x) / width).clamp(0.0, 1.0);
    UiAction::SetVolume {
        value_milli: (ratio * 1000.0).round() as u16,
    }
}

pub(super) fn rendered_source_rows(style: &StyleTokens, model: &AppModel) -> usize {
    model.sources.rows.len().min(style.sizing.source_rows_max)
}

pub(super) fn sidebar_rows_cache_key(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) -> SidebarRowsCacheKey {
    let sizing = style.sizing;
    SidebarRowsCacheKey {
        root_min_x: f32_to_bits(layout.root.rect.min.x),
        root_min_y: f32_to_bits(layout.root.rect.min.y),
        root_max_x: f32_to_bits(layout.root.rect.max.x),
        root_max_y: f32_to_bits(layout.root.rect.max.y),
        sidebar_rows_min_x: f32_to_bits(layout.sidebar_rows.min.x),
        sidebar_rows_min_y: f32_to_bits(layout.sidebar_rows.min.y),
        sidebar_rows_max_x: f32_to_bits(layout.sidebar_rows.max.x),
        sidebar_rows_max_y: f32_to_bits(layout.sidebar_rows.max.y),
        sidebar_section_gap: f32_to_bits(sizing.sidebar_section_gap),
        panel_section_padding_top: f32_to_bits(sizing.panel_section_padding_top),
        panel_section_padding_bottom: f32_to_bits(sizing.panel_section_padding_bottom),
        source_rows_min_when_split: usize_to_u32(sizing.source_rows_min_when_split),
        folder_rows_min: usize_to_u32(sizing.folder_rows_min),
        source_rows: rendered_source_rows(style, model) as u32,
        folder_rows: rendered_folder_rows(style, model) as u32,
        source_row_height: f32_to_bits(sizing.source_row_height),
        source_row_gap: f32_to_bits(sizing.source_row_gap),
        folder_row_height: f32_to_bits(sizing.folder_row_height),
        folder_row_gap: f32_to_bits(sizing.folder_row_gap),
        folder_header_block_height: f32_to_bits(sizing.folder_header_block_height),
        ui_scale: f32_to_bits(layout.ui_scale),
    }
}

pub(super) fn rendered_folder_rows(style: &StyleTokens, model: &AppModel) -> usize {
    model
        .sources
        .folder_rows
        .len()
        .min(style.sizing.folder_rows_max)
}

pub(super) fn rendered_source_row_rects(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) -> Vec<Rect> {
    let sections = sidebar_sections(layout, style, model);
    build_stacked_rows(
        sections.source_rows,
        rendered_source_rows(style, model),
        style.sizing.source_row_gap,
        style.sizing.source_row_height,
    )
}

pub(super) fn rendered_folder_row_rects(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) -> Vec<Rect> {
    let sections = sidebar_sections(layout, style, model);
    build_stacked_rows(
        sections.folder_rows,
        rendered_folder_rows(style, model),
        style.sizing.folder_row_gap,
        style.sizing.folder_row_height,
    )
}

pub(super) fn interaction_wave(pulse_phase: f32) -> f32 {
    ((pulse_phase.sin() + 1.0) * 0.5).clamp(0.0, 1.0)
}

pub(super) fn focus_fill_blend(style: &StyleTokens, motion_wave: f32) -> f32 {
    (style.state_focus_pulse_blend + (motion_wave * style.motion_focus_wave_amp)).clamp(0.0, 1.0)
}

pub(super) fn focus_text_blend(style: &StyleTokens, motion_wave: f32) -> f32 {
    (style.state_focus_pulse_blend + (motion_wave * style.motion_focus_text_wave_amp))
        .clamp(0.0, 1.0)
}

pub(super) fn tinted_overlay_color(color: Rgba8, alpha: f32) -> Rgba8 {
    let alpha = (alpha.clamp(0.0, 1.0) * (color.a as f32 / 255.0) * 255.0)
        .round()
        .clamp(0.0, 255.0);
    Rgba8 {
        r: color.r,
        g: color.g,
        b: color.b,
        a: alpha as u8,
    }
}

pub(super) fn translucent_overlay_color(base: Rgba8, tint: Rgba8, amount: f32) -> Rgba8 {
    let amount = amount.clamp(0.0, 1.0);
    let mut color = blend_color(base, tint, amount);
    color.a = (amount * (base.a as f32 / 255.0) * (tint.a as f32 / 255.0) * 255.0)
        .round()
        .clamp(0.0, 255.0) as u8;
    color
}

/// Return a subtle whitish row-hover fill used across item lists.
pub(super) fn subtle_item_hover_fill(style: &StyleTokens) -> Rgba8 {
    translucent_overlay_color(
        style.bg_tertiary,
        style.text_primary,
        (style.state_hover_soft * 0.8).clamp(0.10, 0.22),
    )
}

pub(super) fn blend_color(a: Rgba8, b: Rgba8, amount: f32) -> Rgba8 {
    let amount = amount.clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| -> u8 {
        ((x as f32) + ((y as f32 - x as f32) * amount))
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Rgba8 {
        r: mix(a.r, b.r),
        g: mix(a.g, b.g),
        b: mix(a.b, b.b),
        a: mix(a.a, b.a),
    }
}

pub(super) fn truncate_to_width(text: &str, max_width: f32, font_size: f32) -> String {
    let max_width = max_width.max(0.0);
    let approx_char_width = (font_size * 0.56).max(1.0);
    let max_chars = (max_width / approx_char_width).floor() as usize;
    if max_chars == 0 {
        return String::new();
    }
    let mut chars = text.chars();
    let mut output = String::with_capacity(max_chars);
    for _ in 0..max_chars {
        match chars.next() {
            Some(ch) => output.push(ch),
            None => return output,
        }
    }
    if chars.next().is_none() {
        return output;
    }
    if max_chars <= 3 {
        return ".".repeat(max_chars);
    }
    let truncated_chars = max_chars.saturating_sub(3);
    let new_len = output
        .char_indices()
        .nth(truncated_chars)
        .map_or(output.len(), |(idx, _)| idx);
    output.truncate(new_len);
    output.push_str("...");
    output
}

pub(super) fn row_index_for_visible_rows(
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
pub(super) fn row_index_for_stacked_geometry(
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

pub(super) fn browser_rows_cache_key(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) -> BrowserRowsCacheKey {
    let sizing = style.sizing;
    let rows = model.browser.rows.as_slice();
    let row_capacity = browser_rows_capacity(layout.browser_rows, sizing) as u32;
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
    let (window_start, window_end) = browser_rows_window_bounds(layout, model, sizing);
    let row_text_revision = browser_row_text_revision(&rows[window_start..window_end]);
    BrowserRowsCacheKey {
        root_min_x: f32_to_bits(layout.root.rect.min.x),
        root_min_y: f32_to_bits(layout.root.rect.min.y),
        root_max_x: f32_to_bits(layout.root.rect.max.x),
        root_max_y: f32_to_bits(layout.root.rect.max.y),
        browser_rows_min_x: f32_to_bits(layout.browser_rows.min.x),
        browser_rows_min_y: f32_to_bits(layout.browser_rows.min.y),
        browser_rows_max_x: f32_to_bits(layout.browser_rows.max.x),
        browser_rows_max_y: f32_to_bits(layout.browser_rows.max.y),
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

/// Build a truncation-cache invalidation key from the current layout/style/row-revision state.
pub(super) fn browser_row_truncation_cache_key(
    layout: &ShellLayout,
    style: &StyleTokens,
    rows_key: BrowserRowsCacheKey,
) -> BrowserRowTruncationCacheKey {
    BrowserRowTruncationCacheKey {
        browser_rows_min_x: f32_to_bits(layout.browser_rows.min.x),
        browser_rows_min_y: f32_to_bits(layout.browser_rows.min.y),
        browser_rows_max_x: f32_to_bits(layout.browser_rows.max.x),
        browser_rows_max_y: f32_to_bits(layout.browser_rows.max.y),
        font_body_bits: f32_to_bits(style.sizing.font_body),
        font_meta_bits: f32_to_bits(style.sizing.font_meta),
        ui_scale: f32_to_bits(layout.ui_scale),
        row_text_revision: rows_key.row_text_revision,
    }
}

pub(super) fn usize_to_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

pub(super) fn f32_to_bits(value: f32) -> u32 {
    value.to_bits()
}

pub(super) fn rendered_browser_rows(
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
pub(super) fn rendered_browser_rows_cached(
    layout: &ShellLayout,
    model: &AppModel,
    style: &StyleTokens,
    truncation_cache: &mut BrowserRowTruncationCache,
    frame_counts: &mut BrowserRowTruncationFrameCounts,
) -> Vec<CachedBrowserRow> {
    let sizing = style.sizing;
    if model.map.active || model.browser.rows.is_empty() {
        return Vec::new();
    }

    let (window_start, window_end) = browser_rows_window_bounds(layout, model, sizing);
    let window = &model.browser.rows[window_start..window_end];
    let row_rects = build_stacked_rows(
        layout.browser_rows,
        window.len(),
        sizing.browser_row_gap,
        sizing.browser_row_height,
    );

    let mut rendered = Vec::with_capacity(window.len());
    for (row, rect) in window.iter().zip(row_rects) {
        let row_text_layout = compute_browser_row_text_layout(rect, sizing);
        let label_width = row_text_layout.sample_label.width().max(20.0);
        let bucket_label_width = row_text_layout.bucket_label.width().max(10.0);
        let bucket_label = row
            .bucket_label
            .clone()
            .unwrap_or_else(|| match row.column {
                0 => String::from("TRASH"),
                2 => String::from("KEEP"),
                _ => String::from("SAMPLE"),
            });
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
            bucket_label: truncate_browser_row_text_cached(
                truncation_cache,
                frame_counts,
                row.visible_row,
                BrowserRowTextKind::Bucket,
                &bucket_label,
                bucket_label_width,
                sizing.font_meta,
            ),
            column: row.column.min(2),
            selected: row.selected,
            focused: row.focused,
            rect,
        });
    }
    rendered
}

/// Resolve one truncated browser-row text string from cache or compute it on miss.
pub(super) fn truncate_browser_row_text_cached(
    truncation_cache: &mut BrowserRowTruncationCache,
    frame_counts: &mut BrowserRowTruncationFrameCounts,
    row_id: usize,
    text_kind: BrowserRowTextKind,
    text: &str,
    max_width: f32,
    font_size: f32,
) -> String {
    let key = BrowserRowTruncationEntryKey {
        row_id: usize_to_u32(row_id),
        width_bucket: truncation_width_bucket(max_width),
        font_size_bucket: truncation_font_size_bucket(font_size),
        text_kind,
    };
    truncation_cache.resolve(key, text, max_width, font_size, frame_counts)
}

/// Quantize truncation width inputs into stable cache buckets.
pub(super) fn truncation_width_bucket(width: f32) -> u16 {
    ((width.max(0.0) * 2.0).round().clamp(0.0, u16::MAX as f32)) as u16
}

/// Quantize truncation font-size inputs into stable cache buckets.
pub(super) fn truncation_font_size_bucket(font_size: f32) -> u16 {
    ((font_size.max(0.0) * 64.0)
        .round()
        .clamp(0.0, u16::MAX as f32)) as u16
}

pub(super) fn browser_rows_capacity(table_rows_rect: Rect, sizing: SizingTokens) -> usize {
    let row_height = sizing.browser_row_height.max(1.0);
    let row_gap = sizing.browser_row_gap.max(0.0);
    let geometric_capacity = ((table_rows_rect.height() + row_gap) / (row_height + row_gap))
        .floor()
        .max(1.0) as usize;
    geometric_capacity
        .max(1)
        .min(sizing.browser_rows_max_per_column.max(1))
}

/// Resolve browser row-window bounds in model-row index space.
pub(super) fn browser_rows_window_bounds(
    layout: &ShellLayout,
    model: &AppModel,
    sizing: SizingTokens,
) -> (usize, usize) {
    if model.map.active || model.browser.rows.is_empty() {
        return (0, 0);
    }
    let window_len = browser_rows_capacity(layout.browser_rows, sizing);
    let window_start = browser_window_start(
        &model.browser.rows,
        window_len,
        model.browser.selected_visible_row,
        model.browser.anchor_visible_row,
    );
    let window_end = (window_start + window_len).min(model.browser.rows.len());
    (window_start, window_end)
}

/// Hash visible browser-row labels into one revision fingerprint.
pub(super) fn browser_row_text_revision(rows: &[BrowserRowModel]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    rows.len().hash(&mut hasher);
    for row in rows {
        row.visible_row.hash(&mut hasher);
        row.label.hash(&mut hasher);
        row.bucket_label.hash(&mut hasher);
        row.column.hash(&mut hasher);
    }
    hasher.finish()
}

pub(super) fn browser_window_start(
    rows: &[BrowserRowModel],
    window_len: usize,
    selected_visible_row: Option<usize>,
    anchor_visible_row: Option<usize>,
) -> usize {
    if rows.len() <= window_len {
        return 0;
    }
    let focus_index = selected_visible_row
        .and_then(|target| rows.iter().position(|row| row.visible_row == target))
        .or_else(|| {
            anchor_visible_row
                .and_then(|target| rows.iter().position(|row| row.visible_row == target))
        })
        .or_else(|| rows.iter().position(|row| row.focused))
        .or_else(|| rows.iter().position(|row| row.selected))
        .unwrap_or(0);
    let half = window_len / 2;
    let max_start = rows.len() - window_len;
    focus_index.saturating_sub(half).min(max_start)
}
