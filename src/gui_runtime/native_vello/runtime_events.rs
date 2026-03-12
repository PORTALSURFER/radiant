//! Event-loop entrypoints for the native Vello runtime.

use super::*;

impl<B: NativeAppBridge> ApplicationHandler<RuntimeUserEvent> for NativeVelloRunner<B> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.resumed_count = self.resumed_count.saturating_add(1);
        if self.resumed_count <= 2 {
            info!(
                "radiant native vello resumed event: resumed_count={}",
                self.resumed_count
            );
        }
        if self.window.is_none() {
            self.initialize_runtime(event_loop);
            self.request_redraw_if_needed();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if Some(window_id) != self.window_id {
            return;
        }
        self.window_event_count = self.window_event_count.saturating_add(1);
        match event {
            WindowEvent::CloseRequested => {
                warn!("radiant native vello close requested");
                event_loop.exit()
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if self.window_event_count <= 30 {
                    info!(
                        "scale factor changed: window_event_count={}",
                        self.window_event_count
                    );
                }
                self.apply_invalidation_scope(RuntimeInvalidationScope::LayoutAndAll);
            }
            WindowEvent::Resized(size) => {
                if self.window_event_count <= 30 && (size.width == 0 || size.height == 0) {
                    warn!(
                        width = size.width,
                        height = size.height,
                        "radiant native vello received zero-size resize"
                    );
                }
                if size.width > 0 && size.height > 0 && self.window.is_some() {
                    if let (Some(render_ctx), Some(surface)) =
                        (self.render_ctx.as_ref(), self.render_surface.as_mut())
                    {
                        render_ctx.resize_surface(surface, size.width, size.height);
                        self.apply_invalidation_scope(RuntimeInvalidationScope::LayoutAndAll);
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let point = Point::new(position.x as f32, position.y as f32);
                if self.last_cursor == Some(point) {
                    return;
                }
                self.last_cursor = Some(point);
                self.note_cursor_activity(Instant::now());
                self.update_waveform_resize_cursor(point);
                match self.active_pointer_session() {
                    ActivePointerSession::Volume => {
                        if let Some(layout) = self.shell_layout.as_ref()
                            && let Some(action) =
                                self.shell_state.top_bar_volume_drag_action(layout, point)
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
            WindowEvent::CursorLeft { .. } => {
                self.last_cursor = None;
                self.pending_cursor = None;
                self.set_cursor_icon(CursorIcon::Default);
            }
            WindowEvent::MouseInput {
                button,
                state: ElementState::Pressed,
                ..
            } if matches!(
                button,
                MouseButton::Left | MouseButton::Right | MouseButton::Middle
            ) =>
            {
                if self.window.is_none() {
                    return;
                }
                if let Some(point) = self.last_cursor {
                    let _ = self.with_shell_layout(|this, layout| {
                        this.begin_pointer_press_cycle();
                        let mut handled = false;
                        let mut action_emitted = false;
                        let mut source_menu_state_changed = false;
                        match button {
                            MouseButton::Left => {
                                let map_drag_start =
                                    this.model.map.active && layout.browser_rows.contains(point);
                                if let Some(action) = this
                                    .shell_state
                                    .source_context_menu_action_at_point(layout, &this.model, point)
                                {
                                    this.emit_model_action(action);
                                    action_emitted = true;
                                    source_menu_state_changed |=
                                        this.shell_state.close_source_context_menu();
                                    handled = true;
                                } else {
                                    source_menu_state_changed |=
                                        this.shell_state.close_source_context_menu();
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
                            this.apply_invalidation_scope(
                                RuntimeInvalidationScope::StaticAndOverlays,
                            );
                        } else if action_emitted
                            && handled
                            && !this.frame_state.has_pending_rebuild()
                        {
                            this.apply_invalidation_scope(
                                RuntimeInvalidationScope::OverlayStateOnly,
                            );
                        }
                    });
                }
            }
            WindowEvent::MouseInput {
                button,
                state: ElementState::Released,
                ..
            } if matches!(
                button,
                MouseButton::Left | MouseButton::Right | MouseButton::Middle
            ) =>
            {
                self.clear_pointer_release_state();
                self.finish_volume_drag(Some(button));
            }
            WindowEvent::MouseWheel { delta, .. } => self.handle_mouse_wheel(delta),
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::KeyboardInput { event, .. } => self.handle_keyboard_input(event),
            WindowEvent::RedrawRequested => self.redraw(event_loop),
            _ => {}
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: RuntimeUserEvent) {
        match event {
            RuntimeUserEvent::RepaintRequested => {
                self.repaint_event_pending.store(false, Ordering::Release);
                self.apply_invalidation_scope(RuntimeInvalidationScope::ModelAndOverlays);
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let has_pending_input = self.flush_pending_input();
        let needs_animation = self.shell_state.needs_animation();
        let now = Instant::now();
        self.maybe_force_reveal_startup_window_on_stall(now);
        let cursor_activity_redraw_deadline = if !needs_animation && !has_pending_input {
            self.next_cursor_activity_redraw_deadline(now)
        } else {
            None
        };
        let should_refresh_idle_status =
            !needs_animation && !has_pending_input && self.mark_idle_status_refresh_if_due(now);
        if needs_animation || has_pending_input || cursor_activity_redraw_deadline.is_some() {
            self.request_redraw_if_needed();
            let mut next_redraw_at = if let Some(deadline) = cursor_activity_redraw_deadline {
                deadline
            } else {
                let frame_interval = if self.shell_state.is_transport_running() {
                    self.target_frame_interval
                } else {
                    self.focus_animation_interval
                };
                self.last_redraw + frame_interval
            };
            if next_redraw_at < now {
                next_redraw_at = now;
            }
            event_loop.set_control_flow(ControlFlow::WaitUntil(next_redraw_at));
            return;
        }
        if should_refresh_idle_status {
            self.request_redraw_if_needed();
            event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_idle_status_refresh));
            return;
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_idle_status_refresh));
    }
}

impl<B: NativeAppBridge> NativeVelloRunner<B> {
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
            self.begin_waveform_scrollbar_drag(thumb_pointer_offset_x);
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
            *action_emitted = self.handle_pointer_press_action(action, map_drag_start);
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
            self.emit_model_action(UiAction::SelectSourceRow { index });
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
            *action_emitted = self.handle_pointer_press_action(action, false);
            return true;
        }
        false
    }

    fn handle_mouse_wheel(&mut self, delta: MouseScrollDelta) {
        let _ = self.with_shell_layout(|this, layout| {
            let waveform_zoom_action = this
                .last_cursor
                .and_then(|point| waveform_wheel_zoom_action(layout, &this.model, point, delta));
            let waveform_zoom_emitted = if let Some(action) = waveform_zoom_action {
                this.emit_model_action_with_profile(action, Some(InteractionProfileKind::Waveform));
                true
            } else {
                false
            };
            if !waveform_zoom_emitted {
                let fallback_point = Point::new(
                    (layout.browser_rows.min.x + layout.browser_rows.max.x) * 0.5,
                    (layout.browser_rows.min.y + layout.browser_rows.max.y) * 0.5,
                );
                let point = this
                    .last_cursor
                    .filter(|point| layout.browser_panel.contains(*point))
                    .unwrap_or(fallback_point);
                let style = this.cached_style_for_layout(layout);
                if let Some(delta) =
                    browser_wheel_row_delta(layout, &this.model, point, &style, delta)
                {
                    let viewport_len = this.shell_state.browser_viewport_len(layout, &this.model);
                    let current_view_start = this.model.browser.view_start_row;
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

    fn handle_keyboard_input(&mut self, event: winit::event::KeyEvent) {
        let key = match event.physical_key {
            PhysicalKey::Code(code) => key_code_from_winit(code),
            _ => None,
        };
        let allow_repeat = event.repeat && key.is_some_and(|key| self.allows_key_repeat(key));
        if event.state != ElementState::Pressed || (event.repeat && !allow_repeat) {
            return;
        }
        let mut handled = false;
        if matches!(event.logical_key, Key::Named(NamedKey::Escape)) {
            if self.model.confirm_prompt.visible {
                self.emit_model_action(UiAction::CancelPrompt);
                self.deactivate_text_input_target();
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
            handled = if self.model.confirm_prompt.visible {
                false
            } else {
                self.shell_state.handle_key(key)
            };
            if let Some(action) = action_from_key(key, self.modifiers, &self.model) {
                self.update_text_target_after_action(&action);
                self.emit_model_action(action);
                handled = true;
            }
        }
        if handled && !self.frame_state.has_pending_rebuild() {
            self.apply_invalidation_scope(RuntimeInvalidationScope::OverlayStateOnly);
        }
    }
}
