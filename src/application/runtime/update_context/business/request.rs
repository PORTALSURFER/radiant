use std::hash::Hash;

use crate::{
    application::runtime::ResourceTasks,
    application::{CancellationToken, KeyedLatestTasks, LatestTask},
    runtime::{BusinessMessageSink, Command, ResourceKey, ResourceSlot, TaskPriority},
};

use super::{
    BusinessEventSink, BusinessWorkContext,
    keyed_latest::{BusinessKeyedLatestRequest, CancellableBusinessKeyedLatestRequest},
    latest::{BusinessLatestRequest, CancellableBusinessLatestRequest},
    resource::{BusinessResourceRequest, CancellableBusinessResourceRequest},
};
use crate::application::runtime::update_context::UiUpdateContext;

/// Builder for one named business request.
pub struct BusinessRequest<'context, Message> {
    pub(super) context: &'context mut UiUpdateContext<Message>,
    pub(super) name: &'static str,
    pub(super) priority: TaskPriority,
}

impl<'context, Message> BusinessRequest<'context, Message> {
    /// Make this request cooperatively cancellable.
    pub fn cancellable(self) -> CancellableBusinessRequest<'context, Message> {
        CancellableBusinessRequest {
            request: self,
            token: CancellationToken::new(),
        }
    }

    /// Start replace-latest work for one host-owned task slot.
    pub fn latest(self, latest: &mut LatestTask) -> BusinessLatestRequest<'context, Message> {
        BusinessLatestRequest {
            request: self,
            ticket: latest.begin(),
        }
    }

    /// Start replace-latest work for one key in a host-owned task registry.
    pub fn latest_for<Key>(
        self,
        latest: &mut KeyedLatestTasks<Key>,
        key: Key,
    ) -> BusinessKeyedLatestRequest<'context, Message, Key>
    where
        Key: Clone + Eq + Hash,
    {
        BusinessKeyedLatestRequest {
            request: self,
            ticket: latest.begin(key.clone()),
            key,
        }
    }

    /// Start replace-latest work for one generic resource key.
    pub fn latest_for_resource(
        self,
        resources: &mut ResourceTasks,
        key: impl Into<ResourceKey>,
    ) -> BusinessKeyedLatestRequest<'context, Message, ResourceKey> {
        let ticket = resources.begin_latest(key.into());
        let key = ticket.key().clone();
        BusinessKeyedLatestRequest {
            request: self,
            ticket: ticket.ticket(),
            key,
        }
    }

    /// Start exclusive work for one generic resource key.
    ///
    /// Returns `None` when the same key already has an active exclusive task.
    pub fn exclusive_for(
        self,
        resources: &mut ResourceTasks,
        key: impl Into<ResourceKey>,
    ) -> Option<BusinessKeyedLatestRequest<'context, Message, ResourceKey>> {
        let ticket = resources.begin_exclusive(key.into())?;
        let key = ticket.key().clone();
        Some(BusinessKeyedLatestRequest {
            request: self,
            ticket: ticket.ticket(),
            key,
        })
    }

    /// Start a resource load for one host-owned resource slot.
    pub fn resource<Output>(
        self,
        slot: &mut ResourceSlot<Output>,
    ) -> BusinessResourceRequest<'context, Message, Output> {
        BusinessResourceRequest {
            request: self,
            resource: slot.begin_load(),
            output: std::marker::PhantomData,
        }
    }

    /// Run this business request and map its output into a host message.
    pub fn run<Output>(
        self,
        work: impl FnOnce(BusinessWorkContext) -> Output + Send + 'static,
        map: impl FnOnce(Output) -> Message + Send + 'static,
    ) where
        Output: Send + 'static,
    {
        self.run_with_optional_cancellation(None, work, map);
    }

    /// Run this business request and allow worker code to emit intermediate
    /// events before the final output message.
    pub fn stream<Event, Output>(
        self,
        work: impl FnOnce(BusinessWorkContext, BusinessEventSink<Event>) -> Output + Send + 'static,
        map_event: impl Fn(Event) -> Message + Send + Sync + 'static,
        map_final: impl FnOnce(Output) -> Message + Send + 'static,
    ) where
        Event: Send + 'static,
        Output: Send + 'static,
        Message: 'static,
    {
        self.stream_with_optional_cancellation(None, work, map_event, map_final);
    }

    /// Run this business request with coalesced intermediate events.
    ///
    /// Intermediate events are delivered through a per-task latest-message slot:
    /// while the UI loop is behind, a newer event replaces the previous pending
    /// event for this stream. The final output message is still delivered
    /// through the ordinary ordered queue and is not coalesced.
    pub fn stream_latest<Event, Output>(
        self,
        work: impl FnOnce(BusinessWorkContext, BusinessEventSink<Event>) -> Output + Send + 'static,
        map_event: impl Fn(Event) -> Message + Send + Sync + 'static,
        map_final: impl FnOnce(Output) -> Message + Send + 'static,
    ) where
        Event: Send + 'static,
        Output: Send + 'static,
        Message: 'static,
    {
        self.latest_stream_with_optional_cancellation(None, work, map_event, map_final);
    }

    pub(super) fn run_with_optional_cancellation<Output>(
        self,
        token: Option<CancellationToken>,
        work: impl FnOnce(BusinessWorkContext) -> Output + Send + 'static,
        map: impl FnOnce(Output) -> Message + Send + 'static,
    ) where
        Output: Send + 'static,
    {
        let worker_token = token.clone();
        let is_cancelled = token.map(|token| {
            Box::new(move || token.is_cancelled()) as Box<dyn Fn() -> bool + Send + Sync + 'static>
        });
        self.context.queue_command(Command::perform_with_priority(
            self.name,
            self.priority,
            is_cancelled,
            move || work(BusinessWorkContext::new(worker_token)),
            map,
        ));
    }

    pub(super) fn stream_with_optional_cancellation<Event, Output>(
        self,
        token: Option<CancellationToken>,
        work: impl FnOnce(BusinessWorkContext, BusinessEventSink<Event>) -> Output + Send + 'static,
        map_event: impl Fn(Event) -> Message + Send + Sync + 'static,
        map_final: impl FnOnce(Output) -> Message + Send + 'static,
    ) where
        Event: Send + 'static,
        Output: Send + 'static,
        Message: 'static,
    {
        let worker_token = token.clone();
        let is_cancelled = token.map(|token| {
            Box::new(move || token.is_cancelled()) as Box<dyn Fn() -> bool + Send + Sync + 'static>
        });
        self.context
            .queue_command(Command::perform_stream_with_priority(
                self.name,
                self.priority,
                is_cancelled,
                move |message_sink| {
                    let event_sink = BusinessEventSink::new({
                        let message_sink = message_sink.clone();
                        move |event| message_sink.emit(map_event(event))
                    });
                    let output = work(BusinessWorkContext::new(worker_token), event_sink);
                    let _ = message_sink.emit(map_final(output));
                },
            ));
    }

    pub(super) fn latest_stream_with_optional_cancellation<Event, Output>(
        self,
        token: Option<CancellationToken>,
        work: impl FnOnce(BusinessWorkContext, BusinessEventSink<Event>) -> Output + Send + 'static,
        map_event: impl Fn(Event) -> Message + Send + Sync + 'static,
        map_final: impl FnOnce(Output) -> Message + Send + 'static,
    ) where
        Event: Send + 'static,
        Output: Send + 'static,
        Message: 'static,
    {
        let worker_token = token.clone();
        let is_cancelled = token.map(|token| {
            Box::new(move || token.is_cancelled()) as Box<dyn Fn() -> bool + Send + Sync + 'static>
        });
        self.context
            .queue_command(Command::perform_latest_stream_with_priority(
                self.name,
                self.priority,
                is_cancelled,
                move |message_sink| {
                    let event_sink = BusinessEventSink::new({
                        let message_sink = message_sink.clone();
                        move |event| message_sink.emit_latest(map_event(event))
                    });
                    let close_guard = LatestStreamCloseGuard::new(message_sink.clone());
                    let output = work(BusinessWorkContext::new(worker_token), event_sink);
                    close_guard.close();
                    let _ = message_sink.emit(map_final(output));
                },
            ));
    }
}

