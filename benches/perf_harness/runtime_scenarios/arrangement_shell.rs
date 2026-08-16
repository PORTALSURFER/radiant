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
    runtime::{PaintPrimitive, RuntimeBridge, SurfacePaintPlan, SurfaceRuntime},
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
    let theme = ThemeTokens::default();
    let bounds = runtime.layout().rects[&ARRANGEMENT_WIDGET_ID];
    let positions = [
        Point::new(bounds.min.x + 280.0, bounds.center().y),
        Point::new(bounds.min.x + 300.0, bounds.center().y),
    ];
    assert!(positions.iter().all(|position| bounds.contains(*position)));
    assert!(
        positions
            .iter()
            .all(|position| runtime.widget_at(*position) == Some(ARRANGEMENT_WIDGET_ID))
    );

    let first = runtime.dispatch_pointer_move_with_outcome(positions[0]);
    assert_eq!(first.target, Some(ARRANGEMENT_WIDGET_ID));
    assert!(first.routed());
    assert!(first.needs_scene_rebuild());
    assert_eq!(runtime.hovered_widget(), Some(ARRANGEMENT_WIDGET_ID));

    let mut base_plan = SurfacePaintPlan::empty(&theme);
    runtime.base_paint_plan_into(&theme, &mut base_plan);
    assert!(!base_plan.primitives.is_empty());

    let mut overlay = Vec::new();
    runtime.runtime_overlay_paint_into(&theme, &mut overlay);
    assert!(!overlay.is_empty());
    overlay.clear();
    let mut next_position = 1;
    move || {
        hover_paint_only_step(
            &mut runtime,
            &theme,
            &positions,
            &mut next_position,
            &base_plan,
            &mut overlay,
        )
    }
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
    theme: &ThemeTokens,
    positions: &[Point; 2],
    next_position: &mut usize,
    base_plan: &SurfacePaintPlan,
    overlay: &mut Vec<PaintPrimitive>,
) -> ScenarioCounters
where
    Bridge: RuntimeBridge<AppMessage>,
{
    let owner_before = runtime.hovered_widget();
    assert_eq!(owner_before, Some(ARRANGEMENT_WIDGET_ID));
    let before = runtime.refresh_counters();
    let outcome = runtime.dispatch_pointer_move_with_outcome(positions[*next_position]);
    assert_eq!(outcome.target, Some(ARRANGEMENT_WIDGET_ID));
    assert!(outcome.routed());
    assert!(outcome.paint_only_requested);
    assert!(!outcome.hover_changed);
    assert!(!outcome.needs_scene_rebuild());
    assert_eq!(runtime.hovered_widget(), owner_before);

    overlay.clear();
    runtime.runtime_overlay_paint_into(theme, overlay);
    let after = runtime.refresh_counters();
    assert_eq!(after, before);
    assert!(!base_plan.primitives.is_empty());
    assert!(!overlay.is_empty());
    let overlay_primitive_count = overlay.len() as u64;
    black_box((base_plan, overlay));
    *next_position = (*next_position + 1) % positions.len();

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
#[allow(dead_code, unused_imports)]
mod tests {
    use super::{frame_refresh, hover_paint_only, structural_toggle};

    fn counter(counters: super::ScenarioCounters, name: &str) -> u64 {
        counters
            .iter()
            .find(|(counter, _)| *counter == name)
            .map(|(_, value)| value)
            .unwrap_or_else(|| panic!("missing scenario counter {name}"))
    }

    #[test]
    fn repeated_identical_lanes_have_identical_counter_deltas() {
        let mut frame = frame_refresh();
        assert_eq!(frame(), frame());

        let mut structural = structural_toggle();
        assert_eq!(structural(), structural());

        let mut hover = hover_paint_only();
        assert_eq!(hover(), hover());
    }

    #[test]
    fn hover_paint_only_reports_only_overlay_work_after_setup() {
        let mut hover = hover_paint_only();
        let first = hover();
        let second = hover();

        assert_eq!(first, second);
        for (name, expected) in [
            ("scene_rebuild_count", 0),
            ("paint_only_count", 1),
            ("surface_refresh_count", 0),
            ("application_projection_count", 0),
            ("runtime_projection_count", 0),
            ("widget_state_sync_count", 0),
            ("layout_count", 0),
            ("paint_plan_rebuild_count", 0),
            ("overlay_paint_count", 1),
        ] {
            assert_eq!(counter(first, name), expected, "counter {name}");
        }
        assert!(counter(first, "paint_primitive_count") > 0);
    }
}
