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
        source_bytes: 1 << 20,
        summary_bytes: 1 << 20,
        overview_bytes: 1 << 20,
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
fn bounded_overview_reservation_stays_small_for_a_long_source() {
    let frames = 1 << 20;
    let request = SummaryRequest::new(Arc::from(vec![0.0; frames]), frames, 1, 1);

    let reservation = summary_reservation(&request).expect("bounded reservation");

    assert_eq!(reservation.source, frames * std::mem::size_of::<f32>());
    assert!(reservation.summary <= 256 * 1024);
    assert_eq!(
        reservation.total(),
        Some(reservation.source + reservation.summary)
    );
}

#[test]
fn source_and_overview_budgets_are_admitted_and_accounted_independently() {
    let samples: Arc<[f32]> = Arc::from([0.0; 4]);
    let request = request(Arc::clone(&samples), 1);
    let reservation = summary_reservation(&request).expect("reservation");
    let target = target();
    let mut broker = broker(Limits {
        source_bytes: reservation.source,
        summary_bytes: reservation.summary,
        overview_bytes: reservation.summary,
        bytes: reservation.total().expect("combined reservation"),
        ..limits()
    });

    assert_eq!(
        broker.request(target, request),
        SummaryRequestState::Pending
    );
    let status = broker.capacity_status();
    assert_eq!(status.source_logical_bytes, reservation.source);
    assert_eq!(status.summary_logical_bytes, reservation.summary);
    assert_eq!(
        status.logical_bytes,
        reservation.total().expect("combined reservation")
    );

    broker.release_target(target);
    broker.maintain_retired();
    let status = broker.capacity_status();
    assert_eq!(status.source_logical_bytes, 0);
    assert_eq!(status.summary_logical_bytes, 0);
    assert_eq!(status.logical_bytes, 0);
}

#[test]
fn source_and_overview_budgets_each_reject_oversized_admission() {
    let samples: Arc<[f32]> = Arc::from([0.0; 4]);
    let request = request(Arc::clone(&samples), 1);
    let reservation = summary_reservation(&request).expect("reservation");

    let mut source_limited = broker(Limits {
        source_bytes: reservation.source - 1,
        bytes: usize::MAX,
        ..limits()
    });
    assert_eq!(
        source_limited.request(target(), request.clone()),
        SummaryRequestState::Unavailable
    );

    let mut summary_limited = broker(Limits {
        summary_bytes: reservation.summary - 1,
        bytes: usize::MAX,
        ..limits()
    });
    assert_eq!(
        summary_limited.request(target(), request),
        SummaryRequestState::Unavailable
    );
}

