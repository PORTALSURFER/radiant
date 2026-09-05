use super::{
    CommandId, CommandKey, CommandRegistry, CommandScope, CommandShortcut, Keymap, KeymapError,
};
use crate::gui::shortcuts::ShortcutPlatform;

/// Reason a proposed binding needs an explicit editing decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeymapConflictKind {
    /// An enabled different command has an equivalent binding at the same precedence.
    SameScope,
    /// A host-supplied platform reservation excludes this combination.
    PlatformReserved,
    /// Focused text editing requires this combination.
    TextEditing,
    /// Another active scope deliberately shadows this combination.
    Shadowed,
    /// The command currently has no registration and will remain inactive.
    Unavailable,
}

/// Explicit editing choices; validation never applies one automatically.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeymapResolutionChoice {
    /// Keep the existing keymap.
    KeepExisting,
    /// Choose a different combination.
    ChooseAnotherBinding,
    /// Explicitly unbind the competing command before applying this override.
    UnbindCompetingCommand,
    /// Accept visible intentional shadowing between different scopes.
    AcceptShadowing,
    /// Preserve an unavailable command as inactive data.
    PreserveInactive,
}

/// Typed conflict or shadowing diagnostic for a proposed keymap edit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeymapConflict {
    /// Kind of conflict or deliberate shadowing.
    pub kind: KeymapConflictKind,
    /// Proposed semantic identity.
    pub command: CommandId,
    /// Other semantic identity when a competing binding exists.
    pub competing: Option<CommandId>,
    /// Scope containing the other binding when applicable.
    pub scope: Option<String>,
    /// Proposed binding involved in this diagnostic.
    pub binding: CommandShortcut,
    /// Available explicit editing decisions.
    pub choices: Vec<KeymapResolutionChoice>,
}

/// Current immutable context supplied by the keymap editor and host adapter.
pub struct KeymapValidation<'a, Context> {
    /// Current static registrations.
    pub registry: &'a CommandRegistry,
    /// Current materialized active scopes.
    pub scopes: &'a [CommandScope<Context>],
    /// Primary-modifier platform conventions.
    pub platform: ShortcutPlatform,
    /// Current platform-reserved combinations supplied by the host.
    pub reserved: &'a [CommandShortcut],
    /// Required text-editing combinations supplied by the current editor policy.
    pub text_required: &'a [CommandShortcut],
}

