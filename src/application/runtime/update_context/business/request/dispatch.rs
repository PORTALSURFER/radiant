use super::{BusinessRequest, stream_guard::LatestStreamCloseGuard};
use crate::application::runtime::update_context::business::{
    BusinessEventSink, BusinessWorkContext,
};
use crate::{application::CancellationToken, runtime::Command};

impl<'context, Message> BusinessRequest<'context, Message> {
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

    pub(in crate::application::runtime::update_context::business) fn run_with_optional_cancellation<
        Output,
    >(
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

    pub(in crate::application::runtime::update_context::business) fn stream_with_optional_cancellation<
        Event,
        Output,
    >(
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

    pub(in crate::application::runtime::update_context::business) fn latest_stream_with_optional_cancellation<
        Event,
        Output,
    >(
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
