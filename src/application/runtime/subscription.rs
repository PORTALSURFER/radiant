use super::AppRuntime;
use super::threading::{runtime_alive, spawn_business_thread};
use super::timer::min_timer_delay;
use std::{
    sync::mpsc::RecvTimeoutError,
    sync::{Arc, Weak},
    time::Duration,
};

const SUBSCRIPTION_CANCEL_POLL: Duration = Duration::from_millis(50);

/// App-level subscription sources evaluated when the native runtime starts.
pub enum Subscription<Message> {
    /// No subscription.
    None,
    /// Multiple subscriptions.
    Batch(Vec<Subscription<Message>>),
    /// Dispatch messages on a fixed interval.
    Interval {
        /// Human-readable subscription id.
        id: &'static str,
        /// Delay between emitted messages.
        every: Duration,
        /// Message factory invoked for each tick.
        message: Arc<dyn Fn() -> Message + Send + Sync>,
    },
    /// Forward messages from a host-owned receiver.
    Worker {
        /// Human-readable subscription id.
        id: &'static str,
        /// Receiver drained on a background thread.
        receiver: std::sync::mpsc::Receiver<Message>,
    },
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
            1 => subscriptions.pop().expect("single subscription exists"),
            _ => Self::Batch(subscriptions),
        }
    }

    /// Build an interval subscription.
    pub fn interval(
        id: &'static str,
        every: Duration,
        message: impl Fn() -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::Interval {
            id,
            every,
            message: Arc::new(message),
        }
    }

    /// Build a worker-message subscription from a receiver.
    pub const fn worker(id: &'static str, receiver: std::sync::mpsc::Receiver<Message>) -> Self {
        Self::Worker { id, receiver }
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

pub(super) fn spawn_subscription<Message>(
    runtime: Weak<AppRuntime<Message>>,
    subscription: Subscription<Message>,
) where
    Message: Send + 'static,
{
    match subscription {
        Subscription::None => {}
        Subscription::Batch(subscriptions) => {
            for subscription in subscriptions {
                spawn_subscription(runtime.clone(), subscription);
            }
        }
        Subscription::Interval { id, every, message } => {
            let Some(runtime) = runtime.upgrade() else {
                return;
            };
            if !runtime.schedule_interval(every.max(min_timer_delay()), message) {
                tracing::warn!(
                    subscription.id = id,
                    "Radiant app runtime failed to start interval subscription"
                );
            }
        }
        Subscription::Worker { id, receiver } => {
            spawn_business_thread(format!("worker-subscription-{id}"), move || {
                while let WorkerSubscriptionEvent::Message(message) =
                    receive_worker_message(&runtime, &receiver)
                {
                    let Some(runtime) = runtime.upgrade() else {
                        break;
                    };
                    if !runtime.enqueue(message) {
                        break;
                    }
                }
            });
        }
    }
}

enum WorkerSubscriptionEvent<Message> {
    Message(Message),
    Disconnected,
    Stopped,
}

fn receive_worker_message<Message>(
    runtime: &Weak<AppRuntime<Message>>,
    receiver: &std::sync::mpsc::Receiver<Message>,
) -> WorkerSubscriptionEvent<Message> {
    loop {
        if !runtime_alive(runtime) {
            return WorkerSubscriptionEvent::Stopped;
        }
        match receiver.recv_timeout(SUBSCRIPTION_CANCEL_POLL) {
            Ok(message) => return WorkerSubscriptionEvent::Message(message),
            Err(RecvTimeoutError::Disconnected) => return WorkerSubscriptionEvent::Disconnected,
            Err(RecvTimeoutError::Timeout) => {}
        }
    }
}

#[cfg(test)]
mod tests;
