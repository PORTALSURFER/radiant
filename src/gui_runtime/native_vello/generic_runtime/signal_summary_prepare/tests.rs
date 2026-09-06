use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

struct Wake(AtomicUsize);
impl RepaintSignal for Wake {
    fn request_repaint(&self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

fn broker(limits: Limits) -> SummaryBroker {
    SummaryBroker::with_limits(Arc::new(Wake(AtomicUsize::new(0))), limits)
}
fn limits() -> Limits {
    Limits {
        active: 2,
        queued: 8,
        sources: 64,
        targets: 128,
        bytes: 1 << 20,
    }
}
fn target() -> SummaryTargetId {
    target_at(1)
}
fn target_at(generation: u64) -> SummaryTargetId {
    SummaryTargetId::new(
        WindowId::dummy(),
        NativeAdapterGeneration::from_test_serial(1),
        NativeTargetGeneration::from_test_serial(generation),
        1,
    )
    .expect("monotonic target id")
}
fn request(samples: Arc<[f32]>, revision: u64) -> SummaryRequest {
    SummaryRequest::new(samples, 4, 1, revision)
}

#[test]
fn coalesces_targets_and_ignores_viewport() {
    let samples: Arc<[f32]> = Arc::from([0.0, 1.0, -1.0, 0.5]);
    let mut broker = broker(limits());
    assert_eq!(
        broker.request(target(), request(Arc::clone(&samples), 1)),
        SummaryRequestState::Pending
    );
    assert_eq!(
        broker.request(target(), request(samples, 1)),
        SummaryRequestState::Pending
    );
    assert_eq!(broker.sources.len(), 1);
    assert!(broker.take_dispatch().is_some());
    assert!(broker.take_dispatch().is_none());
}

#[test]
fn waiting_marker_does_not_retain_denied_source() {
    let mut broker = broker(Limits {
        sources: 0,
        ..limits()
    });
    let samples: Arc<[f32]> = Arc::from([0.0; 4]);
    assert_eq!(
        broker.request(target(), request(Arc::clone(&samples), 1)),
        SummaryRequestState::WaitingAdmission
    );
    assert_eq!(broker.sources.len(), 0);
    assert_eq!(Arc::strong_count(&samples), 1);
}

#[test]
fn caps_and_checked_bytes_reject_admission() {
    let mut broker = broker(Limits {
        queued: 1,
        bytes: 1,
        ..limits()
    });
    let samples: Arc<[f32]> = Arc::from([0.0; 4]);
    assert_eq!(
        broker.request(target(), request(samples, 1)),
        SummaryRequestState::Unavailable
    );
    assert!(broker.sources.is_empty());
}

#[test]
fn rejected_dispatch_releases_active_slot() {
    let mut broker = broker(limits());
    let samples: Arc<[f32]> = Arc::from([0.0; 4]);
    broker.request(target(), request(samples, 1));
    let dispatch = broker.take_dispatch().expect("queued dispatch");
    broker.reject_dispatch(dispatch.id);
    drop(dispatch);
    assert_eq!(broker.active, 0);
}

#[test]
fn cancelled_dispatch_publishes_terminal_before_wake() {
    let wake = Arc::new(Wake(AtomicUsize::new(0)));
    let mut broker = SummaryBroker::with_limits(wake.clone(), limits());
    let target = target();
    broker.request(target, request(Arc::from([0.0; 4096]), 1));
    let dispatch = broker.take_dispatch().expect("dispatch");
    broker.release_target(target);
    dispatch.run();
    assert_eq!(wake.0.load(Ordering::Relaxed), 2);
    broker.drain_completions();
    assert_eq!(broker.active, 0);
}

#[test]
fn retired_bytes_remain_until_lease_drops_and_maintenance_runs() {
    let mut broker = broker(limits());
    let target = target();
    broker.request(target, request(Arc::from([0.0; 4]), 1));
    let dispatch = broker.take_dispatch().expect("dispatch");
    dispatch.run();
    broker.drain_completions();
    let prepared = broker.prepared(target).expect("ready summary");
    let bytes = broker.bytes;
    broker.release_target(target);
    broker.maintain_retired();
    assert_eq!(broker.bytes, bytes);
    drop(prepared);
    broker.maintain_retired();
    assert_eq!(broker.bytes, 0);
}

#[test]
fn stable_target_request_does_not_cancel_active_work() {
    let mut broker = broker(limits());
    let target = target();
    let samples: Arc<[f32]> = Arc::from([0.0; 4096]);
    let initial = request(Arc::clone(&samples), 1);
    assert_eq!(
        broker.request(target, initial.clone()),
        SummaryRequestState::Pending
    );
    let dispatch = broker.take_dispatch().expect("active dispatch");
    assert_eq!(
        broker.request(target, initial),
        SummaryRequestState::Pending
    );
    assert!(!dispatch.cancelled.load(Ordering::Acquire));
    dispatch.run();
    assert_eq!(broker.drain_completions(), vec![target]);
    assert!(broker.prepared(target).is_some());
}

#[test]
fn releasing_queued_source_reclaims_exact_queue_capacity() {
    let mut broker = broker(Limits {
        queued: 1,
        ..limits()
    });
    let first = target();
    broker.request(first, request(Arc::from([0.0; 4]), 1));
    broker.release_target(first);
    assert_eq!(broker.capacity_status().queued, 0);
    assert_eq!(
        broker.request(target(), request(Arc::from([1.0; 4]), 2)),
        SummaryRequestState::Pending
    );
}

#[test]
fn cancelled_terminal_does_not_notify_interested_targets() {
    let mut broker = broker(limits());
    let samples: Arc<[f32]> = Arc::from([0.0; 4096]);
    let first = target();
    let second = target();
    broker.request(first, request(Arc::clone(&samples), 1));
    broker.request(second, request(samples, 1));
    let dispatch = broker.take_dispatch().expect("dispatch");
    dispatch.cancelled.store(true, Ordering::Release);
    dispatch.run();
    assert!(broker.drain_completions().is_empty());
    assert_eq!(broker.capacity_status().queued, 1);
}

#[test]
fn retired_leases_wake_but_ready_temporary_handles_do_not() {
    let wake = Arc::new(Wake(AtomicUsize::new(0)));
    let mut broker = SummaryBroker::with_limits(wake.clone(), limits());
    let target = target();
    broker.request(target, request(Arc::from([0.0; 4]), 1));
    broker.take_dispatch().expect("dispatch").run();
    broker.drain_completions();
    let first = broker.prepared(target).expect("ready");
    let second = first.clone();
    drop(second);
    assert_eq!(wake.0.load(Ordering::Relaxed), 1);
    broker.release_target(target);
    let before = wake.0.load(Ordering::Relaxed);
    drop(first);
    assert!(wake.0.load(Ordering::Relaxed) > before);
}

#[test]
fn close_and_recreate_target_only_notifies_current_epoch() {
    let mut broker = broker(limits());
    let old = target_at(1);
    let new = target_at(2);
    let samples: Arc<[f32]> = Arc::from([0.0; 4]);
    broker.request(old, request(Arc::clone(&samples), 1));
    let dispatch = broker.take_dispatch().expect("dispatch");
    broker.release_target(old);
    broker.request(new, request(samples, 1));
    dispatch.run();
    assert!(broker.drain_completions().is_empty());
    broker.take_dispatch().expect("replacement dispatch").run();
    assert_eq!(broker.drain_completions(), vec![new]);
}

#[test]
fn cancelled_completion_with_full_queue_retries_after_capacity_recovers() {
    let mut broker = broker(Limits {
        queued: 1,
        ..limits()
    });
    let first = target();
    let samples: Arc<[f32]> = Arc::from([0.0; 4]);
    broker.request(first, request(samples.clone(), 1));
    let dispatch = broker.take_dispatch().expect("active");
    let second = target();
    broker.request(second, request(Arc::from([1.0; 4]), 1));
    dispatch.cancelled.store(true, Ordering::Release);
    dispatch.run();
    assert!(broker.drain_completions().is_empty());
    assert_eq!(broker.waiting_targets().collect::<Vec<_>>(), vec![first]);
    broker.maintain_retired();
    broker.release_target(second);
    broker.maintain_retired();
    assert_eq!(
        broker.request(first, request(samples, 1)),
        SummaryRequestState::Pending
    );
    broker.take_dispatch().expect("retry").run();
    assert_eq!(broker.drain_completions(), vec![first]);
}

#[test]
fn revived_queued_source_waits_when_queue_is_temporarily_full() {
    let mut broker = broker(Limits {
        queued: 1,
        ..limits()
    });
    let first = target();
    let samples: Arc<[f32]> = Arc::from([0.0; 4]);
    broker.request(first, request(samples.clone(), 1));
    broker.release_target(first);
    let second = target();
    broker.request(second, request(Arc::from([1.0; 4]), 1));
    assert_eq!(
        broker.request(first, request(samples.clone(), 1)),
        SummaryRequestState::WaitingAdmission
    );
    broker.maintain_retired();
    broker.release_target(second);
    broker.maintain_retired();
    assert_eq!(
        broker.request(first, request(samples, 1)),
        SummaryRequestState::Pending
    );
    broker.take_dispatch().expect("retry").run();
    assert_eq!(broker.drain_completions(), vec![first]);
}

#[test]
fn reversed_worker_terminals_keep_exact_sources_and_release_slots() {
    let mut broker = broker(limits());
    let first = target();
    let second = target();
    broker.request(first, request(Arc::from([0.0; 4]), 1));
    broker.request(second, request(Arc::from([1.0; 4]), 2));
    let a = broker.take_dispatch().expect("first");
    let b = broker.take_dispatch().expect("second");
    b.run();
    a.run();
    assert_eq!(broker.drain_completions(), vec![second, first]);
    assert_eq!(broker.active, 0);
    assert_eq!(
        broker.prepared(first).expect("first ready").source()[0],
        0.0
    );
    assert_eq!(
        broker.prepared(second).expect("second ready").source()[0],
        1.0
    );
}

#[test]
fn shutdown_cancels_active_workers_without_waiting() {
    let mut broker = broker(limits());
    broker.request(target(), request(Arc::from([0.0; 4]), 1));
    let dispatch = broker.take_dispatch().expect("active");
    assert!(!dispatch.cancelled.load(Ordering::Acquire));
    drop(broker);
    assert!(dispatch.cancelled.load(Ordering::Acquire));
    dispatch.run();
}

#[test]
fn concurrent_retired_lease_drops_leave_maintenance_wake_and_release_bytes() {
    let wake = Arc::new(Wake(AtomicUsize::new(0)));
    let mut broker = SummaryBroker::with_limits(wake.clone(), limits());
    let target = target();
    broker.request(target, request(Arc::from([0.0; 4]), 1));
    broker.take_dispatch().expect("dispatch").run();
    broker.drain_completions();
    let a = broker.prepared(target).expect("ready");
    let b = a.clone();
    broker.release_target(target);
    let before = wake.0.load(Ordering::Relaxed);
    std::thread::scope(|scope| {
        scope.spawn(move || drop(a));
        scope.spawn(move || drop(b));
    });
    assert!(wake.0.load(Ordering::Relaxed) > before);
    broker.maintain_retired();
    assert_eq!(broker.bytes, 0);
}

#[test]
fn failed_source_replacement_wakes_maintenance_before_capacity_is_reclaimed() {
    let wake = Arc::new(Wake(AtomicUsize::new(0)));
    let mut broker = SummaryBroker::with_limits(
        wake.clone(),
        Limits {
            sources: 1,
            ..limits()
        },
    );
    let target = target();
    broker.request(target, request(Arc::from([0.0; 4]), 1));
    let dispatch = broker.take_dispatch().expect("dispatch");
    let id = dispatch.id();
    drop(dispatch);
    broker.reject_dispatch(id);
    let before = wake.0.load(Ordering::Relaxed);
    let replacement: Arc<[f32]> = Arc::from([1.0; 4]);
    assert_eq!(
        broker.request(target, request(replacement.clone(), 2)),
        SummaryRequestState::WaitingAdmission
    );
    assert!(wake.0.load(Ordering::Relaxed) > before);
    broker.maintain_retired();
    assert_eq!(
        broker.request(target, request(replacement, 2)),
        SummaryRequestState::Pending
    );
}
