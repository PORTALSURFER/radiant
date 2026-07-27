use super::*;
use crate::application::runtime::subscription::{
    WorkerSubscriptionDelivery, WorkerSubscriptionIdentity,
};
use crate::application::runtime::timer::TimerSink;
use crate::runtime::{PlatformCompletionIdentity, PlatformResultDelivery};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Barrier},
    thread,
    time::{Duration, Instant},
};

#[test]
fn pending_message_queue_retains_capacity_after_drain() {
    let mut runtime = AppRuntime::<u32>::default();
    for message in 0..32 {
        assert!(runtime.enqueue(message));
    }
    let capacity = runtime.pending.capacity();

    let pending = runtime.take_pending();
    assert_eq!(pending.len(), 32);
    assert_eq!(pending.capacity(), capacity);
    assert_eq!(runtime.pending.capacity(), capacity);
}

#[test]
fn ordinary_pending_messages_keep_full_ordering_and_depth() {
    let mut runtime = AppRuntime::<u32>::default();

    for message in 0..100 {
        assert!(runtime.enqueue(message));
    }

    assert_eq!(runtime.take_pending(), (0..100).collect::<Vec<_>>());
    let diagnostics = runtime.diagnostics_snapshot();
    assert_eq!(diagnostics.queue.stream_events_coalesced, 0);
    assert_eq!(diagnostics.queue.max_pending_messages, 100);
    assert_eq!(diagnostics.queue.max_pending_stream_slots, 0);
}

#[test]
fn ui_queue_accepts_rc_backed_messages() {
    let mut runtime = AppRuntime::default();
    let state = Rc::new(RefCell::new(7_u32));

    assert!(runtime.enqueue(Rc::clone(&state)));
    let delivered = runtime.take_pending();

    assert_eq!(delivered.len(), 1);
    assert!(Rc::ptr_eq(&delivered[0], &state));
}

#[test]
fn shared_ingress_is_send_and_sync_without_the_message_type() {
    fn assert_send_sync<T: Send + Sync>() {}

    assert_send_sync::<SharedRuntimeIngress>();
}

#[test]
fn sequenced_sources_preserve_fifo_order() {
    let mut runtime = AppRuntime::<u32>::default();
    let identity = WorkerSubscriptionIdentity { id: 1, epoch: 1 };
    let platform = PlatformCompletionIdentity { id: 1, epoch: 1 };

    assert!(runtime.enqueue(1));
    assert!(
        runtime
            .shared()
            .enqueue_worker_payload(identity, Box::new(2_u32))
    );
    assert!(
        runtime.shared().enqueue_platform_completion_reserved(
            runtime
                .shared()
                .reserve_delivery()
                .expect("slot is available"),
            PlatformResultDelivery::Completed {
                identity: platform,
                result: Ok(crate::runtime::PlatformResponse::Completed),
            },
        )
    );
    assert!(runtime.enqueue(4));

    let mapped = runtime.take_pending_with_mappers(
        |delivery| match delivery {
            WorkerSubscriptionDelivery::Payload { payload, .. } => {
                Some(*payload.downcast::<u32>().expect("u32 payload"))
            }
            WorkerSubscriptionDelivery::Disconnected { .. } => None,
        },
        |_| Some(3),
        |_| None,
    );
    assert_eq!(mapped, vec![1, 2, 3, 4]);
}

#[test]
fn platform_reservations_remain_bounded_until_ui_drain() {
    let mut runtime = AppRuntime::<()>::default();
    for id in 0..64 {
        let reservation = runtime
            .shared()
            .reserve_delivery()
            .expect("capacity should admit the bounded platform lane");
        assert!(runtime.shared().enqueue_platform_completion_reserved(
            reservation,
            PlatformResultDelivery::Completed {
                identity: PlatformCompletionIdentity { id, epoch: 1 },
                result: Ok(crate::runtime::PlatformResponse::Completed),
            },
        ));
    }
    assert!(runtime.shared().reserve_delivery().is_none());

    let _ = runtime.take_pending_with_mappers(|_| None, |_| None, |_| None);
    assert!(runtime.shared().reserve_delivery().is_some());
}

