use crate::application::LatestTaskTransaction;
use crate::runtime::command::EffectLifecycle;
use crate::runtime::{
    PlatformCompletion, PlatformCompletionIdentity, PlatformRequest, PlatformResult,
    PlatformResultDelivery, TextClipboardReceipt,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};

use super::SurfaceRuntime;
use super::commands::CommandOutcome;
use super::owner::{AuxiliaryWindowOwner, EffectOrigin, LifecycleDescriptor, RuntimeOwner};
use crate::runtime::RuntimeBridge;

pub(super) struct PlatformCompletionRegistry<Message> {
    owner: RuntimeOwner,
    entries: HashMap<PlatformCompletionIdentity, RegisteredPlatformCompletion<Message>>,
    next_id: u64,
    epoch: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PlatformRegistrationState {
    Pending,
    Accepted,
    Rejected,
}

enum PlatformResultValidation {
    /// Legacy `Command::platform_request` callbacks receive the host result unchanged.
    Legacy,
    /// Qualified `Effect::platform` callbacks require the request's result shape.
    Qualified(PlatformRequest),
}

enum PlatformCompletionTarget<Message> {
    Application(PlatformCompletion<Message>),
    Editor {
        receipt: TextClipboardReceipt,
        timestamp: Option<crate::gui::input::InputTimestamp>,
    },
}

impl<Message> Default for PlatformCompletionRegistry<Message> {
    fn default() -> Self {
        Self::new(RuntimeOwner::new())
    }
}

impl<Message> PlatformCompletionRegistry<Message> {
    pub(super) fn new(owner: RuntimeOwner) -> Self {
        Self {
            owner,
            entries: HashMap::new(),
            next_id: 1,
            epoch: 1,
        }
    }
    #[cfg(test)]
    pub(super) fn register(
        &mut self,
        completion: PlatformCompletion<Message>,
        origin: &EffectOrigin,
    ) -> PlatformCompletionIdentity {
        self.register_inner(
            PlatformCompletionTarget::Application(completion),
            PlatformResultValidation::Legacy,
            origin,
            None,
            None,
        )
    }

    pub(super) fn register_legacy_for_request(
        &mut self,
        completion: PlatformCompletion<Message>,
        _request: &PlatformRequest,
        origin: &EffectOrigin,
    ) -> PlatformCompletionIdentity {
        self.register_inner(
            PlatformCompletionTarget::Application(completion),
            PlatformResultValidation::Legacy,
            origin,
            None,
            None,
        )
    }

    pub(super) fn register_text_clipboard(
        &mut self,
        receipt: TextClipboardReceipt,
        origin: &EffectOrigin,
        timestamp: Option<crate::gui::input::InputTimestamp>,
    ) -> PlatformCompletionIdentity {
        let request = receipt.request();
        self.register_inner(
            PlatformCompletionTarget::Editor { receipt, timestamp },
            PlatformResultValidation::Qualified(request),
            origin,
            None,
            Some(PlatformRegistrationState::Accepted),
        )
    }

    pub(super) fn register_effect(
        &mut self,
        completion: PlatformCompletion<Message>,
        request: &PlatformRequest,
        origin: &EffectOrigin,
        lifecycle: &EffectLifecycle,
        transaction: LatestTaskTransaction,
    ) -> PlatformCompletionIdentity {
        let transaction_probe = transaction.cancellation_probe();
        let cancellation = combine_cancellation_probes(
            Some(Arc::clone(&lifecycle.cancellation)),
            Some(transaction_probe),
        );
        let cancellation = combine_cancellation_probes(cancellation, origin.cancellation_probe());
        let rejection_cancellation = combine_cancellation_probes(
            Some(Arc::clone(&lifecycle.cancellation)),
            origin.cancellation_probe(),
        );
        let rejection_cancellation = combine_cancellation_probes(
            rejection_cancellation,
            Some(transaction.newer_replacement_probe()),
        );
        self.register_inner(
            PlatformCompletionTarget::Application(completion),
            PlatformResultValidation::Qualified(request.clone()),
            origin,
            Some(RegisteredPlatformEffect {
                lifecycle: LifecycleDescriptor::new_for_effect(
                    self.owner.clone(),
                    self.next_id,
                    lifecycle,
                    cancellation,
                ),
                rejection_lifecycle: LifecycleDescriptor::new(
                    self.owner.clone(),
                    self.next_id,
                    None,
                    self.epoch,
                    rejection_cancellation,
                ),
                transaction,
                identity: lifecycle.identity,
                generation: lifecycle.generation.0,
            }),
            Some(PlatformRegistrationState::Pending),
        )
    }

    fn register_inner(
        &mut self,
        target: PlatformCompletionTarget<Message>,
        validation: PlatformResultValidation,
        origin: &EffectOrigin,
        effect: Option<RegisteredPlatformEffect>,
        state: Option<PlatformRegistrationState>,
    ) -> PlatformCompletionIdentity {
        let identity = PlatformCompletionIdentity {
            id: self.next_id,
            epoch: self.epoch,
        };
        self.next_id = self.next_id.saturating_add(1);
        let (lifecycle, rejection_lifecycle, transaction, effect_identity, generation) = effect
            .map_or_else(
                || {
                    let lifecycle = LifecycleDescriptor::new(
                        self.owner.clone(),
                        identity.id,
                        None,
                        identity.epoch,
                        origin.cancellation_probe(),
                    );
                    (lifecycle.clone(), lifecycle, None, None, identity.epoch)
                },
                |effect| {
                    (
                        effect.lifecycle,
                        effect.rejection_lifecycle,
                        Some(effect.transaction),
                        Some(effect.identity),
                        effect.generation,
                    )
                },
            );
        self.entries.insert(
            identity,
            RegisteredPlatformCompletion {
                target,
                validation,
                origin: origin.clone(),
                lifecycle,
                rejection_lifecycle,
                transaction,
                effect_identity,
                generation,
                state: state.unwrap_or(PlatformRegistrationState::Accepted),
            },
        );
        identity
    }

