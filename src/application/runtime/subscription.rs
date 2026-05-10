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
        let mut subscriptions = Vec::new();
        for subscription in subscription_iter {
            subscription.append_to_batch(&mut subscriptions);
        }
        if subscriptions.is_empty() {
            Self::None
        } else {
            Self::Batch(subscriptions)
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
    pub const fn worker(
        id: &'static str,
        receiver: std::sync::mpsc::Receiver<Message>,
    ) -> Self {
        Self::Worker { id, receiver }
    }

    fn append_to_batch(self, subscriptions: &mut Vec<Subscription<Message>>) {
        match self {
            Self::None => {}
            Self::Batch(nested) => {
                for subscription in nested {
                    subscription.append_to_batch(subscriptions);
                }
            }
            subscription => subscriptions.push(subscription),
        }
    }
}

fn spawn_subscription<Message>(
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
            let delay = every.max(Duration::from_millis(1));
            let _ = thread::Builder::new()
                .name(format!("radiant-subscription-{id}"))
                .spawn(move || {
                    loop {
                        thread::sleep(delay);
                        let Some(runtime) = runtime.upgrade() else {
                            break;
                        };
                        if !runtime.enqueue(message()) {
                            break;
                        }
                    }
                });
        }
        Subscription::Worker { id, receiver } => {
            let _ = thread::Builder::new()
                .name(format!("radiant-worker-subscription-{id}"))
                .spawn(move || {
                    for message in receiver {
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

#[cfg(test)]
mod subscription_tests {
    use super::Subscription;
    use std::{sync::mpsc, time::Duration};

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
        assert!(matches!(subscriptions[0], Subscription::Interval { id: "first", .. }));
        assert!(matches!(subscriptions[1], Subscription::Worker { id: "second", .. }));
        assert!(matches!(subscriptions[2], Subscription::Interval { id: "third", .. }));
    }
}
