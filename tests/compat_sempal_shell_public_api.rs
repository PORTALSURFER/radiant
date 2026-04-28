//! Public API coverage for the Sempal compatibility namespace.

#![allow(deprecated)]

use radiant::compat::sempal_shell::{
    AppModel, DEFAULT_APP_TITLE, NativeRunOptions, UiAction, capture_gui_automation_snapshot,
    run_native_vello_preview,
};
use std::fs;

#[test]
fn sempal_shell_namespace_exposes_legacy_shell_runtime_contract() {
    let model: AppModel = radiant::app::AppModel::default();
    let snapshot = capture_gui_automation_snapshot([1440.0, 810.0], &model);

    assert_eq!(snapshot.root.id.0, "shell.root");
    assert_eq!(snapshot.viewport_width, 1440);
    assert_eq!(snapshot.viewport_height, 810);
}

#[test]
fn legacy_app_alias_and_compat_namespace_share_the_same_shell_contract() {
    let legacy_model = radiant::app::AppModel::default();
    let compat_model: AppModel = legacy_model.clone();
    let legacy_action: radiant::app::UiAction = UiAction::ToggleTransport;
    let preview: fn(NativeRunOptions) -> Result<(), String> = run_native_vello_preview;

    assert_eq!(legacy_model, compat_model);
    assert_eq!(NativeRunOptions::default().title, DEFAULT_APP_TITLE);
    assert_eq!(legacy_action, radiant::app::UiAction::ToggleTransport);

    let _ = preview;
}

#[test]
fn legacy_app_alias_is_marked_deprecated_at_the_crate_root() {
    let lib_rs = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"))
        .expect("crate root should be readable");
    let deprecated = lib_rs
        .find("#[deprecated(")
        .expect("crate root app alias should carry a deprecation marker");
    let app_alias = lib_rs[deprecated..]
        .find("pub mod app")
        .expect("deprecated marker should apply to the app alias");

    assert!(
        app_alias < 512,
        "radiant::app should remain the explicitly deprecated compatibility alias"
    );
    assert!(
        lib_rs.contains("radiant::compat::sempal_shell"),
        "deprecation guidance should point compatibility callers at compat::sempal_shell"
    );
}
