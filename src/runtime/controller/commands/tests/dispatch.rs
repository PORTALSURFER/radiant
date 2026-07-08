use super::{
    super::*,
    fixtures::{
        DeferredFocusBridge, DeferredPlatformFallbackBridge, DeferredScrollBridge,
        DeferredScrollFocusBridge,
    },
};
use crate::runtime::{FileDialogRequest, PlatformRequest};

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

    assert_eq!(
        runtime.focused_widget(),
        Some(43),
        "layout-dependent commands after a fallback completion should see its projection changes"
    );
    assert_eq!(
        runtime.bridge().project_count,
        3,
        "the fallback message should dirty the shared deferred freshness state"
    );
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
