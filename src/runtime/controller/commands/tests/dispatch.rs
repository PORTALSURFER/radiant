use super::{super::*, fixtures::DeferredFocusBridge};

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