#[test]
fn shared_ingress_preserves_worker_before_later_timer_order() {
    let mut runtime = AppRuntime::<u32>::default();
    let identity = WorkerSubscriptionIdentity { id: 2, epoch: 1 };

    assert!(
        runtime
            .shared()
            .enqueue_worker_payload(identity, Box::new(1_u32))
    );
    assert!(TimerSink::enqueue_timer_wake(
        runtime.shared().as_ref(),
        RuntimeTimerWake::controller(1, 0, 1),
    ));

    let mut items = Vec::new();
    assert!(!runtime.drain_pending_item_batch_into(&mut items, 8));

    let first = items.remove(0);
    let RuntimeQueueItem::Delivery(first) = first else {
        panic!("worker delivery should remain opaque until the UI queue head");
    };
    let Ok(first) = first.downcast::<SharedRuntimeDelivery>() else {
        panic!("application delivery type");
    };
    assert_eq!(
        match first {
            SharedRuntimeDelivery::Worker(delivery) =>
                map_u32_worker_delivery(delivery).expect("worker message"),
            _ => panic!("first item should be the worker delivery"),
        },
        1
    );
    assert!(matches!(
        items.as_slice(),
        [RuntimeQueueItem::Timer(wake)]
            if *wake == RuntimeTimerWake::controller(1, 0, 1)
    ));
}

#[test]
fn opaque_worker_messages_obey_normal_and_interactive_budgets() {
    let mut runtime = AppRuntime::<u32> {
        shared: Arc::new(SharedRuntimeIngress::with_capacity_for_test(128)),
        pending: Vec::new(),
        pending_frame: None,
    };
    let identity = WorkerSubscriptionIdentity { id: 2, epoch: 1 };
    for message in 0..65_u32 {
        assert!(
            runtime
                .shared()
                .enqueue_worker_payload(identity, Box::new(message))
        );
    }

    let mut normal = Vec::new();
    assert!(runtime.drain_pending_batch_into_with_mappers(
        &mut normal,
        64,
        map_u32_worker_delivery,
        |_| None,
        |_| None,
    ));
    assert_eq!(normal.len(), 64);

    let mut interactive = Vec::new();
    assert!(!runtime.drain_pending_batch_into_with_mappers(
        &mut interactive,
        8,
        map_u32_worker_delivery,
        |_| None,
        |_| None,
    ));
    assert_eq!(interactive, vec![64]);
}

#[test]
fn shared_ingress_rejects_newest_at_capacity_and_keeps_fifo() {
    let runtime = Arc::new(SharedRuntimeIngress::default());
    let identity = WorkerSubscriptionIdentity { id: 11, epoch: 1 };
    for message in 0..64_u32 {
        assert!(runtime.enqueue_worker_payload(identity, Box::new(message)));
    }
    assert!(!runtime.enqueue_worker_payload(identity, Box::new(64_u32)));

    let queued = runtime.drain_incoming();
    assert_eq!(queued.len(), 64);
    let values = queued
        .into_iter()
        .map(|delivery| match delivery.value {
            SharedRuntimeDelivery::Worker(WorkerSubscriptionDelivery::Payload {
                payload, ..
            }) => *payload.downcast::<u32>().expect("u32 payload"),
            _ => panic!("worker payload expected"),
        })
        .collect::<Vec<_>>();
    assert_eq!(values, (0..64).collect::<Vec<_>>());
    assert_eq!(
        runtime.diagnostics_snapshot().queue.shared_ingress_rejected,
        1
    );
}

#[test]
fn one_shot_timer_reservation_rejects_without_registration_leak() {
    let runtime = Arc::new(SharedRuntimeIngress::default());
    let identity = WorkerSubscriptionIdentity { id: 12, epoch: 1 };
    for message in 0..64_u32 {
        assert!(runtime.enqueue_worker_payload(identity, Box::new(message)));
    }
    let timer = runtime.allocate_timer_identity(0);
    assert!(!runtime.schedule_timer_wake(Duration::ZERO, timer));
    assert_eq!(runtime.application_timer_identity_count(), 0);
}

#[test]
fn invalidated_one_shot_timer_releases_reservation_when_due() {
    let runtime = Arc::new(SharedRuntimeIngress::with_capacity_for_test(1));
    let stale = runtime.allocate_timer_identity(0);
    assert!(runtime.schedule_timer(Duration::from_millis(5), stale, false));
    runtime.invalidate_timer_for_test(stale);

    let replacement = runtime.allocate_timer_identity(0);
    let deadline = Instant::now() + Duration::from_secs(1);
    while !runtime.schedule_timer(Duration::from_secs(60), replacement, false)
        && Instant::now() < deadline
    {
        thread::yield_now();
    }
    assert!(
        Instant::now() < deadline,
        "stale timer reservation was not released"
    );
}

#[test]
fn duplicate_one_shot_wake_coalescing_releases_extra_reservation() {
    let runtime = Arc::new(SharedRuntimeIngress::with_capacity_for_test(2));
    let identity = runtime.allocate_timer_identity(0);
    assert!(runtime.schedule_timer(Duration::from_secs(60), identity, false));
    assert!(runtime.schedule_timer(Duration::from_secs(60), identity, false));

    assert!(TimerSink::enqueue_timer_wake(runtime.as_ref(), identity));
    assert!(!TimerSink::enqueue_timer_wake(runtime.as_ref(), identity));

    let replacement = runtime.allocate_timer_identity(0);
    assert!(runtime.schedule_timer(Duration::from_secs(60), replacement, false));
    assert_eq!(
        runtime
            .diagnostics_snapshot()
            .queue
            .shared_ingress_coalesced,
        1
    );
}

