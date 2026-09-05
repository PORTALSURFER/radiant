use super::CommandShortcut;
use crate::application::TextKey;
use std::{collections::BTreeMap, fmt, sync::Arc};

/// Stable application-defined command identifier used by every presentation and keymap.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandId(Arc<str>);

impl CommandId {
    /// Validate a nonempty identifier of at most 256 UTF-8 bytes, without control characters.
    pub fn new(value: impl Into<String>) -> Result<Self, CommandRegistrationError> {
        let value = value.into();
        if value.is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
            return Err(CommandRegistrationError::InvalidId);
        }
        Ok(Self(Arc::from(value)))
    }
    /// Borrow the persistent identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CommandId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Immutable static command metadata; dynamic availability belongs to an active scope.
#[derive(Clone, Debug)]
pub struct CommandDescriptor {
    pub(super) id: CommandId,
    pub(super) label: TextKey,
    pub(super) description: Option<TextKey>,
    pub(super) category: Option<TextKey>,
    pub(super) accessibility: Option<TextKey>,
    pub(super) defaults: Vec<CommandShortcut>,
    pub(super) repeat: bool,
}

impl CommandDescriptor {
    /// Register a stable identity and one localized label source.
    pub fn new(id: CommandId, label: TextKey) -> Self {
        Self {
            id,
            label,
            description: None,
            category: None,
            accessibility: None,
            defaults: Vec::new(),
            repeat: false,
        }
    }
    /// Set the localized description.
    pub fn description(mut self, value: TextKey) -> Self {
        self.description = Some(value);
        self
    }
    /// Set the localized command category.
    pub fn category(mut self, value: TextKey) -> Self {
        self.category = Some(value);
        self
    }
    /// Set an explicit accessibility label; otherwise the visible label is used.
    pub fn accessibility(mut self, value: TextKey) -> Self {
        self.accessibility = Some(value);
        self
    }
    /// Append one default logical or physical binding.
    pub fn default_binding(mut self, binding: CommandShortcut) -> Self {
        self.defaults.push(binding);
        self
    }
    /// Permit repeated key presses; the default suppresses repeats terminally.
    pub fn repeats(mut self, allowed: bool) -> Self {
        self.repeat = allowed;
        self
    }
    /// Return the stable identifier.
    pub fn id(&self) -> &CommandId {
        &self.id
    }
    /// Return the single label source used by all presentations.
    pub fn label(&self) -> &TextKey {
        &self.label
    }
    /// Return the default bindings before keymap overrides.
    pub fn default_bindings(&self) -> &[CommandShortcut] {
        &self.defaults
    }
}

/// Static registry construction failed before any command became executable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandRegistrationError {
    /// An identifier was empty, over the bound, or contained control characters.
    InvalidId,
    /// The same stable command was registered more than once.
    DuplicateId(CommandId),
    /// A command declared an invalid or duplicate exact default binding.
    InvalidBinding(CommandId),
    /// The registry exceeded 4,096 commands or a command exceeded 32 bindings.
    Capacity,
}

impl fmt::Display for CommandRegistrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "command registration failed: {self:?}")
    }
}
impl std::error::Error for CommandRegistrationError {}

/// Immutable metadata registry, containing no invocation callbacks or domain state.
#[derive(Clone, Debug)]
pub struct CommandRegistry {
    pub(super) commands: Arc<BTreeMap<CommandId, CommandDescriptor>>,
    pub(super) identity: Arc<()>,
}

impl CommandRegistry {
    /// Validate all registrations, rejecting duplicate identities and malformed defaults.
    pub fn new(
        commands: impl IntoIterator<Item = CommandDescriptor>,
    ) -> Result<Self, CommandRegistrationError> {
        let mut entries = BTreeMap::new();
        for command in commands {
            if entries.len() == 4096 || command.defaults.len() > 32 {
                return Err(CommandRegistrationError::Capacity);
            }
            if command.defaults.iter().any(|binding| !binding.valid())
                || command
                    .defaults
                    .iter()
                    .enumerate()
                    .any(|(index, binding)| command.defaults[..index].contains(binding))
            {
                return Err(CommandRegistrationError::InvalidBinding(command.id));
            }
            if entries.contains_key(&command.id) {
                return Err(CommandRegistrationError::DuplicateId(command.id));
            }
            entries.insert(command.id.clone(), command);
        }
        Ok(Self {
            commands: Arc::new(entries),
            identity: Arc::new(()),
        })
    }
    /// Look up the one static metadata record for an identity.
    pub fn get(&self, id: &CommandId) -> Option<&CommandDescriptor> {
        self.commands.get(id)
    }
    /// Iterate registered commands in stable identifier order.
    pub fn commands(&self) -> impl Iterator<Item = &CommandDescriptor> {
        self.commands.values()
    }
}
