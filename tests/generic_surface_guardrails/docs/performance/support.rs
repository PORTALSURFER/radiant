use std::{fs, path::PathBuf};

pub(super) const PERF_SCENARIOS: &[&str] = &[
    "layout_deep_nesting",
    "layout_wrap_1k",
    "layout_virtualized_10k",
    "layout_virtualized_fixed_10k",
    "layout_virtualized_fixed_scroll_10k",
    "layout_mark_dirty_subtree_10k",
    "app_virtual_list_projection_10k",
    "app_virtual_list_projection_generated_child_ids_10k",
    "app_virtual_selectable_list_projection_10k",
    "app_virtual_list_window_projection_10k",
    "runtime_surface_large_tree",
    "runtime_text_paint_plan_1k",
    "runtime_horizontal_scroll_paint_1k",
    "runtime_virtualized_list_wheel_10k",
    "runtime_virtualized_list_hover_10k",
    "runtime_virtualized_list_stable_hover_10k",
    "runtime_virtualized_list_hover_paint_10k",
    "runtime_pointer_overlay_paint_10k",
    "runtime_retained_segment_invalidation_1k",
    "runtime_virtualized_nested_scroll_hover_10k",
    "runtime_refresh_large_tree",
    "runtime_resize_large_tree",
    "runtime_command_flattening_512",
    "runtime_command_drain_1k",
    "runtime_nested_command_drain_1k",
    "resource_slot_stale_completions_1k",
    "text_line_cache_1k",
    "text_word_selection_1k",
    "text_word_deletion_1k",
    "gpu_signal_summary",
    "gpu_surface_projection",
    "gpu_surface_stack_projection_128",
    "gpu_custom_shader_projection",
];

pub(super) fn read_project_file(relative_path: &str) -> String {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(manifest_dir.join(relative_path))
        .unwrap_or_else(|error| panic!("{relative_path} should be readable: {error}"))
}

pub(super) fn normalized_words(source: &str) -> String {
    source.split_whitespace().collect::<Vec<_>>().join(" ")
}