struct LatestStreamCloseGuard<Message> {
    sink: Option<BusinessMessageSink<Message>>,
}

impl<Message> LatestStreamCloseGuard<Message> {
    fn new(sink: BusinessMessageSink<Message>) -> Self {
        Self { sink: Some(sink) }
    }

    fn close(mut self) {
        self.close_inner();
    }

    fn close_inner(&mut self) {
        if let Some(sink) = self.sink.take() {
            sink.close_latest();
        }
    }
}

impl<Message> Drop for LatestStreamCloseGuard<Message> {
    fn drop(&mut self) {
        self.close_inner();
    }
}

/// Cancellable builder for one named business request.
pub struct CancellableBusinessRequest<'context, Message> {
    pub(super) request: BusinessRequest<'context, Message>,
    pub(super) token: CancellationToken,
}

impl<Message> CancellableBusinessRequest<'_, Message> {
    /// Return a clone of the cancellation token owned by this request.
    pub fn token(&self) -> CancellationToken {
        self.token.clone()
    }
}

impl<'context, Message> CancellableBusinessRequest<'context, Message> {
    /// Start replace-latest work for one host-owned task slot.
    pub fn latest(
        self,
        latest: &mut LatestTask,
    ) -> CancellableBusinessLatestRequest<'context, Message> {
        let ticket = latest.begin();
        CancellableBusinessLatestRequest {
            request: self.request,
            token: self.token,
            ticket,
        }
    }

    /// Start replace-latest work for one key in a host-owned task registry.
    pub fn latest_for<Key>(
        self,
        latest: &mut KeyedLatestTasks<Key>,
        key: Key,
    ) -> CancellableBusinessKeyedLatestRequest<'context, Message, Key>
    where
        Key: Clone + Eq + Hash,
    {
        let ticket = latest.begin(key.clone());
        CancellableBusinessKeyedLatestRequest {
            request: self.request,
            token: self.token,
            ticket,
            key,
        }
    }

    /// Start cancellable replace-latest work for one generic resource key.
    pub fn latest_for_resource(
        self,
        resources: &mut ResourceTasks,
        key: impl Into<ResourceKey>,
    ) -> CancellableBusinessKeyedLatestRequest<'context, Message, ResourceKey> {
        let ticket = resources.begin_latest(key.into());
        let key = ticket.key().clone();
        CancellableBusinessKeyedLatestRequest {
            request: self.request,
            token: self.token,
            ticket: ticket.ticket(),
            key,
        }
    }

    /// Start cancellable exclusive work for one generic resource key.
    ///
    /// Returns `None` when the same key already has an active exclusive task.
    pub fn exclusive_for(
        self,
        resources: &mut ResourceTasks,
        key: impl Into<ResourceKey>,
    ) -> Option<CancellableBusinessKeyedLatestRequest<'context, Message, ResourceKey>> {
        let ticket = resources.begin_exclusive(key.into())?;
        let key = ticket.key().clone();
        Some(CancellableBusinessKeyedLatestRequest {
            request: self.request,
            token: self.token,
            ticket: ticket.ticket(),
            key,
        })
    }

    /// Start a resource load for one host-owned resource slot.
    pub fn resource<Output>(
        self,
        slot: &mut ResourceSlot<Output>,
    ) -> CancellableBusinessResourceRequest<'context, Message, Output> {
        CancellableBusinessResourceRequest {
            request: self.request,
            token: self.token,
            resource: slot.begin_load(),
            output: std::marker::PhantomData,
        }
    }

    /// Run this cancellable request and return its cancellation token.
    pub fn run<Output>(
        self,
        work: impl FnOnce(BusinessWorkContext) -> Output + Send + 'static,
        map: impl FnOnce(Output) -> Message + Send + 'static,
    ) -> CancellationToken
    where
        Output: Send + 'static,
    {
        let token = self.token.clone();
        self.request
            .run_with_optional_cancellation(Some(self.token), work, map);
        token
    }

    /// Run this cancellable request as a stream and return its cancellation token.
    pub fn stream<Event, Output>(
        self,
        work: impl FnOnce(BusinessWorkContext, BusinessEventSink<Event>) -> Output + Send + 'static,
        map_event: impl Fn(Event) -> Message + Send + Sync + 'static,
        map_final: impl FnOnce(Output) -> Message + Send + 'static,
    ) -> CancellationToken
    where
        Event: Send + 'static,
        Output: Send + 'static,
        Message: 'static,
    {
        let token = self.token.clone();
        self.request.stream_with_optional_cancellation(
            Some(self.token),
            work,
            map_event,
            map_final,
        );
        token
    }

    /// Run this cancellable request with coalesced intermediate events and return its cancellation token.
    pub fn stream_latest<Event, Output>(
        self,
        work: impl FnOnce(BusinessWorkContext, BusinessEventSink<Event>) -> Output + Send + 'static,
        map_event: impl Fn(Event) -> Message + Send + Sync + 'static,
        map_final: impl FnOnce(Output) -> Message + Send + 'static,
    ) -> CancellationToken
    where
        Event: Send + 'static,
        Output: Send + 'static,
        Message: 'static,
    {
        let token = self.token.clone();
        self.request.latest_stream_with_optional_cancellation(
            Some(self.token),
            work,
            map_event,
            map_final,
        );
        token
    }
}

