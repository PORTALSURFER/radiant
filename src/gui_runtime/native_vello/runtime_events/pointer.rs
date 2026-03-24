use super::*;

impl<B: NativeAppBridge> NativeVelloRunner<B> {
    /// Route one left-pointer press through the production hit-testing path in tests.
    #[cfg(test)]
    pub(crate) fn handle_left_pointer_press_for_tests(
        &mut self,
        layout: &ShellLayout,
        point: Point,
        map_drag_start: bool,
        action_emitted: &mut bool,
    ) -> bool {
        self.begin_pointer_press_cycle();
        self.refresh_cached_model_for_pending_input();
        self.handle_left_pointer_press(layout, point, map_drag_start, action_emitted)
    }

    pub(super) fn handle_cursor_moved(&mut self, point: Point) {
        if self.last_cursor == Some(point) {
            return;
        }
        self.last_cursor = Some(point);
        self.note_cursor_activity(Instant::now());
        let session = self.active_pointer_session();
        if matches!(session, ActivePointerSession::WaveformDrag) {
            self.update_cursor_for_active_waveform_drag();
        } else {
            self.update_waveform_resize_cursor(point);
        }
        match session {
            ActivePointerSession::Volume => {
                if let Some(layout) = self.shell_layout.as_ref()
                    && let Some(action) = self.shell_state.top_bar_volume_drag_action(layout, point)
                {
                    if let UiAction::SetVolume { value_milli } = action {
                        if self.last_emitted_volume_milli != Some(value_milli) {
                            self.last_emitted_volume_milli = Some(value_milli);
                            self.emit_volume_milli_immediately(value_milli);
                        }
                    } else {
                        self.emit_model_action(action);
                    }
                }
            }
            ActivePointerSession::BrowserScrollbar => {
                let _ = self.process_browser_scrollbar_drag_immediately(point);
            }
            ActivePointerSession::WaveformScrollbar => {
                let _ = self.process_waveform_scrollbar_drag_immediately(point);
            }
            ActivePointerSession::WaveformPan => {
                let _ = self.process_waveform_pan_drag_immediately(point);
            }
            ActivePointerSession::WaveformDrag => {
                let _ = self.process_waveform_drag_immediately(point);
            }
            ActivePointerSession::SelectionDrag => {
                let _ = self.process_selection_drag_immediately(point);
            }
            ActivePointerSession::MapFocusDrag => {
                let _ = self.process_map_focus_drag_immediately(point);
            }
            ActivePointerSession::TextInputDrag => {
                if !self.process_text_input_drag(point) {
                    let (processed, _) = self.process_cursor_move_immediately(point);
                    if !processed {
                        self.queue_cursor(point);
                    }
                }
            }
            ActivePointerSession::Hover => {
                let (processed, _) = self.process_cursor_move_immediately(point);
                if !processed {
                    self.queue_cursor(point);
                }
            }
        }
    }

    pub(super) fn handle_mouse_pressed(&mut self, button: MouseButton) {
        if self.window.is_none() {
            return;
        }
        let Some(point) = self.last_cursor else {
            return;
        };
        let _ = self.with_shell_layout(|this, layout| {
            this.begin_pointer_press_cycle();
            let mut handled = false;
            let mut action_emitted = false;
            let mut source_menu_state_changed = false;
            match button {
                MouseButton::Left => {
                    this.refresh_cached_model_for_pending_input();
                    let map_drag_start =
                        this.model.map.active && layout.browser_rows.contains(point);
                    if let Some(action) = this.shell_state.source_context_menu_action_at_point(
                        layout,
                        &this.model,
                        point,
                    ) {
                        this.emit_model_action(action);
                        action_emitted = true;
                        source_menu_state_changed |= this.shell_state.close_source_context_menu();
                        handled = true;
                    } else {
                        source_menu_state_changed |= this.shell_state.close_source_context_menu();
                    }
                    if !handled {
                        if this.handle_browser_search_pointer_press(
                            layout,
                            point,
                            this.modifiers.shift_key(),
                        ) {
                            handled = true;
                        } else if this.handle_waveform_bpm_pointer_press(
                            layout,
                            point,
                            this.modifiers.shift_key(),
                        ) {
                            handled = true;
                        }
                    }
                    if !handled {
                        handled = this.handle_left_pointer_press(
                            layout,
                            point,
                            map_drag_start,
                            &mut action_emitted,
                        );
                    }
                }
                MouseButton::Right => {
                    handled = this.handle_right_pointer_press(
                        layout,
                        point,
                        &mut action_emitted,
                        &mut source_menu_state_changed,
                    );
                }
                MouseButton::Middle => {
                    if layout.waveform_plot.contains(point) {
                        this.begin_waveform_pan_drag(point.x);
                        handled = true;
                    }
                }
                _ => {}
            }
            if source_menu_state_changed {
                this.apply_invalidation_scope(RuntimeInvalidationScope::StaticAndOverlays);
            } else if action_emitted && handled && !this.frame_state.has_pending_rebuild() {
                this.apply_invalidation_scope(RuntimeInvalidationScope::OverlayStateOnly);
            }
        });
    }

