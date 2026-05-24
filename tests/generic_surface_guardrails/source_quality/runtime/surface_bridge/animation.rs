use super::*;

#[test]
fn runtime_animation_activity_keeps_policy_and_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let animation = fs::read_to_string(manifest_dir.join("src/runtime/bridge/animation.rs"))
        .expect("runtime animation activity module should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/runtime/bridge/animation/tests.rs"))
        .expect("runtime animation activity tests should be readable");
    let bridge = fs::read_to_string(manifest_dir.join("src/runtime/bridge.rs"))
        .expect("runtime bridge module should be readable");

    assert!(
        bridge.contains("mod animation;")
            && bridge.contains("pub use animation::RuntimeAnimationActivity;"),
        "runtime bridge root should re-export animation activity from the focused child module"
    );
    assert!(
        animation.contains("pub struct RuntimeAnimationActivity")
            && animation.contains("pub const fn merge(self, other: Self) -> Self")
            && animation.contains("const fn merge_target_fps(")
            && animation.contains("#[path = \"animation/tests.rs\"]")
            && !animation.contains(
                "fn runtime_animation_activity_keeps_frame_messages_bound_to_paint_frames"
            ),
        "runtime animation policy should live in bridge/animation.rs while behavior tests stay delegated"
    );
    assert!(
        tests.contains("fn runtime_animation_activity_keeps_frame_messages_bound_to_paint_frames")
            && tests.contains("fn runtime_animation_activity_merge_keeps_uncapped_source_uncapped"),
        "runtime animation behavior coverage should live in bridge/animation/tests.rs"
    );
}
