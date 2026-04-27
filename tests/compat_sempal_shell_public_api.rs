//! Public API coverage for the Sempal compatibility namespace.

use radiant::compat::sempal_shell::{
    AppModel, DEFAULT_APP_TITLE, NativeRunOptions, UiAction, capture_gui_automation_snapshot,
    run_native_vello_preview,
};

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