#[test]
fn recurring_wake_rejects_when_terminal_reservation_uses_last_slot() {
    let runtime = Arc::new(SharedRuntimeIngress::with_capacity_for_test(1));
    let delayed = runtime.allocate_timer_identity(0);
    assert!(runtime.schedule_timer(Duration::from_secs(60), delayed, false));

    let recurring = runtime.allocate_timer_identity(0);
    assert!(runtime.schedule_timer(Duration::from_secs(60), recurring, true));
    assert!(!TimerSink::enqueue_timer_wake(runtime.as_ref(), recurring));

    assert!(TimerSink::enqueue_timer_wake(runtime.as_ref(), delayed));
}

#[test]
fn shared_terminal_reservation_rolls_back_without_leaking_capacity() {
    let runtime = Arc::new(SharedRuntimeIngress::with_capacity_for_test(1));
    let reservation = runtime.reserve_delivery().expect("slot is available");
    assert!(!runtime.enqueue_worker_payload(
        WorkerSubscriptionIdentity { id: 13, epoch: 1 },
        Box::new(1_u32),
    ));
    drop(reservation);
    assert!(runtime.enqueue_worker_payload(
        WorkerSubscriptionIdentity { id: 13, epoch: 1 },
        Box::new(1_u32),
    ));
}

#[test]
fn delivery_reservation_does_not_retain_runtime_ingress() {
    let runtime = Arc::new(SharedRuntimeIngress::default());
    let weak = Arc::downgrade(&runtime);
    let reservation = runtime.reserve_delivery().expect("slot is available");

    drop(runtime);
    assert!(weak.upgrade().is_none());
    drop(reservation);
}

#[test]
fn blocked_reservation_closure_does_not_retain_runtime_ingress() {
    let runtime = Arc::new(SharedRuntimeIngress::default());
    let weak = Arc::downgrade(&runtime);
    let reservation = runtime.reserve_delivery().expect("slot is available");
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let closure = thread::spawn(move || {
        started_tx.send(()).expect("closure starts");
        release_rx.recv().expect("closure release");
        drop(reservation);
    });

    started_rx.recv().expect("closure started");
    drop(runtime);
    assert!(weak.upgrade().is_none());
    release_tx.send(()).expect("release closure");
    closure.join().expect("closure exits");
}

#[test]
fn recurring_timer_wakes_coalesce_and_continue_after_saturation() {
    let runtime = Arc::new(SharedRuntimeIngress::default());
    let identity = runtime.allocate_timer_identity(0);
    assert!(runtime.schedule_timer(Duration::from_secs(60), identity, true));
    assert!(TimerSink::enqueue_timer_wake(runtime.as_ref(), identity));
    assert!(!TimerSink::enqueue_timer_wake(runtime.as_ref(), identity));
    assert_eq!(
        runtime
            .diagnostics_snapshot()
            .queue
            .shared_ingress_coalesced,
        1
    );
}

#[test]
fn worker_payload_shutdown_fence_rechecks_liveness_before_append() {
    let mut runtime = AppRuntime::<u32>::default();
    let identity = WorkerSubscriptionIdentity { id: 3, epoch: 1 };
    let pre_append = Arc::new(Barrier::new(2));
    let release_append = Arc::new(Barrier::new(2));
    let worker_runtime = Arc::clone(runtime.shared());
    let worker_pre_append = Arc::clone(&pre_append);
    let worker_release_append = Arc::clone(&release_append);
    let worker = thread::spawn(move || {
        worker_runtime.enqueue_worker_payload_with_pre_append_hook(
            identity,
            Box::new(99_u32),
            move || {
                worker_pre_append.wait();
                worker_release_append.wait();
            },
        )
    });

    pre_append.wait();
    runtime.shutdown();
    release_append.wait();

    assert!(!worker.join().expect("worker should complete"));
    assert!(runtime.take_pending().is_empty());
}

#[test]
fn worker_disconnect_shutdown_fence_rechecks_liveness_before_append() {
    let mut runtime = AppRuntime::<u32>::default();
    let identity = WorkerSubscriptionIdentity { id: 4, epoch: 1 };
    let pre_append = Arc::new(Barrier::new(2));
    let release_append = Arc::new(Barrier::new(2));
    let worker_runtime = Arc::clone(runtime.shared());
    let worker_pre_append = Arc::clone(&pre_append);
    let worker_release_append = Arc::clone(&release_append);
    let worker = thread::spawn(move || {
        worker_runtime.enqueue_worker_delivery_with_pre_append_hook(
            WorkerSubscriptionDelivery::Disconnected { identity },
            move || {
                worker_pre_append.wait();
                worker_release_append.wait();
            },
        )
    });

    pre_append.wait();
    runtime.shutdown();
    release_append.wait();

    assert!(!worker.join().expect("disconnect worker should complete"));
    assert!(runtime.take_pending().is_empty());
}

