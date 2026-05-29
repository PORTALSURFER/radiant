use super::{
    FrameAnimationRequest, FrameBuildCounts, FrameBuildResult, FrameBuildTiming,
    FrameCadenceConfig, FrameCadenceKind, FrameCadenceMonitor, FramePresentResult,
    FrameRebuildFlags,
};
use std::time::{Duration, Instant};

#[test]
fn frame_build_result_defaults_to_no_work_observed() {
    let result = FrameBuildResult::default();

    assert_eq!(result.counts.primitive_count, 0);
    assert_eq!(result.counts.text_run_count, 0);
    assert!(!result.rebuilds.layout_rebuild);
    assert!(!result.rebuilds.static_rebuild);
    assert!(!result.rebuilds.state_overlay_rebuild);
    assert!(!result.rebuilds.motion_overlay_rebuild);
    assert!(!result.animation.needs_animation);
    assert_eq!(result.timing.frame_total_us, 0);
    assert_eq!(result.timing.present_us, 0);
    assert_eq!(result.timing.frame_budget_us, 0);
    assert!(!result.timing.jank);
    assert!(!result.presentation.presented);
    assert!(!result.presentation.missed_present);
}

#[test]
fn frame_build_result_groups_related_feedback() {
    let result = FrameBuildResult {
        counts: FrameBuildCounts {
            primitive_count: 7,
            text_run_count: 3,
        },
        rebuilds: FrameRebuildFlags {
            layout_rebuild: true,
            static_rebuild: true,
            state_overlay_rebuild: false,
            motion_overlay_rebuild: true,
        },
        animation: FrameAnimationRequest {
            needs_animation: true,
        },
        timing: FrameBuildTiming {
            frame_total_us: 1_500,
            present_us: 400,
            frame_budget_us: 16_667,
            jank: false,
        },
        presentation: FramePresentResult {
            presented: true,
            missed_present: false,
        },
    };

    assert_eq!(result.counts.primitive_count, 7);
    assert!(result.rebuilds.layout_rebuild);
    assert!(result.animation.needs_animation);
    assert_eq!(result.timing.present_us, 400);
    assert!(result.presentation.presented);
}

#[test]
fn frame_cadence_monitor_classifies_start_normal_periodic_and_spikes() {
    let config = FrameCadenceConfig::new(Duration::from_millis(34), Duration::from_millis(100), 4);
    let mut monitor = FrameCadenceMonitor::new();

    let started = monitor.record_delta(None, config);
    assert_eq!(started.frame_index, 1);
    assert_eq!(started.kind, FrameCadenceKind::Started);
    assert!(started.should_report());

    let normal = monitor.record_delta(Some(Duration::from_millis(16)), config);
    assert_eq!(normal.frame_index, 2);
    assert_eq!(normal.kind, FrameCadenceKind::Normal);
    assert!(!normal.should_report());

    let warn = monitor.record_delta(Some(Duration::from_millis(40)), config);
    assert_eq!(warn.kind, FrameCadenceKind::WarnSpike);
    assert_eq!(warn.kind.severity(), Some("warn"));
    assert_eq!(warn.max_delta, Duration::from_millis(40));

    let periodic = monitor.record_delta(Some(Duration::from_millis(16)), config);
    assert_eq!(periodic.frame_index, 4);
    assert_eq!(periodic.kind, FrameCadenceKind::Periodic);
    assert_eq!(periodic.max_delta, Duration::from_millis(40));

    let error = monitor.record_delta(Some(Duration::from_millis(120)), config);
    assert_eq!(error.kind, FrameCadenceKind::ErrorSpike);
    assert_eq!(error.kind.severity(), Some("error"));
    assert_eq!(error.max_delta, Duration::from_millis(120));
}

#[test]
fn frame_cadence_monitor_allows_periodic_reporting_to_be_disabled() {
    let config = FrameCadenceConfig::new(Duration::from_millis(34), Duration::from_millis(100), 0);
    let mut monitor = FrameCadenceMonitor::new();

    monitor.record_delta(None, config);
    for _ in 0..8 {
        let report = monitor.record_delta(Some(Duration::from_millis(16)), config);
        assert_eq!(report.kind, FrameCadenceKind::Normal);
    }
}

#[test]
fn frame_cadence_monitor_saturates_reversed_injected_timestamps() {
    let config = FrameCadenceConfig::new(Duration::from_millis(34), Duration::from_millis(100), 0);
    let mut monitor = FrameCadenceMonitor::new();
    let now = Instant::now();

    monitor.record_at(now, config);
    let report = monitor.record_at(now - Duration::from_millis(1), config);

    assert_eq!(report.delta, Some(Duration::ZERO));
    assert_eq!(report.kind, FrameCadenceKind::Normal);
}
