use crate::gui::shortcuts::ShortcutPlatform;
use serde::{Deserialize, Serialize};

/// Portable logical key or explicitly positional physical key.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum CommandKey {
    /// Exact character text produced by the active keyboard layout.
    Character(String),
    /// Logical non-character key, such as Enter or ArrowLeft.
    Named(String),
    /// Backend-neutral positional code, such as KeyZ or Digit1.
    Physical(String),
}

impl CommandKey {
    pub(super) fn valid(&self) -> bool {
        let text = match self {
            Self::Character(text) | Self::Named(text) | Self::Physical(text) => text,
        };
        !text.is_empty()
            && text.len() <= 64
            && !text.chars().any(char::is_control)
            && (matches!(self, Self::Character(_))
                || text.bytes().all(|byte| byte.is_ascii_alphanumeric()))
    }
}

/// Exact modifier requirements; primary means Command on macOS and Control elsewhere.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CommandModifiers {
    /// Portable primary command modifier.
    pub primary: bool,
    /// Physical Control modifier.
    pub control: bool,
    /// Shift modifier.
    pub shift: bool,
    /// Alt/Option modifier.
    pub alt: bool,
    /// Physical Meta/Super modifier.
    pub meta: bool,
}

impl CommandModifiers {
    pub(super) fn physical(self, platform: ShortcutPlatform) -> [bool; 4] {
        [
            self.control || (self.primary && platform != ShortcutPlatform::Mac),
            self.shift,
            self.alt,
            self.meta || (self.primary && platform == ShortcutPlatform::Mac),
        ]
    }
}

/// One logical or physical shortcut stored without callbacks or host handles.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandShortcut {
    /// Logical character, named logical key, or explicit positional key.
    pub key: CommandKey,
    /// Exact modifier requirements.
    #[serde(default)]
    pub modifiers: CommandModifiers,
}

impl CommandShortcut {
    /// Construct an unmodified shortcut.
    pub fn new(key: CommandKey) -> Self {
        Self {
            key,
            modifiers: CommandModifiers::default(),
        }
    }
    /// Require the portable primary modifier.
    pub fn primary(mut self) -> Self {
        self.modifiers.primary = true;
        self
    }
    /// Require Shift in addition to the other modifiers.
    pub fn shift(mut self) -> Self {
        self.modifiers.shift = true;
        self
    }
    /// Replace the complete exact modifier requirements.
    pub fn modifiers(mut self, modifiers: CommandModifiers) -> Self {
        self.modifiers = modifiers;
        self
    }
    pub(super) fn valid(&self) -> bool {
        self.key.valid()
    }
    pub(super) fn matches(&self, input: &CommandInput) -> bool {
        let key_matches = match &self.key {
            CommandKey::Physical(code) => input.physical.as_ref() == Some(code),
            logical => input.logical.as_ref() == Some(logical),
        };
        key_matches
            && self.modifiers.physical(input.platform) == input.modifiers.physical(input.platform)
    }
    pub(super) fn equivalent(&self, other: &Self, platform: ShortcutPlatform) -> bool {
        self.key == other.key
            && self.modifiers.physical(platform) == other.modifiers.physical(platform)
    }
}

/// One keyboard event after focused text editing has had its required precedence.
#[derive(Clone, Debug)]
pub struct CommandInput {
    /// Character or named key produced by the keyboard layout; never a physical variant.
    pub logical: Option<CommandKey>,
    /// Explicit positional key code supplied by the backend.
    pub physical: Option<String>,
    /// Active modifiers; backends normally report physical bits, with primary false.
    pub modifiers: CommandModifiers,
    /// Platform conventions for the primary modifier.
    pub platform: ShortcutPlatform,
    /// Whether the operating system marks this as a repeated key press.
    pub repeat: bool,
    /// Whether the current platform owns this input before application command routing.
    pub platform_reserved: bool,
    /// Whether focused text editing already consumed the required editing key.
    pub text_consumed: bool,
    /// Whether an input-method composition session currently owns keyboard input.
    pub composing: bool,
}

impl CommandInput {
    /// Construct a fresh logical key press with no modifiers or text preemption.
    pub fn logical(key: CommandKey, platform: ShortcutPlatform) -> Self {
        Self {
            logical: Some(key),
            physical: None,
            modifiers: CommandModifiers::default(),
            platform,
            repeat: false,
            platform_reserved: false,
            text_consumed: false,
            composing: false,
        }
    }
    pub(super) fn valid(&self) -> bool {
        self.logical
            .as_ref()
            .is_none_or(|key| !matches!(key, CommandKey::Physical(_)) && key.valid())
            && self
                .physical
                .as_ref()
                .is_none_or(|code| CommandKey::Physical(code.clone()).valid())
            && (self.logical.is_some() || self.physical.is_some())
    }
}
