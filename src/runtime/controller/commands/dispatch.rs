use super::{CommandOutcome, SurfaceRuntime};
use crate::runtime::RepaintScope;
use crate::runtime::RuntimeUpdateSnapshot;
use crate::runtime::UiUpdateHandlerDiagnosticsMode;
use crate::{
    gui::types::Vector2,
    runtime::{Command, DragSession, ExternalDragSession, RuntimeBridge},
};
use std::{any::type_name, panic::panic_any, time::Instant};

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(in crate::runtime::controller) fn dispatch_message_inner(
        &mut self,
        message: Message,
        outcome: &mut CommandOutcome,
    ) {
        self.dispatch_message_inner_with_refresh(message, outcome, true);
    }

    pub(in crate::runtime::controller) fn dispatch_message_inner_deferred_refresh(
        &mut self,
        message: Message,
        outcome: &mut CommandOutcome,
    ) {
        self.dispatch_message_inner_with_refresh(message, outcome, false);
    }

    fn dispatch_message_inner_with_refresh(
        &mut self,
        message: Message,
        outcome: &mut CommandOutcome,
        refresh_surface: bool,
    ) {
        let mut deferred_surface_is_fresh = refresh_surface;
        self.dispatch_message_inner_with_refresh_state(
            message,
            outcome,
            refresh_surface,
            &mut deferred_surface_is_fresh,
        );
    }

    fn dispatch_message_inner_with_refresh_state(
        &mut self,
        message: Message,
        outcome: &mut CommandOutcome,
        refresh_surface: bool,
        deferred_surface_is_fresh: &mut bool,
    ) {
        let refresh_before = outcome.surface_refresh_requested;
        outcome.messages_dispatched += 1;
        let command = self.run_update_handler(message);
        if !refresh_surface {
            *deferred_surface_is_fresh = false;
            outcome.surface_refresh_applied = false;
        }
        let repaint_scope = command.repaint_scope().unwrap_or(RepaintScope::Surface);
        let requires_fresh_surface = command.requires_fresh_surface_before_dispatch();
        let effective_scope = if requires_fresh_surface {
            RepaintScope::Surface
        } else {
            repaint_scope
        };
        let paint_only = effective_scope.is_paint_only();
        if paint_only {
            self.refresh_with_scope(RepaintScope::PaintOnly);
        }
        if (refresh_surface && !paint_only) || requires_fresh_surface {
            self.refresh_with_scope(effective_scope);
            *deferred_surface_is_fresh = true;
            outcome.surface_refresh_applied = true;
        }
        let messages_before_command = outcome.messages_dispatched;
        self.execute_command_inner_with_refresh_state(
            command,
            outcome,
            refresh_surface,
            deferred_surface_is_fresh,
        );
        let command_dispatched_messages = outcome.messages_dispatched > messages_before_command;
        if !paint_only || command_dispatched_messages {
            outcome.surface_refresh_requested = true;
            outcome.surface_refresh_scope = Some(
                outcome
                    .surface_refresh_scope
                    .map_or(effective_scope, |current| current.merge(effective_scope)),
            );
        } else {
            outcome.surface_refresh_requested = refresh_before;
        }
    }

    fn run_update_handler(&mut self, message: Message) -> Command<Message> {
        let policy = self.update_handler_diagnostics_policy;
        let Some(threshold) = policy.threshold() else {
            return self.run_update_handler_with_snapshot(message);
        };
        let update_started = Instant::now();
        let command = self.run_update_handler_with_snapshot(message);
        let slow = self.diagnostics.record_update_handler(
            update_started.elapsed(),
            threshold,
            type_name::<Bridge>(),
            type_name::<Message>(),
        );
        if let (UiUpdateHandlerDiagnosticsMode::Panic, Some(diagnostic)) = (policy.mode(), slow) {
            panic_any(diagnostic.failure_message());
        }
        command
    }

    fn run_update_handler_with_snapshot(&mut self, message: Message) -> Command<Message> {
        let snapshot =
            RuntimeUpdateSnapshot::with_current_pointer_position(self.current_pointer_position());
        self.bridge.update_with_runtime(message, snapshot)
    }

    pub(in crate::runtime::controller) fn execute_command_inner(
        &mut self,
        command: Command<Message>,
        outcome: &mut CommandOutcome,
    ) {
        if command.requests_paint_only() {
            self.refresh_with_scope(RepaintScope::PaintOnly);
        }
        let mut deferred_surface_is_fresh = true;
        self.execute_command_inner_with_refresh_state(
            command,
            outcome,
            true,
            &mut deferred_surface_is_fresh,
        );
    }

    pub(in crate::runtime::controller) fn execute_command_inner_deferred_refresh(
        &mut self,
        command: Command<Message>,
        outcome: &mut CommandOutcome,
    ) {
        if command.requests_paint_only() {
            self.refresh_with_scope(RepaintScope::PaintOnly);
        }
        let mut deferred_surface_is_fresh = false;
        self.execute_command_inner_with_refresh_state(
            command,
            outcome,
            false,
            &mut deferred_surface_is_fresh,
        );
    }

    fn execute_command_inner_with_refresh_state(
        &mut self,
        command: Command<Message>,
        outcome: &mut CommandOutcome,
        refresh_surface: bool,
        deferred_surface_is_fresh: &mut bool,
    ) {
        if !refresh_surface
            && outcome.surface_refresh_requested
            && !*deferred_surface_is_fresh
            && command.requires_fresh_surface_before_dispatch()
        {
            self.refresh_with_scope(RepaintScope::Surface);
            *deferred_surface_is_fresh = true;
            outcome.surface_refresh_applied = true;
        }
        match command {
            Command::None => {}
            Command::Message(message) => {
                self.dispatch_message_inner_with_refresh_state(
                    message,
                    outcome,
                    refresh_surface,
                    deferred_surface_is_fresh,
                );
            }
            Command::Batch(commands) => {
                for command in commands {
                    self.execute_command_inner_with_refresh_state(
                        command,
                        outcome,
                        refresh_surface,
                        deferred_surface_is_fresh,
                    );
                }
            }
            Command::RequestRepaint => {
                self.repaint_requested = true;
                outcome.repaint_requested = true;
                outcome.surface_repaint_requested = true;
            }
            Command::RequestPaintOnly => {
                self.repaint_requested = true;
                outcome.repaint_requested = true;
                outcome.paint_only_requested = true;
            }
            Command::RequestProjectionRefresh => {
                self.repaint_requested = true;
                outcome.repaint_requested = true;
                outcome.surface_repaint_requested = true;
                outcome.request_surface_refresh(RepaintScope::Projection);
            }
            Command::RequestLayoutRefresh => {
                self.repaint_requested = true;
                outcome.repaint_requested = true;
                outcome.surface_repaint_requested = true;
                outcome.request_surface_refresh(RepaintScope::Layout);
            }
            Command::SetDpiScale(scale) => {
                self.repaint_requested = true;
                outcome.repaint_requested = true;
                outcome.surface_repaint_requested = true;
                outcome.request_surface_refresh(RepaintScope::Surface);
                outcome.dpi_scale_override = Some(scale);
            }
            Command::SetWindowLogicalSize(size) => {
                self.repaint_requested = true;
                outcome.repaint_requested = true;
                outcome.surface_repaint_requested = true;
                outcome.request_surface_refresh(RepaintScope::Surface);
                outcome.window_logical_size = Some(size);
            }
            Command::After { delay, message } => {
                if self.host_schedule_message(delay, message) {
                    outcome.repaint_requested = true;
                }
            }
            Command::Perform {
                name,
                priority,
                is_cancelled,
                work,
            } => {
                if self.host_spawn_message_task(name, priority, is_cancelled, work) {
                    outcome.repaint_requested = true;
                }
            }
            Command::PerformStream {
                name,
                priority,
                is_cancelled,
                work,
            } => {
                if self.host_spawn_streaming_message_task(name, priority, is_cancelled, work) {
                    outcome.repaint_requested = true;
                }
            }
            Command::PerformStreamLatest {
                name,
                priority,
                is_cancelled,
                work,
            } => {
                if self.host_spawn_latest_streaming_message_task(name, priority, is_cancelled, work)
                {
                    outcome.repaint_requested = true;
                }
            }
            Command::Focus(widget_id) => {
                let focused = self.focus_widget(widget_id);
                outcome.repaint_requested |= focused;
                outcome.surface_repaint_requested |= focused;
            }
            Command::ClearFocus => {
                let had_focus = self.focused_widget().is_some();
                self.clear_focus();
                outcome.repaint_requested |= had_focus;
                outcome.surface_repaint_requested |= had_focus;
            }
            Command::ScrollTo { node_id, offset } => {
                let offset = Vector2::new(offset.x.max(0.0), offset.y.max(0.0));
                self.scroll_to_offset(node_id, offset);
                self.repaint_requested = true;
                outcome.repaint_requested = true;
                outcome.surface_repaint_requested = true;
            }
            Command::ScrollIntoView {
                node_id,
                target_y,
                target_height,
                margin_top,
                margin_bottom,
                snap_y,
            } => {
                if let Some(offset) = self.scroll_into_view_offset(
                    node_id,
                    target_y,
                    target_height,
                    margin_top,
                    margin_bottom,
                    snap_y,
                ) {
                    self.scroll_to_offset(node_id, offset);
                }
                self.repaint_requested = true;
                outcome.repaint_requested = true;
                outcome.surface_repaint_requested = true;
            }
            Command::ScrollFixedRowIntoView {
                node_id,
                row_index,
                row_stride,
                leading_context_rows,
                trailing_context_rows,
                direction,
            } => {
                if let Some(offset) = self.scroll_fixed_row_into_view_offset(
                    node_id,
                    row_index,
                    row_stride,
                    leading_context_rows,
                    trailing_context_rows,
                    direction,
                ) {
                    self.scroll_to_offset(node_id, offset);
                }
                self.repaint_requested = true;
                outcome.repaint_requested = true;
                outcome.surface_repaint_requested = true;
            }
            Command::BeginExternalDrag {
                request,
                on_completed,
            } => {
                self.interaction.drag.external_session =
                    Some(ExternalDragSession::new(request, on_completed));
            }
            Command::BeginDrag { request } => {
                self.interaction.drag.session = Some(DragSession::new(request));
                self.repaint_requested = true;
                outcome.repaint_requested = true;
                outcome.surface_repaint_requested = true;
            }
            Command::PlatformRequest {
                request,
                on_completed,
            } => {
                if let Err(fallback) = self.host_request_platform_service(request, on_completed) {
                    let (_request, on_completed) = *fallback;
                    let message = on_completed(Err(String::from(
                        "platform service requests are not supported by this runtime bridge",
                    )));
                    self.dispatch_message_inner_with_refresh_state(
                        message,
                        outcome,
                        refresh_surface,
                        deferred_surface_is_fresh,
                    );
                }
            }
            Command::EndExternalDrag => {
                self.interaction.drag.external_session = None;
            }
            Command::EndDrag => {
                self.interaction.drag.session = None;
                self.repaint_requested = true;
                outcome.repaint_requested = true;
                outcome.surface_repaint_requested = true;
            }
            Command::Exit => {
                outcome.exit_requested = true;
                self.exit_requested = true;
            }
        }
    }
}
