use super::fixtures::{CommandDemoBridge, CommandDemoMessage};
use super::*;

#[test]
fn surface_runtime_treats_mixed_repaint_batches_as_surface_refreshes() {
    let bridge = CommandDemoBridge {
        state: DemoState::default(),
    };
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));

    let outcome = runtime.dispatch_message(CommandDemoMessage::MixedRepaint);

    assert!(outcome.repaint_requested);
    assert!(outcome.surface_repaint_requested);
    assert!(outcome.paint_only_requested);
    assert!(outcome.surface_refresh_requested);
    assert_eq!(
        runtime.last_refresh_diagnostics().invalidation,
        radiant::runtime::SurfaceInvalidation::Surface
    );
}

#[test]
fn surface_runtime_executes_command_messages_and_repaint_requests() {
    let bridge = CommandDemoBridge {
        state: DemoState::default(),
    };
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));

    let outcome = runtime.dispatch_message(CommandDemoMessage::Start);

    assert_eq!(outcome.messages_dispatched, 3);
    assert!(outcome.repaint_requested);
    assert!(outcome.surface_refresh_requested);
    assert!(runtime.repaint_requested());
    assert!(runtime.take_repaint_requested());
    assert!(!runtime.repaint_requested());

    assert_eq!(
        widget_ref::<TextWidget, _>(runtime.surface(), 10, "text").text,
        "Commands (1)"
    );
    assert_eq!(
        widget_ref::<TextInputWidget, _>(runtime.surface(), 12, "text input")
            .state
            .value,
        "Commands"
    );
}

#[test]
fn direct_typed_refresh_commands_apply_the_requested_stage() {
    let bridge = CommandDemoBridge {
        state: DemoState::default(),
    };
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));
    let before_projection = runtime.refresh_counters();

    let projection = runtime.execute_command(Command::repaint(RepaintScope::Projection));

    assert!(projection.surface_refresh_requested);
    assert_eq!(
        projection.surface_invalidation(),
        radiant::runtime::SurfaceInvalidation::Projection
    );
    assert_eq!(
        runtime.refresh_counters().layout,
        before_projection.layout,
        "projection-only commands must reuse layout"
    );

    let layout = runtime.execute_command(Command::repaint(RepaintScope::Layout));

    assert!(layout.surface_refresh_requested);
    assert_eq!(
        layout.surface_invalidation(),
        radiant::runtime::SurfaceInvalidation::Layout
    );
    assert_eq!(
        runtime.refresh_counters().layout,
        before_projection.layout + 1,
        "layout commands must run exactly one layout pass"
    );
}

#[test]
fn narrower_eager_refresh_does_not_consume_broader_pending_refresh() {
    let bridge = CommandDemoBridge {
        state: DemoState::default(),
    };
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));
    let before = runtime.refresh_counters();

    let outcome = runtime.execute_command(Command::batch([
        Command::repaint(RepaintScope::Layout),
        Command::message(CommandDemoMessage::ProjectionRefresh),
    ]));

    assert_eq!(
        outcome.surface_invalidation(),
        radiant::runtime::SurfaceInvalidation::Layout
    );
    assert!(!outcome.surface_refresh_applied);
    let after = runtime.refresh_counters();
    assert_eq!(
        after.application_projection,
        before.application_projection + 2
    );
    assert_eq!(after.runtime_projection, before.runtime_projection + 2);
    assert_eq!(after.widget_state_sync, before.widget_state_sync + 2);
    assert_eq!(after.layout, before.layout + 1);
    assert_eq!(
        runtime.last_refresh_diagnostics().invalidation,
        radiant::runtime::SurfaceInvalidation::Layout
    );
}
