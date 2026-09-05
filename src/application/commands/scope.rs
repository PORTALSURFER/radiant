use super::{CommandId, CommandRegistrationError};
use std::{cmp::Reverse, collections::BTreeSet, fmt, rc::Rc, sync::Arc};

/// Position in the documented command precedence, after required text editing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandScopeKind {
    /// Active modal; larger order is nearer the user.
    Modal {
        /// Current modal stacking order.
        order: u32,
    },
    /// Active overlay; larger order is nearer the user.
    Overlay {
        /// Current overlay stacking order.
        order: u32,
    },
    /// Ancestor of the current focused editor; larger depth is nearer.
    Editor {
        /// Depth in the current materialized focus ancestry.
        depth: u32,
    },
    /// Explicitly active work-surface or selection context.
    Selection,
    /// Current window context.
    Window,
    /// Explicit application-wide context.
    Application,
}

impl CommandScopeKind {
    pub(super) fn precedence(self) -> (u8, Reverse<u32>) {
        match self {
            Self::Modal { order } => (0, Reverse(order)),
            Self::Overlay { order } => (1, Reverse(order)),
            Self::Editor { depth } => (2, Reverse(depth)),
            Self::Selection => (3, Reverse(0)),
            Self::Window => (4, Reverse(0)),
            Self::Application => (5, Reverse(0)),
        }
    }
}

/// One command's dynamic state and opaque application-owned context.
pub struct CommandBinding<Context> {
    pub(super) id: CommandId,
    pub(super) enabled: bool,
    pub(super) checked: Option<bool>,
    pub(super) context: Rc<Context>,
}

impl<Context> Clone for CommandBinding<Context> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            enabled: self.enabled,
            checked: self.checked,
            context: Rc::clone(&self.context),
        }
    }
}

impl<Context> CommandBinding<Context> {
    /// Project an enabled binding with an owned context; no Context: Clone bound is needed.
    pub fn new(id: CommandId, context: Context) -> Self {
        Self {
            id,
            enabled: true,
            checked: None,
            context: Rc::new(context),
        }
    }
    /// Project current availability. Disabled bindings may decline to a lower scope.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    /// Project optional checked state for menus, tools, palettes and accessibility.
    pub fn checked(mut self, checked: Option<bool>) -> Self {
        self.checked = checked;
        self
    }
    /// Return the semantic identity.
    pub fn id(&self) -> &CommandId {
        &self.id
    }
    /// Return projected availability.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    /// Return projected checked state.
    pub fn checked_state(&self) -> Option<bool> {
        self.checked
    }
    /// Borrow the application context. The registry never interprets this value.
    pub fn context(&self) -> &Context {
        &self.context
    }
}

/// Immutable active-scope snapshot; replacing it invalidates previously projected targets.
pub struct CommandScope<Context> {
    pub(super) id: String,
    pub(super) kind: CommandScopeKind,
    pub(super) bindings: Vec<CommandBinding<Context>>,
    pub(super) identity: Rc<()>,
}

impl<Context> Clone for CommandScope<Context> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            kind: self.kind,
            bindings: self.bindings.clone(),
            identity: Rc::clone(&self.identity),
        }
    }
}

/// Invalid scope construction, before an active scope can be installed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandScopeError {
    /// The non-reusing declarative attachment identity space was exhausted.
    IdentityExhausted,
    /// Scope identity does not meet the stable-identifier bounds.
    Identity(CommandRegistrationError),
    /// A scope declared the same semantic command more than once.
    Duplicate(CommandId),
    /// More than 256 bindings were supplied for one scope.
    Capacity,
}
impl fmt::Display for CommandScopeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid command scope: {self:?}")
    }
}
impl std::error::Error for CommandScopeError {}

impl<Context> CommandScope<Context> {
    /// Validate and own one active snapshot. Cloning preserves identity; constructing a replacement retires old targets.
    pub fn new(
        id: impl Into<String>,
        kind: CommandScopeKind,
        bindings: impl IntoIterator<Item = CommandBinding<Context>>,
    ) -> Result<Self, CommandScopeError> {
        let id = id.into();
        CommandId::new(id.clone()).map_err(CommandScopeError::Identity)?;
        let mut seen = BTreeSet::new();
        let mut values = Vec::new();
        for binding in bindings {
            if values.len() == 256 {
                return Err(CommandScopeError::Capacity);
            }
            if !seen.insert(binding.id.clone()) {
                return Err(CommandScopeError::Duplicate(binding.id));
            }
            values.push(binding);
        }
        Ok(Self {
            id,
            kind,
            bindings: values,
            identity: Rc::new(()),
        })
    }
    /// Return the stable scope identity used for diagnostics and presentation.
    pub fn id(&self) -> &str {
        &self.id
    }
    /// Return the active precedence position.
    pub fn kind(&self) -> CommandScopeKind {
        self.kind
    }
    /// Borrow the immutable projected bindings.
    pub fn bindings(&self) -> &[CommandBinding<Context>] {
        &self.bindings
    }
}

/// Provenance of a resolved semantic invocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandSource {
    /// A logical or physical shortcut.
    Shortcut,
    /// A menu item activation.
    Menu,
    /// A toolbar control activation.
    Toolbar,
    /// A command palette selection.
    Palette,
    /// An admitted accessibility action.
    Accessibility,
    /// An explicit application-owned command request.
    Application,
}

/// Typed data delivered through the application's one registered dispatch mapper.
pub struct CommandInvocation<Context> {
    pub(super) id: CommandId,
    pub(super) context: Rc<Context>,
    pub(super) source: CommandSource,
    pub(super) scope: String,
}
impl<Context> Clone for CommandInvocation<Context> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            context: Rc::clone(&self.context),
            source: self.source,
            scope: self.scope.clone(),
        }
    }
}
impl<Context> CommandInvocation<Context> {
    /// Return the semantic command identity.
    pub fn id(&self) -> &CommandId {
        &self.id
    }
    /// Borrow the resolved application context.
    pub fn context(&self) -> &Context {
        &self.context
    }
    /// Return the input's provenance.
    pub fn source(&self) -> CommandSource {
        self.source
    }
    /// Return the scope selected by resolution.
    pub fn scope(&self) -> &str {
        &self.scope
    }
}

/// Opaque presentation target fenced to an immutable registry and scope snapshot.
#[derive(Clone, Debug)]
pub struct CommandTarget {
    pub(super) registry: Arc<()>,
    pub(super) scope_identity: Rc<()>,
    pub(super) scope: String,
    pub(super) command: CommandId,
}
impl CommandTarget {
    /// Return the semantic identity advertised by this target.
    pub fn command(&self) -> &CommandId {
        &self.command
    }
    /// Return the advertised scope identity.
    pub fn scope(&self) -> &str {
        &self.scope
    }
}
