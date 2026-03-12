use super::*;

impl NativeShellState {
    /// Return a waveform-toolbar button rect for one control label in tests.
    #[cfg(test)]
    pub(crate) fn waveform_toolbar_button_rect(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        label: &'static str,
    ) -> Option<Rect> {
        let style = style_for_layout(layout);
        let motion_model = NativeMotionModel::from_app_model(model);
        waveform_toolbar_buttons(
            layout,
            &style,
            &motion_model,
            self.waveform_bpm_input_active,
            self.waveform_bpm_input_display.as_deref(),
        )
        .into_iter()
        .find(|button| button.label == label)
        .map(|button| button.rect)
    }

    /// Return the pointer's offset within the waveform scrollbar thumb when hovered.
    pub(crate) fn waveform_scrollbar_thumb_offset_at_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<f32> {
        let scrollbar = waveform_scrollbar_layout(
            layout.waveform_plot,
            model.waveform.view_start_micros,
            model.waveform.view_end_micros,
        )?;
        scrollbar
            .thumb
            .contains(point)
            .then_some((point.x - scrollbar.thumb.min.x).clamp(0.0, scrollbar.thumb.width()))
    }

    /// Resolve the waveform viewport center for an active scrollbar-thumb drag.
    pub(crate) fn waveform_scrollbar_view_center_for_drag(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        pointer_x: f32,
        thumb_pointer_offset_x: f32,
    ) -> Option<u32> {
        let scrollbar = waveform_scrollbar_layout(
            layout.waveform_plot,
            model.waveform.view_start_micros,
            model.waveform.view_end_micros,
        )?;
        waveform_scrollbar_center_for_pointer(
            scrollbar,
            model.waveform.view_start_micros,
            model.waveform.view_end_micros,
            pointer_x,
            thumb_pointer_offset_x,
        )
    }

    /// Resolve the waveform viewport center for a click inside the scrollbar track.
    pub(crate) fn waveform_scrollbar_view_center_at_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<u32> {
        let scrollbar = waveform_scrollbar_layout(
            layout.waveform_plot,
            model.waveform.view_start_micros,
            model.waveform.view_end_micros,
        )?;
        if !scrollbar.track.contains(point) || scrollbar.thumb.contains(point) {
            return None;
        }
        waveform_scrollbar_center_for_pointer(
            scrollbar,
            model.waveform.view_start_micros,
            model.waveform.view_end_micros,
            point.x,
            scrollbar.thumb.width() * 0.5,
        )
    }

