use super::SharedRuntimeIngress;
use super::threading::{runtime_alive, spawn_business_thread};
use super::timer::{TimerRegistry, min_timer_delay};
use std::{
    any::Any,
    sync::mpsc::RecvTimeoutError,
    sync::{Arc, Weak},
    time::Duration,
};

mod registry;

pub(crate) use registry::{
    WorkerSubscriptionDelivery, WorkerSubscriptionIdentity, WorkerSubscriptionRegistry,
};

const SUBSCRIPTION_CANCEL_POLL: Duration = Duration::from_millis(50);

/// App-level subscription sources evaluated when the native runtime starts.
///
pub enum Subscription<Message> {
    /// No subscription.
    None,
    /// Multiple subscriptions.
    Batch(Vec<Subscription<Message>>),
    /// Dispatch messages on a fixed interval.
    ///
    /// Interval ticks are represented by opaque timer wakes until the UI
    /// runtime drains and maps them. The timer lane never constructs or
    /// transports an application message; the interval factory runs on the
    /// UI owner.
    Interval {
        /// Human-readable subscription id.
        id: &'static str,
        /// Delay between emitted messages.
        every: Duration,
        /// Message factory invoked for each tick.
        message: Arc<dyn Fn() -> Message>,
    },
    /// Forward owned payloads from a host-owned receiver and map them on the UI thread.
    WorkerPayload {
        /// Human-readable subscription id.
        id: &'static str,
        /// Type-erased receiver drained on a background thread.
        receiver: Box<dyn WorkerSubscriptionReceiver>,
        /// UI-owned mapper invoked for each delivered payload.
        mapper: Box<dyn Fn(Box<dyn Any + Send>) -> Option<Message> + 'static>,
    },
}

/// Type-erased receiver used by [`Subscription::worker_payload`].
///
/// Implementations only transport owned `Send` payloads. The mapper supplied to
/// `worker_payload` is retained separately by the UI-owned application bridge.
#[doc(hidden)]
pub trait WorkerSubscriptionReceiver: Send {
    /// Receive one payload, waiting no longer than `timeout`.
    fn recv_timeout(&self, timeout: Duration) -> Result<Box<dyn Any + Send>, RecvTimeoutError>;
}

struct TypedWorkerSubscriptionReceiver<Payload> {
    receiver: std::sync::mpsc::Receiver<Payload>,
}

impl<Payload> WorkerSubscriptionReceiver for TypedWorkerSubscriptionReceiver<Payload>
where
    Payload: Send + 'static,
{
    fn recv_timeout(&self, timeout: Duration) -> Result<Box<dyn Any + Send>, RecvTimeoutError> {
        self.receiver
            .recv_timeout(timeout)
            .map(|payload| Box::new(payload) as Box<dyn Any + Send>)
    }
}

impl<Message> Subscription<Message> {
    /// Empty subscription.
    pub const fn none() -> Self {
        Self::None
    }

    /// Batch multiple subscriptions.
    pub fn batch(subscription_iter: impl IntoIterator<Item = Subscription<Message>>) -> Self {
        let subscription_iter = subscription_iter.into_iter();
        let mut subscriptions = Vec::with_capacity(subscription_iter.size_hint().0);
        for subscription in subscription_iter {
            subscription.append_to_batch(&mut subscriptions);
        }
        match subscriptions.len() {
            0 => Self::None,
            1 => match subscriptions.pop() {
                Some(subscription) => subscription,
                None => Self::None,
            },
            _ => Self::Batch(subscriptions),
        }
    }

    /// Build an interval subscription.
    ///
    /// Each accepted interval wake invokes `message` during the UI runtime's
    /// drain turn. Only the opaque wake crosses the timer-lane boundary, so
    /// the factory and resulting application message remain UI-owned.
    pub fn interval(
        id: &'static str,
        every: Duration,
        message: impl Fn() -> Message + 'static,
    ) -> Self {
        Self::Interval {
            id,
            every,
            message: Arc::new(message),
        }
    }

    /// Build a worker subscription that transports owned payloads and maps them on the UI thread.
    pub fn worker_payload<Payload>(
        id: &'static str,
        receiver: std::sync::mpsc::Receiver<Payload>,
        mapper: impl Fn(Payload) -> Message + 'static,
    ) -> Self
    where
        Payload: Send + 'static,
    {
        Self::WorkerPayload {
            id,
            receiver: Box::new(TypedWorkerSubscriptionReceiver { receiver }),
            mapper: Box::new(move |payload| {
                payload
                    .downcast::<Payload>()
                    .ok()
                    .map(|payload| mapper(*payload))
            }),
        }
    }

