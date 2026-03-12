use super::*;

pub(super) fn sanitize_waveform_bpm_insert(
    current: &str,
    selection_range: (usize, usize),
    inserted: &str,
) -> String {
    let (selection_start, selection_end) = selection_range;
    let mut sanitized = String::with_capacity(inserted.len());
    let mut has_decimal =
        current[..selection_start].contains('.') || current[selection_end..].contains('.');
    for ch in inserted.chars() {
        if ch.is_ascii_digit() {
            sanitized.push(ch);
        } else if ch == '.' && !has_decimal {
            sanitized.push(ch);
            has_decimal = true;
        }
    }
    sanitized
}

pub(super) fn parse_waveform_bpm_input(text: &str) -> Option<f32> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let parsed = trimmed.parse::<f32>().ok()?;
    if !parsed.is_finite() || parsed <= 0.0 {
        return None;
    }
    Some(parsed)
}

pub(super) fn bpm_tenths_from_value(value: f32) -> u16 {
    let scaled = (value * 10.0).round();
    if !scaled.is_finite() {
        return 0;
    }
    scaled.clamp(0.0, u16::MAX as f32) as u16
}

impl<B: NativeAppBridge> NativeVelloRunner<B> {
    fn text_value_for_input_target(&self, target: TextInputTarget) -> Option<String> {
        match target {
            TextInputTarget::None => None,
            TextInputTarget::BrowserSearch => Some(
                self.current_text_value()
                    .unwrap_or_else(|| self.model.browser.search_query.clone()),
            ),
            TextInputTarget::WaveformBpm => Some(
                self.current_text_value()
                    .unwrap_or_else(|| self.waveform_bpm_text_from_model()),
            ),
            TextInputTarget::FolderSearch | TextInputTarget::PromptInput => None,
        }
    }

    fn text_input_rect_for_target(
        &mut self,
        layout: &ShellLayout,
        target: TextInputTarget,
    ) -> Option<UiRect> {
        match target {
            TextInputTarget::BrowserSearch => self
                .shell_state
                .browser_search_text_rect(layout, &self.model),
            TextInputTarget::WaveformBpm => {
                self.shell_state.waveform_bpm_text_rect(layout, &self.model)
            }
            TextInputTarget::None
            | TextInputTarget::FolderSearch
            | TextInputTarget::PromptInput => None,
        }
    }

    fn text_click_byte_index(
        &mut self,
        layout: &ShellLayout,
        point: Point,
        target: TextInputTarget,
    ) -> Option<usize> {
        let text_rect = self.text_input_rect_for_target(layout, target)?;
        let text = self.text_value_for_input_target(target)?;
        let font_size = self.cached_style_for_layout(layout).sizing.font_meta;
        let mut editor = self
            .text_editor_state
            .clone()
            .unwrap_or_else(|| SingleLineTextEditorState::collapsed_at_end(&text));
        let layout_state = build_text_field_layout(
            &mut self.text_renderer,
            &mut editor,
            &text,
            font_size,
            text_rect.width(),
        );
        Some(byte_index_for_local_x(
            &layout_state,
            (point.x - text_rect.min.x).clamp(0.0, text_rect.width()),
        ))
    }

    fn sync_text_editor_visual_state_for_target(&mut self, target: TextInputTarget) {
        match target {
            TextInputTarget::BrowserSearch => self.sync_browser_search_editor_state(),
            TextInputTarget::WaveformBpm => self.sync_waveform_bpm_editor_state(),
            TextInputTarget::None
            | TextInputTarget::FolderSearch
            | TextInputTarget::PromptInput => {}
        }
    }

    fn activate_pointer_text_input_target(&mut self, target: TextInputTarget) {
        match target {
            TextInputTarget::BrowserSearch => {
                if self.text_input_target != TextInputTarget::BrowserSearch {
                    self.emit_model_action(UiAction::FocusBrowserSearch);
                    self.activate_text_input_target(TextInputTarget::BrowserSearch);
                }
            }
            TextInputTarget::WaveformBpm => {
                if self.text_input_target != TextInputTarget::WaveformBpm {
                    self.activate_waveform_bpm_input();
                }
            }
            TextInputTarget::None
            | TextInputTarget::FolderSearch
            | TextInputTarget::PromptInput => {}
        }
    }

