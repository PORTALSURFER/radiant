use super::{
    CommandConflict, CommandInput, CommandInvocation, CommandRegistry, CommandResolution,
    CommandScope, CommandSource, CommandSuppression, CommandTarget, Keymap,
};
use std::rc::Rc;

/// Outcome after optional mapping into an application message, before reducer execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandDispatchStatus {
    /// Exactly one invocation was mapped to a message for the normal reducer boundary.
    Mapped,
    /// No binding matched; a deliberate legacy fallback may continue.
    Unhandled,
    /// Input precedence or validation terminally suppressed dispatch.
    Suppressed(CommandSuppression),
    /// Enabled bindings were ambiguous.
    Conflict(CommandConflict),
    /// A presentation target belonged to an obsolete snapshot.
    Stale,
    /// No current enabled command was available.
    Unavailable,
}

/// A mapped message, if any, and the exact routing outcome.
pub struct CommandDispatch<Message> {
    /// Message to enqueue at the ordinary application reducer boundary.
    pub message: Option<Message>,
    /// Routing result; only Unhandled permits legacy fallback.
    pub status: CommandDispatchStatus,
}
impl<Message> CommandDispatch<Message> {
    /// Return an unmatched result without invoking a mapper.
    pub fn unhandled() -> Self {
        Self {
            message: None,
            status: CommandDispatchStatus::Unhandled,
        }
    }
    /// Return whether a legacy shortcut resolver may handle this input next.
    pub fn allows_fallback(&self) -> bool {
        self.status == CommandDispatchStatus::Unhandled
    }
}

/// One UI-local application mapper shared by shortcut and presentation activation paths.
pub struct CommandDispatcher<Context, Message> {
    mapper: Rc<dyn Fn(CommandInvocation<Context>) -> Message>,
}
impl<Context, Message> Clone for CommandDispatcher<Context, Message> {
    fn clone(&self) -> Self {
        Self {
            mapper: Rc::clone(&self.mapper),
        }
    }
}
impl<Context, Message> CommandDispatcher<Context, Message> {
    /// Register the application's single semantic invocation mapper.
    pub fn new(mapper: impl Fn(CommandInvocation<Context>) -> Message + 'static) -> Self {
        Self {
            mapper: Rc::new(mapper),
        }
    }
    /// Resolve current input and map at most one invocation without executing a reducer.
    pub fn input(
        &self,
        registry: &CommandRegistry,
        scopes: &[CommandScope<Context>],
        keymap: &Keymap,
        input: &CommandInput,
    ) -> CommandDispatch<Message> {
        self.map(registry.resolve(scopes, keymap, input))
    }
    /// Revalidate a current presentation target and use the same registered mapper.
    pub fn target(
        &self,
        registry: &CommandRegistry,
        scopes: &[CommandScope<Context>],
        target: &CommandTarget,
        source: CommandSource,
    ) -> CommandDispatch<Message> {
        self.map(registry.resolve_target(scopes, target, source))
    }
    fn map(&self, resolution: CommandResolution<Context>) -> CommandDispatch<Message> {
        let status = match resolution {
            CommandResolution::Invoked(invocation) => {
                return CommandDispatch {
                    message: Some((self.mapper)(invocation)),
                    status: CommandDispatchStatus::Mapped,
                };
            }
            CommandResolution::Unhandled => CommandDispatchStatus::Unhandled,
            CommandResolution::Suppressed(reason) => CommandDispatchStatus::Suppressed(reason),
            CommandResolution::Conflict(conflict) => CommandDispatchStatus::Conflict(conflict),
            CommandResolution::Stale => CommandDispatchStatus::Stale,
            CommandResolution::Unavailable => CommandDispatchStatus::Unavailable,
        };
        CommandDispatch {
            message: None,
            status,
        }
    }
}
