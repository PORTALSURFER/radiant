use super::{
    super::*,
    fixtures::{
        DeferredFocusBridge, DeferredPlatformFallbackBridge, DeferredScrollBridge,
        DeferredScrollFocusBridge,
    },
};
use crate::runtime::{FileDialogRequest, PlatformRequest, RepaintScope, SurfaceInvalidation};

#[test]
fn frame_refresh_diagnostics_accumulate_eager_refreshes_until_consumed() {
    let mut runtime =
        SurfaceRuntime::new(DeferredFocusBridge::default(), Vector2::new(160.0, 40.0));
    let _ = runtime.take_frame_refresh_diagnostics();

    runtime.refresh_with_scope(RepaintScope::Projection);
    let projection = runtime.last_refresh_diagnostics();
    runtime.refresh_with_scope(RepaintScope::Layout);
    let layout = runtime.last_refresh_diagnostics();

    let frame = runtime.take_frame_refresh_diagnostics();
    assert_eq!(frame.refresh.invalidation, SurfaceInvalidation::Layout);
    assert_eq!(
        frame.refresh.timings.application_projection,
        projection
            .timings
            .application_projection
            .saturating_add(layout.timings.application_projection)
    );
    assert_eq!(
        frame.refresh.timings.runtime_projection,
        projection
            .timings
            .runtime_projection
            .saturating_add(layout.timings.runtime_projection)
    );
    assert_eq!(
        frame.refresh.timings.widget_state_sync,
        projection
            .timings
            .widget_state_sync
            .saturating_add(layout.timings.widget_state_sync)
    );
    assert_eq!(frame.refresh.timings.layout, layout.timings.layout);
    assert!(frame.total >= frame.refresh.timings.total());

    let consumed = runtime.take_frame_refresh_diagnostics();
    assert_eq!(consumed.refresh.invalidation, SurfaceInvalidation::None);
    assert_eq!(consumed.total, std::time::Duration::ZERO);
}

#[test]
fn frame_refresh_take_keeps_latest_paint_segment_snapshot_after_reset() {
    let mut runtime =
        SurfaceRuntime::new(DeferredFocusBridge::default(), Vector2::new(160.0, 40.0));
    let unavailable = runtime.take_frame_refresh_diagnostics();
    assert_eq!(
        unavailable.paint_segments,
        crate::runtime::PaintSegmentObservation::unavailable()
    );

    let observed = crate::runtime::PaintSegmentObservation::empty();
    runtime.record_paint_segment_observation(observed);
    runtime.refresh_with_scope(RepaintScope::Projection);
    let frame = runtime.take_frame_refresh_diagnostics();
    assert_eq!(frame.paint_segments, observed);
    assert_eq!(frame.refresh.invalidation, SurfaceInvalidation::Projection);

    let repeated = runtime.take_frame_refresh_diagnostics();
    assert_eq!(repeated.paint_segments, observed);
    assert_eq!(repeated.refresh.invalidation, SurfaceInvalidation::None);
    assert_eq!(repeated.total, std::time::Duration::ZERO);

    let replacement = crate::runtime::PaintSegmentObservation::unavailable();
    runtime.record_paint_segment_observation(replacement);
    assert_eq!(
        runtime.take_frame_refresh_diagnostics().paint_segments,
        replacement
    );
}

#[test]
fn deferred_message_dispatch_refreshes_before_focus_followup() {
    let mut runtime =
        SurfaceRuntime::new(DeferredFocusBridge::default(), Vector2::new(160.0, 40.0));
    assert_eq!(runtime.bridge().project_count, 1);

    let mut outcome = CommandOutcome::default();
    runtime.dispatch_message_inner_deferred_refresh(1, &mut outcome);

    assert_eq!(
        runtime.focused_widget(),
        Some(42),
        "focus follow-up should see the widget projected by the deferred update"
    );
    assert_eq!(
        runtime.bridge().project_count,
        2,
        "deferred dispatch should refresh only when a follow-up command needs fresh traversal"
    );
    assert!(outcome.surface_refresh_requested);
    assert!(outcome.surface_repaint_requested);
}

#[test]
fn deferred_paint_only_batch_refreshes_before_focus_followup() {
    let mut runtime =
        SurfaceRuntime::new(DeferredFocusBridge::default(), Vector2::new(160.0, 40.0));
    assert_eq!(runtime.bridge().project_count, 1);

    let mut outcome = CommandOutcome::default();
    runtime.dispatch_message_inner_deferred_refresh(2, &mut outcome);

    assert_eq!(
        runtime.focused_widget(),
        Some(42),
        "layout-dependent follow-ups in paint-only batches should see newly projected widgets"
    );
    assert_eq!(
        runtime.bridge().project_count,
        2,
        "paint-only batches should refresh when they contain layout-dependent follow-ups"
    );
    assert!(outcome.paint_only_requested);
    assert!(outcome.surface_repaint_requested);
    assert!(outcome.surface_refresh_requested);
    assert_eq!(
        outcome.surface_invalidation(),
        crate::runtime::SurfaceInvalidation::Surface
    );
    assert_eq!(
        runtime.last_refresh_diagnostics().invalidation,
        crate::runtime::SurfaceInvalidation::Surface
    );
}

