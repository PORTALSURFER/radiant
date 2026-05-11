use super::*;

/// Summary of one command-dispatch pass through a [`SurfaceRuntime`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CommandOutcome {
    /// Number of host-defined messages reduced during this pass.
    pub messages_dispatched: usize,
    /// Whether any command requested a repaint.
    pub repaint_requested: bool,
    /// Whether this pass requires declarative surface reprojection and layout.
    pub surface_refresh_requested: bool,
    /// Whether any command requested runtime exit.
    pub exit_requested: bool,
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Reduce one host-defined message and execute its runtime-visible command.
    pub fn dispatch_message(&mut self, message: Message) -> CommandOutcome {
        let mut outcome = CommandOutcome::default();
        self.dispatch_message_inner(message, &mut outcome);
        self.refresh_if_requested(outcome.surface_refresh_requested);
        if outcome.surface_refresh_requested {
            self.repaint_requested = true;
        }
        if outcome.exit_requested {
            self.exit_requested = true;
        }
        outcome
    }

    /// Execute a command without an initial widget message.
    ///
    /// This is useful for backend adapters or host shells that need to replay a
    /// queued command through the same message/repaint handling path used by
    /// widget dispatch.
    pub fn execute_command(&mut self, command: Command<Message>) -> CommandOutcome {
        let mut outcome = CommandOutcome::default();
        self.execute_command_inner(command, &mut outcome);
        self.refresh_if_requested(outcome.surface_refresh_requested);
        if outcome.surface_refresh_requested {
            self.repaint_requested = true;
        }
        if outcome.exit_requested {
            self.exit_requested = true;
        }
        outcome
    }

    /// Dispatch any messages queued by bridge-owned runtime work.
    pub fn drain_runtime_messages(&mut self) -> CommandOutcome {
        let mut outcome = CommandOutcome::default();
        self.runtime_commands.clear();
        self.bridge
            .drain_runtime_commands_into(&mut self.runtime_commands);
        let mut commands = std::mem::take(&mut self.runtime_commands);
        for command in commands.drain(..) {
            self.execute_command_inner(command, &mut outcome);
        }
        self.runtime_commands = commands;

        self.runtime_messages.clear();
        self.bridge
            .drain_runtime_messages_into(&mut self.runtime_messages);
        let mut messages = std::mem::take(&mut self.runtime_messages);
        for message in messages.drain(..) {
            self.dispatch_message_inner(message, &mut outcome);
        }
        self.runtime_messages = messages;

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
        let paint_only = command_requests_paint_only(&command);
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
            }
            Command::RequestPaintOnly => {
                self.repaint_requested = true;
                outcome.repaint_requested = true;
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
                outcome.repaint_requested |= self.focus_widget(widget_id);
            }
            Command::Exit => {
                outcome.exit_requested = true;
                self.exit_requested = true;
            }
        }
    }
}

fn command_requests_paint_only<Message>(command: &Command<Message>) -> bool {
    match command {
        Command::RequestPaintOnly => true,
        Command::Batch(commands) => commands.iter().any(command_requests_paint_only),
        Command::None
        | Command::Message(_)
        | Command::RequestRepaint
        | Command::After { .. }
        | Command::Perform { .. }
        | Command::Focus(_)
        | Command::Exit => false,
    }
}
