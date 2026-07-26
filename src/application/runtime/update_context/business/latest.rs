use crate::application::{CancellationToken, TaskCompletion, TaskTicket};

use super::{BusinessEventSink, BusinessWorkContext, request::BusinessRequest};

/// Builder for one latest business request.
pub struct BusinessLatestRequest<'context, Message> {
    pub(super) request: BusinessRequest<'context, Message>,
    pub(super) ticket: TaskTicket,
    pub(super) effect_id: u64,
}

impl<Message> BusinessLatestRequest<'_, Message> {
    /// Return the task ticket assigned to this request.
    pub fn ticket(&self) -> TaskTicket {
        self.ticket
    }
}

impl<'context, Message> BusinessLatestRequest<'context, Message> {
    /// Make this latest request cooperatively cancellable.
    pub fn cancellable(self) -> CancellableBusinessLatestRequest<'context, Message> {
        CancellableBusinessLatestRequest {
            request: self.request,
            token: CancellationToken::new(),
            ticket: self.ticket,
            effect_id: self.effect_id,
        }
    }

    /// Run latest work and tag the output with its task ticket.
    pub fn run<Output>(
        self,
        work: impl FnOnce(BusinessWorkContext) -> Output + Send + 'static,
        map: impl FnOnce(TaskCompletion<Output>) -> Message + 'static,
    ) where
        Output: Send + 'static,
    {
        let ticket = self.ticket;
        self.request.context.queue_command(
            crate::runtime::Command::perform_worker_effect_with_identity(
                crate::runtime::EffectId(self.effect_id),
                self.request.name,
                self.request.priority,
                None,
                ticket.id(),
                move || TaskCompletion {
                    ticket,
                    output: work(BusinessWorkContext::new(None)),
                },
                map,
            ),
        );
    }

    /// Run latest worker-only work and map its output on the UI runtime.
    pub fn run_on_ui<Output>(
        self,
        work: impl FnOnce(BusinessWorkContext) -> Output + Send + 'static,
        map: impl FnOnce(TaskCompletion<Output>) -> Message + 'static,
    ) where
        Output: Send + 'static,
    {
        self.run(work, map);
    }

    /// Run latest work that may emit intermediate events tagged with this task ticket.
    pub fn stream<Event, Output>(
        self,
        work: impl FnOnce(BusinessWorkContext, BusinessEventSink<Event>) -> Output + Send + 'static,
        map_event: impl Fn(TaskCompletion<Event>) -> Message + Send + Sync + 'static,
        map_final: impl FnOnce(TaskCompletion<Output>) -> Message + Send + 'static,
    ) where
        Event: Send + 'static,
        Output: Send + 'static,
        Message: 'static,
    {
        let ticket = self.ticket;
        self.request.stream(
            work,
            move |event| {
                map_event(TaskCompletion {
                    ticket,
                    output: event,
                })
            },
            move |output| map_final(TaskCompletion { ticket, output }),
        );
    }

    /// Run latest work with coalesced intermediate events tagged with this task ticket.
    pub fn stream_latest<Event, Output>(
        self,
        work: impl FnOnce(BusinessWorkContext, BusinessEventSink<Event>) -> Output + Send + 'static,
        map_event: impl Fn(TaskCompletion<Event>) -> Message + Send + Sync + 'static,
        map_final: impl FnOnce(TaskCompletion<Output>) -> Message + Send + 'static,
    ) where
        Event: Send + 'static,
        Output: Send + 'static,
        Message: 'static,
    {
        let ticket = self.ticket;
        self.request.stream_latest(
            work,
            move |event| {
                map_event(TaskCompletion {
                    ticket,
                    output: event,
                })
            },
            move |output| map_final(TaskCompletion { ticket, output }),
        );
    }
}