impl Keymap {
    /// Validate a proposal without changing any keymap or scope. Cross-kind keyboard-layout overlaps are also checked when an actual input resolves.
    pub fn validate_override<Context>(
        &self,
        id: &CommandId,
        proposed: &[CommandShortcut],
        context: KeymapValidation<'_, Context>,
    ) -> Result<Vec<KeymapConflict>, KeymapError> {
        if context.scopes.len() > 64
            || context.reserved.len() > 1024
            || context.text_required.len() > 1024
        {
            return Err(KeymapError::Capacity);
        }
        let mut scope_ids = std::collections::BTreeSet::new();
        if context
            .scopes
            .iter()
            .any(|scope| !scope_ids.insert(scope.id.as_str()))
        {
            return Err(KeymapError::InvalidScopes);
        }
        let own_positions: std::collections::BTreeSet<_> = context
            .scopes
            .iter()
            .filter(|scope| {
                scope
                    .bindings
                    .iter()
                    .any(|value| &value.id == id && value.enabled)
            })
            .map(|scope| scope.kind.precedence())
            .collect();
        let proposal = self.clone().override_bindings(id, proposed.to_vec())?;
        let mut conflicts = Vec::new();
        for binding in proposed {
            let unavailable = context.registry.get(id).is_none();
            if unavailable {
                conflicts.push(KeymapConflict {
                    kind: KeymapConflictKind::Unavailable,
                    command: id.clone(),
                    competing: None,
                    scope: None,
                    binding: binding.clone(),
                    choices: vec![
                        KeymapResolutionChoice::PreserveInactive,
                        KeymapResolutionChoice::KeepExisting,
                    ],
                });
            }
            for (kind, excluded) in [
                (KeymapConflictKind::PlatformReserved, context.reserved),
                (KeymapConflictKind::TextEditing, context.text_required),
            ] {
                if excluded
                    .iter()
                    .any(|other| binding.equivalent(other, context.platform))
                {
                    conflicts.push(KeymapConflict {
                        kind,
                        command: id.clone(),
                        competing: None,
                        scope: None,
                        binding: binding.clone(),
                        choices: vec![
                            KeymapResolutionChoice::ChooseAnotherBinding,
                            KeymapResolutionChoice::KeepExisting,
                        ],
                    });
                }
            }
            if unavailable {
                continue;
            }
            for other_scope in context.scopes {
                for other in other_scope
                    .bindings
                    .iter()
                    .filter(|other| &other.id != id && other.enabled)
                {
                    let Some(descriptor) = context.registry.get(&other.id) else {
                        continue;
                    };
                    if !proposal
                        .effective(descriptor)
                        .iter()
                        .any(|other| binding.equivalent(other, context.platform))
                    {
                        continue;
                    }
                    let position = other_scope.kind.precedence();
                    for same in [true, false] {
                        if !own_positions.iter().any(|own| (*own == position) == same) {
                            continue;
                        }
                        let diagnostic = KeymapConflict {
                            kind: if same {
                                KeymapConflictKind::SameScope
                            } else {
                                KeymapConflictKind::Shadowed
                            },
                            command: id.clone(),
                            competing: Some(other.id.clone()),
                            scope: Some(other_scope.id.clone()),
                            binding: binding.clone(),
                            choices: if same {
                                vec![
                                    KeymapResolutionChoice::UnbindCompetingCommand,
                                    KeymapResolutionChoice::ChooseAnotherBinding,
                                    KeymapResolutionChoice::KeepExisting,
                                ]
                            } else {
                                vec![
                                    KeymapResolutionChoice::AcceptShadowing,
                                    KeymapResolutionChoice::ChooseAnotherBinding,
                                    KeymapResolutionChoice::KeepExisting,
                                ]
                            },
                        };
                        if conflicts.len() >= 256 {
                            return Err(KeymapError::Capacity);
                        }
                        if !conflicts.contains(&diagnostic) {
                            conflicts.push(diagnostic);
                        }
                    }
                }
            }
        }
        Ok(conflicts)
    }
}

impl CommandShortcut {
    /// Return compact and spoken text from the same logical/physical binding for all command presentations.
    pub fn presentation(&self, platform: ShortcutPlatform) -> CommandShortcutPresentation {
        let [control, shift, alt, meta] = self.modifiers.physical(platform);
        let mut compact = String::new();
        let mut spoken = Vec::new();
        for (enabled, mac, desktop, name) in [
            (
                meta,
                "⌘",
                if platform == ShortcutPlatform::Windows {
                    "Win+"
                } else {
                    "Super+"
                },
                if platform == ShortcutPlatform::Mac {
                    "Command"
                } else {
                    "Super"
                },
            ),
            (control, "⌃", "Ctrl+", "Control"),
            (shift, "⇧", "Shift+", "Shift"),
            (
                alt,
                "⌥",
                "Alt+",
                if platform == ShortcutPlatform::Mac {
                    "Option"
                } else {
                    "Alt"
                },
            ),
        ] {
            if enabled {
                compact.push_str(if platform == ShortcutPlatform::Mac {
                    mac
                } else {
                    desktop
                });
                spoken.push(name.to_owned());
            }
        }
        let (display, speech, physical) = match &self.key {
            CommandKey::Character(text) => (
                if text.is_ascii() {
                    text.to_uppercase()
                } else {
                    text.clone()
                },
                text.clone(),
                false,
            ),
            CommandKey::Named(name) => (name.clone(), name.clone(), false),
            CommandKey::Physical(code) => (format!("[{code}]"), format!("physical {code}"), true),
        };
        compact.push_str(&display);
        spoken.push(speech);
        CommandShortcutPresentation {
            compact,
            spoken: spoken.join("+"),
            physical,
        }
    }
}

/// One shared binding display for menus, tools, palettes, help and accessibility.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandShortcutPresentation {
    /// Compact platform modifier text and distinct positional-key notation.
    pub compact: String,
    /// Expanded spoken modifier and key description.
    pub spoken: String,
    /// Whether this binding explicitly addresses a physical key position.
    pub physical: bool,
}