    fn append_to_batch(self, subscriptions: &mut Vec<Subscription<Message>>) {
        match self {
            Self::None => {}
            Self::Batch(nested) => {
                subscriptions.reserve(nested.len());
                for subscription in nested {
                    subscription.append_to_batch(subscriptions);
                }
            }
            subscription => subscriptions.push(subscription),
        }
    }
}

pub(super) fn spawn_subscription_with_registry<Message>(
    runtime: Weak<SharedRuntimeIngress>,
    timers: &mut TimerRegistry<Message>,
    workers: &mut WorkerSubscriptionRegistry<Message>,
    subscription: Subscription<Message>,
) {
    match subscription {
        Subscription::None => {}
        Subscription::Batch(subscriptions) => {
            for subscription in subscriptions {
                spawn_subscription_with_registry(runtime.clone(), timers, workers, subscription);
            }
        }
        Subscription::Interval { id, every, message } => {
            let Some(runtime) = runtime.upgrade() else {
                return;
            };
            if !timers.schedule_interval(&runtime, every.max(min_timer_delay()), message) {
                tracing::warn!(
                    subscription.id = id,
                    "Radiant app runtime failed to start interval subscription"
                );
            }
        }
        Subscription::WorkerPayload {
            id,
            receiver,
            mapper,
        } => {
            spawn_worker_subscription(runtime, workers, id, receiver, mapper);
        }
    }
}

fn spawn_worker_subscription<Message>(
    runtime: Weak<SharedRuntimeIngress>,
    workers: &mut WorkerSubscriptionRegistry<Message>,
    id: &'static str,
    receiver: Box<dyn WorkerSubscriptionReceiver>,
    mapper: Box<dyn Fn(Box<dyn Any + Send>) -> Option<Message> + 'static>,
) {
    let Some(runtime_owner) = runtime.upgrade() else {
        return;
    };
    let Some(terminal_reservation) = runtime_owner.reserve_delivery() else {
        tracing::warn!(
            subscription.id = id,
            "Radiant worker subscription rejected because shared ingress is saturated"
        );
        return;
    };
    let identity = workers.register(mapper);
    let worker_identity = identity;
    if !spawn_business_thread(format!("worker-subscription-{id}"), move || {
        let mut terminal_reservation = Some(terminal_reservation);
        loop {
            match receive_worker_payload(&runtime, receiver.as_ref()) {
                WorkerSubscriptionEvent::Payload(payload) => {
                    let Some(runtime) = runtime.upgrade() else {
                        break;
                    };
                    if !runtime.enqueue_worker_payload(worker_identity, payload) {
                        if let Some(reservation) = terminal_reservation.take() {
                            let _ = runtime
                                .enqueue_worker_disconnect_reserved(reservation, worker_identity);
                        }
                        break;
                    }
                }
                WorkerSubscriptionEvent::Disconnected => {
                    if let Some(runtime) = runtime.upgrade()
                        && let Some(reservation) = terminal_reservation.take()
                    {
                        let _ = runtime
                            .enqueue_worker_disconnect_reserved(reservation, worker_identity);
                    }
                    break;
                }
                WorkerSubscriptionEvent::Stopped => break,
            }
        }
    }) {
        workers.remove(identity);
    }
}

enum WorkerSubscriptionEvent {
    Payload(Box<dyn Any + Send>),
    Disconnected,
    Stopped,
}

fn receive_worker_payload(
    runtime: &Weak<SharedRuntimeIngress>,
    receiver: &dyn WorkerSubscriptionReceiver,
) -> WorkerSubscriptionEvent {
    loop {
        if !runtime_alive(runtime) {
            return WorkerSubscriptionEvent::Stopped;
        }
        match receiver.recv_timeout(SUBSCRIPTION_CANCEL_POLL) {
            Ok(payload) => return WorkerSubscriptionEvent::Payload(payload),
            Err(RecvTimeoutError::Disconnected) => return WorkerSubscriptionEvent::Disconnected,
            Err(RecvTimeoutError::Timeout) => {}
        }
    }
}

#[cfg(test)]
mod tests;
