//! Bounded runtime command/message queue batching.

use crate::runtime::Command;

pub(super) const DEFAULT_RUNTIME_MESSAGES_PER_DRAIN: usize = 64;
pub(super) const DEFAULT_RUNTIME_COMMANDS_PER_DRAIN: usize = 64;
pub(super) const INTERACTIVE_RUNTIME_MESSAGES_PER_DRAIN: usize = 8;
pub(super) const INTERACTIVE_RUNTIME_COMMANDS_PER_DRAIN: usize = 8;

pub(super) fn take_runtime_command_batch_into<Message>(
    commands: &mut Vec<Command<Message>>,
    batch: &mut Vec<Command<Message>>,
    max_commands: usize,
) {
    debug_assert!(batch.is_empty());
    let max_commands = max_commands.max(1);
    if !commands.iter().any(command_contains_runtime_batch) {
        if commands.len() <= max_commands {
            batch.extend(commands.drain(..).rev());
            return;
        }
        batch.extend(commands.drain(..max_commands).rev());
        debug_assert_eq!(batch.len(), max_commands);
        return;
    }

    let retained_capacity = commands.capacity();
    let mut pending = std::mem::replace(commands, Vec::with_capacity(retained_capacity));

    for command in pending.drain(..) {
        let budget = max_commands.saturating_sub(batch.len());
        collect_runtime_command_batch(command, batch, commands, budget, max_commands);
    }

    batch.reverse();
    debug_assert!(batch.len() <= max_commands);
}

pub(super) fn take_runtime_message_batch_into<Message>(
    messages: &mut Vec<Message>,
    batch: &mut Vec<Message>,
    max_messages: usize,
) {
    debug_assert!(batch.is_empty());
    let max_messages = max_messages.max(1);
    if messages.len() <= max_messages {
        batch.extend(messages.drain(..).rev());
        return;
    }
    batch.extend(messages.drain(..max_messages).rev());
    debug_assert_eq!(batch.len(), max_messages);
}

fn collect_runtime_command_batch<Message>(
    command: Command<Message>,
    batch: &mut Vec<Command<Message>>,
    remaining: &mut Vec<Command<Message>>,
    budget: usize,
    max_commands: usize,
) {
    if budget == 0 {
        remaining.push(command);
        return;
    }
    match command {
        Command::None => {}
        Command::Batch(commands) => {
            for command in commands {
                let budget = max_commands.saturating_sub(batch.len());
                collect_runtime_command_batch(command, batch, remaining, budget, max_commands);
            }
        }
        command => batch.push(command),
    }
}

fn command_contains_runtime_batch<Message>(command: &Command<Message>) -> bool {
    matches!(command, Command::Batch(_))
}