    pub(super) fn accept_effect(&mut self, identity: PlatformCompletionIdentity) {
        let Some(effect_identity) = self
            .entries
            .get(&identity)
            .and_then(|entry| entry.effect_identity)
        else {
            return;
        };
        let admissible = self.entries.get(&identity).is_some_and(|entry| {
            entry.state == PlatformRegistrationState::Pending
                && entry.origin.is_live()
                && entry.lifecycle.admits_effect(
                    &self.owner,
                    identity.id,
                    entry.lifecycle.identity(),
                    entry.generation,
                    true,
                )
        });
        if !admissible {
            let _ = self.entries.remove(&identity);
            return;
        }
        let superseded = self
            .entries
            .iter()
            .filter(|(candidate, entry)| {
                **candidate != identity
                    && entry.effect_identity == Some(effect_identity)
                    && entry.state != PlatformRegistrationState::Rejected
            })
            .map(|(candidate, _)| *candidate)
            .collect::<Vec<_>>();
        for candidate in superseded {
            self.entries.remove(&candidate);
        }
        if let Some(entry) = self.entries.get_mut(&identity)
            && entry.state == PlatformRegistrationState::Pending
        {
            if let Some(transaction) = entry.transaction.as_ref() {
                transaction.accept();
            }
            entry.state = PlatformRegistrationState::Accepted;
        }
    }

    pub(super) fn effect_is_current(&self, identity: PlatformCompletionIdentity) -> bool {
        self.entries.get(&identity).is_some_and(|entry| {
            entry.state == PlatformRegistrationState::Pending
                && entry.origin.is_live()
                && entry.lifecycle.admits_effect(
                    &self.owner,
                    identity.id,
                    entry.lifecycle.identity(),
                    entry.generation,
                    true,
                )
        })
    }

    pub(super) fn reject_effect(&mut self, identity: PlatformCompletionIdentity) {
        let Some(entry) = self.entries.get_mut(&identity) else {
            return;
        };
        if entry.state != PlatformRegistrationState::Pending {
            return;
        }
        if let Some(transaction) = entry.transaction.take() {
            transaction.reject();
        }
        entry.lifecycle = entry.rejection_lifecycle.clone();
        entry.generation = identity.epoch;
        entry.state = PlatformRegistrationState::Rejected;
        entry.effect_identity = None;
    }

    #[cfg(test)]
    pub(super) fn map_delivery(
        &mut self,
        delivery: PlatformResultDelivery,
    ) -> Option<MappedPlatformMessage<Message>> {
        match self.map_delivery_target(delivery)? {
            MappedPlatformCompletion::Application(mapped) => Some(mapped),
            MappedPlatformCompletion::Editor(_) => None,
        }
    }

    pub(super) fn map_delivery_target(
        &mut self,
        delivery: PlatformResultDelivery,
    ) -> Option<MappedPlatformCompletion<Message>> {
        match delivery {
            PlatformResultDelivery::Completed { identity, result } => {
                let mapper = self.entries.get(&identity)?;
                if let PlatformResultValidation::Qualified(request) = &mapper.validation
                    && request.validate_result(&result).is_err()
                {
                    self.entries.remove(&identity);
                    return None;
                }
                let current = mapper.origin.is_live()
                    && match mapper.state {
                        PlatformRegistrationState::Pending => false,
                        PlatformRegistrationState::Accepted => mapper.lifecycle.admits_effect(
                            &self.owner,
                            identity.id,
                            mapper.lifecycle.identity(),
                            mapper.generation,
                            true,
                        ),
                        PlatformRegistrationState::Rejected => mapper.lifecycle.admits(
                            &self.owner,
                            identity.id,
                            mapper.generation,
                            true,
                        ),
                    };
                if !current {
                    self.entries.remove(&identity);
                    return None;
                }
                let mapper = self.entries.remove(&identity)?;
                let fence = Some(PlatformMappingFence {
                    accepted: mapper.state == PlatformRegistrationState::Accepted,
                    identity,
                    generation: mapper.generation,
                    lifecycle: mapper.lifecycle.clone(),
                    origin: mapper.origin.clone(),
                });
                match mapper.target {
                    PlatformCompletionTarget::Application(completion) => Some(
                        MappedPlatformCompletion::Application(MappedPlatformMessage {
                            message: completion(result),
                            origin: mapper.origin,
                            fence,
                        }),
                    ),
                    PlatformCompletionTarget::Editor { receipt, timestamp } => {
                        Some(MappedPlatformCompletion::Editor(MappedTextClipboard {
                            receipt,
                            timestamp,
                            result,
                            origin: mapper.origin,
                            fence,
                        }))
                    }
                }
            }
            PlatformResultDelivery::Discarded { identity } => {
                self.entries.remove(&identity);
                None
            }
        }
    }

    pub(super) fn remove(
        &mut self,
        identity: PlatformCompletionIdentity,
    ) -> Option<PlatformCompletion<Message>> {
        self.entries
            .remove(&identity)
            .and_then(|entry| match entry.target {
                PlatformCompletionTarget::Application(completion) => Some(completion),
                PlatformCompletionTarget::Editor { .. } => None,
            })
    }

