use super::{BusinessRequest, stream_guard::LatestStreamCloseGuard};
use crate::application::runtime::update_context::business::{
    BusinessEventSink, BusinessWorkContext,
    admission::{AdmissionReceiptGuard, BusinessTaskAdmissionReceipt},
};
use crate::{
    application::{CancellationToken, LatestTaskTransaction},
    runtime::{Command, EffectId},
};

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

    /// Run this business request and return a UI-local receipt for host admission.
    ///
    /// The receipt is resolved by the controller after it attempts actual host
    /// admission. It does not dispatch a callback or enqueue a retry.
    pub fn run_with_receipt<Output>(
        self,
        work: impl FnOnce(BusinessWorkContext) -> Output + Send + 'static,
        map: impl FnOnce(Output) -> Message + 'static,
    ) -> BusinessTaskAdmissionReceipt
    where
        Output: Send + 'static,
    {
        let receipt = BusinessTaskAdmissionReceipt::new();
        let guard = AdmissionReceiptGuard(receipt.weak());
        self.context
            .queue_command(Command::perform_worker_effect_with_priority_and_receipt(
                self.name,
                self.priority,
                None,
                0,
                Some(guard),
                move || work(BusinessWorkContext::new(None)),
                map,
            ));
        receipt
    }

    /// Run one business worker only when `owner` resolves to one current,
    /// eligible keyed-node or overlay owner in the accepted surface.
    ///
    /// This is intentionally a qualified owner-scoped API. The controller
    /// resolves the public handle after refreshing the accepted surface and
    /// fences the worker, mapper, and any chained command to that owner
    /// generation.
    pub fn run_for_owner_with_receipt<Output>(
        self,
        owner: crate::application::DeclarativeEffectOwner,
        work: impl FnOnce(BusinessWorkContext) -> Output + Send + 'static,
        map: impl FnOnce(Output) -> Message + 'static,
    ) -> BusinessTaskAdmissionReceipt
    where
        Output: Send + 'static,
    {
        let receipt = BusinessTaskAdmissionReceipt::new();
        let guard = AdmissionReceiptGuard(receipt.weak());
        self.context.queue_command(
            Command::perform_worker_effect_with_priority_and_receipt_for_owner(
                owner,
                self.name,
                self.priority,
                Some(guard),
                move |cancellation_probe| {
                    work(BusinessWorkContext::new_with_probe(
                        None,
                        cancellation_probe,
                    ))
                },
                map,
            ),
        );
        receipt
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

    /// Run one ordinary ordered business stream only while `owner` resolves to
    /// one current eligible keyed-node or overlay owner.
    ///
    /// Intermediate events keep the existing bounded FIFO ingress and the
    /// final output remains an uncoalesced terminal delivery. The controller
    /// resolves and fences the owner generation before worker admission;
    /// invalid owner selections reject the receipt without fallback.
    pub fn stream_for_owner_with_receipt<Event, Output>(
        self,
        owner: crate::application::DeclarativeEffectOwner,
        work: impl FnOnce(BusinessWorkContext, BusinessEventSink<Event>) -> Output + Send + 'static,
        map_event: impl Fn(Event) -> Message + 'static,
        map_final: impl FnOnce(Output) -> Message + 'static,
    ) -> BusinessTaskAdmissionReceipt
    where
        Event: Send + 'static,
        Output: Send + 'static,
        Message: 'static,
    {
        self.stream_for_owner_with_optional_cancellation(owner, None, work, map_event, map_final)
    }

    pub(in crate::application::runtime::update_context::business) fn stream_for_owner_with_optional_cancellation<
        Event,
        Output,
    >(
        self,
        owner: crate::application::DeclarativeEffectOwner,
        token: Option<CancellationToken>,
        work: impl FnOnce(BusinessWorkContext, BusinessEventSink<Event>) -> Output + Send + 'static,
        map_event: impl Fn(Event) -> Message + 'static,
        map_final: impl FnOnce(Output) -> Message + 'static,
    ) -> BusinessTaskAdmissionReceipt
    where
        Event: Send + 'static,
        Output: Send + 'static,
        Message: 'static,
    {
        let receipt = BusinessTaskAdmissionReceipt::new();
        let guard = AdmissionReceiptGuard(receipt.weak());
        let worker_token = token.clone();
        let is_cancelled = token.map(|token| {
            Box::new(move || token.is_cancelled()) as Box<dyn Fn() -> bool + Send + Sync + 'static>
        });
        self.context.queue_command(
            Command::perform_worker_stream_with_priority_and_receipt_for_owner_with_options(
                owner,
                self.name,
                self.priority,
                Some(guard),
                crate::runtime::WorkerStreamOptions {
                    is_cancelled,
                    generation: 0,
                    latest: false,
                },
                move |sink, cancellation_probe| {
                    let event_sink =
                        BusinessEventSink::new(move |event| sink.emit(Box::new(event)));
                    work(
                        BusinessWorkContext::new_with_probe(worker_token, cancellation_probe),
                        event_sink,
                    )
                },
                map_event,
                map_final,
            ),
        );
        receipt
    }

    /// Run one coalesced ordinary business stream only while `owner` resolves
    /// to one current eligible keyed-node or overlay owner.
    ///
    /// Intermediate events use the existing latest-wins stream slot while the
    /// UI is behind; the final output remains an uncoalesced terminal delivery.
    /// The controller resolves and fences the owner generation before worker
    /// admission, and invalid owner selections reject the receipt without
    /// fallback.
    pub fn stream_latest_for_owner_with_receipt<Event, Output>(
        self,
        owner: crate::application::DeclarativeEffectOwner,
        work: impl FnOnce(BusinessWorkContext, BusinessEventSink<Event>) -> Output + Send + 'static,
        map_event: impl Fn(Event) -> Message + 'static,
        map_final: impl FnOnce(Output) -> Message + 'static,
    ) -> BusinessTaskAdmissionReceipt
    where
        Event: Send + 'static,
        Output: Send + 'static,
        Message: 'static,
    {
        let receipt = BusinessTaskAdmissionReceipt::new();
        let guard = AdmissionReceiptGuard(receipt.weak());
        self.context.queue_command(
            Command::perform_worker_stream_latest_with_priority_and_receipt_for_owner(
                owner,
                self.name,
                self.priority,
                Some(guard),
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
                map_event,
                map_final,
            ),
        );
        receipt
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

    pub(in crate::application::runtime::update_context::business) fn run_with_latest_transaction<
        Output,
    >(
        self,
        effect_id: u64,
        transaction: LatestTaskTransaction,
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
        let generation = transaction.generation();
        self.context.queue_command(
            Command::perform_worker_effect_with_identity_and_transaction(
                EffectId(effect_id),
                self.name,
                self.priority,
                is_cancelled,
                generation,
                Some(transaction),
                move || work(BusinessWorkContext::new(worker_token)),
                map,
            ),
        );
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
                crate::runtime::WorkerStreamOptions {
                    is_cancelled,
                    generation: 0,
                    latest: false,
                },
                move |sink| {
                    let event_sink =
                        BusinessEventSink::new(move |event| sink.emit(Box::new(event)));
                    work(BusinessWorkContext::new(worker_token), event_sink)
                },
                map_event,
                map_final,
            ));
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::application::runtime::update_context::business) fn stream_with_latest_transaction<
        Event,
        Output,
    >(
        self,
        effect_id: u64,
        transaction: LatestTaskTransaction,
        token: Option<CancellationToken>,
        work: impl FnOnce(BusinessWorkContext, BusinessEventSink<Event>) -> Output + Send + 'static,
        map_event: impl Fn(Event) -> Message + 'static,
        map_final: impl FnOnce(Output) -> Message + 'static,
        latest: bool,
    ) where
        Event: Send + 'static,
        Output: Send + 'static,
        Message: 'static,
    {
        let worker_token = token.clone();
        let is_cancelled = token.map(|token| {
            Box::new(move || token.is_cancelled()) as Box<dyn Fn() -> bool + Send + Sync + 'static>
        });
        let generation = transaction.generation();
        let options = crate::runtime::WorkerStreamOptions {
            is_cancelled,
            generation,
            latest,
        };
        self.context.queue_command(
            Command::perform_worker_stream_with_identity_and_transaction(
                EffectId(effect_id),
                self.name,
                self.priority,
                options,
                Some(transaction),
                move |sink| {
                    let event_sink = if latest {
                        let sink = sink.clone();
                        BusinessEventSink::new(move |event| sink.emit_latest(Box::new(event)))
                    } else {
                        let sink = sink.clone();
                        BusinessEventSink::new(move |event| sink.emit(Box::new(event)))
                    };
                    let close_guard = LatestStreamCloseGuard::new(sink.clone());
                    let output = work(BusinessWorkContext::new(worker_token), event_sink);
                    close_guard.close();
                    output
                },
                map_event,
                map_final,
            ),
        );
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
                crate::runtime::WorkerStreamOptions {
                    is_cancelled,
                    generation: 0,
                    latest: true,
                },
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
