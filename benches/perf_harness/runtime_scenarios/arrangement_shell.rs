//! Measured runtime lanes for the maintained standalone arrangement-shell example.

use crate::{
    arrangement_shell::{
        ARRANGEMENT_WIDGET_ID, AppMessage, ArrangementShellState, ShellMessage, project_surface,
        update,
    },
    runner::ScenarioCounters,
};
use radiant::{
    gui::types::{Point, Vector2},
    runtime::{RuntimeBridge, SurfaceRuntime},
    theme::ThemeTokens,
};
use std::hint::black_box;

fn viewport() -> Vector2 {
    Vector2::new(1_180.0, 700.0)
}

pub(super) fn frame_refresh() -> impl FnMut() -> ScenarioCounters {
    let mut runtime = arrangement_shell_runtime();
    move || frame_refresh_step(&mut runtime)
}

pub(super) fn structural_toggle() -> impl FnMut() -> ScenarioCounters {
    let mut runtime = arrangement_shell_runtime();
    move || structural_toggle_step(&mut runtime)
}

pub(super) fn hover_paint_only() -> impl FnMut() -> ScenarioCounters {
    let mut runtime = arrangement_shell_runtime();
    move || hover_paint_only_step(&mut runtime)
}

fn arrangement_shell_runtime() -> SurfaceRuntime<impl RuntimeBridge<AppMessage>, AppMessage> {
    let bridge = radiant::app(ArrangementShellState::default())
        .view(project_surface)
        .animation(|state| state.running)
        .on_frame(|| AppMessage::Frame)
        .update(update)
        .into_bridge();
    SurfaceRuntime::new(bridge, viewport())
}

fn frame_refresh_step<Bridge>(runtime: &mut SurfaceRuntime<Bridge, AppMessage>) -> ScenarioCounters
where
    Bridge: RuntimeBridge<AppMessage>,
{
    let before = runtime.refresh_counters();
    assert!(runtime.host_queue_animation_frame());
    let outcome = runtime.drain_runtime_messages();
    assert_eq!(outcome.messages_dispatched, 1);
    let plan = runtime.paint_plan(&ThemeTokens::default());
    let after = runtime.refresh_counters();
    let delta = refresh_delta(before, after);

    assert_eq!(delta.application_projection, 1);
    assert_eq!(delta.runtime_projection, 1);
    assert_eq!(delta.widget_state_sync, 1);
    assert_eq!(delta.layout, 1);
    assert!(!plan.primitives.is_empty());
    let paint_primitive_count = plan.primitives.len() as u64;
    black_box(plan);

    ScenarioCounters::default()
        .with_scene_rebuild_count(1)
        .with_surface_refresh_count(1)
        .with_application_projection_count(delta.application_projection)
        .with_runtime_projection_count(delta.runtime_projection)
        .with_widget_state_sync_count(delta.widget_state_sync)
        .with_layout_count(delta.layout)
        .with_paint_plan_rebuild_count(1)
        .with_paint_primitive_count(paint_primitive_count)
}

fn structural_toggle_step<Bridge>(
    runtime: &mut SurfaceRuntime<Bridge, AppMessage>,
) -> ScenarioCounters
where
    Bridge: RuntimeBridge<AppMessage>,
{
    let before = runtime.refresh_counters();
    let outcome = runtime.dispatch_message(AppMessage::Shell(ShellMessage::ToggleBrowser));
    assert!(outcome.surface_refresh_requested);
    let plan = runtime.paint_plan(&ThemeTokens::default());
    let after = runtime.refresh_counters();
    let delta = refresh_delta(before, after);

    assert_eq!(delta.application_projection, 1);
    assert_eq!(delta.runtime_projection, 1);
    assert_eq!(delta.widget_state_sync, 1);
    assert_eq!(delta.layout, 1);
    assert!(!plan.primitives.is_empty());
    let paint_primitive_count = plan.primitives.len() as u64;
    black_box(plan);

    ScenarioCounters::default()
        .with_scene_rebuild_count(1)
        .with_surface_refresh_count(1)
        .with_application_projection_count(delta.application_projection)
        .with_runtime_projection_count(delta.runtime_projection)
        .with_widget_state_sync_count(delta.widget_state_sync)
        .with_layout_count(delta.layout)
        .with_paint_plan_rebuild_count(1)
        .with_paint_primitive_count(paint_primitive_count)
}

fn hover_paint_only_step<Bridge>(
    runtime: &mut SurfaceRuntime<Bridge, AppMessage>,
) -> ScenarioCounters
where
    Bridge: RuntimeBridge<AppMessage>,
{
    let bounds = runtime.layout().rects[&ARRANGEMENT_WIDGET_ID];
    let first = runtime
        .dispatch_pointer_move_with_outcome(Point::new(bounds.min.x + 160.0, bounds.center().y));
    assert!(first.routed());
    assert!(first.needs_scene_rebuild());

    let before = runtime.refresh_counters();
    let second = runtime
        .dispatch_pointer_move_with_outcome(Point::new(bounds.min.x + 280.0, bounds.center().y));
    assert!(second.routed());
    assert!(second.paint_only_requested);
    assert!(!second.needs_scene_rebuild());

    let plan = runtime.paint_plan(&ThemeTokens::default());
    let mut overlay = Vec::new();
    runtime.runtime_overlay_paint_into(&ThemeTokens::default(), &mut overlay);
    let after = runtime.refresh_counters();
    assert_eq!(after, before);
    assert!(!plan.primitives.is_empty());
    assert!(!overlay.is_empty());
    let overlay_primitive_count = overlay.len() as u64;
    black_box((plan, overlay));
    // Leave the retained example state in its pre-hover condition so every
    // measured iteration exercises the same first-hover transition.
    black_box(runtime.dispatch_pointer_move_with_outcome(Point::new(-1.0, -1.0)));

    ScenarioCounters::default()
        .with_scene_rebuild_count(0)
        .with_paint_only_count(1)
        .with_surface_refresh_count(0)
        .with_application_projection_count(0)
        .with_runtime_projection_count(0)
        .with_widget_state_sync_count(0)
        .with_layout_count(0)
        .with_paint_plan_rebuild_count(0)
        .with_overlay_paint_count(1)
        .with_paint_primitive_count(overlay_primitive_count)
}

fn refresh_delta(
    before: radiant::runtime::SurfaceRefreshCounters,
    after: radiant::runtime::SurfaceRefreshCounters,
) -> radiant::runtime::SurfaceRefreshCounters {
    radiant::runtime::SurfaceRefreshCounters {
        application_projection: after
            .application_projection
            .saturating_sub(before.application_projection),
        runtime_projection: after
            .runtime_projection
            .saturating_sub(before.runtime_projection),
        widget_state_sync: after
            .widget_state_sync
            .saturating_sub(before.widget_state_sync),
        layout: after.layout.saturating_sub(before.layout),
        base_paint_plan_rebuilds: after
            .base_paint_plan_rebuilds
            .saturating_sub(before.base_paint_plan_rebuilds),
    }
}

#[cfg(test)]
#[allow(unused_imports)]
mod tests {
    use super::{frame_refresh, hover_paint_only, structural_toggle};

    #[test]
    fn repeated_identical_lanes_have_identical_counter_deltas() {
        let mut frame = frame_refresh();
        assert_eq!(frame(), frame());

        let mut structural = structural_toggle();
        assert_eq!(structural(), structural());

        let mut hover = hover_paint_only();
        assert_eq!(hover(), hover());
    }
}