    pub(super) fn retire_origin(&mut self, origin: &EffectOrigin) {
        let current_ids = self
            .entries
            .iter()
            .filter(|(_, registered)| registered.origin.eq(origin))
            .map(|(identity, _)| *identity)
            .collect::<Vec<_>>();
        for identity in current_ids {
            self.entries.remove(&identity);
        }
    }

    pub(super) fn retire_auxiliary_owner(&mut self, owner: &AuxiliaryWindowOwner) {
        owner.retire();
        let origin = EffectOrigin::Auxiliary(owner.clone());
        self.retire_origin(&origin);
    }

    pub(super) fn clear(&mut self) {
        self.entries.clear();
        self.epoch = self.epoch.saturating_add(1);
    }
}

struct RegisteredPlatformCompletion<Message> {
    target: PlatformCompletionTarget<Message>,
    validation: PlatformResultValidation,
    lifecycle: LifecycleDescriptor,
    rejection_lifecycle: LifecycleDescriptor,
    transaction: Option<LatestTaskTransaction>,
    effect_identity: Option<crate::runtime::command::EffectId>,
    generation: u64,
    state: PlatformRegistrationState,
    origin: EffectOrigin,
}

pub(super) enum MappedPlatformCompletion<Message> {
    Application(MappedPlatformMessage<Message>),
    Editor(MappedTextClipboard),
}

pub(super) struct MappedTextClipboard {
    receipt: TextClipboardReceipt,
    timestamp: Option<crate::gui::input::InputTimestamp>,
    result: PlatformResult,
    origin: EffectOrigin,
    fence: Option<PlatformMappingFence>,
}

impl MappedTextClipboard {
    fn is_current(&self, owner: &RuntimeOwner) -> bool {
        self.origin.is_live()
            && self
                .fence
                .as_ref()
                .is_none_or(|fence| fence.is_current(owner))
    }
}

struct RegisteredPlatformEffect {
    lifecycle: LifecycleDescriptor,
    rejection_lifecycle: LifecycleDescriptor,
    transaction: LatestTaskTransaction,
    identity: crate::runtime::command::EffectId,
    generation: u64,
}

pub(super) struct MappedPlatformMessage<Message> {
    pub(super) message: Message,
    pub(super) origin: EffectOrigin,
    fence: Option<PlatformMappingFence>,
}

#[derive(Clone)]
struct PlatformMappingFence {
    accepted: bool,
    identity: PlatformCompletionIdentity,
    generation: u64,
    lifecycle: LifecycleDescriptor,
    origin: EffectOrigin,
}

impl PlatformMappingFence {
    fn is_current(&self, owner: &RuntimeOwner) -> bool {
        self.origin.is_live()
            && if self.accepted {
                self.lifecycle.admits_effect(
                    owner,
                    self.identity.id,
                    self.lifecycle.identity(),
                    self.generation,
                    true,
                )
            } else {
                self.lifecycle
                    .admits(owner, self.identity.id, self.generation, true)
            }
    }
}

impl<Message> MappedPlatformMessage<Message> {
    pub(super) fn is_current(&self, owner: &RuntimeOwner) -> bool {
        self.fence
            .as_ref()
            .is_none_or(|fence| fence.is_current(owner))
    }
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Begin one deferred native clipboard operation for the exact focused text widget.
    /// Host completion is always committed to the platform ingress for a later drain turn.
    pub fn begin_focused_text_clipboard(
        &mut self,
        operation: crate::runtime::TextClipboardOperation,
        timestamp: Option<crate::gui::input::InputTimestamp>,
    ) -> bool {
        if !self.lifecycle_accepts_work() {
            return false;
        }
        let Some(widget_id) = self.interaction.focus.focused_widget() else {
            return false;
        };
        if !self.is_authoritative_focus_target(widget_id) {
            return false;
        }
        let Some(receipt) = self
            .surface_widget(widget_id)
            .and_then(|widget| widget.widget_object().text_clipboard_receipt(operation))
        else {
            return false;
        };
        if receipt.widget != widget_id {
            return false;
        }
        let request = receipt.request();
        if request.validate().is_err() {
            return false;
        }
        let Some(capability) = self.host_capabilities.platform_result.as_ref() else {
            return false;
        };
        // Native focused input belongs to this surface runtime. Its owner closes on
        // runtime shutdown, so an already queued clipboard lane call observes that
        // terminal fence before touching the OS.
        let origin = EffectOrigin::Application;
        let receipt_cancellation = receipt.cancellation_probe();
        let runtime_owner = self.effect_owner.clone();
        let cancellation = Arc::new(move || receipt_cancellation() || !runtime_owner.is_open());
        let identity = self
            .platform_registry
            .register_text_clipboard(receipt, &origin, timestamp);
        let Some(reservation) = PlatformResultIngress::reserve(&self.platform_results) else {
            let _ = self.platform_registry.remove(identity);
            return false;
        };
        let sink = crate::runtime::RuntimePlatformResultSink::new(identity, move |delivery| {
            let _ = reservation.commit(delivery);
        })
        .with_cancellation(cancellation);
        if let Err(fallback) = (capability.request_platform_result)(&mut self.bridge, request, sink)
        {
            let (request, sink) = *fallback;
            sink.send(Err(crate::runtime::PlatformFailure::Unavailable(
                request.service(),
            )));
        }
        true
    }

