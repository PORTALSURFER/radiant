use super::*;

#[test]
fn generic_core_empty_runtime_wakeup_does_not_need_redraw() {
    let mut core = GenericNativeRuntimeCore::new(demo_bridge(), Vector2::new(320.0, 40.0));

    let outcome = core.drain_runtime_messages();

    assert!(!outcome.routed);
    assert!(!outcome.needs_redraw());
    assert!(!outcome.runtime_work_remaining);
}

#[test]
fn generic_core_keeps_paint_only_runtime_frames_off_scene_rebuild_path() {
    let mut core =
        GenericNativeRuntimeCore::new(PaintOnlyFrameBridge::default(), Vector2::new(320.0, 40.0));

    let outcome = core.drain_runtime_messages();

    assert!(outcome.routed);
    assert!(outcome.needs_redraw());
    assert!(!outcome.needs_scene_rebuild());
}

#[test]
fn generic_core_can_enable_layout_debug_before_first_frame() {
    let core = GenericNativeRuntimeCore::new_with_debug_layout(
        demo_bridge(),
        Vector2::new(320.0, 40.0),
        true,
    );

    assert_eq!(
        core.runtime.layout_debug_options(),
        LayoutDebugOptions::bounds_only()
    );
    assert!(!core.runtime.layout().debug_primitives.is_empty());
}
