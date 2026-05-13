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
    /// Whether runtime-owned background work still has queued commands/messages.
    ///
    /// Native backends use this to keep the UI/event/render owner responsive:
    /// one drain pass handles a bounded slice of background commands/messages,
    /// then schedules another wakeup instead of monopolizing the UI path.
    pub runtime_work_remaining: bool,
}

const MAX_RUNTIME_MESSAGES_PER_DRAIN: usize = 64;
const MAX_RUNTIME_COMMANDS_PER_DRAIN: usize = 64;

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

fn take_runtime_command_batch_into<Message>(
    commands: &mut Vec<Command<Message>>,
    batch: &mut Vec<Command<Message>>,
) {
    debug_assert!(batch.is_empty());
    if !commands.iter().any(command_contains_runtime_batch) {
        if commands.len() <= MAX_RUNTIME_COMMANDS_PER_DRAIN {
            batch.extend(commands.drain(..).rev());
            return;
        }
        batch.extend(commands.drain(..MAX_RUNTIME_COMMANDS_PER_DRAIN).rev());
        debug_assert_eq!(batch.len(), MAX_RUNTIME_COMMANDS_PER_DRAIN);
        return;
    }

    let mut pending = std::mem::take(commands);
    let mut remaining = Vec::new();

    for command in pending.drain(..) {
        let budget = MAX_RUNTIME_COMMANDS_PER_DRAIN.saturating_sub(batch.len());
        collect_runtime_command_batch(command, batch, &mut remaining, budget);
    }

    batch.reverse();
    *commands = remaining;
    debug_assert!(batch.len() <= MAX_RUNTIME_COMMANDS_PER_DRAIN);
}

fn take_runtime_message_batch_into<Message>(messages: &mut Vec<Message>, batch: &mut Vec<Message>) {
    debug_assert!(batch.is_empty());
    if messages.len() <= MAX_RUNTIME_MESSAGES_PER_DRAIN {
        batch.extend(messages.drain(..).rev());
        return;
    }
    batch.extend(messages.drain(..MAX_RUNTIME_MESSAGES_PER_DRAIN).rev());
    debug_assert_eq!(batch.len(), MAX_RUNTIME_MESSAGES_PER_DRAIN);
}

fn collect_runtime_command_batch<Message>(
    command: Command<Message>,
    batch: &mut Vec<Command<Message>>,
    remaining: &mut Vec<Command<Message>>,
    budget: usize,
) {
    if budget == 0 {
        remaining.push(command);
        return;
    }
    match command {
        Command::None => {}
        Command::Batch(commands) => {
            for command in commands {
                let budget = MAX_RUNTIME_COMMANDS_PER_DRAIN.saturating_sub(batch.len());
                collect_runtime_command_batch(command, batch, remaining, budget);
            }
        }
        command => batch.push(command),
    }
}

fn command_contains_runtime_batch<Message>(command: &Command<Message>) -> bool {
    matches!(command, Command::Batch(_))
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

#[cfg(test)]
mod tests;
