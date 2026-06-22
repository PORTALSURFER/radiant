use crate::application::{CancellationToken, KeyedTaskCompletion, TaskTicket};

use super::{BusinessEventSink, BusinessWorkContext, request::BusinessRequest};

/// Builder for one keyed-latest business request.
pub struct BusinessKeyedLatestRequest<'context, Message, Key> {
    pub(super) request: BusinessRequest<'context, Message>,
    pub(super) ticket: TaskTicket,
    pub(super) key: Key,
}

impl<Message, Key> BusinessKeyedLatestRequest<'_, Message, Key> {
    /// Return the task ticket assigned to this request.
    pub fn ticket(&self) -> TaskTicket {
        self.ticket
    }

    /// Return the host-owned key for this request.
    pub fn key(&self) -> &Key {
        &self.key
    }
}

impl<'context, Message, Key> BusinessKeyedLatestRequest<'context, Message, Key>
where
    Key: Clone + Send + Sync + 'static,
{
    /// Make this keyed latest request cooperatively cancellable.
    pub fn cancellable(self) -> CancellableBusinessKeyedLatestRequest<'context, Message, Key> {
        CancellableBusinessKeyedLatestRequest {
            request: self.request,
            token: CancellationToken::new(),
            ticket: self.ticket,
            key: self.key,
        }
    }

    /// Run keyed latest work and tag the output with its key and task ticket.
    pub fn run<Output>(
        self,
        work: impl FnOnce(BusinessWorkContext) -> Output + Send + 'static,
        map: impl FnOnce(KeyedTaskCompletion<Key, Output>) -> Message + Send + 'static,
    ) where
        Output: Send + 'static,
    {
        let key = self.key;
        let ticket = self.ticket;
        self.request.run(
            move |context| KeyedTaskCompletion {
                key,
                ticket,
                output: work(context),
            },
            map,
        );
    }

    /// Run keyed latest work that may emit intermediate events tagged with its key and task ticket.
    pub fn stream<Event, Output>(
        self,
        work: impl FnOnce(BusinessWorkContext, BusinessEventSink<Event>) -> Output + Send + 'static,
        map_event: impl Fn(KeyedTaskCompletion<Key, Event>) -> Message + Send + Sync + 'static,
        map_final: impl FnOnce(KeyedTaskCompletion<Key, Output>) -> Message + Send + 'static,
    ) where
        Event: Send + 'static,
        Output: Send + 'static,
        Message: 'static,
    {
        let event_key = self.key.clone();
        let final_key = self.key;
        let ticket = self.ticket;
        self.request.stream(
            work,
            move |event| {
                map_event(KeyedTaskCompletion {
                    key: event_key.clone(),
                    ticket,
                    output: event,
                })
            },
            move |output| {
                map_final(KeyedTaskCompletion {
                    key: final_key,
                    ticket,
                    output,
                })
            },
        );
    }
}

/// Cancellable builder for one keyed-latest business request.
pub struct CancellableBusinessKeyedLatestRequest<'context, Message, Key> {
    pub(super) request: BusinessRequest<'context, Message>,
    pub(super) token: CancellationToken,
    pub(super) ticket: TaskTicket,
    pub(super) key: Key,
}

impl<Message, Key> CancellableBusinessKeyedLatestRequest<'_, Message, Key> {
    /// Return the task ticket assigned to this request.
    pub fn ticket(&self) -> TaskTicket {
        self.ticket
    }

    /// Return a clone of the cancellation token owned by this request.
    pub fn token(&self) -> CancellationToken {
        self.token.clone()
    }

    /// Return the host-owned key for this request.
    pub fn key(&self) -> &Key {
        &self.key
    }
}

impl<'context, Message, Key> CancellableBusinessKeyedLatestRequest<'context, Message, Key>
where
    Key: Clone + Send + Sync + 'static,
{
    /// Run cancellable keyed latest work and return its cancellation token.
    pub fn run<Output>(
        self,
        work: impl FnOnce(BusinessWorkContext) -> Output + Send + 'static,
        map: impl FnOnce(KeyedTaskCompletion<Key, Output>) -> Message + Send + 'static,
    ) -> CancellationToken
    where
        Output: Send + 'static,
    {
        let token = self.token.clone();
        let key = self.key;
        let ticket = self.ticket;
        self.request.run_with_optional_cancellation(
            Some(self.token),
            move |context| KeyedTaskCompletion {
                key,
                ticket,
                output: work(context),
            },
            map,
        );
        token
    }

    /// Run cancellable keyed latest work that may emit intermediate events tagged with its key and task ticket.
    pub fn stream<Event, Output>(
        self,
        work: impl FnOnce(BusinessWorkContext, BusinessEventSink<Event>) -> Output + Send + 'static,
        map_event: impl Fn(KeyedTaskCompletion<Key, Event>) -> Message + Send + Sync + 'static,
        map_final: impl FnOnce(KeyedTaskCompletion<Key, Output>) -> Message + Send + 'static,
    ) -> CancellationToken
    where
        Event: Send + 'static,
        Output: Send + 'static,
        Message: 'static,
    {
        let token = self.token.clone();
        let event_key = self.key.clone();
        let final_key = self.key;
        let ticket = self.ticket;
        self.request.stream_with_optional_cancellation(
            Some(self.token),
            work,
            move |event| {
                map_event(KeyedTaskCompletion {
                    key: event_key.clone(),
                    ticket,
                    output: event,
                })
            },
            move |output| {
                map_final(KeyedTaskCompletion {
                    key: final_key,
                    ticket,
                    output,
                })
            },
        );
        token
    }
}

#[cfg(test)]
mod tests {
    use crate::application::KeyedLatestTasks;
    use crate::application::runtime::update_context::UiUpdateContext;
    use crate::runtime::{Command, ResourceKey, TaskPriority};

    #[test]
    fn keyed_latest_stream_tags_intermediate_and_final_outputs() {
        let mut context = UiUpdateContext::<String>::default();
        let mut latest = KeyedLatestTasks::new();
        context
            .business()
            .interactive("keyed-stream-test")
            .latest_for(&mut latest, ResourceKey::scoped("sample", "C:/kick.wav"))
            .stream(
                |_context, events| {
                    assert!(events.emit("preview"));
                    "done"
                },
                |event| format!("{}:{}:{}", event.key, event.ticket.id(), event.output),
                |output| format!("{}:{}:{}", output.key, output.ticket.id(), output.output),
            );

        let command = context.into_command();
        let Command::PerformStream { priority, .. } = &command else {
            panic!("expected stream command");
        };
        assert_eq!(*priority, TaskPriority::Interactive);
    }
}
