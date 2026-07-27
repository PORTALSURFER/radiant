use crate::application::{CancellationToken, LatestTaskTransaction, TaskCompletion, TaskTicket};

use super::{
    BusinessEventSink, BusinessWorkContext,
    request::{BusinessRequest, LatestStreamCloseGuard},
};

/// Builder for one latest business request.
pub struct BusinessLatestRequest<'context, Message> {
    pub(super) request: BusinessRequest<'context, Message>,
    pub(super) ticket: TaskTicket,
    pub(super) effect_id: u64,
    pub(super) transaction: LatestTaskTransaction,
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
            transaction: self.transaction,
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
        let transaction = self.transaction;
        self.request.context.queue_command(
            crate::runtime::Command::perform_worker_effect_with_identity_and_transaction(
                crate::runtime::EffectId(self.effect_id),
                self.request.name,
                self.request.priority,
                None,
                ticket.id(),
                Some(transaction),
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
        map_event: impl Fn(TaskCompletion<Event>) -> Message + 'static,
        map_final: impl FnOnce(TaskCompletion<Output>) -> Message + 'static,
    ) where
        Event: Send + 'static,
        Output: Send + 'static,
        Message: 'static,
    {
        let effect_id = crate::runtime::EffectId(self.effect_id);
        let ticket = self.ticket;
        let transaction = self.transaction;
        self.request.context.queue_command(
            crate::runtime::Command::perform_worker_stream_with_identity_and_transaction(
                effect_id,
                self.request.name,
                self.request.priority,
                crate::runtime::WorkerStreamOptions {
                    is_cancelled: None,
                    generation: ticket.id(),
                    latest: false,
                },
                Some(transaction),
                move |sink| {
                    let event_sink =
                        BusinessEventSink::new(move |event| sink.emit(Box::new(event)));
                    work(BusinessWorkContext::new(None), event_sink)
                },
                move |event| {
                    map_event(TaskCompletion {
                        ticket,
                        output: event,
                    })
                },
                move |output| map_final(TaskCompletion { ticket, output }),
            ),
        );
    }

    /// Run latest work with coalesced intermediate events tagged with this task ticket.
    pub fn stream_latest<Event, Output>(
        self,
        work: impl FnOnce(BusinessWorkContext, BusinessEventSink<Event>) -> Output + Send + 'static,
        map_event: impl Fn(TaskCompletion<Event>) -> Message + 'static,
        map_final: impl FnOnce(TaskCompletion<Output>) -> Message + 'static,
    ) where
        Event: Send + 'static,
        Output: Send + 'static,
        Message: 'static,
    {
        let effect_id = crate::runtime::EffectId(self.effect_id);
        let ticket = self.ticket;
        let transaction = self.transaction;
        self.request.context.queue_command(
            crate::runtime::Command::perform_worker_stream_with_identity_and_transaction(
                effect_id,
                self.request.name,
                self.request.priority,
                crate::runtime::WorkerStreamOptions {
                    is_cancelled: None,
                    generation: ticket.id(),
                    latest: true,
                },
                Some(transaction),
                move |sink| {
                    let event_sink = BusinessEventSink::new({
                        let sink = sink.clone();
                        move |event| sink.emit_latest(Box::new(event))
                    });
                    let close_guard = LatestStreamCloseGuard::new(sink.clone());
                    let output = work(BusinessWorkContext::new(None), event_sink);
                    close_guard.close();
                    output
                },
                move |event| {
                    map_event(TaskCompletion {
                        ticket,
                        output: event,
                    })
                },
                move |output| map_final(TaskCompletion { ticket, output }),
            ),
        );
    }
}