/// Cancellable builder for one latest business request.
pub struct CancellableBusinessLatestRequest<'context, Message> {
    pub(super) request: BusinessRequest<'context, Message>,
    pub(super) token: CancellationToken,
    pub(super) ticket: TaskTicket,
    pub(super) effect_id: u64,
}

impl<Message> CancellableBusinessLatestRequest<'_, Message> {
    /// Return the task ticket assigned to this request.
    pub fn ticket(&self) -> TaskTicket {
        self.ticket
    }

    /// Return a clone of the cancellation token owned by this request.
    pub fn token(&self) -> CancellationToken {
        self.token.clone()
    }
}

impl<'context, Message> CancellableBusinessLatestRequest<'context, Message> {
    /// Run cancellable latest work and return its cancellation token.
    pub fn run<Output>(
        self,
        work: impl FnOnce(BusinessWorkContext) -> Output + Send + 'static,
        map: impl FnOnce(TaskCompletion<Output>) -> Message + 'static,
    ) -> CancellationToken
    where
        Output: Send + 'static,
    {
        let token = self.token.clone();
        let worker_token = self.token.clone();
        let ticket = self.ticket;
        let is_cancelled = Some(Box::new({
            let token = self.token.clone();
            move || token.is_cancelled()
        }) as Box<dyn Fn() -> bool + Send + Sync + 'static>);
        self.request.context.queue_command(
            crate::runtime::Command::perform_worker_effect_with_identity(
                crate::runtime::EffectId(self.effect_id),
                self.request.name,
                self.request.priority,
                is_cancelled,
                ticket.id(),
                move || TaskCompletion {
                    ticket,
                    output: work(BusinessWorkContext::new(Some(worker_token))),
                },
                map,
            ),
        );
        token
    }

    /// Run cancellable latest worker-only work and map its output on the UI runtime.
    pub fn run_on_ui<Output>(
        self,
        work: impl FnOnce(BusinessWorkContext) -> Output + Send + 'static,
        map: impl FnOnce(TaskCompletion<Output>) -> Message + 'static,
    ) -> CancellationToken
    where
        Output: Send + 'static,
    {
        self.run(work, map)
    }

    /// Run cancellable latest work that may emit intermediate events tagged with this task ticket.
    pub fn stream<Event, Output>(
        self,
        work: impl FnOnce(BusinessWorkContext, BusinessEventSink<Event>) -> Output + Send + 'static,
        map_event: impl Fn(TaskCompletion<Event>) -> Message + Send + Sync + 'static,
        map_final: impl FnOnce(TaskCompletion<Output>) -> Message + Send + 'static,
    ) -> CancellationToken
    where
        Event: Send + 'static,
        Output: Send + 'static,
        Message: 'static,
    {
        let token = self.token.clone();
        let ticket = self.ticket;
        self.request.stream_with_optional_cancellation(
            Some(self.token),
            work,
            move |event| {
                map_event(TaskCompletion {
                    ticket,
                    output: event,
                })
            },
            move |output| map_final(TaskCompletion { ticket, output }),
        );
        token
    }

    /// Run cancellable latest work with coalesced intermediate events and return its cancellation token.
    pub fn stream_latest<Event, Output>(
        self,
        work: impl FnOnce(BusinessWorkContext, BusinessEventSink<Event>) -> Output + Send + 'static,
        map_event: impl Fn(TaskCompletion<Event>) -> Message + Send + Sync + 'static,
        map_final: impl FnOnce(TaskCompletion<Output>) -> Message + Send + 'static,
    ) -> CancellationToken
    where
        Event: Send + 'static,
        Output: Send + 'static,
        Message: 'static,
    {
        let token = self.token.clone();
        let ticket = self.ticket;
        self.request.latest_stream_with_optional_cancellation(
            Some(self.token),
            work,
            move |event| {
                map_event(TaskCompletion {
                    ticket,
                    output: event,
                })
            },
            move |output| map_final(TaskCompletion { ticket, output }),
        );
        token
    }
}
