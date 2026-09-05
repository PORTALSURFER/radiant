use super::{
    CommandBinding, CommandId, CommandInput, CommandInvocation, CommandRegistry, CommandScope,
    CommandSource, CommandTarget, Keymap,
};
use std::{collections::BTreeSet, rc::Rc, sync::Arc};

/// Competing enabled commands at the same precedence; no declaration-order winner exists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandConflict {
    /// Distinct competing semantic identities in deterministic order.
    pub commands: Vec<CommandId>,
    /// Scope identities containing the conflicting bindings.
    pub scopes: Vec<String>,
}

/// A terminal decision that must not fall through to legacy shortcut routing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandSuppression {
    /// An active declarative attachment uses a different application context type.
    ContextMismatch,
    /// Focused text editing already handled this input.
    TextEditing,
    /// Input-method composition currently owns the key.
    Composition,
    /// The selected command does not accept operating-system repeat events.
    Repeat,
    /// The current platform reserves this input before application command routing.
    PlatformReserved,
    /// The key input is malformed.
    MalformedInput,
    /// Active scope identities are ambiguous.
    InvalidScopes,
    /// More than 64 active scopes were supplied.
    Capacity,
}

/// Pure resolution outcome; only Invoked carries data for the application reducer.
pub enum CommandResolution<Context> {
    /// One current enabled command and owned application context were selected.
    Invoked(CommandInvocation<Context>),
    /// No enabled binding matched; an explicit compatibility fallback may continue.
    Unhandled,
    /// A terminal input-precedence or validity decision.
    Suppressed(CommandSuppression),
    /// Enabled bindings are ambiguous at the same scope precedence.
    Conflict(CommandConflict),
    /// A presentation target belongs to a replaced registry, scope or precedence owner.
    Stale,
    /// The current command or its active enabled binding is unavailable.
    Unavailable,
}

fn ordered_scopes<Context>(
    scopes: &[CommandScope<Context>],
) -> Result<Vec<&CommandScope<Context>>, CommandSuppression> {
    if scopes.len() > 64 {
        return Err(CommandSuppression::Capacity);
    }
    let mut ids = BTreeSet::new();
    if scopes.iter().any(|scope| !ids.insert(scope.id.as_str())) {
        return Err(CommandSuppression::InvalidScopes);
    }
    let mut ordered: Vec<_> = scopes.iter().collect();
    ordered.sort_by(|a, b| {
        a.kind
            .precedence()
            .cmp(&b.kind.precedence())
            .then(a.id.cmp(&b.id))
    });
    Ok(ordered)
}

fn invocation<Context>(
    scope: &CommandScope<Context>,
    binding: &CommandBinding<Context>,
    source: CommandSource,
) -> CommandInvocation<Context> {
    CommandInvocation {
        id: binding.id.clone(),
        context: Rc::clone(&binding.context),
        scope: scope.id.clone(),
        source,
    }
}

