use super::*;

impl<B: NativeAppBridge> NativeVelloRunner<B> {
    #[cfg(test)]
    pub(crate) fn handle_hotkey_press_for_tests(&mut self, key: KeyCode) {
        self.refresh_cached_model_for_pending_input();
        let resolution =
            action_from_key(key, self.modifiers, &self.model, self.pending_hotkey_chord);
        self.pending_hotkey_chord = resolution.pending_chord;
        if let Some(action) = resolution.action {
            let action = rewrite_folder_create_hotkey_action(
                action,
                &self.model,
                self.shell_state.hovered_folder_row_index(),
            );
            self.update_text_target_after_action(&action);
            self.emit_model_action(action.clone());
            self.refresh_cached_model_after_folder_create_action(&action);
        }
    }

    #[cfg(test)]
    pub(crate) fn handle_character_key_for_tests(&mut self, key: KeyCode, character: &str) {
        self.refresh_cached_model_for_pending_input();
        if self.text_input_target != TextInputTarget::None
            && !self.modifiers.control_key()
            && !self.modifiers.super_key()
            && !self.modifiers.alt_key()
        {
            let _ = self.append_text(character);
            if !self.frame_state.has_pending_rebuild() {
                self.apply_invalidation_scope(RuntimeInvalidationScope::OverlayStateOnly);
            }
            return;
        }

        let resolution =
            action_from_key(key, self.modifiers, &self.model, self.pending_hotkey_chord);
        self.pending_hotkey_chord = resolution.pending_chord;
        if let Some(action) = resolution.action {
            let action = rewrite_folder_create_hotkey_action(
                action,
                &self.model,
                self.shell_state.hovered_folder_row_index(),
            );
            self.update_text_target_after_action(&action);
            self.emit_model_action(action.clone());
            self.refresh_cached_model_after_folder_create_action(&action);
        }
        if !self.frame_state.has_pending_rebuild() {
            self.apply_invalidation_scope(RuntimeInvalidationScope::OverlayStateOnly);
        }
    }

    #[cfg(test)]
    pub(crate) fn handle_mouse_wheel_for_tests(&mut self, delta: MouseScrollDelta) {
        self.handle_mouse_wheel(delta);
    }

    #[cfg(test)]
    pub(crate) fn handle_enter_for_tests(&mut self) {
        self.refresh_cached_model_for_pending_input();
        let mut handled = false;
        if matches!(
            self.text_input_target,
            TextInputTarget::BrowserSearch
                | TextInputTarget::FolderSearch
                | TextInputTarget::WaveformBpm
        ) {
            self.deactivate_text_input_target();
            handled = true;
        } else if self.text_input_target == TextInputTarget::FolderCreate {
            handled = true;
            if folder_create_confirm_enabled(&self.model) {
                let action = UiAction::ConfirmFolderCreate;
                self.update_text_target_after_action(&action);
                self.emit_model_action(action.clone());
                self.refresh_cached_model_after_folder_create_action(&action);
            }
        } else if self.text_input_target == TextInputTarget::None
            && matches!(
                self.model.focus_context,
                crate::app::FocusContextModel::SampleBrowser
            )
            && self.model.browser.duplicate_cleanup_active
        {
            let action = UiAction::ConfirmBrowserDuplicateCleanup;
            self.emit_model_action(action);
            handled = true;
        }
        if handled && !self.frame_state.has_pending_rebuild() {
            self.apply_invalidation_scope(RuntimeInvalidationScope::OverlayStateOnly);
        }
    }

    #[cfg(test)]
    pub(crate) fn handle_escape_for_tests(&mut self) {
        self.refresh_cached_model_for_pending_input();
        self.pending_hotkey_chord = None;
        if self.model.confirm_prompt.visible {
            self.emit_model_action(UiAction::CancelPrompt);
            self.deactivate_text_input_target();
        } else if self.text_input_target == TextInputTarget::FolderCreate {
            let action = UiAction::CancelFolderCreate;
            self.update_text_target_after_action(&action);
            self.emit_model_action(action.clone());
            self.refresh_cached_model_after_folder_create_action(&action);
        } else if self.text_input_target != TextInputTarget::None {
            self.deactivate_text_input_target();
        } else {
            let action = UiAction::HandleEscape;
            self.update_text_target_after_action(&action);
            self.emit_model_action(action);
        }
        if !self.frame_state.has_pending_rebuild() {
            self.apply_invalidation_scope(RuntimeInvalidationScope::OverlayStateOnly);
        }
    }

