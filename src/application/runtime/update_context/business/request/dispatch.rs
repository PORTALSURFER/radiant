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
        map: impl FnOnce(Output) -> Message + 'static,
    ) where
        Output: Send + 'static,
    {
        self.run_with_optional_cancellation(None, work, map);
    }

    /// Run worker-only work and map its owned output on the UI runtime.
    ///
    /// The existing [`Self::run`] spelling remains the compatibility path;
    /// this explicit variant proves the worker/output ownership boundary.
    pub fn run_on_ui<Output>(
        self,
        work: impl FnOnce(BusinessWorkContext) -> Output + Send + 'static,
        map: impl FnOnce(Output) -> Message + 'static,
    ) where
        Output: Send + 'static,
    {
        self.run(work, map);
    }

    /// Run this business request and allow worker code to emit intermediate
    /// events before the final output message.
    pub fn stream<Event, Output>(
        self,
        work: impl FnOnce(BusinessWorkContext, BusinessEventSink<Event>) -> Output + Send + 'static,
        map_event: impl Fn(Event) -> Message + 'static,
        map_final: impl FnOnce(Output) -> Message + 'static,
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
        map_event: impl Fn(Event) -> Message + 'static,
        map_final: impl FnOnce(Output) -> Message + 'static,
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
        map: impl FnOnce(Output) -> Message + 'static,
    ) where
        Output: Send + 'static,
    {
        let worker_token = token.clone();
        let is_cancelled = token.map(|token| {
            Box::new(move || token.is_cancelled()) as Box<dyn Fn() -> bool + Send + Sync + 'static>
        });
        self.context
            .queue_command(Command::perform_worker_effect_with_priority(
                self.name,
                self.priority,
                is_cancelled,
                0,
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
        map_event: impl Fn(Event) -> Message + 'static,
        map_final: impl FnOnce(Output) -> Message + 'static,
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
            .queue_command(Command::perform_worker_stream_with_priority(
                self.name,
                self.priority,
                is_cancelled,
                0,
                false,
                move |sink| {
                    let event_sink =
                        BusinessEventSink::new(move |event| sink.emit(Box::new(event)));
                    work(BusinessWorkContext::new(worker_token), event_sink)
                },
                map_event,
                map_final,
            ));
    }

    pub(in crate::application::runtime::update_context::business) fn latest_stream_with_optional_cancellation<
        Event,
        Output,
    >(
        self,
        token: Option<CancellationToken>,
        work: impl FnOnce(BusinessWorkContext, BusinessEventSink<Event>) -> Output + Send + 'static,
        map_event: impl Fn(Event) -> Message + 'static,
        map_final: impl FnOnce(Output) -> Message + 'static,
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
            .queue_command(Command::perform_worker_stream_with_priority(
                self.name,
                self.priority,
                is_cancelled,
                0,
                true,
                move |sink| {
                    let event_sink = BusinessEventSink::new({
                        let sink = sink.clone();
                        move |event| sink.emit_latest(Box::new(event))
                    });
                    let close_guard = LatestStreamCloseGuard::new(sink.clone());
                    let output = work(BusinessWorkContext::new(worker_token), event_sink);
                    close_guard.close();
                    output
                },
                map_event,
                map_final,
            ));
    }
}
