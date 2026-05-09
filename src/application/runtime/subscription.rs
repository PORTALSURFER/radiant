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
    pub fn batch(subscriptions: impl IntoIterator<Item = Subscription<Message>>) -> Self {
        let subscriptions = subscriptions
            .into_iter()
            .filter(|subscription| !matches!(subscription, Subscription::None))
            .collect::<Vec<_>>();
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