    pub(super) fn handle_mouse_wheel(&mut self, delta: MouseScrollDelta) {
        let _ = self.with_shell_layout(|this, layout| {
            let waveform_zoom_action = this
                .last_cursor
                .and_then(|point| waveform_wheel_zoom_action(layout, &this.model, point, delta));
            let waveform_zoom_emitted = if let Some(action) = waveform_zoom_action {
                this.emit_model_action_with_profile(action, Some(InteractionProfileKind::Waveform));
                this.waveform_view_refresh_pending = true;
                true
            } else {
                false
            };
            if !waveform_zoom_emitted {
                let style = this.cached_style_for_layout(layout);
                if let Some(point) = this.last_cursor.filter(|point| {
                    this.shell_state
                        .folder_panel_contains_point(layout, &this.model, *point)
                }) {
                    if let Some(delta) = folder_wheel_row_delta(
                        &mut this.shell_state,
                        layout,
                        &this.model,
                        point,
                        &style,
                        delta,
                    ) {
                        let viewport_len =
                            this.shell_state.folder_viewport_len(layout, &this.model);
                        let current_view_start = this
                            .shell_state
                            .folder_viewport_start_row(layout, &this.model)
                            .unwrap_or(0);
                        if let Some(view_start_row) = browser_view_start_after_wheel(
                            current_view_start,
                            this.model.sources.folder_rows.len(),
                            viewport_len,
                            delta,
                        ) {
                            let _ = this.process_folder_view_start_immediately(view_start_row);
                        }
                        return;
                    }
                }
                let fallback_point = Point::new(
                    (layout.browser_rows.min.x + layout.browser_rows.max.x) * 0.5,
                    (layout.browser_rows.min.y + layout.browser_rows.max.y) * 0.5,
                );
                let point = this
                    .last_cursor
                    .filter(|point| layout.browser_panel.contains(*point))
                    .unwrap_or(fallback_point);
                if let Some(delta) =
                    browser_wheel_row_delta(layout, &this.model, point, &style, delta)
                {
                    let viewport_len = this.shell_state.browser_viewport_len(layout, &this.model);
                    let current_view_start = this
                        .shell_state
                        .browser_viewport_start_row(layout, &this.model)
                        .unwrap_or(this.model.browser.view_start_row);
                    if let Some(visible_row) = browser_view_start_after_wheel(
                        current_view_start,
                        this.model.browser.visible_count,
                        viewport_len,
                        delta,
                    ) {
                        let _ = this.process_wheel_rows_immediately(visible_row);
                    }
                }
            }
        });
    }

