use crate::runtime::{Command, RuntimeTimerWake};
use std::any::Any;

/// Type-erased host delivery whose application mapper must run on the UI owner.
///
/// Worker and platform lanes may transport a `Send` payload into a shared host
/// ingress, then wrap it here without constructing an application message.
/// [`RuntimeQueueHost::map_runtime_queue_delivery`] maps the payload only when
/// this delivery reaches the ordered UI queue head.
pub struct RuntimeQueueDelivery {
    payload: Box<dyn Any + Send>,
}

impl RuntimeQueueDelivery {
    /// Wrap a host delivery for deferred UI-owned mapping.
    pub fn new<Payload>(payload: Payload) -> Self
    where
        Payload: Any + Send,
    {
        Self {
            payload: Box::new(payload),
        }
    }

    /// Recover a typed host delivery on the UI owner.
    pub fn downcast<Payload>(self) -> Result<Payload, Self>
    where
        Payload: Any + Send,
    {
        match self.payload.downcast::<Payload>() {
            Ok(payload) => Ok(*payload),
            Err(payload) => Err(Self { payload }),
        }
    }
}

/// One UI-owned item drained from a host's ordered runtime ingress.
///
/// Hosts that combine worker, platform, and timer lanes should emit items in
/// admission order. The runtime reduces messages directly and maps opaque timer
/// wakes only after they reach this UI-owned queue.
pub enum RuntimeQueueItem<Message> {
    /// An application message ready for UI-owned reduction.
    Message(Message),
    /// An opaque timer wake awaiting UI-owned validation and mapping.
    Timer(RuntimeTimerWake),
    /// An opaque host delivery awaiting UI-owned mapping.
    Delivery(RuntimeQueueDelivery),
}

/// Optional host capability for runtime-owned command, message, and timer-wake
/// queues.
///
/// A custom host exposes timer completion as [`RuntimeTimerWake`] values, not
/// application messages. The UI runtime drains those wakes, validates their
/// owner and generation, invokes the application mapper, and reduces any
/// resulting message on the UI owner. No timer-thread message path exists.
pub trait RuntimeQueueHost<Message> {
    /// Drain commands delivered by app startup or bridge-owned work.
    fn take_runtime_commands(&mut self) -> Vec<Command<Message>> {
        Vec::new()
    }

    /// Drain commands into caller-owned scratch storage.
    fn drain_runtime_commands_into(&mut self, commands: &mut Vec<Command<Message>>) {
        commands.extend(self.take_runtime_commands());
    }

    /// Drain messages delivered by app tasks or worker subscriptions.
    ///
    /// Timer completions use [`Self::take_runtime_timer_wakes`] and are mapped
    /// on the UI turn instead of arriving here as ordinary timer messages.
    fn take_runtime_messages(&mut self) -> Vec<Message> {
        Vec::new()
    }

    /// Drain opaque timer wakes delivered by a host timer lane.
    ///
    /// Custom hosts must implement this ingress for delayed commands and
    /// interval subscriptions. The timer lane carries only the wake; the UI
    /// runtime owns FIFO ordering, generation/epoch validation, mapper
    /// invocation, and message reduction. Omitting this ingress drops timer
    /// work before the UI controller can map or repaint it.
    fn take_runtime_timer_wakes(&mut self) -> Vec<RuntimeTimerWake> {
        Vec::new()
    }

    /// Map an application-owned timer wake on the UI turn.
    ///
    /// The runtime calls this only when the wake reaches the unified FIFO head.
    /// It owns generation/epoch validation and invokes this mapper on the UI
    /// owner; controller-owned wakes are mapped by the runtime controller.
    /// A host must not invoke this method from its timer thread.
    fn map_runtime_timer_wake(&mut self, _wake: RuntimeTimerWake) -> Option<Message> {
        None
    }

    /// Map an opaque host delivery on the UI turn.
    ///
    /// The runtime invokes this only when the delivery reaches the ordered
    /// queue head. Hosts should downcast the payload and run the corresponding
    /// UI-owned worker or platform mapper here.
    fn map_runtime_queue_delivery(&mut self, _delivery: RuntimeQueueDelivery) -> Option<Message> {
        None
    }

    /// Drain messages into caller-owned scratch storage.
    fn drain_runtime_messages_into(&mut self, messages: &mut Vec<Message>) {
        messages.extend(self.take_runtime_messages());
    }

    /// Drain one bounded controller pass and report whether more remain.
    fn drain_runtime_message_batch_into(
        &mut self,
        messages: &mut Vec<Message>,
        _max_messages: usize,
    ) -> bool {
        self.drain_runtime_messages_into(messages);
        false
    }

    /// Drain ordered messages and timer wakes into caller-owned scratch storage.
    ///
    /// The default preserves the legacy host behavior of draining timer wakes
    /// before ordinary messages. Hosts with a shared ingress should override
    /// this method and preserve the admission order across both item kinds.
    fn drain_runtime_queue_item_batch_into(
        &mut self,
        items: &mut Vec<RuntimeQueueItem<Message>>,
        max_items: usize,
    ) -> bool {
        items.extend(
            self.take_runtime_timer_wakes()
                .into_iter()
                .map(RuntimeQueueItem::Timer),
        );
        let mut messages = Vec::new();
        let remaining = self.drain_runtime_message_batch_into(&mut messages, max_items);
        items.extend(messages.into_iter().map(RuntimeQueueItem::Message));
        remaining
    }
}

pub(crate) struct RuntimeQueueCapability<Bridge, Message> {
    pub drain_runtime_commands_into: fn(&mut Bridge, &mut Vec<Command<Message>>),
    pub drain_runtime_queue_item_batch_into:
        fn(&mut Bridge, &mut Vec<RuntimeQueueItem<Message>>, usize) -> bool,
    pub map_runtime_timer_wake: fn(&mut Bridge, RuntimeTimerWake) -> Option<Message>,
    pub map_runtime_queue_delivery: fn(&mut Bridge, RuntimeQueueDelivery) -> Option<Message>,
}

impl<Bridge, Message> RuntimeQueueCapability<Bridge, Message>
where
    Bridge: RuntimeQueueHost<Message>,
{
    pub const fn new() -> Self {
        Self {
            drain_runtime_commands_into: Bridge::drain_runtime_commands_into,
            drain_runtime_queue_item_batch_into: Bridge::drain_runtime_queue_item_batch_into,
            map_runtime_timer_wake: Bridge::map_runtime_timer_wake,
            map_runtime_queue_delivery: Bridge::map_runtime_queue_delivery,
        }
    }
}
