use super::{
    AppRuntime, Subscription, WorkerSubscriptionEvent, receive_worker_payload,
    spawn_subscription_with_registry,
};
use crate::application::runtime::timer::TimerRegistry;
use std::{
    cell::RefCell,
    rc::Rc,
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
            Subscription::worker_payload("second", receiver, |message| message),
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
        Subscription::WorkerPayload { id: "second", .. }
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
    let mut registry = TimerRegistry::default();

    spawn_subscription_with_registry(
        Arc::downgrade(&runtime),
        &mut registry,
        &mut super::registry::WorkerSubscriptionRegistry::default(),
        Subscription::interval("tick", Duration::from_millis(1), || 1),
    );

    let started = Instant::now();
    let mut delivered = Vec::new();
    while started.elapsed() < Duration::from_secs(1) {
        for wake in runtime.take_timer_wakes() {
            if let Some(message) = registry.map_wake(wake) {
                let _ = runtime.enqueue(message);
            }
        }
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
    let (_sender, receiver) = mpsc::channel::<u32>();
    runtime.shutdown();

    let started = Instant::now();
    let event = receive_worker_payload(&weak, &super::TypedWorkerSubscriptionReceiver { receiver });

    assert!(matches!(event, WorkerSubscriptionEvent::Stopped));
    assert!(started.elapsed() < Duration::from_secs(1));
}

#[test]
fn worker_payload_mapper_runs_on_ui_thread_and_drops_after_disconnect() {
    let runtime = Arc::new(AppRuntime::<u32>::default());
    let mut timers = TimerRegistry::default();
    let mut workers = super::registry::WorkerSubscriptionRegistry::default();
    let (sender, receiver) = mpsc::channel::<u32>();
    let ui_thread = thread::current().id();
    let mapped = Rc::new(RefCell::new(Vec::new()));
    let mapper_state = Rc::clone(&mapped);
    let marker = Rc::new(());
    let mapper_marker = Rc::clone(&marker);

    spawn_subscription_with_registry(
        Arc::downgrade(&runtime),
        &mut timers,
        &mut workers,
        Subscription::worker_payload("payload", receiver, move |payload| {
            let _marker = &mapper_marker;
            mapper_state
                .borrow_mut()
                .push((payload, thread::current().id()));
            payload + 1
        }),
    );
    assert_eq!(Rc::strong_count(&marker), 2);

    sender.send(41).expect("worker receiver should be live");
    drop(sender);
    let started = Instant::now();
    let mut delivered = Vec::new();
    while started.elapsed() < Duration::from_secs(1) {
        delivered.extend(
            runtime.take_pending_with_worker_mapper(|delivery| workers.map_delivery(delivery)),
        );
        if !delivered.is_empty() && Rc::strong_count(&marker) == 1 {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    runtime.shutdown();

    assert_eq!(delivered, vec![42]);
    assert_eq!(mapped.borrow().as_slice(), &[(41, ui_thread)]);
    assert_eq!(Rc::strong_count(&marker), 1);
}