    pub(super) fn handle_keyboard_input(&mut self, event: winit::event::KeyEvent) {
        let key = match event.physical_key {
            PhysicalKey::Code(code) => key_code_from_winit(code),
            _ => None,
        };
        let allow_repeat = event.repeat && key.is_some_and(|key| self.allows_key_repeat(key));
        if event.state != ElementState::Pressed || (event.repeat && !allow_repeat) {
            return;
        }
        self.refresh_cached_model_for_pending_input();
        let mut handled = false;
        if matches!(event.logical_key, Key::Named(NamedKey::Escape)) {
            self.pending_hotkey_chord = None;
            if self.model.confirm_prompt.visible {
                self.emit_model_action(UiAction::CancelPrompt);
                self.deactivate_text_input_target();
                handled = true;
            } else if self.text_input_target == TextInputTarget::FolderCreate {
                let action = UiAction::CancelFolderCreate;
                self.update_text_target_after_action(&action);
                self.emit_model_action(action.clone());
                self.refresh_cached_model_after_folder_create_action(&action);
                handled = true;
            } else if self.text_input_target != TextInputTarget::None {
                self.deactivate_text_input_target();
                handled = true;
            } else {
                let action = UiAction::HandleEscape;
                self.update_text_target_after_action(&action);
                self.emit_model_action(action);
                handled = true;
            }
        }
        if !handled && matches!(event.logical_key, Key::Named(NamedKey::Backspace)) {
            handled = self.backspace_text();
        }
        if !handled && matches!(event.logical_key, Key::Named(NamedKey::Delete)) {
            handled = self.delete_text_forward();
        }
        if !handled
            && matches!(event.logical_key, Key::Named(NamedKey::Enter))
            && matches!(
                self.text_input_target,
                TextInputTarget::BrowserSearch
                    | TextInputTarget::FolderSearch
                    | TextInputTarget::WaveformBpm
            )
        {
            self.deactivate_text_input_target();
            handled = true;
        }
        if !handled
            && matches!(event.logical_key, Key::Named(NamedKey::Enter))
            && self.text_input_target == TextInputTarget::FolderCreate
        {
            handled = true;
            if folder_create_confirm_enabled(&self.model) {
                let action = UiAction::ConfirmFolderCreate;
                self.update_text_target_after_action(&action);
                self.emit_model_action(action.clone());
                self.refresh_cached_model_after_folder_create_action(&action);
            }
        }
        if !handled
            && matches!(event.logical_key, Key::Named(NamedKey::Enter))
            && self.text_input_target == TextInputTarget::None
            && matches!(
                self.model.focus_context,
                crate::app::FocusContextModel::SampleBrowser
            )
            && self.model.browser.duplicate_cleanup_active
        {
            self.emit_model_action(UiAction::ConfirmBrowserDuplicateCleanup);
            handled = true;
        }
        if !handled && let Some(key) = key {
            handled =
                match key {
                    KeyCode::ArrowUp => self
                        .step_waveform_bpm_input(if self.modifiers.shift_key() { 1 } else { 10 }),
                    KeyCode::ArrowDown => self
                        .step_waveform_bpm_input(if self.modifiers.shift_key() { -1 } else { -10 }),
                    _ => false,
                };
        }
        if !handled
            && self.text_input_target != TextInputTarget::None
            && let Some(key) = key
        {
            handled = self.move_text_cursor(key, self.modifiers.shift_key());
        }
        if !handled
            && self.text_input_target != TextInputTarget::None
            && (self.modifiers.control_key() || self.modifiers.super_key())
            && !self.modifiers.alt_key()
            && let Some(key) = key
        {
            handled = match key {
                KeyCode::A => self.select_all_text(),
                KeyCode::C => self.copy_selected_text(),
                KeyCode::V => self.paste_text(),
                KeyCode::X => self.cut_selected_text(),
                _ => false,
            };
        }
        if !handled
            && self.text_input_target != TextInputTarget::None
            && !self.modifiers.control_key()
            && !self.modifiers.super_key()
            && !self.modifiers.alt_key()
            && let Some(text) = event.text.as_ref()
        {
            let appended: String = text.chars().filter(|ch| !ch.is_control()).collect();
            if !appended.is_empty() {
                handled = self.append_text(&appended);
            }
        }
        if !handled
            && self.text_input_target == TextInputTarget::None
            && let Some(key) = key
        {
            handled = matches!(
                self.model.focus_context,
                crate::app::FocusContextModel::None
            ) && self.shell_state.handle_key(key);
            if !handled {
                let resolution =
                    action_from_key(key, self.modifiers, &self.model, self.pending_hotkey_chord);
                self.pending_hotkey_chord = resolution.pending_chord;
                if let Some(action) = resolution.action {
                    let action = rewrite_folder_create_hotkey_action(
                        action,
                        &self.model,
                        self.shell_state.hovered_folder_row_index(),
                    );
                    self.update_text_target_after_action(&action);
                    self.emit_model_action(action.clone());
                    self.refresh_cached_model_after_folder_create_action(&action);
                    handled = true;
                } else if resolution.handled {
                    handled = true;
                }
            }
        }
        if handled && !self.frame_state.has_pending_rebuild() {
            self.apply_invalidation_scope(RuntimeInvalidationScope::OverlayStateOnly);
        }
    }

    fn refresh_cached_model_after_folder_create_action(&mut self, action: &UiAction) {
        if folder_create_action_requires_immediate_model_refresh(action) {
            self.refresh_cached_model_for_pending_input();
        }
    }
}

fn folder_create_confirm_enabled(model: &AppModel) -> bool {
    model
        .sources
        .folder_rows
        .iter()
        .find(|row| {
            matches!(
                row.kind,
                crate::app::FolderRowKind::CreateDraft | crate::app::FolderRowKind::RenameDraft
            )
        })
        .and_then(|row| row.input_error.as_ref())
        .is_none_or(|error| error.trim().is_empty())
}

fn rewrite_folder_create_hotkey_action(
    action: UiAction,
    model: &AppModel,
    hovered_folder_row_index: Option<usize>,
) -> UiAction {
    if action != UiAction::StartNewFolder
        || !matches!(
            model.focus_context,
            crate::app::FocusContextModel::SourceFolders
        )
    {
        return action;
    }
    let Some(row_index) = hovered_folder_row_index else {
        return action;
    };
    let Some(row) = model.sources.folder_rows.get(row_index) else {
        return action;
    };
    if matches!(
        row.kind,
        crate::app::FolderRowKind::CreateDraft | crate::app::FolderRowKind::RenameDraft
    ) {
        return action;
    }
    row.source_index
        .map(|index| UiAction::StartNewFolderAtFolderRow { index })
        .unwrap_or(action)
}

fn folder_create_action_requires_immediate_model_refresh(action: &UiAction) -> bool {
    matches!(
        action,
        UiAction::StartNewFolder
            | UiAction::StartNewFolderAtFolderRow { .. }
            | UiAction::StartNewFolderAtRoot
            | UiAction::StartFolderRename
            | UiAction::ConfirmFolderCreate
            | UiAction::CancelFolderCreate
    )
}
