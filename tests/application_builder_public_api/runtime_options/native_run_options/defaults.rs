use super::*;

#[test]
fn native_run_options_default_uses_generic_radiant_title() {
    let options = NativeRunOptions::default();

    assert_eq!(options.title, DEFAULT_NATIVE_WINDOW_TITLE);
    assert_eq!(options.title, "Radiant");
    assert!(options.drag_and_drop);
}

#[test]
fn native_run_options_expose_platform_neutral_drag_and_drop_policy() {
    let options = NativeRunOptions {
        drag_and_drop: false,
        ..NativeRunOptions::default()
    };

    assert!(!options.drag_and_drop);
}

#[test]
fn native_run_options_normalize_animation_frame_rate_policy() {
    let zero = NativeRunOptions {
        target_fps: 0,
        ..NativeRunOptions::default()
    };
    let default = NativeRunOptions::default();
    let high = NativeRunOptions {
        target_fps: u32::MAX,
        ..NativeRunOptions::default()
    };

    assert_eq!(zero.normalized_target_fps(), MIN_NATIVE_TARGET_FPS);
    assert_eq!(default.normalized_target_fps(), default.target_fps);
    assert_eq!(high.normalized_target_fps(), MAX_NATIVE_TARGET_FPS);
}

#[test]
fn native_run_options_expose_layout_debug_overlay_policy() {
    let options = NativeRunOptions {
        debug_layout: true,
        ..NativeRunOptions::default()
    };

    assert!(!NativeRunOptions::default().debug_layout);
    assert!(options.debug_layout);
}
