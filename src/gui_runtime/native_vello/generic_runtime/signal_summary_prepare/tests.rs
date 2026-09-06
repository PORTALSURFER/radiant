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
    SummaryTargetId::new(
        WindowId::dummy(),
        NativeAdapterGeneration::from_test_serial(1),
        NativeTargetGeneration::from_test_serial(1),
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
        SummaryRequestState::WaitingAdmission
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
    assert_eq!(wake.0.load(Ordering::Relaxed), 1);
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