    pub(super) fn dispatch_mapped_platform_completion(
        &mut self,
        mapped: MappedPlatformCompletion<Message>,
        outcome: &mut CommandOutcome,
    ) {
        match mapped {
            MappedPlatformCompletion::Application(mapped) => {
                if !mapped.is_current(&self.effect_owner)
                    || !self.lifecycle_accepts_work()
                    || !self.effect_origin_is_active(&mapped.origin)
                {
                    return;
                }
                self.dispatch_message_inner_with_origin(mapped.message, outcome, mapped.origin);
            }
            MappedPlatformCompletion::Editor(mapped) => {
                if !mapped.is_current(&self.effect_owner)
                    || !self.lifecycle_accepts_work()
                    || !self.effect_origin_is_active(&mapped.origin)
                    || self.interaction.focus.focused_widget() != Some(mapped.receipt.widget)
                    || !self.is_authoritative_focus_target(mapped.receipt.widget)
                    || !self
                        .surface_widget(mapped.receipt.widget)
                        .is_some_and(|widget| {
                            widget
                                .widget_object()
                                .accepts_text_clipboard_receipt(&mapped.receipt)
                        })
                {
                    return;
                }
                use crate::widgets::{TextEditCommand, WidgetInput};
                let command = match (mapped.receipt.operation, mapped.result) {
                    (crate::runtime::TextClipboardOperation::Copy, Ok(_)) => return,
                    (
                        crate::runtime::TextClipboardOperation::Cut,
                        Ok(crate::runtime::PlatformResponse::Completed),
                    ) => TextEditCommand::CutSelection,
                    (
                        crate::runtime::TextClipboardOperation::Paste,
                        Ok(crate::runtime::PlatformResponse::Text(text)),
                    ) => TextEditCommand::PasteText(text),
                    _ => return,
                };
                let _ = self.dispatch_input(
                    mapped.receipt.widget,
                    WidgetInput::text_edit_with_timestamp(command, mapped.timestamp),
                );
            }
        }
    }

    pub(super) fn shutdown_platform_services(&mut self) {
        {
            let mut ingress = self
                .platform_results
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            ingress.close();
        }
        self.platform_registry.clear();
    }
}

#[derive(Default)]
pub(super) struct PlatformResultIngress {
    pending: Vec<PlatformResultDelivery>,
    overflow: Option<PlatformResultDelivery>,
    reservations: usize,
    closed: bool,
}

impl PlatformResultIngress {
    const CAPACITY: usize = 64;

    pub(super) fn reserve(ingress: &Arc<Mutex<Self>>) -> Option<PlatformResultReservation> {
        let mut state = ingress
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.closed || state.pending.len().saturating_add(state.reservations) >= Self::CAPACITY
        {
            return None;
        }
        state.reservations += 1;
        Some(PlatformResultReservation {
            ingress: Arc::downgrade(ingress),
            committed: false,
        })
    }

    #[cfg(test)]
    pub(super) fn take_pending(&mut self) -> Vec<PlatformResultDelivery> {
        self.take_frozen_pending_batch(self.pending_len(), usize::MAX)
            .0
    }

    pub(super) fn take_frozen_pending_batch(
        &mut self,
        frozen_count: usize,
        max_deliveries: usize,
    ) -> (Vec<PlatformResultDelivery>, bool) {
        let take_count = frozen_count.min(max_deliveries);
        if take_count == 0 {
            return (Vec::new(), frozen_count != 0);
        }
        let take = self.pending.len().min(take_count);
        let mut pending = self.pending.drain(..take).collect::<Vec<_>>();
        if pending.len() < take_count
            && let Some(delivery) = self.overflow.take()
        {
            pending.push(delivery);
        }
        // Keep an older overflow delivery ahead of arrivals committed while
        // the frozen prefix is being mapped. The remaining pending entries
        // are older than that overflow, so appending preserves global FIFO.
        if let Some(delivery) = self.overflow.take() {
            self.pending.push(delivery);
        }
        (pending, frozen_count > max_deliveries)
    }

    /// Snapshot the eligible prefix and remove its budgeted portion while the
    /// caller still holds the ingress lock. Later reservations therefore
    /// cannot enter the frozen turn ahead of an older overflow delivery.
    pub(super) fn take_budgeted_pending_batch(
        &mut self,
        max_deliveries: usize,
    ) -> (Vec<PlatformResultDelivery>, bool) {
        let frozen_count = self.pending_len();
        self.take_frozen_pending_batch(frozen_count, max_deliveries)
    }

    pub(super) fn pending_len(&self) -> usize {
        self.pending.len() + usize::from(self.overflow.is_some())
    }

    pub(super) fn close(&mut self) {
        self.closed = true;
        self.pending.clear();
        self.overflow = None;
        self.reservations = 0;
    }

    pub(super) fn enqueue_overflow(&mut self, delivery: PlatformResultDelivery) -> bool {
        if self.closed || self.overflow.is_some() {
            false
        } else {
            self.overflow = Some(delivery);
            true
        }
    }
}

pub(super) struct PlatformResultReservation {
    ingress: Weak<Mutex<PlatformResultIngress>>,
    committed: bool,
}

impl PlatformResultReservation {
    pub(super) fn commit(mut self, delivery: PlatformResultDelivery) -> bool {
        let Some(ingress) = self.ingress.upgrade() else {
            self.committed = true;
            return false;
        };
        let mut state = ingress
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.closed || state.reservations == 0 {
            self.committed = true;
            return false;
        }
        state.reservations -= 1;
        state.pending.push(delivery);
        self.committed = true;
        true
    }
}

impl Drop for PlatformResultReservation {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        if let Some(ingress) = self.ingress.upgrade() {
            let mut state = ingress
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.reservations = state.reservations.saturating_sub(1);
        }
    }
}

