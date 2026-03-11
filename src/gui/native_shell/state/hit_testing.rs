//! Hit-testing, hover resolution, and pointer-geometry helpers for native shell state.

use super::*;

pub(super) fn browser_action_hit_test_cache_key(
    layout: &ShellLayout,
    model: &AppModel,
) -> BrowserActionHitTestCacheKey {
    BrowserActionHitTestCacheKey {
        browser_toolbar_min_x: f32_to_bits(layout.browser_toolbar.min.x),
        browser_toolbar_min_y: f32_to_bits(layout.browser_toolbar.min.y),
        browser_toolbar_max_x: f32_to_bits(layout.browser_toolbar.max.x),
        browser_toolbar_max_y: f32_to_bits(layout.browser_toolbar.max.y),
        ui_scale: f32_to_bits(layout.ui_scale),
        model_signature: browser_action_model_signature(model),
    }
}

pub(super) fn browser_action_model_signature(model: &AppModel) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    model.browser_actions.can_rename.hash(&mut hasher);
    model.browser_actions.can_tag.hash(&mut hasher);
    model.browser_actions.can_delete.hash(&mut hasher);
    model.browser.active_rating_filters.hash(&mut hasher);
    model.selected_column.min(2).hash(&mut hasher);
    for index in 0..3 {
        if let Some(column) = model.columns.get(index) {
            column.title.hash(&mut hasher);
            column.item_count.hash(&mut hasher);
        } else {
            index.hash(&mut hasher);
        }
    }
    hasher.finish()
}

pub(super) fn waveform_toolbar_hit_test_cache_key(
    layout: &ShellLayout,
    model: &NativeMotionModel,
    bpm_editor_active: bool,
    bpm_editor_display: Option<&str>,
) -> WaveformToolbarHitTestCacheKey {
    WaveformToolbarHitTestCacheKey {
        waveform_header_min_x: f32_to_bits(layout.waveform_header.min.x),
        waveform_header_min_y: f32_to_bits(layout.waveform_header.min.y),
        waveform_header_max_x: f32_to_bits(layout.waveform_header.max.x),
        waveform_header_max_y: f32_to_bits(layout.waveform_header.max.y),
        ui_scale: f32_to_bits(layout.ui_scale),
        model_flags: waveform_toolbar_model_flags(model),
        tempo_label_signature: waveform_tempo_label_signature(model),
        bpm_editor_active,
        bpm_editor_display_signature: text_signature(bpm_editor_display),
    }
}

pub(super) fn waveform_toolbar_model_flags(model: &NativeMotionModel) -> u16 {
    let mut bits = 0u16;
    if model.waveform_channel_view == crate::app::WaveformChannelViewModel::Stereo {
        bits |= 1 << 0;
    }
    if model.waveform_normalized_audition_enabled {
        bits |= 1 << 1;
    }
    if model.waveform_bpm_snap_enabled {
        bits |= 1 << 2;
    }
    if model.waveform_transient_snap_enabled {
        bits |= 1 << 3;
    }
    if model.waveform_transient_markers_enabled {
        bits |= 1 << 4;
    }
    if model.waveform_slice_mode_enabled {
        bits |= 1 << 5;
    }
    if model.waveform_loop_enabled {
        bits |= 1 << 6;
    }
    if model.transport_running {
        bits |= 1 << 7;
    }
    bits
}

pub(super) fn waveform_tempo_label_signature(model: &NativeMotionModel) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    model.waveform_tempo_label.hash(&mut hasher);
    hasher.finish()
}

pub(super) fn text_signature(value: Option<&str>) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

