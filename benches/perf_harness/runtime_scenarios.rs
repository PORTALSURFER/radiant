//! Runtime surface performance scenarios.

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

pub(super) fn resize_large_tree() -> impl FnMut() -> crate::runner::ScenarioCounters {
    surface::resize_large_tree()
}

pub(super) fn animation_frame_cadence_1k() -> impl FnMut() -> crate::runner::ScenarioCounters {
    frame_cadence::animation_frame_cadence_1k()
}

pub(super) fn command_flattening_512() -> impl FnMut() -> crate::runner::ScenarioCounters {
    surface::command_flattening_512()
}