fn combine_cancellation_probes(
    first: Option<super::owner::CancellationProbe>,
    second: Option<super::owner::CancellationProbe>,
) -> Option<super::owner::CancellationProbe> {
    match (first, second) {
        (None, None) => None,
        (Some(probe), None) | (None, Some(probe)) => Some(probe),
        (Some(first), Some(second)) => Some(Arc::new(move || first() || second())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{
        PlatformResponse, RuntimeBridge, RuntimeHostCapabilities, RuntimePlatformResultHost,
        RuntimePlatformResultSink, SurfaceNode, UiSurface,
    };
    use crate::{
        application::{IntoView, column, text},
        gui::types::Vector2,
        runtime::SurfaceRuntime,
    };
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
        sync::{Arc, Mutex},
    };

    #[derive(Default)]
    struct TextClipboardBridge {
        sinks: Vec<RuntimePlatformResultSink>,
    }

    impl RuntimeBridge<()> for TextClipboardBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            let editor = crate::widgets::TextEditorWidget::uncontrolled(
                41,
                "old",
                crate::widgets::WidgetSizing::fixed(Vector2::new(120.0, 48.0)),
            )
            .expect("bounded editor");
            crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::static_widget(editor)))
        }

        fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, ()> {
            RuntimeHostCapabilities::new().with_platform_results()
        }
    }

    impl RuntimePlatformResultHost for TextClipboardBridge {
        fn request_platform_result(
            &mut self,
            _request: PlatformRequest,
            sink: RuntimePlatformResultSink,
        ) -> Result<(), crate::runtime::PlatformResultServiceFallback> {
            self.sinks.push(sink);
            Ok(())
        }
    }

    fn editor_text(runtime: &SurfaceRuntime<TextClipboardBridge, ()>) -> String {
        runtime
            .surface_widget(41)
            .and_then(|widget| {
                widget
                    .widget_object()
                    .as_any()
                    .downcast_ref::<crate::widgets::TextEditorWidget>()
            })
            .expect("editor")
            .text()
            .to_owned()
    }

    #[test]
    fn focused_clipboard_paste_is_deferred_and_applies_only_after_success() {
        let mut runtime =
            SurfaceRuntime::new(TextClipboardBridge::default(), Vector2::new(160.0, 80.0));
        assert!(runtime.focus_widget(41));
        assert!(
            runtime
                .begin_focused_text_clipboard(crate::runtime::TextClipboardOperation::Paste, None,)
        );
        let sink = runtime.bridge_mut().sinks.pop().expect("paste sink");
        sink.send(Ok(PlatformResponse::Text("new".into())));
        assert_eq!(editor_text(&runtime), "old");
        let _ = runtime.drain_runtime_messages();
        assert_eq!(editor_text(&runtime), "newold");
    }

    #[test]
    fn failed_clipboard_cut_never_deletes_the_selection() {
        let mut runtime =
            SurfaceRuntime::new(TextClipboardBridge::default(), Vector2::new(160.0, 80.0));
        assert!(runtime.focus_widget(41));
        assert!(
            runtime
                .dispatch_focused_input(crate::widgets::WidgetInput::text_edit(
                    crate::widgets::TextEditCommand::SelectAll,
                ))
                .is_some()
        );
        assert!(
            runtime
                .begin_focused_text_clipboard(crate::runtime::TextClipboardOperation::Cut, None,)
        );
        runtime
            .bridge_mut()
            .sinks
            .pop()
            .expect("cut sink")
            .send(Err(crate::runtime::PlatformFailure::Unavailable(
                crate::runtime::PlatformService::Clipboard,
            )));
        let _ = runtime.drain_runtime_messages();
        assert_eq!(editor_text(&runtime), "old");
    }

    #[test]
    fn stale_focused_clipboard_paste_is_rejected_after_focus_or_revision_changes() {
        let mut runtime =
            SurfaceRuntime::new(TextClipboardBridge::default(), Vector2::new(160.0, 80.0));
        assert!(runtime.focus_widget(41));
        assert!(
            runtime
                .begin_focused_text_clipboard(crate::runtime::TextClipboardOperation::Paste, None)
        );
        let sink = runtime.bridge_mut().sinks.pop().expect("focus fenced sink");
        runtime.clear_focus();
        sink.send(Ok(PlatformResponse::Text("late".into())));
        let _ = runtime.drain_runtime_messages();
        assert_eq!(editor_text(&runtime), "old");

        assert!(runtime.focus_widget(41));
        assert!(
            runtime
                .begin_focused_text_clipboard(crate::runtime::TextClipboardOperation::Paste, None)
        );
        let sink = runtime
            .bridge_mut()
            .sinks
            .pop()
            .expect("revision fenced sink");
        assert!(
            runtime
                .dispatch_focused_input(crate::widgets::WidgetInput::text_edit(
                    crate::widgets::TextEditCommand::InsertText("local".into()),
                ))
                .is_some()
        );
        sink.send(Ok(PlatformResponse::Text("late".into())));
        let _ = runtime.drain_runtime_messages();
        assert_eq!(editor_text(&runtime), "localold");
    }

    #[test]
    fn closed_runtime_discards_a_late_clipboard_completion() {
        let mut runtime =
            SurfaceRuntime::new(TextClipboardBridge::default(), Vector2::new(160.0, 80.0));
        assert!(runtime.focus_widget(41));
        assert!(
            runtime
                .begin_focused_text_clipboard(crate::runtime::TextClipboardOperation::Paste, None)
        );
        let sink = runtime.bridge_mut().sinks.pop().expect("close fenced sink");
        assert!(
            runtime
                .execute_command(crate::runtime::Command::Exit)
                .exit_requested
        );
        sink.send(Ok(PlatformResponse::Text("late".into())));
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
    }

    fn declarative_origins() -> (EffectOrigin, EffectOrigin, EffectOrigin) {
        let phase = Rc::new(Cell::new(0_u8));
        let project_phase = Rc::clone(&phase);
        let mut runtime = SurfaceRuntime::new_declarative_owned(
            (),
            Vector2::new(80.0, 40.0),
            move |_| {
                if project_phase.get() == 1 {
                    text::<usize>("raw").into_surface()
                } else {
                    column([text::<usize>("old").key("old")]).into_surface()
                }
            },
            |_, _| {},
        );
        let old = runtime
            .declarative_owner_ledger()
            .live_records()
            .first()
            .expect("old declarative owner")
            .token
            .clone();
        let sibling_runtime = SurfaceRuntime::new_declarative_owned(
            (),
            Vector2::new(80.0, 40.0),
            |_| column([text::<usize>("sibling").key("sibling")]).into_surface(),
            |_, _| {},
        );
        let sibling = sibling_runtime
            .declarative_owner_ledger()
            .live_records()
            .first()
            .expect("sibling declarative owner")
            .token
            .clone();
        phase.set(1);
        runtime.refresh();
        phase.set(2);
        runtime.refresh();
        let new = runtime
            .declarative_owner_ledger()
            .live_records()
            .first()
            .expect("later declarative owner generation")
            .token
            .clone();
        (
            EffectOrigin::Declarative(old),
            EffectOrigin::Declarative(sibling),
            EffectOrigin::Declarative(new),
        )
    }

    #[test]
    fn mapper_runs_once_and_duplicate_delivery_is_ignored() {
        let calls = Rc::new(RefCell::new(0));
        let marker = Rc::clone(&calls);
        let mut registry = PlatformCompletionRegistry::<usize>::default();
        let identity = registry.register(
            Box::new(move |_| {
                *marker.borrow_mut() += 1;
                1
            }),
            &EffectOrigin::Application,
        );
        let delivery = || PlatformResultDelivery::Completed {
            identity,
            result: Ok(PlatformResponse::Completed),
        };
        assert_eq!(
            registry
                .map_delivery(delivery())
                .map(|mapped| mapped.message),
            Some(1)
        );
        assert!(registry.map_delivery(delivery()).is_none());
        assert_eq!(*calls.borrow(), 1);
    }

    #[test]
    fn malformed_success_is_dropped_before_platform_mapper() {
        let calls = Rc::new(RefCell::new(0));
        let calls_for_mapper = Rc::clone(&calls);
        let request = PlatformRequest::ReadText;
        let mut registry = PlatformCompletionRegistry::<usize>::default();
        let mut latest = crate::application::LatestTask::new();
        let transaction = latest.begin_replacement();
        let lifecycle = EffectLifecycle::from_token(
            crate::runtime::command::EffectId(1),
            crate::runtime::command::EffectGeneration(transaction.generation()),
            Some(&transaction),
            crate::runtime::EffectOwner::Application,
            crate::application::CancellationToken::new(),
        );
        let identity = registry.register_effect(
            Box::new(move |_| {
                *calls_for_mapper.borrow_mut() += 1;
                1
            }),
            &request,
            &EffectOrigin::Application,
            &lifecycle,
            transaction,
        );
        registry.accept_effect(identity);

        assert!(
            registry
                .map_delivery(PlatformResultDelivery::Completed {
                    identity,
                    result: Ok(PlatformResponse::Completed),
                })
                .is_none()
        );
        assert_eq!(*calls.borrow(), 0);
    }

    #[test]
    fn legacy_platform_request_forwards_host_result_to_mapper() {
        let calls = Rc::new(RefCell::new(0));
        let calls_for_mapper = Rc::clone(&calls);
        let request = PlatformRequest::ReadText;
        let mut registry = PlatformCompletionRegistry::<usize>::default();
        let identity = registry.register_legacy_for_request(
            Box::new(move |_| {
                *calls_for_mapper.borrow_mut() += 1;
                1
            }),
            &request,
            &EffectOrigin::Application,
        );

        assert_eq!(
            registry
                .map_delivery(PlatformResultDelivery::Completed {
                    identity,
                    result: Ok(PlatformResponse::Completed),
                })
                .map(|mapped| mapped.message),
            Some(1)
        );
        assert_eq!(*calls.borrow(), 1);
    }

    #[test]
    fn declarative_origin_maps_live_and_vetoes_late_platform_result() {
        let mut owner_runtime = SurfaceRuntime::new_declarative_owned(
            (),
            Vector2::new(80.0, 40.0),
            |_| column([text::<usize>("keyed").key("keyed")]).into_surface(),
            |_, _| {},
        );
        let token = owner_runtime
            .declarative_owner_ledger()
            .live_records()
            .first()
            .expect("keyed declarative owner")
            .token
            .clone();
        let origin = EffectOrigin::Declarative(token.clone());
        let mut registry = PlatformCompletionRegistry::<usize>::default();
        let identity = registry.register(Box::new(|_| 7), &origin);
        let delivery = PlatformResultDelivery::Completed {
            identity,
            result: Ok(PlatformResponse::Completed),
        };
        let mapped = registry
            .map_delivery(delivery)
            .expect("live declarative platform result");
        assert_eq!(mapped.message, 7);
        assert!(mapped.origin == origin);

        let calls = Rc::new(RefCell::new(0));
        let calls_for_mapper = Rc::clone(&calls);
        let late_identity = registry.register(
            Box::new(move |_| {
                *calls_for_mapper.borrow_mut() += 1;
                8
            }),
            &origin,
        );
        assert!(owner_runtime.begin_closing());
        assert!(!token.is_live());
        assert!(
            registry
                .map_delivery(PlatformResultDelivery::Completed {
                    identity: late_identity,
                    result: Ok(PlatformResponse::Completed),
                })
                .is_none()
        );
        assert_eq!(*calls.borrow(), 0);
    }

    #[test]
    fn clear_fences_stale_delivery_and_releases_mapper() {
        let marker = Rc::new(());
        let captured = Rc::clone(&marker);
        let mut registry = PlatformCompletionRegistry::<()>::default();
        let identity = registry.register(
            Box::new(move |_| {
                let _ = &captured;
            }),
            &EffectOrigin::Application,
        );
        assert_eq!(Rc::strong_count(&marker), 2);
        registry.clear();
        assert_eq!(Rc::strong_count(&marker), 1);
        assert!(
            registry
                .map_delivery(PlatformResultDelivery::Completed {
                    identity,
                    result: Ok(PlatformResponse::Completed),
                })
                .is_none()
        );
    }

    #[test]
    fn saturated_overflow_releases_rejected_mapper() {
        let marker = Rc::new(());
        let mut registry = PlatformCompletionRegistry::<()>::default();
        let first_marker = Rc::clone(&marker);
        let first = registry.register(
            Box::new(move |_| {
                let _ = &first_marker;
            }),
            &EffectOrigin::Application,
        );
        let second_marker = Rc::clone(&marker);
        let second = registry.register(
            Box::new(move |_| {
                let _ = &second_marker;
            }),
            &EffectOrigin::Application,
        );
        assert_eq!(Rc::strong_count(&marker), 3);

        let ingress = Arc::new(Mutex::new(PlatformResultIngress::default()));
        let mut ingress_state = ingress.lock().expect("ingress lock");
        assert!(
            ingress_state.enqueue_overflow(PlatformResultDelivery::Completed {
                identity: first,
                result: Ok(PlatformResponse::Completed),
            })
        );
        assert!(
            !ingress_state.enqueue_overflow(PlatformResultDelivery::Completed {
                identity: second,
                result: Ok(PlatformResponse::Completed),
            })
        );
        drop(ingress_state);
        let _ = registry.remove(second);
        assert_eq!(Rc::strong_count(&marker), 2);
        let delivery = ingress
            .lock()
            .expect("ingress lock")
            .take_pending()
            .pop()
            .expect("bounded overflow delivery");
        assert!(registry.map_delivery(delivery).is_some());
        assert_eq!(Rc::strong_count(&marker), 1);
    }

    #[test]
    fn auxiliary_retirement_releases_only_exact_generation_registrations() {
        let marker = Rc::new(());
        let old_owner = AuxiliaryWindowOwner::new("settings");
        let sibling_owner = AuxiliaryWindowOwner::new("inspector");
        let new_owner = AuxiliaryWindowOwner::new("settings");
        let mut registry = PlatformCompletionRegistry::<usize>::default();

        let application = {
            let marker = Rc::clone(&marker);
            registry.register(
                Box::new(move |_| {
                    let _ = &marker;
                    1
                }),
                &EffectOrigin::Application,
            )
        };
        let old = {
            let marker = Rc::clone(&marker);
            registry.register(
                Box::new(move |_| {
                    let _ = &marker;
                    2
                }),
                &EffectOrigin::Auxiliary(old_owner.clone()),
            )
        };
        let sibling = {
            let marker = Rc::clone(&marker);
            registry.register(
                Box::new(move |_| {
                    let _ = &marker;
                    3
                }),
                &EffectOrigin::Auxiliary(sibling_owner.clone()),
            )
        };
        let new_generation = {
            let marker = Rc::clone(&marker);
            registry.register(
                Box::new(move |_| {
                    let _ = &marker;
                    4
                }),
                &EffectOrigin::Auxiliary(new_owner.clone()),
            )
        };
        assert_eq!(Rc::strong_count(&marker), 5);

        registry.retire_auxiliary_owner(&old_owner);
        assert!(!old_owner.is_open());
        assert!(new_owner.is_open());
        assert_eq!(Rc::strong_count(&marker), 4);

        let delivery = |identity| PlatformResultDelivery::Completed {
            identity,
            result: Ok(PlatformResponse::Completed),
        };
        assert!(registry.map_delivery(delivery(old)).is_none());
        assert_eq!(
            registry
                .map_delivery(delivery(application))
                .map(|mapped| mapped.message),
            Some(1)
        );
        assert_eq!(
            registry
                .map_delivery(delivery(sibling))
                .map(|mapped| mapped.message),
            Some(3)
        );
        assert_eq!(
            registry
                .map_delivery(delivery(new_generation))
                .map(|mapped| mapped.message),
            Some(4)
        );
        assert_eq!(Rc::strong_count(&marker), 1);
    }

    #[test]
    fn declarative_retirement_drops_only_matching_completion_mappers() {
        let (old_origin, sibling_origin, new_origin) = declarative_origins();
        let marker = Rc::new(());
        let mut registry = PlatformCompletionRegistry::<usize>::default();
        let application = {
            let marker = Rc::clone(&marker);
            registry.register(
                Box::new(move |_| {
                    let _ = &marker;
                    1
                }),
                &EffectOrigin::Application,
            )
        };
        let old = {
            let marker = Rc::clone(&marker);
            registry.register(
                Box::new(move |_| {
                    let _ = &marker;
                    2
                }),
                &old_origin,
            )
        };
        let sibling = {
            let marker = Rc::clone(&marker);
            registry.register(
                Box::new(move |_| {
                    let _ = &marker;
                    3
                }),
                &sibling_origin,
            )
        };
        let new_generation = {
            let marker = Rc::clone(&marker);
            registry.register(
                Box::new(move |_| {
                    let _ = &marker;
                    4
                }),
                &new_origin,
            )
        };
        assert_eq!(Rc::strong_count(&marker), 5);

        registry.retire_origin(&old_origin);

        assert_eq!(Rc::strong_count(&marker), 4);
        assert!(registry.map_delivery(delivery_for(old)).is_none());
        assert_eq!(
            registry
                .map_delivery(delivery_for(application))
                .map(|mapped| mapped.message),
            Some(1)
        );
        assert_eq!(
            registry
                .map_delivery(delivery_for(sibling))
                .map(|mapped| mapped.message),
            Some(3)
        );
        assert_eq!(
            registry
                .map_delivery(delivery_for(new_generation))
                .map(|mapped| mapped.message),
            Some(4)
        );
        assert_eq!(Rc::strong_count(&marker), 1);

        registry.retire_origin(&old_origin);
        assert!(registry.map_delivery(delivery_for(old)).is_none());
    }

    #[test]
    fn retired_origin_late_duplicate_discarded_and_overflow_deliveries_are_inert() {
        let marker = Rc::new(RefCell::new(0usize));
        let owner = AuxiliaryWindowOwner::new("settings");
        let mut registry = PlatformCompletionRegistry::<usize>::default();
        let captured = Rc::clone(&marker);
        let identity = registry.register(
            Box::new(move |_| {
                *captured.borrow_mut() += 1;
                1
            }),
            &EffectOrigin::Auxiliary(owner.clone()),
        );
        owner.retire();

        let delivery = || PlatformResultDelivery::Completed {
            identity,
            result: Ok(PlatformResponse::Completed),
        };
        assert!(registry.map_delivery(delivery()).is_none());
        assert!(registry.map_delivery(delivery()).is_none());
        assert_eq!(*marker.borrow(), 0);
        assert_eq!(Rc::strong_count(&marker), 1);

        let discarded = registry.register(Box::new(|_| 2), &EffectOrigin::Application);
        assert!(
            registry
                .map_delivery(PlatformResultDelivery::Discarded {
                    identity: discarded
                })
                .is_none()
        );

        let overflow_marker = Rc::new(RefCell::new(0usize));
        let overflow_owner = AuxiliaryWindowOwner::new("overflow");
        let captured = Rc::clone(&overflow_marker);
        let overflow_identity = registry.register(
            Box::new(move |_| {
                *captured.borrow_mut() += 1;
                3
            }),
            &EffectOrigin::Auxiliary(overflow_owner.clone()),
        );
        overflow_owner.retire();
        let ingress = Arc::new(Mutex::new(PlatformResultIngress::default()));
        assert!(
            ingress
                .lock()
                .expect("ingress lock")
                .enqueue_overflow(delivery_for(overflow_identity))
        );
        let delivery = ingress
            .lock()
            .expect("ingress lock")
            .take_pending()
            .pop()
            .expect("overflow delivery");
        assert!(registry.map_delivery(delivery).is_none());
        assert_eq!(*overflow_marker.borrow(), 0);
    }

    fn delivery_for(identity: PlatformCompletionIdentity) -> PlatformResultDelivery {
        PlatformResultDelivery::Completed {
            identity,
            result: Ok(PlatformResponse::Completed),
        }
    }

    #[test]
    fn frozen_batch_excludes_arrivals_after_turn_snapshot() {
        let identity = PlatformCompletionIdentity { id: 1, epoch: 1 };
        let delivery = || PlatformResultDelivery::Completed {
            identity,
            result: Ok(PlatformResponse::Completed),
        };
        let mut ingress = PlatformResultIngress::default();
        ingress.pending.push(delivery());
        ingress.pending.push(delivery());
        let frozen_count = ingress.pending_len();
        ingress.pending.push(delivery());

        let (batch, frozen_remainder) = ingress.take_frozen_pending_batch(frozen_count, 64);

        assert_eq!(batch.len(), 2);
        assert!(!frozen_remainder);
        assert_eq!(ingress.pending_len(), 1);
    }

    #[test]
    fn atomic_frozen_batch_keeps_late_reservation_behind_older_overflow() {
        let ingress = Arc::new(Mutex::new(PlatformResultIngress::default()));
        for id in 0..63 {
            let reservation = PlatformResultIngress::reserve(&ingress).expect("old reservation");
            assert!(reservation.commit(PlatformResultDelivery::Completed {
                identity: PlatformCompletionIdentity { id, epoch: 1 },
                result: Ok(PlatformResponse::Completed),
            }));
        }
        let late_reservation =
            PlatformResultIngress::reserve(&ingress).expect("outstanding late reservation");
        {
            let mut state = ingress.lock().expect("ingress lock");
            assert!(state.enqueue_overflow(PlatformResultDelivery::Completed {
                identity: PlatformCompletionIdentity { id: 100, epoch: 1 },
                result: Ok(PlatformResponse::Completed),
            }));
            let (frozen, frozen_remainder) = state.take_budgeted_pending_batch(8);
            assert!(frozen_remainder);
            assert_eq!(frozen.len(), 8);
            assert!(frozen.iter().all(|delivery| match delivery {
                PlatformResultDelivery::Completed { identity, .. } => identity.id < 8,
                PlatformResultDelivery::Discarded { .. } => false,
            }));
        }
        assert!(late_reservation.commit(PlatformResultDelivery::Completed {
            identity: PlatformCompletionIdentity { id: 101, epoch: 1 },
            result: Ok(PlatformResponse::Completed),
        }));

        let remainder = ingress.lock().expect("ingress lock").take_pending();
        let ids = remainder
            .into_iter()
            .map(|delivery| match delivery {
                PlatformResultDelivery::Completed { identity, .. } => identity.id,
                PlatformResultDelivery::Discarded { identity } => identity.id,
            })
            .collect::<Vec<_>>();
        assert_eq!(ids.first(), Some(&8));
        assert_eq!(ids.last(), Some(&101));
    }
}