pub(super) fn waveform_toolbar_hover_hint(label: &str) -> Option<WaveformToolbarHoverHint> {
    match label {
        "Channel" => Some(WaveformToolbarHoverHint::ChannelView),
        "Norm" => Some(WaveformToolbarHoverHint::NormalizedAudition),
        "BPM Value" => Some(WaveformToolbarHoverHint::BpmValue),
        "BPM Snap" => Some(WaveformToolbarHoverHint::BpmSnap),
        "Tr Snap" => Some(WaveformToolbarHoverHint::TransientSnap),
        "Show Tr" => Some(WaveformToolbarHoverHint::ShowTransients),
        "Slice" => Some(WaveformToolbarHoverHint::SliceMode),
        "Loop" => Some(WaveformToolbarHoverHint::Loop),
        "Stop" => Some(WaveformToolbarHoverHint::Stop),
        "Play" => Some(WaveformToolbarHoverHint::Play),
        "Rec" => Some(WaveformToolbarHoverHint::Record),
        _ => None,
    }
}

/// Return hovered waveform marker x-position for one pointer point.
pub(super) fn waveform_hover_x_for_point(
    layout: &ShellLayout,
    hover: Option<ShellNodeKind>,
    point: Point,
) -> Option<f32> {
    if hover != Some(ShellNodeKind::WaveformCard) || !layout.waveform_plot.contains(point) {
        return None;
    }
    Some(
        point
            .x
            .clamp(layout.waveform_plot.min.x, layout.waveform_plot.max.x)
            .round(),
    )
}

/// Return the hovered waveform resize-edge target for one pointer point.
pub(super) fn hovered_waveform_resize_edge_for_point(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    hover: Option<ShellNodeKind>,
) -> Option<WaveformResizeHoverEdge> {
    if hover != Some(ShellNodeKind::WaveformCard) || !layout.waveform_plot.contains(point) {
        return None;
    }
    hovered_resize_edge_for_range(layout, model, point, model.waveform.edit_selection_milli)
        .map(|left_edge| {
            if left_edge {
                WaveformResizeHoverEdge::EditSelectionStart
            } else {
                WaveformResizeHoverEdge::EditSelectionEnd
            }
        })
        .or_else(|| {
            hovered_resize_edge_for_range(layout, model, point, model.waveform.selection_milli).map(
                |left_edge| {
                    if left_edge {
                        WaveformResizeHoverEdge::SelectionStart
                    } else {
                        WaveformResizeHoverEdge::SelectionEnd
                    }
                },
            )
        })
}

/// Return whether the pointer is hovering the start (`true`) or end (`false`) edge of one range.
pub(super) fn hovered_resize_edge_for_range(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    range: Option<crate::app::NormalizedRangeModel>,
) -> Option<bool> {
    let range = range?;
    let start_micros = range.start_micros.min(range.end_micros);
    let end_micros = range.start_micros.max(range.end_micros);
    if end_micros <= start_micros {
        return None;
    }
    let (handle_top, handle_bottom) = waveform_centered_resize_edge_y_bounds(layout.waveform_plot);
    if point.y < handle_top || point.y > handle_bottom {
        return None;
    }
    let start_x = waveform_x_for_micros(layout.waveform_plot, model, start_micros);
    let end_x = waveform_x_for_micros(layout.waveform_plot, model, end_micros);
    let threshold = 7.0;
    let start_distance = (point.x - start_x).abs();
    let end_distance = (point.x - end_x).abs();
    if start_distance > threshold && end_distance > threshold {
        return None;
    }
    Some(start_distance <= end_distance)
}

/// Convert one normalized waveform micro position into plot-space x.
pub(super) fn waveform_x_for_micros(plot: Rect, model: &AppModel, micros: u32) -> f32 {
    let view_start = model.waveform.view_start_micros.min(1_000_000) as f32 / 1_000_000.0;
    let view_end = model.waveform.view_end_micros.min(1_000_000) as f32 / 1_000_000.0;
    let view_width = (view_end - view_start).max(f32::EPSILON);
    let absolute_ratio = micros.min(1_000_000) as f32 / 1_000_000.0;
    let ratio_in_view = ((absolute_ratio - view_start) / view_width).clamp(0.0, 1.0);
    plot.min.x + (plot.width() * ratio_in_view)
}

