//! Registered performance scenarios for the standalone Radiant harness.

use crate::{
    app_projection, bench_gpu_custom_shader_projection, bench_gpu_signal_summary,
    bench_gpu_surface_projection, command_drain, layout_scenarios, resource_scenarios,
    runner::ScenarioRunner, runtime_scenarios, text_scenarios,
};

const LAYOUT_ITERATIONS: usize = 120;
const RUNTIME_ITERATIONS: usize = 100;
const GPU_ITERATIONS: usize = crate::GPU_ITERATIONS;

macro_rules! perf_scenario_catalog {
    ($apply:ident $($prefix:tt)*) => {
        $apply! {
            $($prefix)*
            [
            ("layout_deep_nesting", LAYOUT_ITERATIONS, layout_scenarios::deep_nesting),
            ("layout_wrap_1k", LAYOUT_ITERATIONS, layout_scenarios::wrap_1k),
            ("layout_virtualized_10k", LAYOUT_ITERATIONS, layout_scenarios::virtualized_10k),
            ("layout_virtualized_fixed_10k", LAYOUT_ITERATIONS, layout_scenarios::virtualized_fixed_10k),
            ("layout_virtualized_fixed_scroll_10k", LAYOUT_ITERATIONS, layout_scenarios::virtualized_fixed_scroll_10k),
            ("layout_mark_dirty_subtree_10k", LAYOUT_ITERATIONS, layout_scenarios::mark_dirty_subtree_10k),
            ("app_virtual_list_projection_10k", RUNTIME_ITERATIONS, app_projection::virtual_list_projection_10k),
            ("app_virtual_list_projection_generated_child_ids_10k", RUNTIME_ITERATIONS, app_projection::virtual_list_projection_generated_child_ids_10k),
            ("app_virtual_selectable_list_projection_10k", RUNTIME_ITERATIONS, app_projection::virtual_selectable_list_projection_10k),
            ("app_virtual_list_window_projection_10k", RUNTIME_ITERATIONS, app_projection::virtual_list_window_projection_10k),
            ("runtime_surface_large_tree", RUNTIME_ITERATIONS, runtime_scenarios::surface_large_tree),
            ("runtime_text_paint_plan_1k", RUNTIME_ITERATIONS, runtime_scenarios::text_paint_plan_1k),
            ("runtime_horizontal_scroll_paint_1k", RUNTIME_ITERATIONS, runtime_scenarios::horizontal_scroll_paint_1k),
            ("runtime_virtualized_list_wheel_10k", RUNTIME_ITERATIONS, runtime_scenarios::virtualized_list_wheel_10k),
            ("runtime_virtualized_list_hover_10k", RUNTIME_ITERATIONS, runtime_scenarios::virtualized_list_hover_10k),
            ("runtime_virtualized_list_stable_hover_10k", RUNTIME_ITERATIONS, runtime_scenarios::virtualized_list_stable_hover_10k),
            ("runtime_virtualized_list_hover_paint_10k", RUNTIME_ITERATIONS, runtime_scenarios::virtualized_list_hover_paint_10k),
            ("runtime_pointer_overlay_paint_10k", RUNTIME_ITERATIONS, runtime_scenarios::pointer_overlay_paint_10k),
            ("runtime_virtualized_nested_scroll_hover_10k", RUNTIME_ITERATIONS, runtime_scenarios::virtualized_nested_scroll_hover_10k),
            ("runtime_refresh_large_tree", RUNTIME_ITERATIONS, runtime_scenarios::refresh_large_tree),
            ("runtime_resize_large_tree", RUNTIME_ITERATIONS, runtime_scenarios::resize_large_tree),
            ("runtime_command_flattening_512", RUNTIME_ITERATIONS, runtime_scenarios::command_flattening_512),
            ("runtime_command_drain_1k", RUNTIME_ITERATIONS, command_drain::flat_command_drain),
            ("runtime_nested_command_drain_1k", RUNTIME_ITERATIONS, command_drain::nested_command_drain),
            ("resource_slot_stale_completions_1k", RUNTIME_ITERATIONS, resource_scenarios::resource_slot_stale_completions_1k),
            ("text_line_cache_1k", RUNTIME_ITERATIONS, text_scenarios::text_line_cache_1k),
            ("text_word_selection_1k", RUNTIME_ITERATIONS, text_scenarios::text_word_selection_1k),
            ("gpu_signal_summary", GPU_ITERATIONS, || bench_gpu_signal_summary),
            ("gpu_surface_projection", GPU_ITERATIONS, || bench_gpu_surface_projection),
            ("gpu_custom_shader_projection", GPU_ITERATIONS, || bench_gpu_custom_shader_projection),
            ]
        }
    };
}

macro_rules! scenario_names {
    ([$(($name:literal, $iterations:expr, $build:expr)),+ $(,)?]) => {
        &[$($name),+]
    };
}

macro_rules! run_registered_scenario_entries {
    ($runner:ident [$(($name:literal, $iterations:expr, $build:expr)),+ $(,)?]) => {
        $($runner.run_scenario($name, $iterations, $build);)+
    };
}

pub(super) const PERF_SCENARIOS: &[&str] = perf_scenario_catalog!(scenario_names);

pub(super) fn run_registered_scenarios(runner: &mut ScenarioRunner) {
    perf_scenario_catalog!(run_registered_scenario_entries runner);
}
