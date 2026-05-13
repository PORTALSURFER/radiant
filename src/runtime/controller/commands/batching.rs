//! Bounded runtime command/message queue batching.

use crate::runtime::Command;

const MAX_RUNTIME_MESSAGES_PER_DRAIN: usize = 64;
const MAX_RUNTIME_COMMANDS_PER_DRAIN: usize = 64;

pub(super) fn take_runtime_command_batch_into<Message>(
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

pub(super) fn take_runtime_message_batch_into<Message>(
    messages: &mut Vec<Message>,
    batch: &mut Vec<Message>,
) {
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