#[test]
fn pending_message_queue_drains_into_reused_output_without_replacing_queue_storage() {
    let mut runtime = AppRuntime::<u32>::default();
    for message in 0..32 {
        assert!(runtime.enqueue(message));
    }
    let queue_capacity = runtime.pending.capacity();
    let mut pending = Vec::with_capacity(64);
    let output_capacity = pending.capacity();

    runtime.drain_pending_into(&mut pending);

    assert_eq!(pending, (0..32).collect::<Vec<_>>());
    assert_eq!(pending.capacity(), output_capacity);
    assert!(runtime.pending.is_empty());
    assert_eq!(runtime.pending.capacity(), queue_capacity);
}

#[test]
fn budgeted_pending_drain_reports_remaining_runtime_work() {
    let mut runtime = AppRuntime::<u32>::default();
    let mut batch = Vec::new();

    for message in 0..10 {
        assert!(runtime.enqueue(message));
    }

    assert!(runtime.drain_pending_batch_into(&mut batch, 8));
    assert_eq!(batch, (0..8).collect::<Vec<_>>());

    batch.clear();
    assert!(!runtime.drain_pending_batch_into(&mut batch, 8));
    assert_eq!(batch, vec![8, 9]);
}

#[test]
fn pending_frame_drains_before_regular_messages() {
    let mut runtime = AppRuntime::<u32>::default();

    assert!(runtime.enqueue(1));
    assert!(runtime.enqueue_frame(99));
    assert!(runtime.enqueue(2));

    assert_eq!(runtime.take_pending(), vec![99, 1, 2]);
}

#[test]
fn pending_frame_drains_before_retained_backlog() {
    let mut runtime = AppRuntime::<u32>::default();
    let mut pending = vec![10, 11];

    assert!(runtime.enqueue(1));
    assert!(runtime.enqueue_frame(99));
    runtime.drain_pending_into(&mut pending);

    assert_eq!(pending, vec![99, 10, 11, 1]);
}

#[test]
fn pending_frame_is_coalesced_until_drained() {
    let mut runtime = AppRuntime::<u32>::default();

    assert!(runtime.enqueue_frame(1));
    assert!(!runtime.enqueue_frame(2));
    assert_eq!(runtime.take_pending(), vec![1]);

    assert!(runtime.enqueue_frame(3));
    assert_eq!(runtime.take_pending(), vec![3]);
}

#[test]
fn delayed_messages_use_runtime_timer_lane() {
    let mut runtime = AppRuntime::<u32>::default();
    let identity = runtime.shared().allocate_timer_identity(0);

    assert!(
        runtime
            .shared()
            .schedule_timer_wake(Duration::from_millis(1), identity)
    );

    let started = Instant::now();
    let mut delivered = Vec::new();
    while started.elapsed() < Duration::from_secs(1) {
        delivered.extend(runtime.take_pending_with_mappers(|_| None, |_| None, |_| Some(7)));
        if !delivered.is_empty() {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }

    assert_eq!(delivered, vec![7]);
}

#[test]
fn delayed_messages_stop_after_runtime_shutdown() {
    let runtime = Arc::new(SharedRuntimeIngress::default());
    let identity = runtime.allocate_timer_identity(0);

    runtime.shutdown();

    assert!(!runtime.schedule_timer_wake(Duration::ZERO, identity));
    thread::sleep(Duration::from_millis(1));
    assert!(runtime.drain_incoming().is_empty());
}

#[test]
fn controller_timer_wakes_do_not_enter_application_identity_registry() {
    let runtime = Arc::new(SharedRuntimeIngress::with_capacity_for_test(256));
    let baseline = runtime.application_timer_identity_count();

    for id in 1..=256 {
        assert!(
            runtime.schedule_timer_wake(Duration::ZERO, RuntimeTimerWake::controller(id, 0, 1),)
        );
    }

    assert_eq!(runtime.application_timer_identity_count(), baseline);
    runtime.shutdown();
}

fn map_u32_worker_delivery(delivery: WorkerSubscriptionDelivery) -> Option<u32> {
    match delivery {
        WorkerSubscriptionDelivery::Payload { payload, .. } => {
            Some(*payload.downcast::<u32>().expect("u32 payload"))
        }
        WorkerSubscriptionDelivery::Disconnected { .. } => None,
    }
}
