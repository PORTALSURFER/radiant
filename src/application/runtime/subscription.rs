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
                eprintln!("Radiant app runtime: failed to start interval subscription {id}");
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
mod subscription_tests {
    use super::{
        AppRuntime, Subscription, WorkerSubscriptionEvent, receive_worker_message,
        spawn_subscription,
    };
    use std::{
        sync::{Arc, mpsc},
        thread,
        time::{Duration, Instant},
    };

    #[test]
    fn batch_drops_empty_subscriptions() {
        let subscription = Subscription::<u32>::batch([Subscription::none()]);

        assert!(matches!(subscription, Subscription::None));
    }

    #[test]
    fn batch_flattens_nested_subscriptions_in_order() {
        let (_sender, receiver) = mpsc::channel::<u32>();
        let subscription = Subscription::batch([
            Subscription::interval("first", Duration::from_millis(10), || 1),
            Subscription::batch([
                Subscription::none(),
                Subscription::worker("second", receiver),
                Subscription::batch([Subscription::interval(
                    "third",
                    Duration::from_millis(10),
                    || 3,
                )]),
            ]),
        ]);

        let Subscription::Batch(subscriptions) = subscription else {
            panic!("non-empty subscriptions should stay batched");
        };

        assert_eq!(subscriptions.len(), 3);
        assert!(matches!(
            subscriptions[0],
            Subscription::Interval { id: "first", .. }
        ));
        assert!(matches!(
            subscriptions[1],
            Subscription::Worker { id: "second", .. }
        ));
        assert!(matches!(
            subscriptions[2],
            Subscription::Interval { id: "third", .. }
        ));
    }

    #[test]
    fn batch_collapses_single_subscription_groups() {
        let subscription = Subscription::batch([
            Subscription::none(),
            Subscription::batch([Subscription::interval(
                "only",
                Duration::from_millis(10),
                || 1_u32,
            )]),
        ]);

        assert!(matches!(
            subscription,
            Subscription::Interval { id: "only", .. }
        ));
    }

    #[test]
    fn interval_subscription_delivers_ticks_from_runtime_timer_lane() {
        let runtime = Arc::new(AppRuntime::<u32>::default());

        spawn_subscription(
            Arc::downgrade(&runtime),
            Subscription::interval("tick", Duration::from_millis(1), || 1),
        );

        let started = Instant::now();
        let mut delivered = Vec::new();
        while started.elapsed() < Duration::from_secs(1) {
            delivered.extend(runtime.take_pending());
            if !delivered.is_empty() {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        runtime.shutdown();

        assert!(!delivered.is_empty());
        assert!(delivered.iter().all(|message| *message == 1));
    }

    #[test]
    fn worker_receive_stops_while_sender_remains_open() {
        let runtime = Arc::new(AppRuntime::<u32>::default());
        let weak = Arc::downgrade(&runtime);
        let (_sender, receiver) = mpsc::channel();
        runtime.shutdown();

        let started = Instant::now();
        let event = receive_worker_message(&weak, &receiver);

        assert!(matches!(event, WorkerSubscriptionEvent::Stopped));
        assert!(started.elapsed() < Duration::from_secs(1));
    }
}
