//! Pointer-release and deferred-input helpers for the native Vello runner.

use super::super::*;

/// Horizontal click slop used to distinguish waveform clicks from drags.
const WAVEFORM_CLICK_SEEK_SLOP_PX: f32 = 3.0;

impl<Bridge> NativeVelloRunner<Bridge>
where
    Bridge: NativeAppBridge,
{
    pub(crate) fn finish_volume_drag(&mut self, released_button: Option<MouseButton>) {
        let finish_edit_fade_drag = self
            .waveform_drag_mode
            .is_some_and(waveform_drag_mode_is_edit_fade);
        let finish_selection_range_drag = matches!(released_button, Some(MouseButton::Left))
            && self.waveform_drag_mode.is_some_and(|mode| {
                matches!(
                    mode,
                    WaveformPointerDragMode::Selection { .. }
                        | WaveformPointerDragMode::SelectionShift { .. }
                )
            })
            && self.last_emitted_waveform_drag_action.is_some();
        let finish_edit_selection_drag = matches!(released_button, Some(MouseButton::Left))
            && self.waveform_drag_mode.is_some_and(|mode| {
                matches!(
                    mode,
                    WaveformPointerDragMode::EditSelection { .. }
                        | WaveformPointerDragMode::EditSelectionShift { .. }
                )
            })
            && self.last_emitted_waveform_drag_action.is_some();
        let finish_selection_drag =
            self.selection_drag_active && matches!(released_button, Some(MouseButton::Left));
        let finish_selection_smart_scale_drag = matches!(released_button, Some(MouseButton::Left))
            && self.waveform_drag_mode.is_some_and(|mode| {
                matches!(mode, WaveformPointerDragMode::SelectionSmartScale { .. })
            });
        let click_seek_press = self.waveform_click_seek_press;
        let seek_on_waveform_click_release = matches!(released_button, Some(MouseButton::Left))
            && self.last_emitted_waveform_drag_action.is_none()
            && click_seek_press.is_some()
            && self
                .waveform_drag_mode
                .is_none_or(|mode| matches!(mode, WaveformPointerDragMode::Selection { .. }))
            && self.last_cursor.is_some_and(|point| {
                click_seek_press.is_some_and(|press| {
                    (point.x - press.press_x).abs() <= WAVEFORM_CLICK_SEEK_SLOP_PX
                })
            });
        let _ = self.flush_pending_volume_action();
        if self.volume_drag_active {
            self.emit_model_action(UiAction::CommitVolumeSetting);
        }
        self.volume_drag_active = false;
        self.last_emitted_volume_milli = None;
        if finish_edit_fade_drag {
            self.emit_model_action(UiAction::FinishWaveformEditFadeDrag);
        }
        if finish_selection_range_drag {
            self.emit_model_action(UiAction::FinishWaveformSelectionRangeDrag);
        }
        if finish_selection_drag {
            self.emit_model_action(UiAction::FinishWaveformSelectionDrag);
        }
        if finish_selection_smart_scale_drag {
            self.emit_model_action(UiAction::FinishWaveformSelectionSmartScaleDrag);
        }
        if finish_edit_selection_drag {
            self.emit_model_action(UiAction::FinishWaveformEditSelectionDrag);
        }
        self.clear_pointer_drag_session();
        if let Some(point) = self.last_cursor {
            let _ = self.process_cursor_move_immediately(point);
            self.update_waveform_resize_cursor(point);
        }
        if seek_on_waveform_click_release && let Some(click_seek_press) = click_seek_press {
            if click_seek_press.clear_selection_on_release {
                self.emit_model_action(UiAction::ClearWaveformSelection);
            }
            self.emit_model_action_with_profile(
                UiAction::SeekWaveformPrecise {
                    position_nanos: click_seek_press.position_nanos,
                },
                Some(InteractionProfileKind::Waveform),
            );
            self.sync_model_after_waveform_click_release();
            self.start_playback_after_waveform_click_release_if_stopped();
        }
    }

    pub(crate) fn flush_pending_input(&mut self) -> bool {
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

    pub(crate) fn mark_idle_status_refresh_if_due(&mut self, now: Instant) -> bool {
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

    /// Pull the latest host model after click-to-seek release so queued bridge
    /// waveform actions become visible and audible immediately.
    fn sync_model_after_waveform_click_release(&mut self) {
        self.model = self.bridge.pull_model_arc();
        self.waveform_view_refresh_pending = false;
        self.shell_state.sync_from_model(&self.model);
        self.refresh_motion_model_from_model();
        self.sync_text_input_target();
    }

    /// Ensure click-release seek gestures restart playback from the clicked point.
    ///
    /// Some hosts apply the precise seek immediately but still leave transport
    /// stopped after the first bridge/model sync. When that happens, follow up
    /// with `PlayFromCurrentPlayhead` so plain waveform clicks behave like
    /// direct click-to-play instead of only moving the cursor.
    fn start_playback_after_waveform_click_release_if_stopped(&mut self) {
        if self.model.transport_running {
            return;
        }
        self.emit_model_action(UiAction::PlayFromCurrentPlayhead);
        self.sync_model_after_waveform_click_release();
    }
}