#[test]
fn rejected_dispatch_releases_active_slot() {
    let mut broker = broker(limits());
    let samples: Arc<[f32]> = Arc::from([0.0; 4]);
    broker.request(target(), request(samples, 1));
    let dispatch = broker.take_dispatch().expect("queued dispatch");
    broker.reject_dispatch(dispatch.id());
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
    assert!(!cancel_flag(&dispatch).load(Ordering::Acquire));
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
    cancel_flag(&dispatch).store(true, Ordering::Release);
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
    cancel_flag(&dispatch).store(true, Ordering::Release);
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
    assert!(!cancel_flag(&dispatch).load(Ordering::Acquire));
    drop(broker);
    assert!(cancel_flag(&dispatch).load(Ordering::Acquire));
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

impl SummaryBroker {
    pub(in crate::gui_runtime::native_vello::generic_runtime) fn prepare_for_test(
        content: &GpuSurfaceContent,
        revision: u64,
    ) -> (Self, PreparedSummary) {
        struct NoopWake;
        impl RepaintSignal for NoopWake {
            fn request_repaint(&self) {}
        }

        let mut broker = Self::new(Arc::new(NoopWake));
        let target = SummaryTargetId::new(
            WindowId::dummy(),
            NativeAdapterGeneration::from_test_serial(1),
            NativeTargetGeneration::from_test_serial(1),
            1,
        )
        .expect("test target serial");
        let request = SummaryRequest::from_raw_surface(content, revision)
            .expect("renderable raw signal content");
        assert_eq!(
            broker.request(target, request),
            SummaryRequestState::Pending
        );
        broker
            .take_dispatch()
            .expect("admitted test dispatch")
            .run();
        broker.drain_completions();
        let prepared = broker.prepared(target).expect("prepared test summary");
        (broker, prepared)
    }
}

#[test]
fn maintenance_during_off_thread_last_drop_observes_released_token() {
    struct SynchronousWake {
        armed: AtomicBool,
        events: SyncSender<()>,
        ack: std::sync::Mutex<Receiver<()>>,
    }
    impl RepaintSignal for SynchronousWake {
        fn request_repaint(&self) {
            if self.armed.load(Ordering::Acquire) {
                self.events.send(()).expect("maintenance event");
                self.ack
                    .lock()
                    .expect("ack lock")
                    .recv_timeout(std::time::Duration::from_secs(5))
                    .expect("maintenance completed during wake");
            }
        }
    }
    let (events, receiver) = sync_channel(1);
    let (ack, ack_receiver) = sync_channel(1);
    let wake = Arc::new(SynchronousWake {
        armed: AtomicBool::new(false),
        events,
        ack: std::sync::Mutex::new(ack_receiver),
    });
    let mut broker = SummaryBroker::with_limits(wake.clone(), limits());
    let target = target();
    broker.request(target, request(Arc::from([0.0; 4]), 1));
    broker.take_dispatch().expect("dispatch").run();
    broker.drain_completions();
    let prepared = broker.prepared(target).expect("ready");
    broker.release_target(target);
    wake.armed.store(true, Ordering::Release);
    let observed_bytes = std::thread::scope(|scope| {
        let dropper = scope.spawn(move || drop(prepared));
        receiver
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("drop wake");
        // The worker is deliberately paused inside request_repaint, before its
        // Drop implementation returns. No later maintenance is allowed to mask
        // token-release ordering.
        broker.maintain_retired();
        let bytes = broker.capacity_status().logical_bytes;
        ack.send(()).expect("acknowledge maintenance");
        dropper.join().expect("dropper");
        bytes
    });
    assert_eq!(observed_bytes, 0);
    assert!(broker.sources.is_empty());
}

fn cancel_flag(dispatch: &SummaryDispatch) -> &AtomicBool {
    match dispatch {
        SummaryDispatch::Overview(job) => &job.cancelled,
        SummaryDispatch::Tile(_) => panic!("expected overview fixture"),
    }
}

#[test]
fn long_source_publishes_overview_then_shared_detail_and_reuses_pan_page() {
    let frames = 65_536;
    let samples: Arc<[f32]> = (0..frames)
        .map(|frame| if frame == 21 { 1.0 } else { 0.0 })
        .collect();
    let mut broker = SummaryBroker::new(Arc::new(Wake(AtomicUsize::new(0))));
    let a = target();
    let b = target();
    let content = GpuSurfaceContent::SignalBands {
        frames,
        band_count: 1,
        frame_range: [10.25, 50.25],
        samples,
    };
    broker.request(a, SummaryRequest::from_raw_surface(&content, 1).unwrap());
    broker.request(b, SummaryRequest::from_raw_surface(&content, 1).unwrap());
    broker.take_dispatch().unwrap().run();
    broker.drain_completions();
    let coarse = broker.prepared(a).unwrap();
    assert!(coarse.summary().levels[0].bucket_frames > 1);
    assert!(coarse.tile().is_none());
    assert_eq!(broker.tiles.len(), 1);
    broker.take_dispatch().unwrap().run();
    broker.drain_completions();
    let detail = broker.prepared(a).unwrap();
    assert_eq!(detail.tile().unwrap().bucket_frames, 1);
    assert_eq!(detail.asset_key(), broker.prepared(b).unwrap().asset_key());
    let mut moved = content.clone();
    if let GpuSurfaceContent::SignalBands { frame_range, .. } = &mut moved {
        *frame_range = [11.25, 51.25];
    }
    broker.request(a, SummaryRequest::from_raw_surface(&moved, 1).unwrap());
    assert_eq!(detail.asset_key(), broker.prepared(a).unwrap().asset_key());
    assert!(broker.take_dispatch().is_none());
    broker.release_target(a);
    broker.release_target(b);
    broker.maintain_retired();
    assert!(broker.capacity_status().source_logical_bytes > 0);
    drop(detail);
    drop(coarse);
    broker.maintain_retired();
    assert_eq!(broker.capacity_status().logical_bytes, 0);
}

#[test]
fn overview_and_detail_share_global_active_capacity() {
    let mut broker = SummaryBroker::new(Arc::new(Wake(AtomicUsize::new(0))));
    let content = GpuSurfaceContent::SignalBands {
        frames: 65_536,
        band_count: 1,
        frame_range: [10.0, 50.0],
        samples: vec![0.0; 65_536].into(),
    };
    broker.request(
        target(),
        SummaryRequest::from_raw_surface(&content, 1).unwrap(),
    );
    broker.take_dispatch().unwrap().run();
    broker.drain_completions();
    let tile = broker.take_dispatch().unwrap();
    broker.request(target(), request(Arc::from([0.0; 4]), 2));
    let overview = broker.take_dispatch().unwrap();
    broker.request(target(), request(Arc::from([1.0; 4]), 3));
    assert!(broker.take_dispatch().is_none());
    assert_eq!(broker.capacity_status().active, 2);
    tile.run();
    overview.run();
    broker.drain_completions();
    assert!(broker.take_dispatch().is_some());
}

#[test]
fn released_gpu_capacity_notifies_only_current_ready_targets_once() {
    let mut broker = broker(limits());
    broker.gpu_budget = SignalGpuBudget::with_limit_for_test(8);
    let a = target();
    broker.request(a, request(Arc::from([0.0; 4]), 1));
    broker.take_dispatch().unwrap().run();
    broker.drain_completions();
    let held = broker.gpu_budget.reserve(8).unwrap();
    assert!(broker.gpu_budget.reserve(1).is_none());
    drop(held);
    assert_eq!(broker.drain_completions(), vec![a]);
    assert!(broker.drain_completions().is_empty());
    let held = broker.gpu_budget.reserve(8).unwrap();
    assert!(broker.gpu_budget.reserve(1).is_none());
    broker.release_target(a);
    drop(held);
    assert!(broker.drain_completions().is_empty());
}

#[test]
fn outside_source_raw_viewports_keep_legacy_clamped_overview() {
    let samples: Arc<[f32]> = vec![0.0; 65_536].into();
    for frame_range in [[-2.0, 20.0], [65_530.0, 65_540.0], [70_000.0, 70_010.0]] {
        let content = GpuSurfaceContent::SignalBands {
            frames: 65_536,
            band_count: 1,
            frame_range,
            samples: samples.clone(),
        };
        let request = SummaryRequest::from_raw_surface(&content, 1).unwrap();
        assert!(request.tile_view.is_none());
    }
}