    pub(super) fn handle_mouse_released(&mut self, button: MouseButton) {
        self.clear_pointer_release_state();
        self.finish_volume_drag(Some(button));
    }

    fn handle_left_pointer_press(
        &mut self,
        layout: &ShellLayout,
        point: Point,
        map_drag_start: bool,
        action_emitted: &mut bool,
    ) -> bool {
        if self
            .shell_state
            .prompt_input_at_point(layout, &self.model, point)
        {
            self.activate_text_input_target(TextInputTarget::PromptInput);
            return true;
        }
        if self.text_input_target != TextInputTarget::None {
            self.deactivate_text_input_target();
            return false;
        }
        if let Some(action) = self
            .shell_state
            .top_bar_volume_action_at_point(layout, point)
        {
            if let UiAction::SetVolume { value_milli } = action {
                self.last_emitted_volume_milli = Some(value_milli);
                self.emit_volume_milli_immediately(value_milli);
            } else {
                self.emit_model_action(action);
            }
            *action_emitted = true;
            self.volume_drag_active = true;
            return true;
        }
        if let Some(thumb_pointer_offset_y) = self
            .shell_state
            .browser_scrollbar_thumb_offset_at_point(layout, &self.model, point)
        {
            self.begin_browser_scrollbar_drag(thumb_pointer_offset_y);
            return true;
        }
        if let Some(thumb_pointer_offset_x) = self
            .shell_state
            .waveform_scrollbar_thumb_offset_at_point(layout, &self.model, point)
        {
            let thumb_pointer_ratio_x = self
                .shell_state
                .waveform_scrollbar_thumb_ratio_at_point(layout, &self.model, point)
                .unwrap_or(0.0);
            self.begin_waveform_scrollbar_drag(thumb_pointer_offset_x, thumb_pointer_ratio_x);
            return true;
        }
        if self.process_waveform_scrollbar_track_click_immediately(point) {
            *action_emitted = true;
            return true;
        }
        if self.process_browser_scrollbar_track_click_immediately(point) {
            *action_emitted = true;
            return true;
        }
        if let Some(action) = action_from_pointer_with_motion(
            layout,
            &self.model,
            self.motion_model.as_ref(),
            &mut self.shell_state,
            point,
            self.modifiers,
        ) {
            if self.should_emit_waveform_command_edge_adjust_immediately(&action) {
                self.emit_model_action(action);
                *action_emitted = true;
            } else {
                *action_emitted = self.handle_pointer_press_action_at_point(
                    action,
                    map_drag_start,
                    layout,
                    point,
                );
            }
            return true;
        }
        if self.shell_state.handle_primary_click(layout, point)
            && let Some(column) = layout.column_at_point(point)
        {
            self.emit_model_action(UiAction::SelectColumn { index: column });
            *action_emitted = true;
            return true;
        }
        false
    }

    fn handle_right_pointer_press(
        &mut self,
        layout: &ShellLayout,
        point: Point,
        action_emitted: &mut bool,
        source_menu_state_changed: &mut bool,
    ) -> bool {
        if let Some(action) =
            self.shell_state
                .source_context_menu_action_at_point(layout, &self.model, point)
        {
            self.emit_model_action(action);
            *action_emitted = true;
            *source_menu_state_changed |= self.shell_state.close_source_context_menu();
            return true;
        }
        if let Some(index) = self
            .shell_state
            .source_row_at_point(layout, &self.model, point)
        {
            self.emit_model_action(UiAction::FocusSourceRow { index });
            self.shell_state
                .open_source_context_menu_for_row(index, point);
            *source_menu_state_changed = true;
            *action_emitted = true;
            return true;
        }
        *source_menu_state_changed |= self.shell_state.close_source_context_menu();
        if matches!(layout.hit_test(point), Some(ShellNodeKind::WaveformCard)) {
            let action =
                waveform_edit_action_from_pointer(layout, &self.model, point, self.modifiers);
            if self.should_emit_waveform_command_edge_adjust_immediately(&action) {
                self.emit_model_action(action);
                *action_emitted = true;
            } else {
                *action_emitted =
                    self.handle_pointer_press_action_at_point(action, false, layout, point);
            }
            return true;
        }
        false
    }

    /// Return whether one command-click waveform edge adjustment should emit on press.
    fn should_emit_waveform_command_edge_adjust_immediately(&self, action: &UiAction) -> bool {
        let command = self.modifiers.control_key() || self.modifiers.super_key();
        command
            && !self.modifiers.alt_key()
            && matches!(
                action,
                UiAction::SetWaveformSelectionRange { .. }
                    | UiAction::SetWaveformEditSelectionRange { .. }
            )
    }
}