    fn handle_text_input_pointer_press(
        &mut self,
        layout: &ShellLayout,
        field_rect: UiRect,
        point: Point,
        extend_selection: bool,
        target: TextInputTarget,
    ) -> bool {
        if !field_rect.contains(point) {
            return false;
        }
        self.activate_pointer_text_input_target(target);
        let Some(byte_index) = self.text_click_byte_index(layout, point, target) else {
            return false;
        };
        let Some(text) = self.text_value_for_input_target(target) else {
            return false;
        };
        let editor = self
            .text_editor_state
            .get_or_insert_with(|| SingleLineTextEditorState::collapsed_at_end(&text));
        editor.set_cursor(&text, byte_index, extend_selection);
        self.text_input_drag_active = true;
        self.sync_text_editor_visual_state_for_target(target);
        self.apply_invalidation_scope(RuntimeInvalidationScope::OverlayStateOnly);
        true
    }

    pub(super) fn handle_browser_search_pointer_press(
        &mut self,
        layout: &ShellLayout,
        point: Point,
        extend_selection: bool,
    ) -> bool {
        let Some(field_rect) = self
            .shell_state
            .browser_search_field_rect(layout, &self.model)
        else {
            return false;
        };
        self.handle_text_input_pointer_press(
            layout,
            field_rect,
            point,
            extend_selection,
            TextInputTarget::BrowserSearch,
        )
    }

    pub(super) fn handle_waveform_bpm_pointer_press(
        &mut self,
        layout: &ShellLayout,
        point: Point,
        extend_selection: bool,
    ) -> bool {
        let Some(field_rect) = self
            .shell_state
            .waveform_bpm_input_rect(layout, &self.model)
        else {
            return false;
        };
        self.handle_text_input_pointer_press(
            layout,
            field_rect,
            point,
            extend_selection,
            TextInputTarget::WaveformBpm,
        )
    }

    pub(super) fn process_text_input_drag(&mut self, point: Point) -> bool {
        if !self.text_input_drag_active {
            return false;
        }
        let target = self.text_input_target;
        let Some((byte_index, text)) = self
            .with_shell_layout(|this, layout| {
                let byte_index = this.text_click_byte_index(layout, point, target)?;
                let text = this.text_value_for_input_target(target)?;
                Some((byte_index, text))
            })
            .flatten()
        else {
            return false;
        };
        let Some(editor) = self.text_editor_state.as_mut() else {
            return false;
        };
        editor.set_cursor(&text, byte_index, true);
        self.sync_text_editor_visual_state_for_target(target);
        self.apply_invalidation_scope(RuntimeInvalidationScope::OverlayStateOnly);
        true
    }

    pub(super) fn sync_text_input_target(&mut self) {
        if self.model.confirm_prompt.visible && self.model.confirm_prompt.input_value.is_some() {
            self.text_input_target = super::TextInputTarget::PromptInput;
        } else if self.text_input_target == super::TextInputTarget::PromptInput {
            self.text_input_target = super::TextInputTarget::None;
        }
        if self.text_input_target != super::TextInputTarget::None {
            match self.text_input_target {
                super::TextInputTarget::BrowserSearch
                | super::TextInputTarget::FolderSearch
                | super::TextInputTarget::PromptInput => {
                    if self.text_input_buffer.is_none() {
                        self.text_input_buffer = Some(match self.text_input_target {
                            super::TextInputTarget::BrowserSearch => {
                                self.model.browser.search_query.clone()
                            }
                            super::TextInputTarget::FolderSearch => {
                                self.model.sources.folder_search_query.clone()
                            }
                            super::TextInputTarget::PromptInput => self
                                .model
                                .confirm_prompt
                                .input_value
                                .clone()
                                .unwrap_or_default(),
                            super::TextInputTarget::None | super::TextInputTarget::WaveformBpm => {
                                String::new()
                            }
                        });
                    }
                }
                super::TextInputTarget::WaveformBpm => {
                    if self.waveform_bpm_input_buffer.is_none() {
                        self.waveform_bpm_input_buffer = Some(self.waveform_bpm_text_from_model());
                    }
                }
                super::TextInputTarget::None => {}
            }
            let current_text = self.current_text_value().unwrap_or_default();
            let mut editor = self
                .text_editor_state
                .take()
                .unwrap_or_else(|| SingleLineTextEditorState::collapsed_at_end(&current_text));
            editor.clamp_to_text(&current_text);
            self.text_editor_state = Some(editor);
        } else {
            self.text_input_buffer = None;
            self.text_editor_state = None;
            self.text_input_drag_active = false;
        }
        if self.text_input_target != super::TextInputTarget::WaveformBpm {
            self.waveform_bpm_input_buffer = None;
        }
        self.sync_waveform_bpm_editor_state();
        self.sync_browser_search_editor_state();
    }

