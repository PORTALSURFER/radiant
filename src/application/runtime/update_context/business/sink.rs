use std::sync::Arc;

/// Worker-side sender for intermediate business-work events.
///
/// Values emitted through this sink are delivered back to the host application
/// through the normal message queue. The sink does not expose UI state mutation
/// or runtime internals to worker code.
pub struct BusinessEventSink<Event> {
    emit: Arc<dyn Fn(Event) -> bool + Send + Sync + 'static>,
}

impl<Event> Clone for BusinessEventSink<Event> {
    fn clone(&self) -> Self {
        Self {
            emit: Arc::clone(&self.emit),
        }
    }
}

impl<Event> BusinessEventSink<Event> {
    pub(super) fn new(emit: impl Fn(Event) -> bool + Send + Sync + 'static) -> Self {
        Self {
            emit: Arc::new(emit),
        }
    }

    /// Emit one intermediate event. Returns `false` if the runtime no longer
    /// accepts messages, for example after shutdown.
    pub fn emit(&self, event: Event) -> bool {
        (self.emit)(event)
    }
}
