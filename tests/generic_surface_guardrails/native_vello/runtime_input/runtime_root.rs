use super::*;

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
