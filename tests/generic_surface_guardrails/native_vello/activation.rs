use super::*;

#[test]
fn macos_activation_and_reopen_models_are_cfg_owned() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let activation = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/activation.rs"),
    )
    .expect("activation source should be readable");
    let runtime_event =
        fs::read_to_string(manifest_dir.join("src/gui_runtime/native_vello/runtime_event.rs"))
            .expect("runtime event source should be readable");
    let lifecycle = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/lifecycle.rs"),
    )
    .expect("lifecycle source should be readable");

    for declaration in [
        "#[cfg(any(target_os = \"macos\", test))]\n    UserRequested,",
        "#[cfg(target_os = \"macos\")]\n    Modern,",
        "#[cfg(target_os = \"macos\")]\n    Compatibility,",
        "#[cfg(target_os = \"macos\")]\n            Self::Modern => \"modern\",",
        "#[cfg(target_os = \"macos\")]\n            Self::Compatibility => \"compatibility\",",
        "#[cfg(any(target_os = \"macos\", test))]\n    pub(super) fn observe_user_reopen",
        "#[cfg(target_os = \"macos\")]\n    pub(super) fn handle_application_reopen_intent",
    ] {
        assert!(
            activation.contains(declaration),
            "macOS activation declaration is missing cfg ownership: {declaration}"
        );
    }

    let reopen_variant = "#[cfg(target_os = \"macos\")]\n    ApplicationReopenRequested,";
    assert!(
        runtime_event.contains(reopen_variant),
        "macOS reopen event is missing cfg ownership"
    );
    assert!(
        lifecycle.contains(
            "#[cfg(target_os = \"macos\")]\n            RuntimeUserEvent::ApplicationReopenRequested => {"
        ),
        "macOS reopen event handling is missing cfg ownership"
    );
}
