//! Drag-session and immediate action emission helpers for native input.

use super::super::*;

/// Horizontal click slop used to distinguish waveform clicks from drags.
const WAVEFORM_CLICK_SEEK_SLOP_PX: f32 = 3.0;

impl<Bridge> NativeVelloRunner<Bridge>
where
    Bridge: NativeAppBridge,
{
    /// Refresh the cached app model before immediate follow-up input uses stale state.
    ///
    /// Selection creation and focus changes can reduce into the bridge one event
    /// before the next redraw rebuild refreshes `self.model`. Keyboard shortcuts
    /// and same-frame pointer hit-testing should pull that pending snapshot so
    /// actions like immediate selection export see the latest focus/selection.
    pub(crate) fn refresh_cached_model_for_pending_input(&mut self) {
        if !self.frame_state.model_dirty {
            return;
        }
        self.model = self.bridge.pull_model_arc();
        self.waveform_view_refresh_pending = false;
        self.shell_state.sync_from_model(&self.model);
        self.refresh_motion_model_from_model();
        self.sync_text_input_target();
    }

    pub(crate) fn queue_volume_milli(&mut self, value_milli: u16) {
        self.pending_volume_milli = Some(value_milli.min(1000));
    }

    /// Emit one normalized volume update immediately for smooth drag visuals.
    pub(crate) fn emit_volume_milli_immediately(&mut self, value_milli: u16) {
        self.queue_volume_milli(value_milli);
        let _ = self.flush_pending_volume_action();
    }

    pub(crate) fn flush_pending_volume_action(&mut self) -> bool {
        let Some(value_milli) = self.pending_volume_milli.take() else {
            return false;
        };
        self.emit_model_action_with_profile(
            UiAction::SetVolume { value_milli },
            Some(InteractionProfileKind::Volume),
        );
        true
    }

    /// Emit one middle-button waveform pan viewport update immediately.
    pub(crate) fn process_waveform_pan_drag_immediately(&mut self, point: Point) -> bool {
        self.refresh_waveform_view_if_needed();
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

    /// Emit one waveform action immediately during active pointer drag.
    pub(crate) fn emit_waveform_drag_action_immediately(&mut self, action: UiAction) {
        if self.last_emitted_waveform_drag_action.as_ref() == Some(&action) {
            return;
        }
        self.last_emitted_waveform_drag_action = Some(action.clone());
        self.emit_model_action_with_profile(action, Some(InteractionProfileKind::Waveform));
    }

    /// Process one waveform drag cursor update when waveform drag mode is active.
    pub(crate) fn process_waveform_drag_immediately(&mut self, point: Point) -> bool {
        self.refresh_waveform_view_if_needed();
        let Some(layout) = self.shell_layout.as_ref() else {
            return false;
        };
        let Some(mode) = self.waveform_drag_mode else {
            return false;
        };
        if self.last_emitted_waveform_drag_action.is_none()
            && !self.waveform_drag_exceeds_click_slop(layout, point, mode)
        {
            return false;
        }
        let (action, next_mode) =
            waveform_drag_action_and_mode_for_point(layout, &self.model, point, mode);
        self.waveform_drag_mode = Some(next_mode);
        self.emit_waveform_drag_action_immediately(action);
        true
    }

    fn waveform_drag_exceeds_click_slop(
        &self,
        layout: &ShellLayout,
        point: Point,
        mode: WaveformPointerDragMode,
    ) -> bool {
        if let (WaveformPointerDragMode::Selection { .. }, Some(click_seek_press)) =
            (mode, self.waveform_click_seek_press)
        {
            return (point.x - click_seek_press.press_x).abs() > WAVEFORM_CLICK_SEEK_SLOP_PX;
        }
        waveform_drag_exceeds_click_slop(layout, &self.model, point, mode)
    }

    /// Refresh the local waveform view once after a wheel zoom changed it mid-drag.
    ///
    /// Wheel zoom reduces into the bridge immediately, but the runner's cached
    /// `AppModel` normally updates on the next scene rebuild. When the user
    /// keeps dragging before that rebuild lands, refresh the local snapshot
    /// first so pointer-to-time conversion uses the latest view bounds.
    pub(crate) fn refresh_waveform_view_if_needed(&mut self) {
        if !self.waveform_view_refresh_pending {
            return;
        }
        self.model = self.bridge.pull_model_arc();
        self.shell_state.sync_from_model(&self.model);
        self.refresh_motion_model_from_model();
        self.waveform_view_refresh_pending = false;
    }

    /// Process one waveform-selection export drag cursor update.
    pub(crate) fn process_selection_drag_immediately(&mut self, point: Point) -> bool {
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
    pub(crate) fn process_map_focus_drag_immediately(&mut self, point: Point) -> bool {
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

    /// Handle one pointer-press action, deferring drag-only waveform edits until movement.
    pub(crate) fn handle_pointer_press_action(
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
        let click_seek_press = self.waveform_click_seek_press_for_action(&action);
        self.begin_waveform_pointer_interaction(&action, click_seek_press);
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

    fn waveform_click_seek_press_for_action(
        &self,
        action: &UiAction,
    ) -> Option<WaveformClickSeekPress> {
        if !matches!(action, UiAction::BeginWaveformSelectionAt { .. }) {
            return None;
        }
        let Some(layout) = self.shell_layout.as_ref() else {
            return None;
        };
        let Some(point) = self.last_cursor else {
            return None;
        };
        Some(WaveformClickSeekPress {
            press_x: point.x,
            position_nanos: waveform_position_nanos_from_point(layout, &self.model, point),
            clear_selection_on_release: true,
        })
    }
}
