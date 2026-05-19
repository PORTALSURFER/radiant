use super::RuntimeBridge;

/// Public application contract for declarative Radiant hosts.
///
/// `App` is a named API concept for any host object that can project a
/// [`UiSurface`](crate::runtime::UiSurface) and reduce widget-emitted messages.
/// It is implemented automatically for every [`RuntimeBridge`], so existing
/// closure-driven and custom bridge hosts remain allocation-free and do not
/// need adapter wrappers.
pub trait App<Message>: RuntimeBridge<Message> {}

impl<Bridge, Message> App<Message> for Bridge where Bridge: RuntimeBridge<Message> {}
