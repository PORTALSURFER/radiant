use super::{
    CommandId, CommandRegistry, CommandScope, CommandShortcutPresentation, CommandTarget, Keymap,
};
use crate::{gui::shortcuts::ShortcutPlatform, runtime::ResolvedEnvironment};

/// One localized command projection shared by every presentation surface.
#[derive(Clone, Debug)]
pub struct CommandPresentation {
    /// Stable semantic command identity.
    pub id: CommandId,
    /// Localized visible label from the single registry record.
    pub label: String,
    /// Optional localized description.
    pub description: Option<String>,
    /// Optional localized category.
    pub category: Option<String>,
    /// Localized accessibility label, defaulting to the visible label.
    pub accessibility: String,
    /// Current availability from the selected active scope.
    pub enabled: bool,
    /// Optional current checked state from the same selected binding.
    pub checked: Option<bool>,
    /// Effective bindings after validated stored overrides.
    pub shortcuts: Vec<CommandShortcutPresentation>,
    /// Opaque activation target, re-resolved against current scopes when used.
    pub target: Option<CommandTarget>,
}

impl CommandRegistry {
    /// Project one command without dispatching, reading domain state or invoking providers.
    pub fn present<Context>(
        &self,
        scopes: &[CommandScope<Context>],
        keymap: &Keymap,
        id: &CommandId,
        environment: &ResolvedEnvironment,
        platform: ShortcutPlatform,
    ) -> Option<CommandPresentation> {
        let descriptor = self.get(id)?;
        let binding = self
            .selected_binding(scopes, id)
            .map(|(_, binding)| binding);
        let label = environment.localized(&descriptor.label).as_str().to_owned();
        Some(CommandPresentation {
            id: id.clone(),
            accessibility: descriptor.accessibility.as_ref().map_or_else(
                || label.clone(),
                |key| environment.localized(key).as_str().to_owned(),
            ),
            label,
            description: descriptor
                .description
                .as_ref()
                .map(|key| environment.localized(key).as_str().to_owned()),
            category: descriptor
                .category
                .as_ref()
                .map(|key| environment.localized(key).as_str().to_owned()),
            enabled: binding.is_some_and(|binding| binding.enabled),
            checked: binding.and_then(|binding| binding.checked),
            shortcuts: keymap
                .effective(descriptor)
                .iter()
                .map(|shortcut| shortcut.presentation(platform))
                .collect(),
            target: self.target(scopes, id),
        })
    }
}
