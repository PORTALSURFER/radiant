//! Public API coverage for the Sempal compatibility namespace.

use radiant::compat::sempal_shell::{
    AppModel, DEFAULT_APP_TITLE, NativeRunOptions, UiAction, capture_gui_automation_snapshot,
    run_native_vello_preview,
};
use radiant::runtime::DEFAULT_NATIVE_WINDOW_TITLE;
use std::fs;

#[test]
fn sempal_shell_namespace_exposes_legacy_shell_runtime_contract() {
    let model = AppModel::default();
    let snapshot = capture_gui_automation_snapshot([1440.0, 810.0], &model);

    assert_eq!(snapshot.root.id.0, "shell.root");
    assert_eq!(snapshot.viewport_width, 1440);
    assert_eq!(snapshot.viewport_height, 810);
}

#[test]
fn sempal_shell_namespace_exposes_model_actions_and_runtime_helpers() {
    let compat_model = AppModel::default();
    let compat_action = UiAction::ToggleTransport;
    let preview: fn(NativeRunOptions) -> Result<(), String> = run_native_vello_preview;

    assert_eq!(compat_model, AppModel::default());
    assert_eq!(DEFAULT_APP_TITLE, "Radiant");
    assert_eq!(
        NativeRunOptions::default().title,
        DEFAULT_NATIVE_WINDOW_TITLE
    );
    assert_eq!(compat_action, UiAction::ToggleTransport);

    let _ = preview;
}

#[test]
fn crate_root_does_not_export_legacy_app_alias() {
    let lib_rs = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"))
        .expect("crate root should be readable");

    assert!(
        !lib_rs.contains("pub mod app"),
        "radiant::app should not remain a crate-root compatibility alias"
    );
    assert!(
        lib_rs.contains("compat::sempal_shell"),
        "crate-root guidance should point compatibility callers at compat::sempal_shell"
    );
}