    pub(super) fn current_text_value(&self) -> Option<String> {
        match self.text_input_target {
            super::TextInputTarget::None => None,
            super::TextInputTarget::BrowserSearch
            | super::TextInputTarget::FolderSearch
            | super::TextInputTarget::PromptInput => {
                self.text_input_buffer
                    .clone()
                    .or_else(|| match self.text_input_target {
                        super::TextInputTarget::BrowserSearch => {
                            Some(self.model.browser.search_query.clone())
                        }
                        super::TextInputTarget::FolderSearch => {
                            Some(self.model.sources.folder_search_query.clone())
                        }
                        super::TextInputTarget::PromptInput => {
                            self.model.confirm_prompt.input_value.clone()
                        }
                        super::TextInputTarget::None | super::TextInputTarget::WaveformBpm => None,
                    })
            }
            super::TextInputTarget::WaveformBpm => Some(
                self.waveform_bpm_input_buffer
                    .clone()
                    .unwrap_or_else(|| self.waveform_bpm_text_from_model()),
            ),
        }
    }

    pub(super) fn set_text_value(&mut self, value: String) -> bool {
        let action = match self.text_input_target {
            super::TextInputTarget::None => return false,
            super::TextInputTarget::BrowserSearch => {
                self.text_input_buffer = Some(value.clone());
                UiAction::SetBrowserSearch { query: value }
            }
            super::TextInputTarget::FolderSearch => {
                self.text_input_buffer = Some(value.clone());
                UiAction::SetFolderSearch { query: value }
            }
            super::TextInputTarget::PromptInput => {
                self.text_input_buffer = Some(value.clone());
                UiAction::SetPromptInput { value }
            }
            super::TextInputTarget::WaveformBpm => {
                self.waveform_bpm_input_buffer = Some(value.clone());
                self.sync_waveform_bpm_editor_state();
                self.apply_invalidation_scope(super::RuntimeInvalidationScope::StaticAndOverlays);
                if let Some(parsed) = parse_waveform_bpm_input(&value) {
                    UiAction::SetWaveformBpmValue {
                        value_tenths: bpm_tenths_from_value(parsed),
                    }
                } else {
                    return true;
                }
            }
        };
        self.emit_model_action(action);
        self.sync_browser_search_editor_state();
        true
    }

    pub(super) fn append_text(&mut self, appended: &str) -> bool {
        let appended = sanitize_single_line_insert(appended);
        if appended.is_empty() {
            return false;
        }
        let Some(value) = self.current_text_value() else {
            return false;
        };
        let Some(editor) = self.text_editor_state.as_mut() else {
            return false;
        };
        let sanitized = if self.text_input_target == super::TextInputTarget::WaveformBpm {
            sanitize_waveform_bpm_insert(&value, editor.selection_range(), &appended)
        } else {
            appended
        };
        if sanitized.is_empty() {
            return false;
        }
        let next = editor.replace_selection(&value, &sanitized);
        self.set_text_value(next)
    }

    pub(super) fn waveform_bpm_text_from_model(&self) -> String {
        self.model
            .waveform
            .tempo_label
            .as_deref()
            .and_then(crate::app::parse_waveform_tempo_number_text)
            .unwrap_or_else(|| String::from("120.0"))
    }
}
