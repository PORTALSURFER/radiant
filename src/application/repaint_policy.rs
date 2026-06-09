use crate::runtime::RepaintScope;

type RepaintResolver<Message> = dyn Fn(&Message) -> Option<RepaintScope>;

/// Automatic repaint behavior applied after app messages are reduced.
///
/// Use this when an application reducer should stay focused on state changes
/// and runtime commands while Radiant owns the ordinary "message implies
/// repaint" policy at the app boundary.
pub struct RepaintPolicy<Message> {
    resolve: Box<RepaintResolver<Message>>,
}

impl<Message> RepaintPolicy<Message> {
    /// Request a surface repaint after every reduced message.
    pub fn after_every_message() -> Self {
        Self::new(|_| Some(RepaintScope::Surface))
    }

    /// Do not add any automatic repaint command after reduced messages.
    pub fn none() -> Self {
        Self::new(|_| None)
    }

    /// Request a surface repaint after every message except excluded messages.
    pub fn after_messages_except(exclude: impl Fn(&Message) -> bool + 'static) -> Self {
        Self::new(move |message| (!exclude(message)).then_some(RepaintScope::Surface))
    }

    /// Request a surface repaint after every message except one exact value.
    pub fn after_messages_except_value(excluded: Message) -> Self
    where
        Message: PartialEq + 'static,
    {
        Self::after_messages_except(move |message| message == &excluded)
    }

    /// Resolve the repaint scope for one message.
    pub(in crate::application) fn scope_for(&self, message: &Message) -> Option<RepaintScope> {
        (self.resolve)(message)
    }

    fn new(resolve: impl Fn(&Message) -> Option<RepaintScope> + 'static) -> Self {
        Self {
            resolve: Box::new(resolve),
        }
    }
}
