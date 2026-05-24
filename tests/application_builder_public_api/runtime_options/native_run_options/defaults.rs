use super::*;

#[test]
fn native_run_options_default_uses_generic_radiant_title() {
    let options = NativeRunOptions::default();

    assert_eq!(options.window.title, DEFAULT_NATIVE_WINDOW_TITLE);
    assert_eq!(options.window.title, "Radiant");
    assert!(options.window.behavior.drag_and_drop);
}

#[test]
fn native_run_options_expose_platform_neutral_drag_and_drop_policy() {
    let options = NativeRunOptions {
        window: NativeWindowOptions {
            behavior: NativeWindowBehavior {
                drag_and_drop: false,
                ..NativeWindowBehavior::default()
            },
            ..NativeWindowOptions::default()
        },
        ..NativeRunOptions::default()
    };

    assert!(!options.window.behavior.drag_and_drop);
}

#[test]
fn native_run_options_normalize_animation_frame_rate_policy() {
    let zero = NativeRunOptions {
        frame: NativeFrameOptions {
            target_fps: 0,
            ..NativeFrameOptions::default()
        },
        ..NativeRunOptions::default()
    };
    let default = NativeRunOptions::default();
    let high = NativeRunOptions {
        frame: NativeFrameOptions {
            target_fps: u32::MAX,
            ..NativeFrameOptions::default()
        },
        ..NativeRunOptions::default()
    };

    assert_eq!(zero.normalized_target_fps(), MIN_NATIVE_TARGET_FPS);
    assert_eq!(default.normalized_target_fps(), default.frame.target_fps);
    assert_eq!(high.normalized_target_fps(), MAX_NATIVE_TARGET_FPS);
}

#[test]
fn native_run_options_expose_layout_debug_overlay_policy() {
    let options = NativeRunOptions {
        frame: NativeFrameOptions {
            debug_layout: true,
            ..NativeFrameOptions::default()
        },
        ..NativeRunOptions::default()
    };

    assert!(!NativeRunOptions::default().frame.debug_layout);
    assert!(options.frame.debug_layout);
}
