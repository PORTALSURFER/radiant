//! Winit application lifecycle for the generic native Vello runner.

use super::*;

impl<Bridge, Message> ApplicationHandler<RuntimeUserEvent>
    for GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            self.initialize_runtime(event_loop);
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
        match event {
            WindowEvent::CloseRequested => {
                if self.core.runtime.bridge_mut().close_requested() {
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(size) => self.resize_surface(size),
            WindowEvent::ScaleFactorChanged { .. } => self.request_redraw_if_needed(),
            WindowEvent::CursorMoved { position, .. } => {
                let position = Point::new(position.x as f32, position.y as f32);
                let previous = self.last_cursor;
                self.last_cursor = Some(position);
                if self.native_hover_surface_contains(position) {
                    self.update_gpu_surface_cursor_overlay(position);
                    self.request_redraw_if_needed();
                    return;
                }
                if previous.is_some_and(|previous| self.native_hover_surface_contains(previous))
                    && previous
                        .is_some_and(|previous| self.clear_gpu_surface_cursor_overlay(previous))
                {
                    self.request_redraw_if_needed();
                    return;
                }
                let started = Instant::now();
                let outcome = self.core.route_pointer_move(position);
                maybe_log_route_profile("pointer_move", started.elapsed(), outcome);
                self.handle_gpu_surface_pointer_move_outcome(outcome, previous, position);
            }
            WindowEvent::CursorLeft { .. } => {
                if let Some(previous) = self.last_cursor
                    && self.clear_gpu_surface_cursor_overlay(previous)
                {
                    self.request_redraw_if_needed();
                }
                self.last_cursor = None;
            }
            WindowEvent::MouseInput { button, state, .. } => {
                let Some(position) = self.last_cursor else {
                    return;
                };
                let Some(button) = pointer_button_from_winit(button) else {
                    return;
                };
                let started = Instant::now();
                let routed = match state {
                    ElementState::Pressed => self.core.route_pointer_press(position, button),
                    ElementState::Released => self.core.route_pointer_release(position, button),
                };
                maybe_log_route_profile("pointer_button", started.elapsed(), routed);
                self.handle_route_outcome(event_loop, routed);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let Some(position) = self.last_cursor else {
                    return;
                };
                let delta = scroll_delta_to_logical(delta);
                if self.can_coalesce_gpu_surface_wheel(position, delta) {
                    self.queue_gpu_surface_wheel(position, delta);
                    return;
                }
                let started = Instant::now();
                let routed = if self.can_fast_path_gpu_surface_route(position, delta) {
                    self.core.route_scroll_deferred_refresh(position, delta)
                } else {
                    self.core.route_scroll(position, delta)
                };
                maybe_log_route_profile("wheel", started.elapsed(), routed);
                self.handle_gpu_surface_route_outcome(routed, position, delta);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.handle_keyboard_event(event_loop, event)
            }
            WindowEvent::ModifiersChanged(modifiers) => self.modifiers = modifiers.state(),
            WindowEvent::RedrawRequested => self.redraw(event_loop),
            _ => {}
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: RuntimeUserEvent) {
        match event {
            RuntimeUserEvent::RepaintRequested => {
                self.repaint_event_pending
                    .store(false, std::sync::atomic::Ordering::Release);
                let outcome = self.core.drain_runtime_messages();
                if outcome.exit_requested {
                    event_loop.exit();
                    return;
                }
                self.rebuild_scene();
                self.request_redraw_if_needed();
                self.request_runtime_wakeup_if_needed(outcome);
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            event_loop.set_control_flow(ControlFlow::Wait);
            return;
        }
        let needs_animation = self.core.needs_animation();
        if !needs_animation && !self.core.has_focused_text_input() {
            event_loop.set_control_flow(ControlFlow::Wait);
            return;
        }
        let now = Instant::now();
        let interval = animation_frame_interval(self.options.target_fps);
        let next_frame = self.last_redraw.checked_add(interval).unwrap_or(now);
        if now >= next_frame {
            if !self.redraw_requested {
                if needs_animation {
                    self.core.queue_animation_frame();
                }
                let outcome = self.core.drain_runtime_messages();
                if outcome.exit_requested {
                    event_loop.exit();
                    return;
                }
                self.rebuild_scene();
                self.request_redraw_if_needed();
                self.request_runtime_wakeup_if_needed(outcome);
            }
            event_loop.set_control_flow(ControlFlow::WaitUntil(now + interval));
        } else {
            event_loop.set_control_flow(ControlFlow::WaitUntil(next_frame));
        }
    }
}
