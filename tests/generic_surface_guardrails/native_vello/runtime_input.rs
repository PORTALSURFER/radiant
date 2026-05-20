use super::*;

#[test]
fn native_event_routing_tests_stay_grouped_by_input_concern() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/tests/event_routing.rs"),
    )
    .expect("native event-routing test root should be readable");
    let host = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/tests/event_routing/host.rs"),
    )
    .expect("native host event-routing tests should be readable");
    let canvas = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/tests/event_routing/canvas.rs"),
    )
    .expect("native canvas event-routing tests should be readable");
    let scroll = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/tests/event_routing/scroll.rs"),
    )
    .expect("native scroll event-routing tests should be readable");
    let repaint = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/tests/event_routing/repaint.rs"),
    )
    .expect("native repaint event-routing tests should be readable");
    let drag_drop = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/tests/event_routing/drag_drop.rs"),
    )
    .expect("native drag/drop event-routing tests should be readable");

    assert!(
        root.contains("mod host;")
            && root.contains("mod canvas;")
            && root.contains("mod scroll;")
            && root.contains("mod repaint;")
            && root.contains("mod drag_drop;")
            && !root.contains("fn generic_core_routes_pointer_and_key_input")
            && !root.contains("struct DropBridge"),
        "native event-routing test root should index focused input groups instead of owning all event cases"
    );
    assert!(
        host.contains("fn generic_core_routes_text_edit_commands_only_to_text_inputs")
            && canvas.contains("fn generic_canvas_can_receive_keyboard_focus_and_text_input")
            && scroll
                .contains("fn scrollbar_drag_state_survives_view_refresh_after_offset_message")
            && repaint.contains("fn generic_core_drains_command_repaint_requests_after_routing")
            && drag_drop.contains("struct DropBridge")
            && drag_drop.contains("fn captured_drag_routes_pointer_move_to_hovered_drop_target"),
        "native event-routing tests should stay grouped by host, canvas, scroll, repaint, and drag/drop concerns"
    );
}

#[test]
fn native_runtime_test_fixtures_stay_grouped_by_runtime_concern() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/tests/fixtures.rs"),
    )
    .expect("native runtime fixture root should be readable");
    let demo = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/tests/fixtures/demo.rs"),
    )
    .expect("native runtime demo fixtures should be readable");
    let frame = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/tests/fixtures/frame.rs"),
    )
    .expect("native runtime frame fixtures should be readable");
    let input = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/tests/fixtures/input.rs"),
    )
    .expect("native runtime input fixtures should be readable");
    let gpu_wheel = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/tests/fixtures/gpu_wheel.rs"),
    )
    .expect("native runtime GPU wheel fixtures should be readable");

    assert!(
        root.contains("mod demo;")
            && root.contains("mod frame;")
            && root.contains("mod input;")
            && root.contains("mod gpu_wheel;")
            && !root.contains("struct TestGpuWheelWidget"),
        "native runtime fixture root should index focused fixture groups instead of owning all test support"
    );
    assert!(
        demo.contains("fn demo_surface")
            && frame.contains("struct PaintOnlyFrameBridge")
            && input.contains("struct WheelRefreshBridge")
            && gpu_wheel.contains("struct TestGpuWheelWidget"),
        "native runtime fixtures should stay grouped by demo, frame, input, and GPU wheel concerns"
    );
}

#[test]
fn native_generic_runtime_root_tests_stay_grouped_by_runtime_concern() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/tests.rs"),
    )
    .expect("native generic runtime test root should be readable");
    let runtime_core = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/tests/runtime_core.rs"),
    )
    .expect("native generic runtime core tests should be readable");
    let timing = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/tests/timing.rs"),
    )
    .expect("native generic runtime timing tests should be readable");
    let window_policy = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/tests/window_policy.rs"),
    )
    .expect("native generic runtime window policy tests should be readable");

    assert!(
        root.contains("mod runtime_core;")
            && root.contains("mod timing;")
            && root.contains("mod window_policy;")
            && !root.contains("fn generic_core_empty_runtime_wakeup")
            && !root.contains("fn generic_native_window_starts_hidden"),
        "native generic runtime test root should index focused runtime groups instead of owning all cases"
    );
    assert!(
        runtime_core.contains("fn generic_core_empty_runtime_wakeup_does_not_need_redraw")
            && runtime_core.contains("fn generic_core_can_enable_layout_debug_before_first_frame")
            && timing.contains("fn hover_redraws_do_not_reset_timed_animation_deadline")
            && timing.contains("struct TestFrameMessageBridge")
            && window_policy.contains("fn generic_native_window_applies_floating_popup_policy"),
        "native generic runtime tests should stay grouped by runtime core, timing, and window policy concerns"
    );
}

#[test]
fn native_pointer_motion_tests_stay_grouped_by_redraw_concern() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/tests/pointer_motion.rs"),
    )
    .expect("native pointer-motion test root should be readable");
    let fixtures = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/tests/pointer_motion/fixtures.rs"),
    )
    .expect("native pointer-motion fixtures should be readable");
    let redraw = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/tests/pointer_motion/redraw.rs"),
    )
    .expect("native pointer-motion redraw tests should be readable");
    let paint_only =
        fs::read_to_string(manifest_dir.join(
            "src/gui_runtime/native_vello/generic_runtime/tests/pointer_motion/paint_only.rs",
        ))
        .expect("native pointer-motion paint-only tests should be readable");

    assert!(
        root.contains("mod fixtures;")
            && root.contains("mod redraw;")
            && root.contains("mod paint_only;")
            && !root.contains("struct PointerMoveWidget")
            && !root.contains("fn pointer_move_inside_same_widget"),
        "native pointer-motion test root should index focused redraw groups instead of owning fixtures and all cases"
    );
    assert!(
        fixtures.contains("struct PointerMoveWidget")
            && fixtures.contains("struct PaintOnlyPointerMoveWidget")
            && redraw
                .contains("fn pointer_move_inside_same_widget_does_not_request_redundant_redraw")
            && redraw.contains("fn local_pointer_move_state_inside_same_widget_requests_redraw")
            && paint_only.contains(
                "fn paint_only_pointer_move_overlay_skips_scene_rebuild_after_hover_enters"
            ),
        "native pointer-motion tests should stay grouped by fixtures, redraw decisions, and paint-only overlay behavior"
    );
}

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
