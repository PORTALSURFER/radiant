use crate::application::{CancellationToken, LatestTaskTransaction, TaskCompletion, TaskTicket};

use super::{
    BusinessEventSink, BusinessWorkContext,
    admission::{AdmissionReceiptGuard, BusinessTaskAdmissionReceipt},
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

    /// Run latest work and return a UI-local receipt for host admission.
    pub fn run_with_receipt<Output>(
        self,
        work: impl FnOnce(BusinessWorkContext) -> Output + Send + 'static,
        map: impl FnOnce(TaskCompletion<Output>) -> Message + 'static,
    ) -> BusinessTaskAdmissionReceipt
    where
        Output: Send + 'static,
    {
        let receipt = BusinessTaskAdmissionReceipt::new();
        let guard = AdmissionReceiptGuard(receipt.weak());
        let ticket = self.ticket;
        let transaction = self.transaction;
        self.request.context.queue_command(
            crate::runtime::Command::perform_worker_effect_with_identity_and_transaction_and_receipt(
                crate::runtime::EffectId(self.effect_id),
                self.request.name,
                self.request.priority,
                None,
                ticket.id(),
                Some(transaction),
                Some(guard),
                move || TaskCompletion {
                    ticket,
                    output: work(BusinessWorkContext::new(None)),
                },
                map,
            ),
        );
        receipt
    }

    /// Run latest work only when `owner` resolves to one current, eligible
    /// keyed-node or overlay owner in the accepted surface, and return a
    /// UI-local receipt for host admission.
    ///
    /// The latest replacement is rolled back when owner or host admission
    /// fails. Accepted work retains the latest ticket and is fenced by both
    /// that ticket and the resolved declarative owner generation.
    pub fn run_for_owner_with_receipt<Output>(
        self,
        owner: crate::application::DeclarativeEffectOwner,
        work: impl FnOnce(BusinessWorkContext) -> Output + Send + 'static,
        map: impl FnOnce(TaskCompletion<Output>) -> Message + 'static,
    ) -> BusinessTaskAdmissionReceipt
    where
        Output: Send + 'static,
    {
        let receipt = BusinessTaskAdmissionReceipt::new();
        let guard = AdmissionReceiptGuard(receipt.weak());
        let ticket = self.ticket;
        let transaction = self.transaction;
        self.request.context.queue_command(
            crate::runtime::Command::perform_worker_effect_with_identity_and_transaction_and_receipt_for_owner(
                crate::runtime::EffectId(self.effect_id),
                self.request.name,
                self.request.priority,
                None,
                ticket.id(),
                Some(transaction),
                Some(guard),
                Some(owner),
                move |cancellation_probe| TaskCompletion {
                    ticket,
                    output: work(BusinessWorkContext::new_with_probe(
                        None,
                        cancellation_probe,
                    )),
                },
                map,
            ),
        );
        receipt
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

    /// Run ordered latest work only while `owner` resolves to one current,
    /// eligible keyed-node or overlay owner, and return a UI-local receipt
    /// for host admission.
    ///
    /// Intermediate events retain FIFO delivery and both event and final
    /// outputs carry this request's exact latest-task ticket. Admission and
    /// mapping are fenced by the latest ticket and resolved owner generation.
    pub fn stream_for_owner_with_receipt<Event, Output>(
        self,
        owner: crate::application::DeclarativeEffectOwner,
        work: impl FnOnce(BusinessWorkContext, BusinessEventSink<Event>) -> Output + Send + 'static,
        map_event: impl Fn(TaskCompletion<Event>) -> Message + 'static,
        map_final: impl FnOnce(TaskCompletion<Output>) -> Message + 'static,
    ) -> BusinessTaskAdmissionReceipt
    where
        Event: Send + 'static,
        Output: Send + 'static,
        Message: 'static,
    {
        let receipt = BusinessTaskAdmissionReceipt::new();
        let guard = AdmissionReceiptGuard(receipt.weak());
        let effect_id = crate::runtime::EffectId(self.effect_id);
        let ticket = self.ticket;
        let transaction = self.transaction;
        self.request.context.queue_command(
            crate::runtime::Command::perform_worker_stream_with_identity_and_transaction_and_receipt_for_owner(
                effect_id,
                self.request.name,
                self.request.priority,
                crate::runtime::WorkerStreamOptions {
                    is_cancelled: None,
                    generation: ticket.id(),
                    latest: false,
                },
                Some(transaction),
                Some(guard),
                Some(owner),
                move |sink, cancellation_probe| {
                    let event_sink =
                        BusinessEventSink::new(move |event| sink.emit(Box::new(event)));
                    work(
                        BusinessWorkContext::new_with_probe(None, cancellation_probe),
                        event_sink,
                    )
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
        receipt
    }

    /// Run coalesced latest work only while `owner` resolves to one current,
    /// eligible keyed-node or overlay owner, and return a UI-local receipt
    /// for host admission.
    ///
    /// Intermediate events use the existing latest-wins stream slot while the
    /// UI is behind; the final output remains an uncoalesced terminal delivery.
    /// Both outputs carry this request's exact latest-task ticket, and
    /// admission and mapping are fenced by the latest ticket and resolved
    /// owner generation.
    pub fn stream_latest_for_owner_with_receipt<Event, Output>(
        self,
        owner: crate::application::DeclarativeEffectOwner,
        work: impl FnOnce(BusinessWorkContext, BusinessEventSink<Event>) -> Output + Send + 'static,
        map_event: impl Fn(TaskCompletion<Event>) -> Message + 'static,
        map_final: impl FnOnce(TaskCompletion<Output>) -> Message + 'static,
    ) -> BusinessTaskAdmissionReceipt
    where
        Event: Send + 'static,
        Output: Send + 'static,
        Message: 'static,
    {
        let receipt = BusinessTaskAdmissionReceipt::new();
        let guard = AdmissionReceiptGuard(receipt.weak());
        let effect_id = crate::runtime::EffectId(self.effect_id);
        let ticket = self.ticket;
        let transaction = self.transaction;
        self.request.context.queue_command(
            crate::runtime::Command::perform_worker_stream_with_identity_and_transaction_and_receipt_for_owner(
                effect_id,
                self.request.name,
                self.request.priority,
                crate::runtime::WorkerStreamOptions {
                    is_cancelled: None,
                    generation: ticket.id(),
                    latest: true,
                },
                Some(transaction),
                Some(guard),
                Some(owner),
                move |sink, cancellation_probe| {
                    let event_sink = BusinessEventSink::new({
                        let sink = sink.clone();
                        move |event| sink.emit_latest(Box::new(event))
                    });
                    let close_guard = LatestStreamCloseGuard::new(sink.clone());
                    let output = work(
                        BusinessWorkContext::new_with_probe(None, cancellation_probe),
                        event_sink,
                    );
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
        receipt
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

    /// Run cancellable latest work only while `owner` resolves to one current,
    /// eligible keyed-node or overlay owner, and return a UI-local admission
    /// receipt.
    ///
    /// Call [`Self::token`] before consuming the request when the caller needs
    /// to cancel the admitted work later. The receipt reports admission only;
    /// cancelling the token does not change an already resolved receipt. The
    /// latest ticket and owner generation fence work, mapping, and reduction.
    pub fn run_for_owner_with_receipt<Output>(
        self,
        owner: crate::application::DeclarativeEffectOwner,
        work: impl FnOnce(BusinessWorkContext) -> Output + Send + 'static,
        map: impl FnOnce(TaskCompletion<Output>) -> Message + 'static,
    ) -> BusinessTaskAdmissionReceipt
    where
        Output: Send + 'static,
    {
        let receipt = BusinessTaskAdmissionReceipt::new();
        let guard = AdmissionReceiptGuard(receipt.weak());
        let worker_token = self.token.clone();
        let is_cancelled = Some(Box::new({
            let token = self.token.clone();
            move || token.is_cancelled()
        }) as Box<dyn Fn() -> bool + Send + Sync + 'static>);
        let ticket = self.ticket;
        let transaction = self.transaction;
        self.request.context.queue_command(
            crate::runtime::Command::perform_worker_effect_with_identity_and_transaction_and_receipt_for_owner(
                crate::runtime::EffectId(self.effect_id),
                self.request.name,
                self.request.priority,
                is_cancelled,
                ticket.id(),
                Some(transaction),
                Some(guard),
                Some(owner),
                move |cancellation_probe| TaskCompletion {
                    ticket,
                    output: work(BusinessWorkContext::new_with_probe(
                        Some(worker_token),
                        cancellation_probe,
                    )),
                },
                map,
            ),
        );
        receipt
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

    /// Run cancellable ordered latest work only while `owner` resolves to one
    /// current, eligible keyed-node or overlay owner, and return an admission
    /// receipt.
    ///
    /// Call [`Self::token`] before consuming the request when the caller needs
    /// to cancel the admitted work later. Intermediate events retain FIFO
    /// delivery and both event and final mapping are fenced by the explicit
    /// token, latest ticket, and owner generation.
    pub fn stream_for_owner_with_receipt<Event, Output>(
        self,
        owner: crate::application::DeclarativeEffectOwner,
        work: impl FnOnce(BusinessWorkContext, BusinessEventSink<Event>) -> Output + Send + 'static,
        map_event: impl Fn(TaskCompletion<Event>) -> Message + 'static,
        map_final: impl FnOnce(TaskCompletion<Output>) -> Message + 'static,
    ) -> BusinessTaskAdmissionReceipt
    where
        Event: Send + 'static,
        Output: Send + 'static,
        Message: 'static,
    {
        let receipt = BusinessTaskAdmissionReceipt::new();
        let guard = AdmissionReceiptGuard(receipt.weak());
        let worker_token = self.token.clone();
        let is_cancelled = Some(Box::new({
            let token = self.token.clone();
            move || token.is_cancelled()
        }) as Box<dyn Fn() -> bool + Send + Sync + 'static>);
        let effect_id = crate::runtime::EffectId(self.effect_id);
        let ticket = self.ticket;
        let transaction = self.transaction;
        self.request.context.queue_command(
            crate::runtime::Command::perform_worker_stream_with_identity_and_transaction_and_receipt_for_owner(
                effect_id,
                self.request.name,
                self.request.priority,
                crate::runtime::WorkerStreamOptions {
                    is_cancelled,
                    generation: ticket.id(),
                    latest: false,
                },
                Some(transaction),
                Some(guard),
                Some(owner),
                move |sink, cancellation_probe| {
                    let event_sink =
                        BusinessEventSink::new(move |event| sink.emit(Box::new(event)));
                    work(
                        BusinessWorkContext::new_with_probe(
                            Some(worker_token),
                            cancellation_probe,
                        ),
                        event_sink,
                    )
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
        receipt
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

    /// Run cancellable coalesced latest work only while `owner` resolves to
    /// one current, eligible keyed-node or overlay owner, and return an
    /// admission receipt.
    ///
    /// Call [`Self::token`] before consuming the request when the caller needs
    /// to cancel the admitted work later. Intermediate events use the existing
    /// latest-wins slot while the UI is behind; the final remains an
    /// uncoalesced terminal delivery. Both mappings are fenced by the explicit
    /// token, latest ticket, and owner generation.
    pub fn stream_latest_for_owner_with_receipt<Event, Output>(
        self,
        owner: crate::application::DeclarativeEffectOwner,
        work: impl FnOnce(BusinessWorkContext, BusinessEventSink<Event>) -> Output + Send + 'static,
        map_event: impl Fn(TaskCompletion<Event>) -> Message + 'static,
        map_final: impl FnOnce(TaskCompletion<Output>) -> Message + 'static,
    ) -> BusinessTaskAdmissionReceipt
    where
        Event: Send + 'static,
        Output: Send + 'static,
        Message: 'static,
    {
        let receipt = BusinessTaskAdmissionReceipt::new();
        let guard = AdmissionReceiptGuard(receipt.weak());
        let worker_token = self.token.clone();
        let is_cancelled = Some(Box::new({
            let token = self.token.clone();
            move || token.is_cancelled()
        }) as Box<dyn Fn() -> bool + Send + Sync + 'static>);
        let effect_id = crate::runtime::EffectId(self.effect_id);
        let ticket = self.ticket;
        let transaction = self.transaction;
        self.request.context.queue_command(
            crate::runtime::Command::perform_worker_stream_with_identity_and_transaction_and_receipt_for_owner(
                effect_id,
                self.request.name,
                self.request.priority,
                crate::runtime::WorkerStreamOptions {
                    is_cancelled,
                    generation: ticket.id(),
                    latest: true,
                },
                Some(transaction),
                Some(guard),
                Some(owner),
                move |sink, cancellation_probe| {
                    let event_sink = BusinessEventSink::new({
                        let sink = sink.clone();
                        move |event| sink.emit_latest(Box::new(event))
                    });
                    let close_guard = LatestStreamCloseGuard::new(sink.clone());
                    let output = work(
                        BusinessWorkContext::new_with_probe(
                            Some(worker_token),
                            cancellation_probe,
                        ),
                        event_sink,
                    );
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
        receipt
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

    #[test]
    fn abandoned_latest_builders_restore_the_previous_ticket() {
        let mut latest = crate::application::LatestTask::new();
        let predecessor = latest.begin();
        let mut context = UiUpdateContext::<()>::default();
        let request = context.business().background("latest").latest(&mut latest);
        assert_ne!(request.ticket(), predecessor);
        drop(request);
        assert_eq!(latest.active(), Some(predecessor));

        let mut context = UiUpdateContext::<()>::default();
        let request = context
            .business()
            .background("latest")
            .cancellable()
            .latest(&mut latest);
        assert_ne!(request.ticket(), predecessor);
        drop(request);
        assert_eq!(latest.active(), Some(predecessor));
    }

    #[test]
    fn owner_latest_worker_form_publishes_owner_transaction_and_receipt() {
        let owner = crate::application::DeclarativeEffectOwner::new();
        let mut latest = crate::application::LatestTask::new();
        let predecessor = latest.begin();
        let mut context = UiUpdateContext::<()>::default();
        let request = context
            .business()
            .background("owner-latest")
            .latest(&mut latest);
        let ticket = request.ticket();
        let receipt = request.run_for_owner_with_receipt(owner, |_| 1_u8, |_| ());

        let Command::PerformWorker(effect) = context.into_command() else {
            panic!("owner latest request should queue a worker effect");
        };
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Pending
        );
        assert_eq!(effect.owner, Some(owner));
        assert_eq!(effect.generation.0, ticket.id());
        assert_eq!(
            effect
                .transaction
                .as_ref()
                .expect("owner latest effect should carry its transaction")
                .replacement(),
            ticket
        );
        assert_eq!(latest.active(), Some(ticket));
        assert_ne!(ticket, predecessor);
    }

    #[test]
    fn owner_latest_ordered_stream_publishes_transaction_receipt_and_identity() {
        let owner = crate::application::DeclarativeEffectOwner::new();
        let mut latest = crate::application::LatestTask::new();
        let predecessor = latest.begin();
        let mut context = UiUpdateContext::<()>::default();
        let request = context
            .business()
            .background("owner-latest-stream")
            .latest(&mut latest);
        let ticket = request.ticket();
        let receipt = request.stream_for_owner_with_receipt(
            owner,
            |_, events| {
                assert!(events.emit(1_u8));
                2_u8
            },
            |_| (),
            |_| (),
        );

        let Command::PerformWorker(effect) = context.into_command() else {
            panic!("owner latest stream should queue a worker effect");
        };
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Pending
        );
        assert_eq!(effect.owner, Some(owner));
        assert_eq!(effect.generation.0, ticket.id());
        assert_eq!(
            effect
                .transaction
                .as_ref()
                .expect("owner latest stream should carry its transaction")
                .replacement(),
            ticket
        );
        assert_eq!(latest.active(), Some(ticket));
        assert_ne!(ticket, predecessor);
    }

    #[test]
    fn owner_latest_coalesced_stream_publishes_owner_receipt_generation_transaction_and_latest_mode()
     {
        let owner = crate::application::DeclarativeEffectOwner::new();
        let mut latest = crate::application::LatestTask::new();
        let predecessor = latest.begin();
        let mut context = UiUpdateContext::<()>::default();
        let request = context
            .business()
            .background("owner-latest-coalesced-stream")
            .latest(&mut latest);
        let ticket = request.ticket();
        let receipt = request.stream_latest_for_owner_with_receipt(
            owner,
            |_, events| {
                assert!(events.emit(1_u8));
                2_u8
            },
            |_| (),
            |_| (),
        );

        let Command::PerformWorker(effect) = context.into_command() else {
            panic!("owner latest coalesced stream should queue a worker effect");
        };
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Pending
        );
        assert_eq!(effect.owner, Some(owner));
        assert_eq!(effect.generation.0, ticket.id());
        assert_ne!(effect.id.0, 0);
        assert_eq!(
            effect
                .transaction
                .as_ref()
                .expect("owner latest coalesced stream should carry its transaction")
                .replacement(),
            ticket
        );
        assert_eq!(latest.active(), Some(ticket));
        assert_ne!(ticket, predecessor);
    }

    #[test]
    fn cancellable_owner_latest_forms_publish_token_transaction_generation_and_mode() {
        let owner = crate::application::DeclarativeEffectOwner::new();

        let mut latest = crate::application::LatestTask::new();
        let predecessor = latest.begin();
        let mut context = UiUpdateContext::<()>::default();
        let request = context
            .business()
            .background("cancellable-owner-latest")
            .cancellable()
            .latest(&mut latest);
        let token = request.token();
        let ticket = request.ticket();
        let receipt = request.run_for_owner_with_receipt(owner, |_| 1_u8, |_| ());
        let Command::PerformWorker(effect) = context.into_command() else {
            panic!("cancellable owner latest request should queue a worker effect");
        };
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Pending
        );
        assert_eq!(effect.owner, Some(owner));
        assert_eq!(effect.generation.0, ticket.id());
        assert_eq!(
            effect
                .transaction
                .as_ref()
                .expect("cancellable owner latest effect should carry its transaction")
                .replacement(),
            ticket
        );
        assert!(!effect.is_cancelled.as_ref().expect(
            "cancellable owner latest effect should carry its token probe"
        )());
        token.cancel();
        assert!(effect.is_cancelled.as_ref().expect(
            "cancellable owner latest effect should retain its token probe"
        )());
        assert_eq!(latest.active(), Some(ticket));
        assert_ne!(ticket, predecessor);

        let mut latest = crate::application::LatestTask::new();
        let predecessor = latest.begin();
        let mut context = UiUpdateContext::<()>::default();
        let request = context
            .business()
            .background("cancellable-owner-latest-stream")
            .cancellable()
            .latest(&mut latest);
        let token = request.token();
        let ticket = request.ticket();
        let receipt = request.stream_for_owner_with_receipt(
            owner,
            |_, _: BusinessEventSink<u8>| 1_u8,
            |_: TaskCompletion<u8>| (),
            |_: TaskCompletion<u8>| (),
        );
        let Command::PerformWorker(effect) = context.into_command() else {
            panic!("cancellable owner ordered latest stream should queue a worker effect");
        };
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Pending
        );
        assert_eq!(effect.owner, Some(owner));
        assert_eq!(effect.generation.0, ticket.id());
        assert_eq!(
            effect
                .transaction
                .as_ref()
                .expect("cancellable owner ordered latest stream should carry its transaction")
                .replacement(),
            ticket
        );
        assert!(!effect.is_cancelled.as_ref().expect(
            "cancellable owner ordered latest stream should carry its token probe"
        )());
        token.cancel();
        assert!(effect.is_cancelled.as_ref().expect(
            "cancellable owner ordered latest stream should retain its token probe"
        )());
        assert_eq!(latest.active(), Some(ticket));
        assert_ne!(ticket, predecessor);

        let mut latest = crate::application::LatestTask::new();
        let predecessor = latest.begin();
        let mut context = UiUpdateContext::<()>::default();
        let request = context
            .business()
            .background("cancellable-owner-latest-coalesced-stream")
            .cancellable()
            .latest(&mut latest);
        let token = request.token();
        let ticket = request.ticket();
        let receipt = request.stream_latest_for_owner_with_receipt(
            owner,
            |_, _: BusinessEventSink<u8>| 1_u8,
            |_: TaskCompletion<u8>| (),
            |_: TaskCompletion<u8>| (),
        );
        let Command::PerformWorker(effect) = context.into_command() else {
            panic!("cancellable owner coalesced latest stream should queue a worker effect");
        };
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Pending
        );
        assert_eq!(effect.owner, Some(owner));
        assert_eq!(effect.generation.0, ticket.id());
        assert_eq!(
            effect
                .transaction
                .as_ref()
                .expect("cancellable owner coalesced latest stream should carry its transaction")
                .replacement(),
            ticket
        );
        assert!(!effect.is_cancelled.as_ref().expect(
            "cancellable owner coalesced latest stream should carry its token probe"
        )());
        token.cancel();
        assert!(effect.is_cancelled.as_ref().expect(
            "cancellable owner coalesced latest stream should retain its token probe"
        )());
        assert_eq!(latest.active(), Some(ticket));
        assert_ne!(ticket, predecessor);
    }
}
