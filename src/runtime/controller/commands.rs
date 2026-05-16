use super::*;

mod batching;
mod external_drag;
mod scrolling;

use batching::{take_runtime_command_batch_into, take_runtime_message_batch_into};

/// Summary of one command-dispatch pass through a [`SurfaceRuntime`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CommandOutcome {
    /// Number of host-defined messages reduced during this pass.
    pub messages_dispatched: usize,
    /// Whether any command requested a repaint.
    pub repaint_requested: bool,
    /// Whether any command requested a redraw without surface reprojection.
    pub paint_only_requested: bool,
    /// Whether any command requested a repaint of the projected surface.
    pub surface_repaint_requested: bool,
    /// Whether this pass requires declarative surface reprojection and layout.
    pub surface_refresh_requested: bool,
    /// Whether any command requested runtime exit.
    pub exit_requested: bool,
    /// Whether runtime-owned background work still has queued commands/messages.
    ///
    /// Native backends use this to keep the UI/event/render owner responsive:
    /// one drain pass handles a bounded slice of background commands/messages,
    /// then schedules another wakeup instead of monopolizing the UI path.
    pub runtime_work_remaining: bool,
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Reduce one host-defined message and execute its runtime-visible command.
    pub fn dispatch_message(&mut self, message: Message) -> CommandOutcome {
        let mut outcome = CommandOutcome::default();
        self.dispatch_message_inner(message, &mut outcome);
        self.finish_command_outcome(outcome)
    }

    /// Execute a command without an initial widget message.
    ///
    /// This is useful for backend adapters or host shells that need to replay a
    /// queued command through the same message/repaint handling path used by
    /// widget dispatch.
    pub fn execute_command(&mut self, command: Command<Message>) -> CommandOutcome {
        let mut outcome = CommandOutcome::default();
        self.execute_command_inner(command, &mut outcome);
        self.finish_command_outcome(outcome)
    }

    /// Dispatch any messages queued by bridge-owned runtime work.
    pub fn drain_runtime_messages(&mut self) -> CommandOutcome {
        let mut outcome = CommandOutcome::default();
        self.bridge
            .drain_runtime_commands_into(&mut self.runtime_commands);
        take_runtime_command_batch_into(
            &mut self.runtime_commands,
            &mut self.runtime_command_batch,
        );
        let mut command_batch = std::mem::take(&mut self.runtime_command_batch);
        while let Some(command) = command_batch.pop() {
            self.execute_command_inner(command, &mut outcome);
        }
        self.runtime_command_batch = command_batch;

        self.bridge
            .drain_runtime_messages_into(&mut self.runtime_messages);
        take_runtime_message_batch_into(
            &mut self.runtime_messages,
            &mut self.runtime_message_batch,
        );
        while let Some(message) = self.runtime_message_batch.pop() {
            self.dispatch_message_inner(message, &mut outcome);
        }

        if !self.runtime_commands.is_empty() || !self.runtime_messages.is_empty() {
            outcome.runtime_work_remaining = true;
            outcome.repaint_requested = true;
            self.repaint_requested = true;
        }

        self.finish_command_outcome(outcome)
    }

    fn finish_command_outcome(&mut self, outcome: CommandOutcome) -> CommandOutcome {
        self.refresh_if_requested(outcome.surface_refresh_requested);
        if outcome.surface_refresh_requested {
            self.repaint_requested = true;
        }
        if outcome.exit_requested {
            self.exit_requested = true;
        }
        outcome
    }

    fn refresh_if_requested(&mut self, requested: bool) {
        if requested {
            self.refresh();
        }
    }

    pub(super) fn dispatch_message_inner(
        &mut self,
        message: Message,
        outcome: &mut CommandOutcome,
    ) {
        let refresh_before = outcome.surface_refresh_requested;
        outcome.messages_dispatched += 1;
        let command = self.bridge.update(message);
        let paint_only = command.requests_paint_only();
        let messages_before_command = outcome.messages_dispatched;
        self.execute_command_inner(command, outcome);
        let command_dispatched_messages = outcome.messages_dispatched > messages_before_command;
        if !paint_only || command_dispatched_messages {
            outcome.surface_refresh_requested = true;
        } else {
            outcome.surface_refresh_requested = refresh_before;
        }
    }

    fn execute_command_inner(&mut self, command: Command<Message>, outcome: &mut CommandOutcome) {
        match command {
            Command::None => {}
            Command::Message(message) => self.dispatch_message_inner(message, outcome),
            Command::Batch(commands) => {
                for command in commands {
                    self.execute_command_inner(command, outcome);
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
            Command::After { delay, message } => {
                if self.bridge.schedule_message(delay, message) {
                    outcome.repaint_requested = true;
                }
            }
            Command::Perform { name, work } => {
                if self.bridge.spawn_message_task(name, work) {
                    outcome.repaint_requested = true;
                }
            }
            Command::Focus(widget_id) => {
                let focused = self.focus_widget(widget_id);
                outcome.repaint_requested |= focused;
                outcome.surface_repaint_requested |= focused;
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
                self.external_drag_session = Some(ExternalDragSession::new(request, on_completed));
            }
            Command::PlatformRequest {
                request,
                on_completed,
            } => {
                if let Err((_request, on_completed)) =
                    self.bridge.request_platform_service(request, on_completed)
                {
                    let message = on_completed(Err(String::from(
                        "platform service requests are not supported by this runtime bridge",
                    )));
                    self.dispatch_message_inner(message, outcome);
                }
            }
            Command::EndExternalDrag => {
                self.external_drag_session = None;
            }
            Command::Exit => {
                outcome.exit_requested = true;
                self.exit_requested = true;
            }
        }
    }
}

#[cfg(test)]
mod tests;
