use super::{CommandDispatch, CommandRequest, CommandScopeProjection, Keymap};
use std::rc::Rc;

pub(crate) type CommandResolver<Message> =
    Rc<dyn Fn(CommandRequest<'_>, CommandScopeProjection<'_>, &Keymap) -> CommandDispatch<Message>>;

/// UI-local command resolver and keymap snapshot for an owned child surface.
///
/// The child supplies its own committed scopes. This does not inherit the parent's
/// focused editor or reduce domain messages; the host forwards mapped messages
/// through its ordinary child-to-parent channel. Refresh this snapshot together
/// with the child projection when the parent keymap changes.
pub struct CommandService<Message> {
    resolver: CommandResolver<Message>,
    keymap: Keymap,
}
impl<Message> Clone for CommandService<Message> {
    fn clone(&self) -> Self {
        Self {
            resolver: Rc::clone(&self.resolver),
            keymap: self.keymap.clone(),
        }
    }
}
impl<Message> CommandService<Message> {
    pub(crate) fn into_resolver(self) -> CommandResolver<Message> {
        self.resolver
    }
    pub(crate) fn from_resolver(resolver: CommandResolver<Message>, keymap: Keymap) -> Self {
        Self { resolver, keymap }
    }
    /// Resolve against qualified child scopes, mapping at most one initial message.
    /// The embedding host remains responsible for lifecycle admission and reduction.
    pub fn resolve(
        &self,
        request: CommandRequest<'_>,
        scopes: CommandScopeProjection<'_>,
    ) -> CommandDispatch<Message> {
        (self.resolver)(request, scopes, &self.keymap)
    }
}

impl<Message: 'static> CommandService<Message> {
    /// Build a child-capable resolver from immutable metadata and one application mapper.
    pub fn new<Context: 'static>(
        registry: super::CommandRegistry,
        dispatcher: super::CommandDispatcher<Context, Message>,
        keymap: Keymap,
    ) -> Self {
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
        Self { resolver, keymap }
    }
}
