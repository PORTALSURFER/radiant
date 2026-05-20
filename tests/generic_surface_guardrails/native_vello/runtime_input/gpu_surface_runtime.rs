use super::*;

#[test]
fn native_gpu_surface_hover_cursor_policy_tests_stay_grouped_by_concern() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/tests/gpu_surface_runtime/hover_cursor_policy.rs",
    ))
    .expect("native GPU-surface hover cursor policy test root should be readable");
    let fixtures = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/tests/gpu_surface_runtime/hover_cursor_policy/fixtures.rs",
    ))
    .expect("native GPU-surface hover cursor fixtures should be readable");
    let lookup = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/tests/gpu_surface_runtime/hover_cursor_policy/lookup.rs",
    ))
    .expect("native GPU-surface hover cursor lookup tests should be readable");
    let overlay = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/tests/gpu_surface_runtime/hover_cursor_policy/overlay.rs",
    ))
    .expect("native GPU-surface hover cursor overlay tests should be readable");

    assert!(
        root.contains("mod fixtures;")
            && root.contains("mod lookup;")
            && root.contains("mod overlay;")
            && !root.contains("fn gpu_surface_lookup_skips_unrenderable_surface_content")
            && !root.contains("fn native_hover_cursor_updates_topmost_surface"),
        "native GPU-surface hover cursor policy root should index focused behavior groups instead of owning all cases"
    );
    assert!(
        fixtures.contains("fn hover_capabilities")
            && fixtures.contains("fn rgba_content")
            && lookup.contains("fn gpu_surface_lookup_skips_unrenderable_surface_content")
            && lookup.contains("fn gpu_surface_lookup_skips_nonfinite_surface_rects_and_positions")
            && overlay.contains("fn native_hover_cursor_clears_stale_overlay_for_invalid_geometry")
            && overlay.contains(
                "fn native_hover_cursor_updates_topmost_surface_and_clears_stale_cursors"
            ),
        "native GPU-surface hover cursor policy tests should stay grouped by fixtures, lookup filtering, and overlay mutation"
    );
}

#[test]
fn native_gpu_surface_hover_overlay_tests_stay_grouped_by_behavior() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/tests/gpu_surface_runtime/hover_overlay.rs",
    ))
    .expect("native GPU-surface hover overlay test root should be readable");
    let update = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/tests/gpu_surface_runtime/hover_overlay/update.rs",
    ))
    .expect("native GPU-surface hover overlay update tests should be readable");
    let clear = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/tests/gpu_surface_runtime/hover_overlay/clear.rs",
    ))
    .expect("native GPU-surface hover overlay clear tests should be readable");
    let rebuild = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/tests/gpu_surface_runtime/hover_overlay/rebuild.rs",
    ))
    .expect("native GPU-surface hover overlay rebuild tests should be readable");

    assert!(
        root.contains("mod update;")
            && root.contains("mod clear;")
            && root.contains("mod rebuild;")
            && !root.contains("fn native_gpu_hover_updates_cached_overlay")
            && !root.contains("fn native_gpu_hover_clear_hides_cached_cursor"),
        "native GPU-surface hover overlay root should index focused behavior groups instead of owning all cases"
    );
    assert!(
        update.contains("fn native_gpu_hover_updates_cached_overlay_without_refreshing_surface")
            && update.contains("fn native_gpu_hover_skips_unchanged_cached_overlay")
            && update.contains("fn native_gpu_hover_collapses_duplicate_cursor_overlays")
            && clear.contains("fn native_gpu_hover_preserves_app_owned_vertical_overlays")
            && clear.contains("fn native_gpu_hover_clear_hides_cached_cursor_without_rebuild")
            && rebuild.contains("fn native_gpu_hover_survives_scene_rebuilds"),
        "native GPU-surface hover overlay tests should stay grouped by update, clear, and rebuild behavior"
    );
}

#[test]
fn native_gpu_surface_fast_path_tests_stay_grouped_by_behavior() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/tests/gpu_surface_runtime/fast_paths.rs",
    ))
    .expect("native GPU-surface fast-path test root should be readable");
    let wheel = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/tests/gpu_surface_runtime/fast_paths/wheel.rs",
    ))
    .expect("native GPU-surface wheel fast-path tests should be readable");
    let pointer_move = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/tests/gpu_surface_runtime/fast_paths/pointer_move.rs",
    ))
    .expect("native GPU-surface pointer-move fast-path tests should be readable");
    let hover = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/tests/gpu_surface_runtime/fast_paths/hover.rs",
    ))
    .expect("native GPU-surface hover fast-path tests should be readable");
    let plain_surface = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/tests/gpu_surface_runtime/fast_paths/plain_surface.rs",
    ))
    .expect("plain GPU-surface fast-path tests should be readable");
    let paint_only = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/tests/gpu_surface_runtime/fast_paths/paint_only.rs",
    ))
    .expect("paint-only pointer fast-path tests should be readable");

    assert!(
        root.contains("mod wheel;")
            && root.contains("mod pointer_move;")
            && root.contains("mod hover;")
            && root.contains("mod plain_surface;")
            && root.contains("mod paint_only;")
            && !root.contains("fn gpu_surface_fast_path_does_not_capture_horizontal_pan")
            && !root.contains("struct PaintOnlyPointerBridge"),
        "native GPU-surface fast-path test root should index focused behavior groups instead of owning all cases and fixtures"
    );
    assert!(
        wheel.contains("fn gpu_surface_fast_path_does_not_capture_horizontal_pan")
            && pointer_move
                .contains("fn gpu_surface_pointer_move_fast_path_only_within_cached_surface")
            && pointer_move.contains(
                "fn gpu_surface_pointer_move_fast_path_is_disabled_during_pointer_capture"
            )
            && hover.contains("fn native_gpu_hover_fast_path_is_disabled_during_pointer_capture")
            && plain_surface.contains("fn plain_gpu_surface_does_not_opt_into_runtime_fast_paths")
            && paint_only.contains("fn native_routed_paint_only_pointer_move_skips_scene_rebuild")
            && paint_only.contains("struct PaintOnlyPointerBridge"),
        "native GPU-surface fast-path tests should stay grouped by wheel, pointer move, hover, plain surface, and paint-only behavior"
    );
}
