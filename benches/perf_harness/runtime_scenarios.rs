//! Runtime surface performance scenarios.

#[path = "runtime_scenarios/pointer_overlay.rs"]
mod pointer_overlay;
#[path = "runtime_scenarios/surface.rs"]
mod surface;
#[path = "runtime_scenarios/virtualized.rs"]
mod virtualized;

pub(super) fn surface_large_tree() -> impl FnMut() {
    surface::surface_large_tree()
}

pub(super) fn text_paint_plan_1k() -> impl FnMut() {
    surface::text_paint_plan_1k()
}

pub(super) fn horizontal_scroll_paint_1k() -> impl FnMut() {
    surface::horizontal_scroll_paint_1k()
}

pub(super) fn virtualized_list_wheel_10k() -> impl FnMut() {
    virtualized::virtualized_list_wheel_10k()
}

pub(super) fn virtualized_list_hover_10k() -> impl FnMut() {
    virtualized::virtualized_list_hover_10k()
}

pub(super) fn virtualized_list_stable_hover_10k() -> impl FnMut() {
    virtualized::virtualized_list_stable_hover_10k()
}

pub(super) fn virtualized_list_hover_paint_10k() -> impl FnMut() {
    virtualized::virtualized_list_hover_paint_10k()
}

pub(super) fn pointer_overlay_paint_10k() -> impl FnMut() {
    pointer_overlay::pointer_overlay_paint_10k()
}

pub(super) fn virtualized_nested_scroll_hover_10k() -> impl FnMut() {
    virtualized::virtualized_nested_scroll_hover_10k()
}

pub(super) fn refresh_large_tree() -> impl FnMut() {
    surface::refresh_large_tree()
}

pub(super) fn resize_large_tree() -> impl FnMut() {
    surface::resize_large_tree()
}

pub(super) fn command_flattening_512() -> impl FnMut() {
    surface::command_flattening_512()
}
