use super::*;
use crate::application::runtime::subscription::{
    WorkerSubscriptionDelivery, WorkerSubscriptionIdentity,
};
use std::{
    sync::{
        Arc, Barrier,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

struct TestRepaintSignal {
    called: Arc<AtomicBool>,
}

impl RepaintSignal for TestRepaintSignal {
    fn request_repaint(&self) {
        self.called.store(true, Ordering::Release);
    }
}

#[test]
fn pending_messages_recover_after_poisoned_queue_lock() {
    let runtime = Arc::new(AppRuntime::<u32>::default());
    let poisoned = Arc::clone(&runtime);
    let _ = thread::spawn(move || {
        let mut pending = poisoned.pending.lock().expect("pending messages lock");
        pending.push(PendingMessage::Ordinary(1));
        panic!("poison pending message queue");
    })
    .join();

    assert!(runtime.enqueue(2));

    assert_eq!(runtime.take_pending(), vec![1, 2]);
}

#[test]
fn repaint_requests_recover_after_poisoned_signal_lock() {
    let runtime = Arc::new(AppRuntime::<u32>::default());
    let called = Arc::new(AtomicBool::new(false));
    let poisoned = Arc::clone(&runtime);
    let signal = Arc::clone(&called);
    let _ = thread::spawn(move || {
        let mut repaint = poisoned.repaint.lock().expect("repaint signal lock");
        *repaint = Some(Arc::new(TestRepaintSignal { called: signal }));
        panic!("poison repaint signal lock");
    })
    .join();

    assert!(runtime.enqueue(1));

    assert!(called.load(Ordering::Acquire));
}

#[test]
fn pending_message_queue_retains_capacity_after_drain() {
    let runtime = AppRuntime::<u32>::default();
    for message in 0..32 {
        assert!(runtime.enqueue(message));
    }
    let capacity = runtime.pending.lock().expect("pending lock").capacity();

    let pending = runtime.take_pending();
    assert_eq!(pending.len(), 32);
    assert_eq!(pending.capacity(), capacity);

    let retained_capacity = runtime.pending.lock().expect("pending lock").capacity();
    assert_eq!(retained_capacity, capacity);
}

#[test]
fn stream_latest_messages_coalesce_by_slot() {
    let runtime = AppRuntime::<u32>::default();
    let slot = runtime.begin_stream_slot();

    for message in 0..100 {
        assert!(runtime.enqueue_stream_latest(slot, message));
    }

    assert_eq!(runtime.take_pending(), vec![99]);
    let diagnostics = runtime.diagnostics_snapshot();
    assert_eq!(diagnostics.queue.stream_events_coalesced, 99);
    assert_eq!(diagnostics.queue.max_pending_messages, 1);
    assert_eq!(diagnostics.queue.max_pending_stream_slots, 1);
    assert_eq!(diagnostics.queue.current_pending_messages, 0);
    assert_eq!(diagnostics.queue.current_pending_stream_slots, 0);
}

#[test]
fn stream_latest_messages_preserve_final_message_order() {
    let runtime = AppRuntime::<u32>::default();
    let slot = runtime.begin_stream_slot();

    assert!(runtime.enqueue(1));
    assert!(runtime.enqueue_stream_latest(slot, 10));
    assert!(runtime.enqueue_stream_latest(slot, 11));
    assert!(runtime.enqueue(2));

    assert_eq!(runtime.take_pending(), vec![1, 11, 2]);
}

#[test]
fn independent_stream_slots_keep_one_latest_message_each() {
    let runtime = AppRuntime::<u32>::default();
    let first = runtime.begin_stream_slot();
    let second = runtime.begin_stream_slot();

    assert!(runtime.enqueue_stream_latest(first, 1));
    assert!(runtime.enqueue_stream_latest(second, 10));
    assert!(runtime.enqueue_stream_latest(first, 2));
    assert!(runtime.enqueue_stream_latest(second, 11));

    assert_eq!(runtime.take_pending(), vec![2, 11]);
    let diagnostics = runtime.diagnostics_snapshot();
    assert_eq!(diagnostics.queue.stream_events_coalesced, 2);
    assert_eq!(diagnostics.queue.max_pending_messages, 2);
    assert_eq!(diagnostics.queue.max_pending_stream_slots, 2);
}

#[test]
fn ordinary_pending_messages_keep_full_ordering_and_depth() {
    let runtime = AppRuntime::<u32>::default();

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
fn opaque_worker_messages_preserve_fifo_with_ordinary_messages() {
    let runtime = AppRuntime::<u32>::default();
    let identity = WorkerSubscriptionIdentity { id: 1, epoch: 1 };

    assert!(runtime.enqueue(1));
    assert!(runtime.enqueue_worker_payload(identity, Box::new(2_u32)));
    assert!(runtime.enqueue(3));

    let mapped = runtime.take_pending_with_worker_mapper(|delivery| match delivery {
        WorkerSubscriptionDelivery::Payload { payload, .. } => {
            Some(*payload.downcast::<u32>().expect("u32 payload"))
        }
        WorkerSubscriptionDelivery::Disconnected { .. } => None,
    });
    assert_eq!(mapped, vec![1, 2, 3]);
}

#[test]
fn opaque_worker_messages_obey_normal_and_interactive_budgets() {
    let runtime = AppRuntime::<u32>::default();
    let identity = WorkerSubscriptionIdentity { id: 2, epoch: 1 };
    for message in 0..65_u32 {
        assert!(runtime.enqueue_worker_payload(identity, Box::new(message)));
    }

    let mut normal = Vec::new();
    assert!(
        runtime.drain_pending_batch_into_with_worker_mapper(&mut normal, 64, |delivery| {
            match delivery {
                WorkerSubscriptionDelivery::Payload { payload, .. } => {
                    Some(*payload.downcast::<u32>().expect("u32 payload"))
                }
                WorkerSubscriptionDelivery::Disconnected { .. } => None,
            }
        },)
    );
    assert_eq!(normal.len(), 64);

    let mut interactive = Vec::new();
    assert!(!runtime.drain_pending_batch_into_with_worker_mapper(
        &mut interactive,
        8,
        |delivery| match delivery {
            WorkerSubscriptionDelivery::Payload { payload, .. } => {
                Some(*payload.downcast::<u32>().expect("u32 payload"))
            }
            WorkerSubscriptionDelivery::Disconnected { .. } => None,
        },
    ));
    assert_eq!(interactive, vec![64]);
}

#[test]
fn worker_payload_shutdown_fence_rechecks_liveness_before_append() {
    let runtime = Arc::new(AppRuntime::<u32>::default());
    let identity = WorkerSubscriptionIdentity { id: 3, epoch: 1 };
    let pre_append = Arc::new(Barrier::new(2));
    let release_append = Arc::new(Barrier::new(2));
    let worker_runtime = Arc::clone(&runtime);
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

    let runtime = Arc::new(AppRuntime::<u32>::default());
    let identity = WorkerSubscriptionIdentity { id: 4, epoch: 1 };
    let pre_append = Arc::new(Barrier::new(2));
    let release_append = Arc::new(Barrier::new(2));
    let worker_runtime = Arc::clone(&runtime);
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
fn stream_diagnostics_count_stale_and_shutdown_drops() {
    let runtime = AppRuntime::<u32>::default();
    let slot = runtime.begin_stream_slot();

    runtime.record_stale_stream_event();
    runtime.shutdown();

    assert!(!runtime.enqueue_stream_latest(slot, 1));
    let diagnostics = runtime.diagnostics_snapshot();
    assert_eq!(diagnostics.queue.stream_events_stale, 1);
    assert_eq!(diagnostics.queue.stream_events_dropped, 1);
}

#[test]
fn pending_message_queue_drains_into_reused_output_without_replacing_queue_storage() {
    let runtime = AppRuntime::<u32>::default();
    for message in 0..32 {
        assert!(runtime.enqueue(message));
    }
    let queue_capacity = runtime.pending.lock().expect("pending lock").capacity();
    let mut pending = Vec::with_capacity(64);
    let output_capacity = pending.capacity();

    runtime.drain_pending_into(&mut pending);

    assert_eq!(pending, (0..32).collect::<Vec<_>>());
    assert_eq!(pending.capacity(), output_capacity);
    let queue = runtime.pending.lock().expect("pending lock");
    assert!(queue.is_empty());
    assert_eq!(queue.capacity(), queue_capacity);
}

#[test]
fn budgeted_pending_drain_preserves_latest_slot_while_backlog_is_full() {
    let runtime = AppRuntime::<u32>::default();
    let slot = runtime.begin_stream_slot();
    let mut retained_backlog = (0..8).collect::<Vec<_>>();

    assert!(runtime.enqueue_stream_latest(slot, 100));
    assert!(runtime.drain_pending_batch_into(&mut retained_backlog, 8));
    assert_eq!(retained_backlog, (0..8).collect::<Vec<_>>());

    assert!(runtime.enqueue_stream_latest(slot, 101));
    let diagnostics = runtime.diagnostics_snapshot();
    assert_eq!(diagnostics.queue.stream_events_coalesced, 1);
    assert_eq!(diagnostics.queue.current_pending_messages, 1);
    assert_eq!(diagnostics.queue.current_pending_stream_slots, 1);

    let mut next_batch = Vec::new();
    assert!(!runtime.drain_pending_batch_into(&mut next_batch, 8));
    assert_eq!(next_batch, vec![101]);
}

#[test]
fn budgeted_pending_drain_reports_remaining_runtime_work() {
    let runtime = AppRuntime::<u32>::default();
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
    let runtime = AppRuntime::<u32>::default();

    assert!(runtime.enqueue(1));
    assert!(runtime.enqueue_frame(99));
    assert!(runtime.enqueue(2));

    assert_eq!(runtime.take_pending(), vec![99, 1, 2]);
}

#[test]
fn pending_frame_drains_before_retained_backlog() {
    let runtime = AppRuntime::<u32>::default();
    let mut pending = vec![10, 11];

    assert!(runtime.enqueue(1));
    assert!(runtime.enqueue_frame(99));
    runtime.drain_pending_into(&mut pending);

    assert_eq!(pending, vec![99, 10, 11, 1]);
}

#[test]
fn pending_frame_is_coalesced_until_drained() {
    let runtime = AppRuntime::<u32>::default();

    assert!(runtime.enqueue_frame(1));
    assert!(!runtime.enqueue_frame(2));
    assert_eq!(runtime.take_pending(), vec![1]);

    assert!(runtime.enqueue_frame(3));
    assert_eq!(runtime.take_pending(), vec![3]);
}

#[test]
fn delayed_messages_use_runtime_timer_lane() {
    let runtime = Arc::new(AppRuntime::<u32>::default());
    let identity = runtime.allocate_timer_identity(0);

    assert!(runtime.schedule_timer_wake(Duration::from_millis(1), identity));

    let started = Instant::now();
    let mut delivered = Vec::new();
    while started.elapsed() < Duration::from_secs(1) {
        if !runtime.take_timer_wakes().is_empty() {
            delivered.push(7);
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }

    assert_eq!(delivered, vec![7]);
}

#[test]
fn delayed_messages_stop_after_runtime_shutdown() {
    let runtime = Arc::new(AppRuntime::<u32>::default());
    let identity = runtime.allocate_timer_identity(0);

    runtime.shutdown();

    assert!(!runtime.schedule_timer_wake(Duration::ZERO, identity));
    thread::sleep(Duration::from_millis(1));
    assert!(runtime.take_pending().is_empty());
}

#[test]
fn controller_timer_wakes_do_not_enter_application_identity_registry() {
    let runtime = Arc::new(AppRuntime::<u32>::default());
    let baseline = runtime
        .timer_identities
        .lock()
        .expect("timer identities lock")
        .len();

    for id in 1..=256 {
        assert!(
            runtime.schedule_timer_wake(Duration::ZERO, RuntimeTimerWake::controller(id, 0, 1),)
        );
    }

    assert_eq!(
        runtime
            .timer_identities
            .lock()
            .expect("timer identities lock")
            .len(),
        baseline
    );
    runtime.shutdown();
}
