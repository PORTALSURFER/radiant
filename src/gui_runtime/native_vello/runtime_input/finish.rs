//! Pointer-release and deferred-input helpers for the native Vello runner.

use super::super::*;

impl<Bridge> NativeVelloRunner<Bridge>
where
    Bridge: NativeAppBridge,
{
    pub(crate) fn finish_volume_drag(&mut self, released_button: Option<MouseButton>) {
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
}
