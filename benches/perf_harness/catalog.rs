//! Registered performance scenarios for the standalone Radiant harness.

use crate::{
    app_projection, bench_gpu_custom_shader_projection, bench_gpu_signal_summary,
    bench_gpu_surface_projection, bench_gpu_surface_stack_projection_128, command_drain,
    layout_scenarios, resource_scenarios,
    runner::{ScenarioRunner, ScenarioSpec},
    runtime_scenarios, text_scenarios,
};

const LAYOUT_ITERATIONS: usize = 120;
const RUNTIME_ITERATIONS: usize = 100;
const GPU_ITERATIONS: usize = crate::GPU_ITERATIONS;

const NO_COUNTERS: &[&str] = &[];
const POINTER_COUNTERS: &[&str] = &[
    "scene_rebuild_count",
    "paint_only_count",
    "overlay_paint_count",
    "paint_primitive_count",
];
const VIRTUAL_LIST_COUNTERS: &[&str] = &[
    "scene_rebuild_count",
    "surface_refresh_count",
    "paint_primitive_count",
    "allocation_sensitive_work_count",
];
const SCENE_CACHE_COUNTERS: &[&str] = &[
    "scene_rebuild_count",
    "surface_refresh_count",
    "paint_only_count",
    "overlay_paint_count",
    "paint_primitive_count",
];
const TEXT_PAINT_COUNTERS: &[&str] = &["paint_primitive_count"];
const TEXT_CACHE_COUNTERS: &[&str] = &["text_cache_hit_count"];
const TEXT_EDIT_COUNTERS: &[&str] = &["allocation_sensitive_work_count"];
const GPU_SURFACE_COUNTERS: &[&str] = &[
    "gpu_surface_count",
    "retained_surface_cache_hit_count",
    "paint_primitive_count",
];
const GPU_DATA_COUNTERS: &[&str] = &["allocation_sensitive_work_count"];
const FRAME_CADENCE_COUNTERS: &[&str] = &[
    "paint_only_count",
    "frame_cadence_due_count",
    "frame_cadence_wait_count",
    "allocation_sensitive_work_count",
];
const RESIZE_COUNTERS: &[&str] = &["relayout_count"];
const COMMAND_COUNTERS: &[&str] = &["paint_only_count", "allocation_sensitive_work_count"];
const COMMAND_DRAIN_COUNTERS: &[&str] = &["allocation_sensitive_work_count"];

