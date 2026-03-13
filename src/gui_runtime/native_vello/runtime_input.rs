//! Immediate cursor, drag, and wheel processing for the native Vello runner.

use super::*;

impl<Bridge> NativeVelloRunner<Bridge>
where
    Bridge: NativeAppBridge,
{
    pub(super) fn sync_browser_viewport_from_shell(&mut self, layout: &ShellLayout) {
        let Some(visible_row) = self
            .shell_state
            .browser_viewport_start_row(layout, &self.model)
        else {
            return;
        };
        if visible_row == self.model.browser.view_start_row
            || self.last_emitted_browser_view_start == Some(visible_row)
        {
            return;
        }
        self.last_emitted_browser_view_start = Some(visible_row);
        self.emit_model_action(UiAction::SetBrowserViewStart { visible_row });
    }

    fn sync_browser_viewport_for_pointer_row_action(&mut self, action: &UiAction) {
        let Some(target_visible_row) = browser_pointer_action_visible_row(action) else {
            return;
        };
        let Some(layout) = self.shell_layout.as_ref() else {
            return;
        };
        let viewport_len = self.shell_state.browser_viewport_len(layout, &self.model);
        let current_view_start = self
            .shell_state
            .browser_viewport_start_row(layout, &self.model)
            .unwrap_or(self.model.browser.view_start_row);
        let Some(next_view_start) = browser_view_start_after_focus(
            current_view_start,
            self.model.browser.visible_count,
            viewport_len,
            target_visible_row,
        ) else {
            return;
        };
        if next_view_start == self.model.browser.view_start_row {
            return;
        }
        self.last_emitted_browser_view_start = Some(next_view_start);
        self.emit_model_action(UiAction::SetBrowserViewStart {
            visible_row: next_view_start,
        });
    }

    pub(super) fn queue_cursor(&mut self, point: Point) {
        self.pending_cursor = Some(point);
    }

    /// Update the native cursor icon only when it changed.
    pub(super) fn set_cursor_icon(&mut self, icon: CursorIcon) {
        if self.cursor_icon == icon {
            return;
        }
        if let Some(window) = self.window.as_ref() {
            window.set_cursor(icon);
        }
        self.cursor_icon = icon;
    }

    /// Resolve waveform-resize hover cursor state for the current pointer.
    pub(super) fn update_waveform_resize_cursor(&mut self, point: Point) {
        let icon = if let Some(layout) = self.shell_layout.as_deref() {
            if waveform_selection_drag_handle_hovered(&layout, &self.model, point) {
                CursorIcon::Grab
            } else if waveform_resize_handle_hovered(&layout, &self.model, point) {
                CursorIcon::EwResize
            } else if self
                .shell_state
                .prompt_input_at_point(&layout, &self.model, point)
                || self
                    .shell_state
                    .waveform_bpm_input_at_point(&layout, &self.model, point)
            {
                CursorIcon::Text
            } else {
                CursorIcon::Default
            }
        } else {
            CursorIcon::Default
        };
        self.set_cursor_icon(icon);
    }

    /// Keep one stable cursor icon for the currently captured waveform drag.
    pub(super) fn update_cursor_for_active_waveform_drag(&mut self) {
        let icon = self
            .waveform_drag_mode
            .map(cursor_icon_for_waveform_drag_mode)
            .unwrap_or(CursorIcon::Default);
        self.set_cursor_icon(icon);
    }

    /// Record recent pointer activity for short-lived high-frequency redraw pacing.
    pub(super) fn note_cursor_activity(&mut self, now: Instant) {
        self.cursor_activity_redraw_until = Some(now + CURSOR_ACTIVITY_REDRAW_WINDOW);
    }

    /// Return the next redraw deadline while recent cursor activity is active.
    pub(super) fn next_cursor_activity_redraw_deadline(&mut self, now: Instant) -> Option<Instant> {
        let until = self.cursor_activity_redraw_until?;
        if now >= until {
            self.cursor_activity_redraw_until = None;
            return None;
        }
        let mut next_redraw_at = self.last_redraw + self.cursor_activity_redraw_interval;
        if next_redraw_at < now {
            next_redraw_at = now;
        }
        if next_redraw_at > until {
            next_redraw_at = until;
        }
        Some(next_redraw_at)
    }

    /// Process one cursor move immediately when layout state is available.
    ///
    /// Returns `(processed, handled)` where:
    /// - `processed` indicates whether layout/model state was available now.
    /// - `handled` indicates whether hover state changed and triggered redraw.
    pub(super) fn process_cursor_move_immediately(&mut self, point: Point) -> (bool, bool) {
        let Some(layout) = self.shell_layout.as_ref() else {
            return (false, false);
        };
        let profile_start = self.profiler.now_if_enabled();
        let effect = self
            .shell_state
            .handle_cursor_move_effect(layout, &self.model, point);
        let handled = effect != CursorMoveEffect::None;
        if handled {
            if let Some(start) = profile_start {
                let kind = if self.model.map.active {
                    InteractionProfileKind::MapPanProxy
                } else {
                    InteractionProfileKind::Hover
                };
                self.profiler.add_interaction_latency(kind, start.elapsed());
            }
            match effect {
                CursorMoveEffect::WaveformHoverOnly => {
                    self.apply_invalidation_scope(RuntimeInvalidationScope::OverlayMotionOnly);
                }
                CursorMoveEffect::GeneralOverlay => self.rebuild_overlay_and_request_redraw(),
                CursorMoveEffect::None => {}
            }
        }
        (true, handled)
    }

    /// Emit one wheel-derived browser viewport-scroll action immediately.
    pub(super) fn process_wheel_rows_immediately(&mut self, visible_row: usize) -> bool {
        self.shell_state.clear_browser_row_hover();
        self.emit_model_action_with_profile(
            UiAction::SetBrowserViewStart { visible_row },
            Some(InteractionProfileKind::Wheel),
        );
        true
    }

    /// Emit one browser-scrollbar drag viewport update immediately.
    pub(super) fn process_browser_scrollbar_drag_immediately(&mut self, point: Point) -> bool {
        let Some(layout) = self.shell_layout.as_ref() else {
            return false;
        };
        let Some(drag) = self.browser_scrollbar_drag else {
            return false;
        };
        let Some(visible_row) = self.shell_state.browser_scrollbar_view_start_for_drag(
            layout,
            &self.model,
            point.y,
            drag.thumb_pointer_offset_y,
        ) else {
            return false;
        };
        if self.last_emitted_browser_view_start == Some(visible_row) {
            return true;
        }
        self.last_emitted_browser_view_start = Some(visible_row);
        self.shell_state.clear_browser_row_hover();
        self.emit_model_action(UiAction::SetBrowserViewStart { visible_row });
        true
    }

    /// Emit one browser-scrollbar track-click viewport update immediately.
    pub(super) fn process_browser_scrollbar_track_click_immediately(
        &mut self,
        point: Point,
    ) -> bool {
        let Some(layout) = self.shell_layout.as_ref() else {
            return false;
        };
        let Some(visible_row) =
            self.shell_state
                .browser_scrollbar_view_start_at_point(layout, &self.model, point)
        else {
            return false;
        };
        self.shell_state.clear_browser_row_hover();
        self.emit_model_action(UiAction::SetBrowserViewStart { visible_row });
        true
    }

    /// Emit one waveform-scrollbar drag viewport update immediately.
    pub(super) fn process_waveform_scrollbar_drag_immediately(&mut self, point: Point) -> bool {
        let Some(layout) = self.shell_layout.as_ref() else {
            return false;
        };
        let Some(drag) = self.waveform_scrollbar_drag else {
            return false;
        };
        let Some(center_micros) = self.shell_state.waveform_scrollbar_view_center_for_drag(
            layout,
            &self.model,
            point.x,
            drag.thumb_pointer_offset_x,
        ) else {
            return false;
        };
        if self.last_emitted_waveform_view_center == Some(center_micros) {
            return true;
        }
        self.last_emitted_waveform_view_center = Some(center_micros);
        self.emit_model_action_with_profile(
            UiAction::SetWaveformViewCenter { center_micros },
            Some(InteractionProfileKind::Waveform),
        );
        true
    }

    /// Emit one waveform-scrollbar track-click viewport update immediately.
    pub(super) fn process_waveform_scrollbar_track_click_immediately(
        &mut self,
        point: Point,
    ) -> bool {
        let Some(layout) = self.shell_layout.as_ref() else {
            return false;
        };
        let Some(center_micros) =
            self.shell_state
                .waveform_scrollbar_view_center_at_point(layout, &self.model, point)
        else {
            return false;
        };
        self.last_emitted_waveform_view_center = Some(center_micros);
        self.emit_model_action_with_profile(
            UiAction::SetWaveformViewCenter { center_micros },
            Some(InteractionProfileKind::Waveform),
        );
        true
    }

    /// Emit one middle-button waveform pan viewport update immediately.
    pub(super) fn process_waveform_pan_drag_immediately(&mut self, point: Point) -> bool {
        let Some(layout) = self.shell_layout.as_ref() else {
            return false;
        };
        let Some(drag) = self.waveform_pan_drag else {
            return false;
        };
        let plot_width = layout.waveform_plot.width().max(1.0);
        let span = drag
            .view_end_micros
            .min(1_000_000)
            .saturating_sub(drag.view_start_micros.min(1_000_000))
            .max(1);
        let max_view_start = 1_000_000u32.saturating_sub(span);
        let delta_ratio = (point.x - drag.origin_x) / plot_width;
        let delta_micros = delta_ratio * span as f32;
        let next_start = (drag.view_start_micros as f32 - delta_micros)
            .clamp(0.0, max_view_start as f32)
            .round() as u32;
        let center_micros = (next_start + (span / 2)).min(1_000_000);
        if self.last_emitted_waveform_view_center == Some(center_micros) {
            return true;
        }
        self.last_emitted_waveform_view_center = Some(center_micros);
        self.emit_model_action_with_profile(
            UiAction::SetWaveformViewCenter { center_micros },
            Some(InteractionProfileKind::Waveform),
        );
        true
    }

    /// Return whether one held-key repeat should be processed for navigation.
    pub(super) fn allows_key_repeat(&self, key: KeyCode) -> bool {
        if self.text_input_target == TextInputTarget::WaveformBpm {
            if self.modifiers.control_key()
                || self.modifiers.super_key()
                || self.modifiers.alt_key()
            {
                return false;
            }
            return matches!(key, KeyCode::ArrowUp | KeyCode::ArrowDown);
        }
        if self.modifiers.shift_key()
            || self.modifiers.control_key()
            || self.modifiers.super_key()
            || self.modifiers.alt_key()
        {
            return false;
        }
        if self.text_input_target != TextInputTarget::None {
            return false;
        }
        matches!(key, KeyCode::ArrowUp | KeyCode::ArrowDown)
    }

    pub(super) fn queue_volume_milli(&mut self, value_milli: u16) {
        self.pending_volume_milli = Some(value_milli.min(1000));
    }

    /// Emit one waveform action immediately during active pointer drag.
    pub(super) fn emit_waveform_drag_action_immediately(&mut self, action: UiAction) {
        if self.last_emitted_waveform_drag_action.as_ref() == Some(&action) {
            return;
        }
        self.last_emitted_waveform_drag_action = Some(action.clone());
        self.emit_model_action_with_profile(action, Some(InteractionProfileKind::Waveform));
    }

    /// Process one waveform drag cursor update when waveform drag mode is active.
    pub(super) fn process_waveform_drag_immediately(&mut self, point: Point) -> bool {
        let Some(layout) = self.shell_layout.as_ref() else {
            return false;
        };
        let Some(mode) = self.waveform_drag_mode else {
            return false;
        };
        if self.last_emitted_waveform_drag_action.is_none()
            && !waveform_drag_exceeds_click_slop(layout, &self.model, point, mode)
        {
            return false;
        }
        let action = waveform_drag_action_for_mode(layout, &self.model, point, mode);
        self.emit_waveform_drag_action_immediately(action);
        true
    }

    /// Process one waveform-selection export drag cursor update.
    pub(super) fn process_selection_drag_immediately(&mut self, point: Point) -> bool {
        let Some(layout) = self.shell_layout.as_ref() else {
            return false;
        };
        let (pointer_x, pointer_y) = ui_action_pointer_coords(point);
        self.emit_model_action_with_profile(
            UiAction::UpdateWaveformSelectionDrag {
                pointer_x,
                pointer_y,
                over_browser_list: !self.model.map.active && layout.browser_rows.contains(point),
                shift_down: self.modifiers.shift_key(),
                alt_down: self.modifiers.alt_key(),
            },
            Some(InteractionProfileKind::Waveform),
        );
        true
    }

    /// Process one map-focus drag cursor update when map drag mode is active.
    pub(super) fn process_map_focus_drag_immediately(&mut self, point: Point) -> bool {
        let Some(layout) = self.shell_layout.as_ref() else {
            return false;
        };
        if !self.model.map.active {
            return false;
        }
        let Some(action) = self
            .shell_state
            .map_sample_action_at_point(layout, &self.model, point)
        else {
            return false;
        };
        let UiAction::FocusMapSample { sample_id } = &action else {
            return false;
        };
        if self.last_emitted_map_drag_sample_id.as_deref() == Some(sample_id.as_str()) {
            return false;
        }
        self.last_emitted_map_drag_sample_id = Some(sample_id.clone());
        self.emit_model_action_with_profile(action, Some(InteractionProfileKind::MapPanProxy));
        true
    }

    /// Emit one normalized volume update immediately for smooth drag visuals.
    pub(super) fn emit_volume_milli_immediately(&mut self, value_milli: u16) {
        self.queue_volume_milli(value_milli);
        let _ = self.flush_pending_volume_action();
    }

    pub(super) fn flush_pending_volume_action(&mut self) -> bool {
        let Some(value_milli) = self.pending_volume_milli.take() else {
            return false;
        };
        self.emit_model_action_with_profile(
            UiAction::SetVolume { value_milli },
            Some(InteractionProfileKind::Volume),
        );
        true
    }

    /// Handle one pointer-press action, deferring drag-only waveform edits until movement.
    pub(super) fn handle_pointer_press_action(
        &mut self,
        action: UiAction,
        map_drag_start: bool,
    ) -> bool {
        if matches!(
            action,
            UiAction::FocusBrowserRow { .. }
                | UiAction::CommitFocusedBrowserRow
                | UiAction::ToggleBrowserRowSelection { .. }
                | UiAction::ExtendBrowserSelectionToRow { .. }
                | UiAction::AddRangeBrowserSelection { .. }
        ) {
            self.shell_state.clear_browser_row_hover();
        }
        let map_drag_sample_id = match &action {
            UiAction::FocusMapSample { sample_id } => Some(sample_id.clone()),
            _ => None,
        };
        self.begin_waveform_pointer_interaction(&action);
        if !waveform_press_action_emits_immediately(&action) {
            return false;
        }
        self.sync_browser_viewport_for_pointer_row_action(&action);
        self.update_text_target_after_action(&action);
        self.emit_model_action(action);
        if map_drag_start {
            self.begin_map_focus_drag(map_drag_sample_id);
        }
        true
    }

    pub(super) fn finish_volume_drag(&mut self, released_button: Option<MouseButton>) {
        let finish_edit_fade_drag = self
            .waveform_drag_mode
            .is_some_and(waveform_drag_mode_is_edit_fade);
        let finish_selection_drag =
            self.selection_drag_active && matches!(released_button, Some(MouseButton::Left));
        let finish_selection_smart_scale_drag = matches!(released_button, Some(MouseButton::Left))
            && self.waveform_drag_mode.is_some_and(|mode| {
                matches!(mode, WaveformPointerDragMode::SelectionSmartScale { .. })
            });
        let seek_on_waveform_click_release = if matches!(released_button, Some(MouseButton::Left))
            && self.last_emitted_waveform_drag_action.is_none()
        {
            if let Some(mode @ WaveformPointerDragMode::Selection { .. }) = self.waveform_drag_mode
            {
                if let (Some(layout), Some(point)) = (self.shell_layout.as_ref(), self.last_cursor)
                {
                    !waveform_drag_exceeds_click_slop(layout, &self.model, point, mode)
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };
        let clear_playback_selection_on_click_release =
            seek_on_waveform_click_release && self.clear_playback_selection_on_click_release;
        let _ = self.flush_pending_volume_action();
        if self.volume_drag_active {
            self.emit_model_action(UiAction::CommitVolumeSetting);
        }
        self.volume_drag_active = false;
        self.last_emitted_volume_milli = None;
        if finish_edit_fade_drag {
            self.emit_model_action(UiAction::FinishWaveformEditFadeDrag);
        }
        if finish_selection_drag {
            self.emit_model_action(UiAction::FinishWaveformSelectionDrag);
        }
        if finish_selection_smart_scale_drag {
            self.emit_model_action(UiAction::FinishWaveformSelectionSmartScaleDrag);
        }
        self.clear_pointer_drag_session();
        if let Some(point) = self.last_cursor {
            let _ = self.process_cursor_move_immediately(point);
            self.update_waveform_resize_cursor(point);
        }
        if seek_on_waveform_click_release
            && let (Some(layout), Some(point)) = (self.shell_layout.as_ref(), self.last_cursor)
        {
            let position_milli = waveform_position_milli_from_point(layout, &self.model, point);
            if clear_playback_selection_on_click_release {
                self.emit_model_action(UiAction::ClearWaveformSelection);
            }
            self.emit_model_action_with_profile(
                UiAction::SeekWaveform { position_milli },
                Some(InteractionProfileKind::Waveform),
            );
        }
    }

    pub(super) fn flush_pending_input(&mut self) -> bool {
        let mut pending_action = false;
        if self.flush_pending_volume_action() {
            pending_action = true;
        }
        if let Some(point) = self.pending_cursor.take() {
            let (_, handled) = self.process_cursor_move_immediately(point);
            if handled {
                pending_action = true;
            }
        }
        pending_action
    }

    pub(super) fn mark_idle_status_refresh_if_due(&mut self, now: Instant) -> bool {
        if now < self.next_idle_status_refresh {
            return false;
        }
        let mut next_refresh = self.next_idle_status_refresh;
        while next_refresh <= now {
            next_refresh += self.idle_status_refresh_interval;
        }
        self.next_idle_status_refresh = next_refresh;
        self.frame_state.mark_motion_overlay_dirty();
        true
    }
}

fn browser_pointer_action_visible_row(action: &UiAction) -> Option<usize> {
    match action {
        UiAction::FocusBrowserRow { visible_row }
        | UiAction::ToggleBrowserRowSelection { visible_row }
        | UiAction::ExtendBrowserSelectionToRow { visible_row }
        | UiAction::AddRangeBrowserSelection { visible_row } => Some(*visible_row),
        _ => None,
    }
}

fn cursor_icon_for_waveform_drag_mode(mode: WaveformPointerDragMode) -> CursorIcon {
    match mode {
        WaveformPointerDragMode::Selection { .. }
        | WaveformPointerDragMode::SelectionSmartScale { .. }
        | WaveformPointerDragMode::EditSelection { .. }
        | WaveformPointerDragMode::EditFadeInEnd
        | WaveformPointerDragMode::EditFadeInMuteStart
        | WaveformPointerDragMode::EditFadeOutStart
        | WaveformPointerDragMode::EditFadeOutMuteEnd => CursorIcon::EwResize,
        WaveformPointerDragMode::SelectionShift { .. }
        | WaveformPointerDragMode::EditSelectionShift { .. } => CursorIcon::Grab,
        WaveformPointerDragMode::Seek
        | WaveformPointerDragMode::Cursor
        | WaveformPointerDragMode::EditFadeInCurve
        | WaveformPointerDragMode::EditFadeOutCurve => CursorIcon::Default,
    }
}

fn browser_view_start_after_focus(
    current_view_start: usize,
    visible_count: usize,
    viewport_len: usize,
    focus_visible_row: usize,
) -> Option<usize> {
    if visible_count == 0 || viewport_len == 0 {
        return None;
    }
    if visible_count <= viewport_len {
        return Some(0);
    }
    let max_start = visible_count.saturating_sub(viewport_len);
    let edge_margin = 3usize.min(viewport_len.saturating_sub(1) / 2);
    let focus_visible_row = focus_visible_row.min(visible_count.saturating_sub(1));
    let mut view_start = current_view_start.min(max_start);
    let view_end = view_start + viewport_len;
    let top_guard = view_start + edge_margin;
    let bottom_guard = view_end.saturating_sub(edge_margin);
    if focus_visible_row < top_guard {
        view_start = focus_visible_row.saturating_sub(edge_margin);
    } else if focus_visible_row >= bottom_guard {
        view_start = focus_visible_row
            .saturating_add(edge_margin + 1)
            .saturating_sub(viewport_len);
    }
    Some(view_start.min(max_start))
}
