use super::*;

#[test]
fn macos_activation_and_reopen_models_are_cfg_owned() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let activation = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/activation.rs"),
    )
    .expect("activation source should be readable");
    let activation_platform = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/activation/platform.rs"),
    )
    .expect("activation platform source should be readable");
    let runtime_event =
        fs::read_to_string(manifest_dir.join("src/gui_runtime/native_vello/runtime_event.rs"))
            .expect("runtime event source should be readable");
    let lifecycle = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/lifecycle.rs"),
    )
    .expect("lifecycle source should be readable");

    assert!(
        !activation.contains("target_os = "),
        "backend-neutral activation policy should not own target-specific cfg branches"
    );
    assert!(
        activation_platform.contains("#[cfg(target_os = \"macos\")]\r\n")
            || activation_platform.contains("#[cfg(target_os = \"macos\")]\n"),
        "macOS activation integration should stay behind the focused platform adapter"
    );
    assert!(runtime_event.contains("ApplicationReopenRequested,"));
    assert!(lifecycle.contains("RuntimeUserEvent::ApplicationReopenRequested => {"));
}
