use super::*;

#[test]
fn app_runtime_api_lifecycle_tests_stay_grouped_by_runtime_concern() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("tests/app_runtime_api.rs"))
        .expect("app runtime API test root should be readable");
    let lifecycle = fs::read_to_string(manifest_dir.join("tests/app_runtime_api/lifecycle.rs"))
        .expect("app runtime lifecycle test root should be readable");
    let startup = fs::read_to_string(
        manifest_dir.join("tests/app_runtime_api/lifecycle/startup_and_exit.rs"),
    )
    .expect("app runtime startup and exit tests should be readable");
    let animation =
        fs::read_to_string(manifest_dir.join("tests/app_runtime_api/lifecycle/animation.rs"))
            .expect("app runtime animation lifecycle tests should be readable");

    assert!(
        root.contains("#[path = \"app_runtime_api/lifecycle.rs\"]"),
        "app runtime API root should keep lifecycle coverage as a named module"
    );
    assert!(
        lifecycle.contains("#[path = \"lifecycle/startup_and_exit.rs\"]")
            && lifecycle.contains("#[path = \"lifecycle/animation.rs\"]")
            && lifecycle.contains("fn drain_until_messages<Bridge>")
            && !lifecycle.contains("fn app_startup_commands_use_full_runtime_dispatch")
            && !lifecycle
                .contains("fn active_animation_frame_messages_are_coalesced_until_drained"),
        "lifecycle test root should own shared fixtures and delegate startup/exit and animation behavior"
    );
    assert!(
        startup.contains("fn app_startup_commands_use_full_runtime_dispatch")
            && startup.contains("fn app_startup_runs_once_when_repaint_signal_is_reinstalled")
            && startup.contains("fn app_runtime_effects_stop_after_runtime_exit"),
        "startup and exit lifecycle coverage should stay in the startup_and_exit module"
    );
    assert!(
        animation.contains("fn active_animation_frame_messages_are_coalesced_until_drained")
            && animation.contains("fn animation_activity_poll_is_reused_for_frame_queue")
            && animation.contains("fn polling_animation_activity_does_not_queue_frame_messages"),
        "animation lifecycle coverage should stay in the animation module"
    );
}