#[test]
fn deferred_command_batch_reuses_fresh_surface_for_followups() {
    let bridge = DeferredFocusBridge {
        show_focus_target: true,
        project_count: 0,
    };
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(160.0, 40.0));
    assert_eq!(runtime.bridge().project_count, 1);

    let mut outcome = CommandOutcome {
        surface_refresh_requested: true,
        ..CommandOutcome::default()
    };
    runtime.execute_command_inner_deferred_refresh(
        Command::batch([Command::focus(42), Command::focus(42)]),
        &mut outcome,
    );

    assert_eq!(runtime.focused_widget(), Some(42));
    assert_eq!(
        runtime.bridge().project_count,
        2,
        "a fresh deferred surface should be reused across layout-dependent batch commands"
    );
}

#[test]
fn deferred_platform_fallback_preserves_freshness_for_later_followups() {
    let mut runtime = SurfaceRuntime::new(
        DeferredPlatformFallbackBridge::default(),
        Vector2::new(160.0, 60.0),
    );
    assert_eq!(runtime.bridge().project_count, 1);

    let mut outcome = CommandOutcome {
        surface_refresh_requested: true,
        ..CommandOutcome::default()
    };
    runtime.execute_command_inner_deferred_refresh(
        Command::batch([
            Command::focus(42),
            Command::platform_request(
                PlatformRequest::PickFolder(FileDialogRequest::new()),
                |result| usize::from(result.is_err()),
            ),
            Command::focus(43),
        ]),
        &mut outcome,
    );

    assert_eq!(runtime.focused_widget(), Some(42));
    assert_eq!(runtime.bridge().project_count, 2);

    let outcome = runtime.drain_runtime_messages();
    assert_eq!(outcome.messages_dispatched, 1);
    assert_eq!(runtime.focused_widget(), Some(42));
    assert_eq!(runtime.bridge().project_count, 3);
}

#[test]
fn deferred_scroll_to_refreshes_before_dispatch_when_surface_is_dirty() {
    let mut runtime =
        SurfaceRuntime::new(DeferredScrollBridge::default(), Vector2::new(120.0, 40.0));
    assert_eq!(runtime.bridge().project_count, 1);

    let mut outcome = CommandOutcome {
        surface_refresh_requested: true,
        ..CommandOutcome::default()
    };
    runtime.execute_command_inner_deferred_refresh(
        Command::scroll_to(10, Vector2::new(0.0, 30.0)),
        &mut outcome,
    );

    assert_eq!(
        runtime.bridge().project_count,
        2,
        "deferred ScrollTo should refresh stale projected layout before clamping offsets"
    );
}

#[test]
fn deferred_scroll_updated_command_refreshes_before_focus_followup() {
    let mut runtime = SurfaceRuntime::new(
        DeferredScrollFocusBridge::default(),
        Vector2::new(120.0, 40.0),
    );
    assert_eq!(runtime.bridge().project_count, 1);

    let scrolled =
        runtime.scroll_at_with_refresh(Point::new(10.0, 10.0), Vector2::new(0.0, 30.0), false);

    assert!(scrolled);
    assert_eq!(runtime.bridge().scroll_updates, 1);
    assert_eq!(
        runtime.focused_widget(),
        Some(42),
        "deferred scroll-updated focus should see the widget revealed by the bridge hook"
    );
    assert_eq!(
        runtime.bridge().project_count,
        2,
        "deferred scroll-updated focus should refresh once before dispatching"
    );
    let pending = runtime.take_pending_input_command_outcome();
    assert!(pending.surface_refresh_requested);
}

#[test]
fn wheel_scroll_fallback_preserves_metadata_and_programmatic_scroll_defaults() {
    let mut runtime = SurfaceRuntime::new(
        DeferredScrollFocusBridge::default(),
        Vector2::new(120.0, 40.0),
    );
    let point = Point::new(10.0, 10.0);
    let delta = Vector2::new(0.0, 30.0);
    let modifiers = crate::widgets::PointerModifiers {
        shift: true,
        alt: true,
        ..crate::widgets::PointerModifiers::default()
    };
    let timestamp = Some(crate::gui::input::InputTimestamp::capture());
    let sequence_range = Some(crate::gui::input::InputSequenceRange::singleton(
        crate::gui::input::InputSequence::from_runtime_value(7),
    ));

    assert!(runtime.wheel_or_scroll_at_with_metadata(
        point,
        delta,
        modifiers,
        timestamp,
        sequence_range,
    ));
    let wheel_update = runtime
        .bridge()
        .last_scroll_update
        .expect("scroll-container fallback should report its update to the bridge");
    assert_eq!(wheel_update.metadata.modifiers, modifiers);
    assert_eq!(wheel_update.metadata.timestamp, timestamp);
    assert_eq!(wheel_update.metadata.sequence_range, sequence_range);

    assert!(runtime.scroll_at(point, delta));
    let programmatic_update = runtime
        .bridge()
        .last_scroll_update
        .expect("programmatic scroll should report its later update to the bridge");
    assert_eq!(runtime.bridge().scroll_updates, 2);
    assert_eq!(
        programmatic_update.metadata,
        crate::runtime::ScrollUpdateMetadata::default()
    );
}
