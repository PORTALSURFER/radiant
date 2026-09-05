use super::{
    CommandDispatch, CommandId, CommandPresentation, CommandRegistry, CommandRequest,
    CommandScopeProjection, CommandSuppression, Keymap,
};
use crate::{gui::shortcuts::ShortcutPlatform, runtime::ResolvedEnvironment};
use std::rc::Rc;

type CommandResolver<Message> =
    Rc<dyn Fn(CommandRequest<'_>, CommandScopeProjection<'_>, &Keymap) -> CommandDispatch<Message>>;

pub(crate) const MAX_PRESENTATIONS: usize = 256;

type CommandPresenter = fn(
    &CommandRegistry,
    CommandScopeProjection<'_>,
    &Keymap,
    &[CommandId],
    &ResolvedEnvironment,
    ShortcutPlatform,
) -> Result<Vec<CommandPresentation>, CommandPresentationError>;

/// Failure to construct one consistent batch of native/control presentations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandPresentationError {
    /// The runtime is closed or no declarative service is registered.
    Unavailable,
    /// The current committed scope projection is invalid.
    Scopes(CommandSuppression),
    /// One requested semantic command is not registered.
    UnknownCommand(CommandId),
    /// More than 256 presentations were requested in one batch.
    Capacity,
}

/// UI-local command resolver and keymap snapshot for an owned child surface.
///
/// The child supplies its own committed scopes, combined with explicit application
/// scopes exported by its ancestors. This does not inherit the parent's
/// focused editor or reduce domain messages; the host forwards mapped messages
/// through its ordinary child-to-parent channel. Refresh this snapshot together
/// with the child projection when the parent keymap changes.
pub struct CommandService<Message> {
    resolver: CommandResolver<Message>,
    registry: CommandRegistry,
    presenter: CommandPresenter,
    keymap: Keymap,
    inherited: Vec<super::ResolvedCommandScope>,
    inherited_error: Option<CommandSuppression>,
}
impl<Message> Clone for CommandService<Message> {
    fn clone(&self) -> Self {
        Self {
            resolver: Rc::clone(&self.resolver),
            registry: self.registry.clone(),
            presenter: self.presenter,
            keymap: self.keymap.clone(),
            inherited: self.inherited.clone(),
            inherited_error: self.inherited_error,
        }
    }
}
impl<Message> CommandService<Message> {
    pub(crate) fn with_application_scopes(mut self, scopes: CommandScopeProjection<'_>) -> Self {
        (self.inherited, self.inherited_error) =
            scopes.combined(&self.inherited, self.inherited_error);
        self
    }

    /// Replace the data-only keymap snapshot while retaining registry and mapper identity.
    pub fn with_keymap(mut self, keymap: Keymap) -> Self {
        self.keymap = keymap;
        self
    }

    /// Project at most 256 commands in request order from one scope/keymap snapshot.
    /// The batch is rejected as a whole on invalid scopes or an unknown command.
    /// This never invokes the application command mapper.
    pub fn presentations(
        &self,
        scopes: CommandScopeProjection<'_>,
        ids: &[CommandId],
        environment: &ResolvedEnvironment,
        platform: ShortcutPlatform,
    ) -> Result<Vec<CommandPresentation>, CommandPresentationError> {
        if ids.len() > MAX_PRESENTATIONS {
            return Err(CommandPresentationError::Capacity);
        }
        let (records, error) = scopes.combined(&self.inherited, self.inherited_error);
        let scopes = CommandScopeProjection::new(&records, error);
        (self.presenter)(
            &self.registry,
            scopes,
            &self.keymap,
            ids,
            environment,
            platform,
        )
    }

    pub(crate) fn resolve_with_keymap(
        &self,
        request: CommandRequest<'_>,
        scopes: CommandScopeProjection<'_>,
        keymap: &Keymap,
    ) -> CommandDispatch<Message> {
        let (records, error) = scopes.combined(&self.inherited, self.inherited_error);
        (self.resolver)(
            request,
            CommandScopeProjection::new(&records, error),
            keymap,
        )
    }

    /// Resolve against qualified child scopes, mapping at most one initial message.
    /// The embedding host remains responsible for lifecycle admission and reduction.
    pub fn resolve(
        &self,
        request: CommandRequest<'_>,
        scopes: CommandScopeProjection<'_>,
    ) -> CommandDispatch<Message> {
        self.resolve_with_keymap(request, scopes, &self.keymap)
    }
}

impl<Message: 'static> CommandService<Message> {
    /// Build a child-capable resolver from immutable metadata and one application mapper.
    pub fn new<Context: 'static>(
        registry: super::CommandRegistry,
        dispatcher: super::CommandDispatcher<Context, Message>,
        keymap: Keymap,
    ) -> Self {
        let presentation_registry = registry.clone();
        let resolver: CommandResolver<Message> = Rc::new(move |request, projection, keymap| {
            let scopes = match projection.scopes::<Context>() {
                Ok(scopes) => scopes,
                Err(reason) => {
                    return crate::application::CommandDispatch {
                        message: None,
                        status: crate::application::CommandDispatchStatus::Suppressed(reason),
                    };
                }
            };
            match request {
                crate::application::CommandRequest::Input(input) => {
                    dispatcher.input(&registry, &scopes, keymap, input)
                }
                crate::application::CommandRequest::Target(target, source) => {
                    dispatcher.target(&registry, &scopes, target, source)
                }
            }
        });
        Self {
            resolver,
            registry: presentation_registry,
            presenter: present::<Context>,
            keymap,
            inherited: Vec::new(),
            inherited_error: None,
        }
    }
}

fn present<Context: 'static>(
    registry: &CommandRegistry,
    projection: CommandScopeProjection<'_>,
    keymap: &Keymap,
    ids: &[CommandId],
    environment: &ResolvedEnvironment,
    platform: ShortcutPlatform,
) -> Result<Vec<CommandPresentation>, CommandPresentationError> {
    let scopes = projection
        .scopes::<Context>()
        .map_err(CommandPresentationError::Scopes)?;
    let mut unique = std::collections::BTreeSet::new();
    if scopes.iter().any(|scope| !unique.insert(scope.id())) {
        return Err(CommandPresentationError::Scopes(
            CommandSuppression::InvalidScopes,
        ));
    }
    if let Some(id) = ids.iter().find(|id| registry.get(id).is_none()) {
        return Err(CommandPresentationError::UnknownCommand(id.clone()));
    }
    ids.iter()
        .map(|id| {
            registry
                .present(&scopes, keymap, id, environment, platform)
                .ok_or_else(|| CommandPresentationError::UnknownCommand(id.clone()))
        })
        .collect()
}