/// Return the centered vertical hit span used by waveform edge-resize targets.
pub(super) fn waveform_centered_resize_edge_y_bounds(plot: Rect) -> (f32, f32) {
    let height = (plot.height() * 0.34).max(1.0).min(plot.height());
    let center_y = plot.min.y + (plot.height() * 0.5);
    let top = (center_y - (height * 0.5)).max(plot.min.y);
    let bottom = (top + height).min(plot.max.y).max(top + 1.0);
    (top, bottom)
}

/// Return one plot-bounded hover marker rectangle for a waveform x-position.
pub(super) fn waveform_hover_marker_rect(
    waveform_plot: Rect,
    marker_width: f32,
    hover_x: f32,
) -> Option<Rect> {
    if waveform_plot.width() <= 0.0 || waveform_plot.height() <= 0.0 {
        return None;
    }
    let width = marker_width.max(1.0);
    let half = width * 0.5;
    let clamped_x = hover_x.clamp(waveform_plot.min.x, waveform_plot.max.x);
    let left = (clamped_x - half).clamp(waveform_plot.min.x, waveform_plot.max.x - 1.0);
    let right = (left + width).min(waveform_plot.max.x).max(left + 1.0);
    Some(Rect::from_min_max(
        Point::new(left, waveform_plot.min.y),
        Point::new(right, waveform_plot.max.y),
    ))
}

pub(super) fn map_point_is_selected(model: &AppModel, point: &crate::app::MapPointModel) -> bool {
    model.map.selected_sample_id.as_deref() == Some(point.sample_id.as_ref())
}

pub(super) fn map_point_is_focused(model: &AppModel, point: &crate::app::MapPointModel) -> bool {
    model.map.focused_sample_id.as_deref() == Some(point.sample_id.as_ref())
}

pub(super) fn map_point_color(
    style: &StyleTokens,
    model: &AppModel,
    point: &crate::app::MapPointModel,
) -> Rgba8 {
    if map_point_is_focused(model, point) {
        return style.accent_warning;
    }
    if map_point_is_selected(model, point) {
        return style.accent_mint;
    }
    match point.cluster_id.map(|id| id.rem_euclid(5)) {
        Some(0) => blend_color(style.accent_mint, style.bg_secondary, 0.42),
        Some(1) => blend_color(style.accent_copper, style.bg_secondary, 0.42),
        Some(2) => blend_color(style.accent_warning, style.bg_secondary, 0.42),
        Some(3) => blend_color(style.text_primary, style.bg_secondary, 0.35),
        Some(_) => blend_color(style.text_muted, style.bg_secondary, 0.35),
        None => blend_color(style.text_muted, style.bg_secondary, 0.5),
    }
}

pub(super) fn map_sample_id_at_point(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> Option<String> {
    if !model.map.active || model.map.points.is_empty() {
        return None;
    }
    let canvas =
        compute_browser_map_canvas_rect(layout.browser_rows, style_for_layout(layout).sizing);
    if !canvas.contains(point) {
        return None;
    }

    let mut best: Option<(f32, &str)> = None;
    for map_point in model.map.points.iter() {
        let center = compute_browser_map_point_center(canvas, map_point.x_milli, map_point.y_milli);
        let radius = if map_point_is_focused(model, map_point) {
            7.0
        } else if map_point_is_selected(model, map_point) {
            6.0
        } else {
            5.0
        };
        let dx = point.x - center.x;
        let dy = point.y - center.y;
        let distance_sq = (dx * dx) + (dy * dy);
        if distance_sq > (radius * radius) {
            continue;
        }
        match best {
            Some((best_distance_sq, _)) if distance_sq >= best_distance_sq => {}
            _ => best = Some((distance_sq, map_point.sample_id.as_ref())),
        }
    }
    best.map(|(_, sample_id)| sample_id.to_string())
}
