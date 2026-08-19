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
    let timing_route_frames = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/tests/timing/route_frames.rs"),
    )
    .expect("native generic runtime frame-routing timing tests should be readable");
    let timing_fixtures = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/tests/timing/fixtures.rs"),
    )
    .expect("native generic runtime timing fixtures should be readable");
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
            && timing.contains("mod route_frames;")
            && timing.contains("mod fixtures;")
            && timing_route_frames
                .contains("fn hover_redraws_do_not_reset_timed_animation_deadline")
            && timing_fixtures.contains("struct TestFrameMessageBridge")
            && window_policy.contains("fn generic_native_window_applies_floating_popup_policy"),
        "native generic runtime tests should stay grouped by runtime core, timing, and window policy concerns"
    );
}

#[test]
fn auxiliary_route_redraw_observation_stays_parent_owned() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lifecycle = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/lifecycle.rs"),
    )
    .expect("native lifecycle should be readable");
    let auxiliary = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/auxiliary.rs"),
    )
    .expect("native auxiliary window routing should be readable");
    let keyboard = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/keyboard.rs"),
    )
    .expect("native keyboard routing should be readable");
    let runner = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/runner.rs"),
    )
    .expect("native runner routing should be readable");
    let present = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/present.rs"),
    )
    .expect("native presentation should be readable");

    assert!(
        lifecycle.contains("CpuFrameObservationOwner::new(ledger, auxiliary_key.clone())")
            && lifecycle.contains("CpuFrameObservationOwner::new(ledger, selected.clone())")
            && auxiliary.contains("observation: Option<&mut CpuFrameObservationOwner<'_>>")
            && keyboard.contains("observation: Option<&mut CpuFrameObservationOwner<'_>>"),
        "auxiliary event and schedule boundaries should receive a parent-owned observation scope"
    );
    assert!(
        runner.contains("pub(super) fn begin_native_visual_request(")
            && runner.contains("pub(super) fn finish_native_visual_request(")
            && !runner.contains("redraw_and_exit_on_error_with_adapter")
            && auxiliary.contains("begin_cpu_frame_observation_with_owner(owner")
            && auxiliary.contains("finish_cpu_frame_observation_with_owner(")
            && present.contains("begin_native_visual_request(&adapter)"),
        "redraw boundaries should use the shared native packet kernel and parent-owned observation"
    );
}