macro_rules! perf_scenario_catalog {
    ($apply:ident $($prefix:tt)*) => {
        $apply! {
            $($prefix)*
            [
            ("layout_deep_nesting", "layout", "general_layout", NO_COUNTERS, LAYOUT_ITERATIONS, layout_scenarios::deep_nesting),
            ("layout_wrap_1k", "layout", "text_layout", NO_COUNTERS, LAYOUT_ITERATIONS, layout_scenarios::wrap_1k),
            ("layout_virtualized_10k", "layout", "virtual_lists", VIRTUAL_LIST_COUNTERS, LAYOUT_ITERATIONS, layout_scenarios::virtualized_10k),
            ("layout_virtualized_fixed_10k", "layout", "virtual_lists", VIRTUAL_LIST_COUNTERS, LAYOUT_ITERATIONS, layout_scenarios::virtualized_fixed_10k),
            ("layout_virtualized_fixed_scroll_10k", "layout", "virtual_lists", VIRTUAL_LIST_COUNTERS, LAYOUT_ITERATIONS, layout_scenarios::virtualized_fixed_scroll_10k),
            ("layout_mark_dirty_subtree_10k", "layout", "scene_cache", SCENE_CACHE_COUNTERS, LAYOUT_ITERATIONS, layout_scenarios::mark_dirty_subtree_10k),
            ("layout_dirty_virtual_cache_10k", "layout", "scene_cache", SCENE_CACHE_COUNTERS, LAYOUT_ITERATIONS, layout_scenarios::dirty_virtual_cache_10k),
            ("app_virtual_list_projection_10k", "application_projection", "virtual_lists", VIRTUAL_LIST_COUNTERS, RUNTIME_ITERATIONS, app_projection::virtual_list_projection_10k),
            ("app_virtual_list_projection_generated_child_ids_10k", "application_projection", "virtual_lists", VIRTUAL_LIST_COUNTERS, RUNTIME_ITERATIONS, app_projection::virtual_list_projection_generated_child_ids_10k),
            ("app_virtual_selectable_list_projection_10k", "application_projection", "virtual_lists", VIRTUAL_LIST_COUNTERS, RUNTIME_ITERATIONS, app_projection::virtual_selectable_list_projection_10k),
            ("app_virtual_list_window_projection_10k", "application_projection", "virtual_lists", VIRTUAL_LIST_COUNTERS, RUNTIME_ITERATIONS, app_projection::virtual_list_window_projection_10k),
            ("runtime_surface_large_tree", "runtime_surface", "scene_cache", SCENE_CACHE_COUNTERS, RUNTIME_ITERATIONS, runtime_scenarios::surface_large_tree),
            ("runtime_text_paint_plan_1k", "runtime_surface", "text_layout", TEXT_PAINT_COUNTERS, RUNTIME_ITERATIONS, runtime_scenarios::text_paint_plan_1k),
            ("runtime_horizontal_scroll_paint_1k", "runtime_surface", "text_layout", TEXT_PAINT_COUNTERS, RUNTIME_ITERATIONS, runtime_scenarios::horizontal_scroll_paint_1k),
            ("runtime_virtualized_list_wheel_10k", "runtime_virtualized", "virtual_lists", VIRTUAL_LIST_COUNTERS, RUNTIME_ITERATIONS, runtime_scenarios::virtualized_list_wheel_10k),
            ("runtime_virtualized_list_hover_10k", "runtime_virtualized", "pointer_motion", POINTER_COUNTERS, RUNTIME_ITERATIONS, runtime_scenarios::virtualized_list_hover_10k),
            ("runtime_virtualized_list_stable_hover_10k", "runtime_virtualized", "pointer_motion", POINTER_COUNTERS, RUNTIME_ITERATIONS, runtime_scenarios::virtualized_list_stable_hover_10k),
            ("runtime_virtualized_list_hover_paint_10k", "runtime_virtualized", "virtual_lists", VIRTUAL_LIST_COUNTERS, RUNTIME_ITERATIONS, runtime_scenarios::virtualized_list_hover_paint_10k),
            ("runtime_pointer_overlay_paint_10k", "runtime_surface", "pointer_motion", POINTER_COUNTERS, RUNTIME_ITERATIONS, runtime_scenarios::pointer_overlay_paint_10k),
            ("runtime_retained_segment_invalidation_1k", "runtime_invalidation", "scene_cache", SCENE_CACHE_COUNTERS, RUNTIME_ITERATIONS, runtime_scenarios::retained_segment_invalidation_1k),
            ("runtime_virtualized_nested_scroll_hover_10k", "runtime_virtualized", "virtual_lists", VIRTUAL_LIST_COUNTERS, RUNTIME_ITERATIONS, runtime_scenarios::virtualized_nested_scroll_hover_10k),
            ("runtime_refresh_large_tree", "runtime_surface", "scene_cache", SCENE_CACHE_COUNTERS, RUNTIME_ITERATIONS, runtime_scenarios::refresh_large_tree),
            ("runtime_resize_large_tree", "runtime_surface", "frame_cadence", RESIZE_COUNTERS, RUNTIME_ITERATIONS, runtime_scenarios::resize_large_tree),
            ("runtime_animation_frame_cadence_1k", "runtime_surface", "frame_cadence", FRAME_CADENCE_COUNTERS, RUNTIME_ITERATIONS, runtime_scenarios::animation_frame_cadence_1k),
            ("runtime_command_flattening_512", "runtime_commands", "runtime_commands", COMMAND_COUNTERS, RUNTIME_ITERATIONS, runtime_scenarios::command_flattening_512),
            ("runtime_command_drain_1k", "runtime_commands", "runtime_commands", COMMAND_DRAIN_COUNTERS, RUNTIME_ITERATIONS, command_drain::flat_command_drain),
            ("runtime_nested_command_drain_1k", "runtime_commands", "runtime_commands", COMMAND_DRAIN_COUNTERS, RUNTIME_ITERATIONS, command_drain::nested_command_drain),
            ("resource_slot_stale_completions_1k", "resource_lifecycle", "resource_lifecycle", NO_COUNTERS, RUNTIME_ITERATIONS, resource_scenarios::resource_slot_stale_completions_1k),
            ("text_line_cache_1k", "text", "text_layout", TEXT_CACHE_COUNTERS, RUNTIME_ITERATIONS, text_scenarios::text_line_cache_1k),
            ("text_word_selection_1k", "text", "text_layout", TEXT_EDIT_COUNTERS, RUNTIME_ITERATIONS, text_scenarios::text_word_selection_1k),
            ("text_word_deletion_1k", "text", "text_layout", TEXT_EDIT_COUNTERS, RUNTIME_ITERATIONS, text_scenarios::text_word_deletion_1k),
            ("gpu_signal_summary", "gpu_data", "retained_gpu_surfaces", GPU_DATA_COUNTERS, GPU_ITERATIONS, || bench_gpu_signal_summary),
            ("gpu_surface_projection", "gpu_surface", "retained_gpu_surfaces", GPU_SURFACE_COUNTERS, GPU_ITERATIONS, || bench_gpu_surface_projection),
            ("gpu_surface_stack_projection_128", "gpu_surface", "retained_gpu_surfaces", GPU_SURFACE_COUNTERS, GPU_ITERATIONS, || bench_gpu_surface_stack_projection_128),
            ("gpu_custom_shader_projection", "gpu_surface", "retained_gpu_surfaces", GPU_SURFACE_COUNTERS, GPU_ITERATIONS, || bench_gpu_custom_shader_projection),
            ]
        }
    };
}

macro_rules! scenario_specs {
    ([$(($name:literal, $category:literal, $group:literal, $counters:expr, $iterations:expr, $build:expr)),+ $(,)?]) => {
        &[$(ScenarioSpec::new($name, $category, $group, $counters, $iterations)),+]
    };
}

macro_rules! run_registered_scenario_entries {
    ($runner:ident [$(($name:literal, $category:literal, $group:literal, $counters:expr, $iterations:expr, $build:expr)),+ $(,)?]) => {
        $($runner.run_scenario($name, $category, $group, $iterations, $build);)+
    };
}

pub(super) const PERF_SCENARIOS: &[ScenarioSpec] = perf_scenario_catalog!(scenario_specs);

pub(super) fn run_registered_scenarios(runner: &mut ScenarioRunner) {
    perf_scenario_catalog!(run_registered_scenario_entries runner);
}