impl CommandRegistry {
    /// Resolve current active scopes after focused text/IME handling, never by incidental same-scope order.
    pub fn resolve<Context>(
        &self,
        scopes: &[CommandScope<Context>],
        keymap: &Keymap,
        input: &CommandInput,
    ) -> CommandResolution<Context> {
        if input.text_consumed {
            return CommandResolution::Suppressed(CommandSuppression::TextEditing);
        }
        if input.composing {
            return CommandResolution::Suppressed(CommandSuppression::Composition);
        }
        if input.platform_reserved {
            return CommandResolution::Suppressed(CommandSuppression::PlatformReserved);
        }
        if !input.valid() {
            return CommandResolution::Suppressed(CommandSuppression::MalformedInput);
        }
        let ordered = match ordered_scopes(scopes) {
            Ok(scopes) => scopes,
            Err(reason) => return CommandResolution::Suppressed(reason),
        };
        let mut offset = 0;
        while offset < ordered.len() {
            let position = ordered[offset].kind.precedence();
            let end = offset
                + ordered[offset..]
                    .iter()
                    .take_while(|scope| scope.kind.precedence() == position)
                    .count();
            let mut matched = Vec::new();
            for scope in &ordered[offset..end] {
                for binding in &scope.bindings {
                    if !binding.enabled {
                        continue;
                    }
                    let Some(descriptor) = self.get(&binding.id) else {
                        continue;
                    };
                    if keymap
                        .effective(descriptor)
                        .iter()
                        .any(|shortcut| shortcut.matches(input))
                    {
                        matched.push((*scope, binding, descriptor));
                    }
                }
            }
            if matched.len() > 1 {
                return CommandResolution::Conflict(CommandConflict {
                    commands: matched
                        .iter()
                        .map(|(_, binding, _)| binding.id.clone())
                        .collect::<BTreeSet<_>>()
                        .into_iter()
                        .collect(),
                    scopes: matched
                        .iter()
                        .map(|(scope, _, _)| scope.id.clone())
                        .collect::<BTreeSet<_>>()
                        .into_iter()
                        .collect(),
                });
            }
            if let Some((scope, binding, descriptor)) = matched.first() {
                return if input.repeat && !descriptor.repeat {
                    CommandResolution::Suppressed(CommandSuppression::Repeat)
                } else {
                    CommandResolution::Invoked(invocation(scope, binding, CommandSource::Shortcut))
                };
            }
            offset = end;
        }
        CommandResolution::Unhandled
    }

    pub(super) fn selected_binding<'a, Context>(
        &self,
        scopes: &'a [CommandScope<Context>],
        id: &CommandId,
    ) -> Option<(&'a CommandScope<Context>, &'a CommandBinding<Context>)> {
        self.get(id)?;
        let ordered = ordered_scopes(scopes).ok()?;
        let mut disabled: Option<(&CommandScope<Context>, &CommandBinding<Context>)> = None;
        let mut ambiguous_disabled = false;
        let mut selected: Option<(&CommandScope<Context>, &CommandBinding<Context>)> = None;
        for scope in ordered {
            let Some(binding) = scope.bindings.iter().find(|binding| &binding.id == id) else {
                continue;
            };
            if !binding.enabled {
                if let Some((prior, _)) = disabled {
                    ambiguous_disabled |= scope.kind.precedence() == prior.kind.precedence();
                } else {
                    disabled = Some((scope, binding));
                }
                continue;
            }
            if let Some((prior, _)) = selected {
                if scope.kind.precedence() == prior.kind.precedence() {
                    return None;
                }
                break;
            }
            selected = Some((scope, binding));
        }
        selected.or(if ambiguous_disabled { None } else { disabled })
    }

    /// Project an opaque target from the currently selected scope, including disabled presentation state.
    pub fn target<Context>(
        &self,
        scopes: &[CommandScope<Context>],
        id: &CommandId,
    ) -> Option<CommandTarget> {
        let (scope, _) = self.selected_binding(scopes, id)?;
        Some(CommandTarget {
            registry: Arc::clone(&self.identity),
            scope_identity: Rc::clone(&scope.identity),
            scope: scope.id.clone(),
            command: id.clone(),
        })
    }

    /// Re-resolve a presentation activation against current scope identity, precedence and availability.
    pub fn resolve_target<Context>(
        &self,
        scopes: &[CommandScope<Context>],
        target: &CommandTarget,
        source: CommandSource,
    ) -> CommandResolution<Context> {
        if !Arc::ptr_eq(&self.identity, &target.registry) {
            return CommandResolution::Stale;
        }
        let Some((scope, binding)) = self.selected_binding(scopes, &target.command) else {
            return CommandResolution::Unavailable;
        };
        if scope.id != target.scope || !Rc::ptr_eq(&scope.identity, &target.scope_identity) {
            return CommandResolution::Stale;
        }
        if !binding.enabled {
            return CommandResolution::Unavailable;
        }
        CommandResolution::Invoked(invocation(scope, binding, source))
    }
}
