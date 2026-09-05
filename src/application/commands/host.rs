use super::{CommandInput, CommandScope, CommandSource, CommandTarget, Keymap};
use crate::{gui::focus::FocusSurface, widgets::WidgetId};

/// Current runtime focus supplied to an application's command projection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CommandFocus {
    /// Current focused widget identity, if any.
    pub widget: Option<WidgetId>,
    /// Optional host-defined semantic focus bucket.
    pub surface: FocusSurface,
}

/// Borrowed activation request; the host resolves it against current state.
#[derive(Clone, Copy)]
pub enum CommandRequest<'a> {
    /// Logical and physical input after text and composition precedence.
    Input(&'a CommandInput),
    /// A queued presentation activation requiring current identity validation.
    Target(&'a CommandTarget, CommandSource),
}

/// Application-owned immutable command state for one resolution or presentation pass.
///
/// Clone existing scopes to preserve their incarnation. Construct replacement scopes
/// when their captured context changes, so older presentation targets become stale.
pub struct CommandSnapshot<Context> {
    /// Current data-only keymap overrides.
    pub keymap: Keymap,
    /// Current active scopes; inactive editors and dismissed overlays must be omitted.
    pub scopes: Vec<CommandScope<Context>>,
}
impl<Context> Clone for CommandSnapshot<Context> {
    fn clone(&self) -> Self {
        Self {
            keymap: self.keymap.clone(),
            scopes: self.scopes.clone(),
        }
    }
}