#[cfg(test)]
mod tests {
    use super::LatestStreamCloseGuard;
    use crate::runtime::BusinessMessageSink;
    use std::{
        panic::{AssertUnwindSafe, catch_unwind},
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
    };

    #[test]
    fn latest_stream_close_guard_closes_when_work_unwinds() {
        let close_count = Arc::new(AtomicUsize::new(0));
        let close_count_for_sink = Arc::clone(&close_count);
        let sink = BusinessMessageSink::new_with_latest(
            |_: ()| true,
            |_: ()| true,
            move || {
                close_count_for_sink.fetch_add(1, Ordering::AcqRel);
            },
        );

        let result = catch_unwind(AssertUnwindSafe(|| {
            let _guard = LatestStreamCloseGuard::new(sink);
            panic!("stream work failed");
        }));

        assert!(result.is_err());
        assert_eq!(close_count.load(Ordering::Acquire), 1);
    }

    #[test]
    fn latest_stream_close_guard_explicit_close_is_not_repeated_on_drop() {
        let close_count = Arc::new(AtomicUsize::new(0));
        let close_count_for_sink = Arc::clone(&close_count);
        let sink = BusinessMessageSink::new_with_latest(
            |_: ()| true,
            |_: ()| true,
            move || {
                close_count_for_sink.fetch_add(1, Ordering::AcqRel);
            },
        );

        LatestStreamCloseGuard::new(sink).close();

        assert_eq!(close_count.load(Ordering::Acquire), 1);
    }
}