/// Cancellable builder for one latest business request.
pub struct CancellableBusinessLatestRequest<'context, Message> {
    pub(super) request: BusinessRequest<'context, Message>,
    pub(super) token: CancellationToken,
    pub(super) ticket: TaskTicket,
    pub(super) effect_id: u64,
    pub(super) transaction: LatestTaskTransaction,
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
        let transaction = self.transaction;
        let is_cancelled = Some(Box::new({
            let token = self.token.clone();
            move || token.is_cancelled()
        }) as Box<dyn Fn() -> bool + Send + Sync + 'static>);
        self.request.context.queue_command(
            crate::runtime::Command::perform_worker_effect_with_identity_and_transaction(
                crate::runtime::EffectId(self.effect_id),
                self.request.name,
                self.request.priority,
                is_cancelled,
                ticket.id(),
                Some(transaction),
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
        map_event: impl Fn(TaskCompletion<Event>) -> Message + 'static,
        map_final: impl FnOnce(TaskCompletion<Output>) -> Message + 'static,
    ) -> CancellationToken
    where
        Event: Send + 'static,
        Output: Send + 'static,
        Message: 'static,
    {
        let effect_id = crate::runtime::EffectId(self.effect_id);
        let token = self.token.clone();
        let worker_token = self.token.clone();
        let ticket = self.ticket;
        let transaction = self.transaction;
        let is_cancelled = Some(Box::new({
            let token = self.token.clone();
            move || token.is_cancelled()
        }) as Box<dyn Fn() -> bool + Send + Sync + 'static>);
        self.request.context.queue_command(
            crate::runtime::Command::perform_worker_stream_with_identity_and_transaction(
                effect_id,
                self.request.name,
                self.request.priority,
                crate::runtime::WorkerStreamOptions {
                    is_cancelled,
                    generation: ticket.id(),
                    latest: false,
                },
                Some(transaction),
                move |sink| {
                    let event_sink =
                        BusinessEventSink::new(move |event| sink.emit(Box::new(event)));
                    work(BusinessWorkContext::new(Some(worker_token)), event_sink)
                },
                move |event| {
                    map_event(TaskCompletion {
                        ticket,
                        output: event,
                    })
                },
                move |output| map_final(TaskCompletion { ticket, output }),
            ),
        );
        token
    }

    /// Run cancellable latest work with coalesced intermediate events and return its cancellation token.
    pub fn stream_latest<Event, Output>(
        self,
        work: impl FnOnce(BusinessWorkContext, BusinessEventSink<Event>) -> Output + Send + 'static,
        map_event: impl Fn(TaskCompletion<Event>) -> Message + 'static,
        map_final: impl FnOnce(TaskCompletion<Output>) -> Message + 'static,
    ) -> CancellationToken
    where
        Event: Send + 'static,
        Output: Send + 'static,
        Message: 'static,
    {
        let effect_id = crate::runtime::EffectId(self.effect_id);
        let token = self.token.clone();
        let worker_token = self.token.clone();
        let ticket = self.ticket;
        let transaction = self.transaction;
        let is_cancelled = Some(Box::new({
            let token = self.token.clone();
            move || token.is_cancelled()
        }) as Box<dyn Fn() -> bool + Send + Sync + 'static>);
        self.request.context.queue_command(
            crate::runtime::Command::perform_worker_stream_with_identity_and_transaction(
                effect_id,
                self.request.name,
                self.request.priority,
                crate::runtime::WorkerStreamOptions {
                    is_cancelled,
                    generation: ticket.id(),
                    latest: true,
                },
                Some(transaction),
                move |sink| {
                    let event_sink = BusinessEventSink::new({
                        let sink = sink.clone();
                        move |event| sink.emit_latest(Box::new(event))
                    });
                    let close_guard = LatestStreamCloseGuard::new(sink.clone());
                    let output = work(BusinessWorkContext::new(Some(worker_token)), event_sink);
                    close_guard.close();
                    output
                },
                move |event| {
                    map_event(TaskCompletion {
                        ticket,
                        output: event,
                    })
                },
                move |output| map_final(TaskCompletion { ticket, output }),
            ),
        );
        token
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::runtime::update_context::UiUpdateContext;
    use crate::runtime::Command;

    fn assert_transaction(command: Command<()>, ticket: TaskTicket) {
        let Command::PerformWorker(effect) = command else {
            panic!("latest business request should queue a worker effect");
        };
        assert_eq!(
            effect
                .transaction
                .as_ref()
                .expect("latest effect should carry its transaction")
                .replacement(),
            ticket
        );
    }

    #[test]
    fn every_non_keyed_latest_worker_form_publishes_a_transaction() {
        let forms = [
            "run",
            "run_on_ui",
            "stream",
            "stream_latest",
            "cancellable_run",
            "cancellable_run_on_ui",
            "cancellable_stream",
            "cancellable_stream_latest",
        ];
        for form in forms {
            let mut latest = crate::application::LatestTask::new();
            let mut context = UiUpdateContext::<()>::default();
            let ticket = match form {
                "run" | "run_on_ui" => {
                    let request = context.business().background("latest").latest(&mut latest);
                    let ticket = request.ticket();
                    if form == "run" {
                        request.run(|_| 1_u8, |_| ());
                    } else {
                        request.run_on_ui(|_| 1_u8, |_| ());
                    }
                    ticket
                }
                "stream" | "stream_latest" => {
                    let request = context.business().background("latest").latest(&mut latest);
                    let ticket = request.ticket();
                    if form == "stream" {
                        request.stream(|_, _: BusinessEventSink<u8>| 1_u8, |_| (), |_| ());
                    } else {
                        request.stream_latest(|_, _: BusinessEventSink<u8>| 1_u8, |_| (), |_| ());
                    }
                    ticket
                }
                "cancellable_run" | "cancellable_run_on_ui" => {
                    let request = context
                        .business()
                        .background("latest")
                        .cancellable()
                        .latest(&mut latest);
                    let ticket = request.ticket();
                    if form == "cancellable_run" {
                        request.run(|_| 1_u8, |_| ());
                    } else {
                        request.run_on_ui(|_| 1_u8, |_| ());
                    }
                    ticket
                }
                "cancellable_stream" | "cancellable_stream_latest" => {
                    let request = context
                        .business()
                        .background("latest")
                        .cancellable()
                        .latest(&mut latest);
                    let ticket = request.ticket();
                    if form == "cancellable_stream" {
                        request.stream(|_, _: BusinessEventSink<u8>| 1_u8, |_| (), |_| ());
                    } else {
                        request.stream_latest(|_, _: BusinessEventSink<u8>| 1_u8, |_| (), |_| ());
                    }
                    ticket
                }
                _ => unreachable!(),
            };
            assert_transaction(context.into_command(), ticket);
        }
    }
}