    /// Resolve a waveform-toolbar control click into a native UI action.
    pub(crate) fn waveform_toolbar_action_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        let motion_model = NativeMotionModel::from_app_model(model);
        self.waveform_toolbar_action_at_point_with_motion(layout, &motion_model, point)
    }

    /// Resolve a waveform-toolbar control click into a native UI action.
    pub(crate) fn waveform_toolbar_action_at_point_with_motion(
        &mut self,
        layout: &ShellLayout,
        motion_model: &NativeMotionModel,
        point: Point,
    ) -> Option<UiAction> {
        let style = style_for_layout(layout);
        let resolved = self
            .cached_waveform_toolbar_buttons(layout, &style, motion_model)
            .into_iter()
            .find(|button| button.enabled && button.rect.contains(point))
            .map(|button| {
                (
                    waveform_toolbar_hover_hint(button.label),
                    button.action.clone(),
                )
            });
        if let Some((Some(hint), _)) = resolved.as_ref() {
            self.trigger_waveform_toolbar_flash(*hint);
        }
        resolved.and_then(|(_, action)| action)
    }

    /// Return whether a point falls inside the waveform BPM text-input widget.
    pub(crate) fn waveform_bpm_input_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> bool {
        let motion_model = NativeMotionModel::from_app_model(model);
        self.waveform_bpm_input_at_point_with_motion(layout, &motion_model, point)
    }

    /// Return whether a point falls inside the waveform BPM text-input widget.
    pub(crate) fn waveform_bpm_input_at_point_with_motion(
        &mut self,
        layout: &ShellLayout,
        motion_model: &NativeMotionModel,
        point: Point,
    ) -> bool {
        let style = style_for_layout(layout);
        let hit = self
            .cached_waveform_toolbar_buttons(layout, &style, motion_model)
            .iter()
            .any(|button| {
                button.label == "BPM Value" && button.enabled && button.rect.contains(point)
            });
        if hit {
            self.trigger_waveform_toolbar_flash(WaveformToolbarHoverHint::BpmValue);
        }
        hit
    }

    /// Return the waveform BPM input rect when the toolbar is available.
    pub(crate) fn waveform_bpm_input_rect(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<Rect> {
        let motion_model = NativeMotionModel::from_app_model(model);
        let style = style_for_layout(layout);
        self.cached_waveform_toolbar_buttons(layout, &style, &motion_model)
            .iter()
            .find(|button| button.label == "BPM Value" && button.enabled)
            .map(|button| button.rect)
    }

    /// Return the waveform BPM text rect used for rendering inside the field.
    pub(crate) fn waveform_bpm_text_rect(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<Rect> {
        let motion_model = NativeMotionModel::from_app_model(model);
        let style = style_for_layout(layout);
        self.cached_waveform_toolbar_buttons(layout, &style, &motion_model)
            .iter()
            .find(|button| button.label == "BPM Value" && button.enabled)
            .map(|button| compute_action_button_text_rect(button.rect, style.sizing))
    }
}

pub(in crate::gui::native_shell::state) fn waveform_toolbar_hit_test_cache_key(
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

pub(in crate::gui::native_shell::state) fn waveform_toolbar_model_flags(
    model: &NativeMotionModel,
) -> u16 {
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

pub(in crate::gui::native_shell::state) fn waveform_tempo_label_signature(
    model: &NativeMotionModel,
) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    model.waveform_tempo_label.hash(&mut hasher);
    hasher.finish()
}

pub(in crate::gui::native_shell::state) fn text_signature(value: Option<&str>) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

pub(in crate::gui::native_shell::state) fn waveform_toolbar_hover_hint(
    label: &str,
) -> Option<WaveformToolbarHoverHint> {
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
pub(in crate::gui::native_shell::state) fn waveform_hover_x_for_point(
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
pub(in crate::gui::native_shell::state) fn hovered_waveform_resize_edge_for_point(
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
pub(in crate::gui::native_shell::state) fn hovered_resize_edge_for_range(
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
pub(in crate::gui::native_shell::state) fn waveform_x_for_micros(
    plot: Rect,
    model: &AppModel,
    micros: u32,
) -> f32 {
    let view_start = model.waveform.view_start_micros.min(1_000_000) as f32 / 1_000_000.0;
    let view_end = model.waveform.view_end_micros.min(1_000_000) as f32 / 1_000_000.0;
    let view_width = (view_end - view_start).max(f32::EPSILON);
    let absolute_ratio = micros.min(1_000_000) as f32 / 1_000_000.0;
    let ratio_in_view = ((absolute_ratio - view_start) / view_width).clamp(0.0, 1.0);
    plot.min.x + (plot.width() * ratio_in_view)
}

/// Return the centered vertical hit span used by waveform edge-resize targets.
pub(in crate::gui::native_shell::state) fn waveform_centered_resize_edge_y_bounds(
    plot: Rect,
) -> (f32, f32) {
    let height = (plot.height() * 0.34).max(1.0).min(plot.height());
    let center_y = plot.min.y + (plot.height() * 0.5);
    let top = (center_y - (height * 0.5)).max(plot.min.y);
    let bottom = (top + height).min(plot.max.y).max(top + 1.0);
    (top, bottom)
}

/// Return one plot-bounded hover marker rectangle for a waveform x-position.
pub(in crate::gui::native_shell::state) fn waveform_hover_marker_rect(
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
