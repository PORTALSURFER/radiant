use super::{
    AppRuntime, Subscription, WorkerSubscriptionEvent, receive_worker_message, spawn_subscription,
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
