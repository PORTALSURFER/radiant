//! Runtime surface performance scenarios.

use radiant::application::IntoView;

#[path = "runtime_scenarios/arrangement_shell.rs"]
mod arrangement_shell;
#[path = "runtime_scenarios/frame_cadence.rs"]
mod frame_cadence;
#[path = "runtime_scenarios/invalidation.rs"]
mod invalidation;
#[path = "runtime_scenarios/pointer_overlay.rs"]
mod pointer_overlay;
#[path = "runtime_scenarios/surface.rs"]
mod surface;
#[path = "runtime_scenarios/virtualized.rs"]
mod virtualized;

pub(super) fn surface_large_tree() -> impl FnMut() -> crate::runner::ScenarioCounters {
    surface::surface_large_tree()
}

pub(super) fn text_paint_plan_1k() -> impl FnMut() -> crate::runner::ScenarioCounters {
    surface::text_paint_plan_1k()
}

pub(super) fn horizontal_scroll_paint_1k() -> impl FnMut() -> crate::runner::ScenarioCounters {
    surface::horizontal_scroll_paint_1k()
}

pub(super) fn virtualized_list_wheel_10k() -> impl FnMut() -> crate::runner::ScenarioCounters {
    virtualized::virtualized_list_wheel_10k()
}

pub(super) fn virtualized_list_hover_10k() -> impl FnMut() -> crate::runner::ScenarioCounters {
    virtualized::virtualized_list_hover_10k()
}

pub(super) fn virtualized_list_stable_hover_10k() -> impl FnMut() -> crate::runner::ScenarioCounters
{
    virtualized::virtualized_list_stable_hover_10k()
}

pub(super) fn virtualized_list_hover_paint_10k() -> impl FnMut() -> crate::runner::ScenarioCounters
{
    virtualized::virtualized_list_hover_paint_10k()
}

pub(super) fn pointer_overlay_paint_10k() -> impl FnMut() -> crate::runner::ScenarioCounters {
    pointer_overlay::pointer_overlay_paint_10k()
}

pub(super) fn retained_segment_invalidation_1k() -> impl FnMut() -> crate::runner::ScenarioCounters
{
    invalidation::retained_segment_invalidation_1k()
}

pub(super) fn virtualized_nested_scroll_hover_10k()
-> impl FnMut() -> crate::runner::ScenarioCounters {
    virtualized::virtualized_nested_scroll_hover_10k()
}

pub(super) fn refresh_large_tree() -> impl FnMut() -> crate::runner::ScenarioCounters {
    surface::refresh_large_tree()
}

pub(super) fn projection_refresh_large_tree() -> impl FnMut() -> crate::runner::ScenarioCounters {
    surface::projection_refresh_large_tree()
}

pub(super) fn layout_reuse_large_cohort_3k() -> impl FnMut() -> crate::runner::ScenarioCounters {
    surface::layout_reuse_large_cohort_3k()
}

pub(super) fn resize_large_tree() -> impl FnMut() -> crate::runner::ScenarioCounters {
    surface::resize_large_tree()
}

pub(super) fn animation_frame_cadence_1k() -> impl FnMut() -> crate::runner::ScenarioCounters {
    frame_cadence::animation_frame_cadence_1k()
}

pub(super) fn arrangement_shell_frame_refresh() -> impl FnMut() -> crate::runner::ScenarioCounters {
    arrangement_shell::frame_refresh()
}

pub(super) fn arrangement_shell_structural_toggle()
-> impl FnMut() -> crate::runner::ScenarioCounters {
    arrangement_shell::structural_toggle()
}

pub(super) fn arrangement_shell_hover_paint_only() -> impl FnMut() -> crate::runner::ScenarioCounters
{
    arrangement_shell::hover_paint_only()
}

pub(super) fn command_flattening_512() -> impl FnMut() -> crate::runner::ScenarioCounters {
    surface::command_flattening_512()
}

/// Clone and release one retained 10k-leaf surface without layout or paint.
pub(super) fn surface_tree_clone_10k() -> impl FnMut() -> crate::runner::ScenarioCounters {
    let surface = radiant::application::column(
        (0..10_000)
            .map(|index| radiant::application::text::<()>(format!("Leaf {index}")).id(index + 1))
            .collect::<Vec<_>>(),
    )
    .id(20_000)
    .into_surface();
    move || {
        std::hint::black_box(surface.clone());
        crate::runner::ScenarioCounters::default()
    }
}

#[path = "runtime_scenarios/components.rs"]
mod components;

pub(super) fn component_projection_cached() -> impl FnMut() -> crate::runner::ScenarioCounters {
    components::projection(false)
}

pub(super) fn component_projection_fresh() -> impl FnMut() -> crate::runner::ScenarioCounters {
    components::projection(true)
}

pub(super) fn component_local_geometry() -> impl FnMut() -> crate::runner::ScenarioCounters {
    components::local_geometry()
}
