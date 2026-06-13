use super::*;

#[test]
fn app_bridge_groups_lifecycle_hooks_and_runtime_flags() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let bridge = fs::read_to_string(manifest_dir.join("src/application/runtime/bridge.rs"))
        .expect("application runtime bridge should be readable");
    let animation = fs::read_to_string(
        manifest_dir.join("src/application/runtime/bridge/adapter/animation.rs"),
    )
    .expect("application runtime bridge animation adapter should be readable");
    let launch_animation = fs::read_to_string(
        manifest_dir.join("src/application/runtime/bridge/adapter/launch_animation.rs"),
    )
    .expect("launch-level animation adapter should be readable");

    assert!(
        bridge.contains("pub(in crate::application) lifecycle: AppBridgeLifecycle<State, Message>")
            && bridge.contains("pub(in crate::application) runtime_flags: AppBridgeRuntimeFlags")
            && bridge.contains("pub(in crate::application) struct AppBridgeRuntimeFlags"),
        "app bridge should keep lifecycle hooks and mutable runtime flags in focused field groups"
    );
    assert!(
        bridge.contains("use super::{")
            && bridge.contains("AppAnimation")
            && bridge.contains("AppRuntime")
            && bridge.contains("AppStartup")
            && bridge.contains("RetainedPainter")
            && bridge.contains("TransientOverlayPainter")
            && bridge.contains("UiUpdateContext")
            && bridge.contains("application::{IntoView, RepaintPolicy}")
            && bridge.contains("gui::{input::KeyPress, shortcuts::ShortcutResolution}")
            && bridge.contains("runtime::{Command, RepaintScope}")
            && !bridge.contains("use super::*;"),
        "app bridge should name runtime lifecycle, queue, retained painter, update context, and command dependencies explicitly"
    );
    let bridge_fields = bridge
        .split("pub(in crate::application) struct AppBridgeRuntimeFlags")
        .next()
        .expect("app bridge source should include fields before runtime flags");
    for flattened_field in [
        "pub(in crate::application) animation:",
        "pub(in crate::application) frame_message:",
        "pub(in crate::application) subscriptions:",
        "pub(in crate::application) retained_painters:",
        "pub(in crate::application) startup_ran:",
    ] {
        assert!(
            !bridge_fields.contains(flattened_field),
            "app bridge should not flatten lifecycle or runtime flag field `{flattened_field}`"
        );
    }
    assert!(
        animation.contains(".lifecycle")
            && animation.contains("launch_animation::poll_launch_animation_activity")
            && animation.contains("launch_animation::poll_launch_frame_message_activity")
            && animation.contains("self.runtime_flags.pending_animation_frame_activity"),
        "animation adapter should route normal frame-clock demand through grouped lifecycle and runtime flag state"
    );
    assert!(
        !animation.contains("legacy_")
            && launch_animation.contains("lifecycle")
            && launch_animation.contains(".animation")
            && launch_animation.contains("lifecycle.frame_message")
            && launch_animation.contains("RuntimeAnimationDemand::from_flags"),
        "launch-level animation compatibility should stay isolated outside the normal animation adapter"
    );
}
