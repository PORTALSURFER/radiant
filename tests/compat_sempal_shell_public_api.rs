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
    assert_eq!(DEFAULT_APP_TITLE, DEFAULT_NATIVE_WINDOW_TITLE);
    assert_eq!(
        NativeRunOptions::default().title,
        DEFAULT_NATIVE_WINDOW_TITLE
    );
    assert_eq!(compat_action, UiAction::ToggleTransport);

    let _ = preview;
}

#[test]
fn default_app_title_is_a_runtime_compat_alias() {
    let app_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/mod.rs"))
        .expect("app module should be readable");
    let shell_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/shell.rs"))
        .expect("shell module should be readable");

    assert!(
        app_mod.contains(
            "pub use crate::gui_runtime::DEFAULT_NATIVE_WINDOW_TITLE as DEFAULT_APP_TITLE;"
        ),
        "compat DEFAULT_APP_TITLE should alias the generic runtime fallback title"
    );
    assert!(
        !shell_mod.contains("pub const DEFAULT_APP_TITLE"),
        "the Sempal app-contract module must not own a duplicate default title constant"
    );
}

#[test]
fn app_contract_no_longer_exports_waveform_tempo_parsing_helper() {
    let app_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/mod.rs"))
        .expect("app module should be readable");

    assert!(
        !app_mod.contains("parse_waveform_tempo_number_text"),
        "waveform tempo parsing is a Sempal-owned DTO helper, not part of the Radiant compat export"
    );
}

#[test]
fn keypress_value_type_is_owned_by_generic_input_module() {
    let hotkeys_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/app/hotkeys/mod.rs"
    ))
    .expect("hotkeys module should be readable");
    let input_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/input.rs"))
        .expect("input module should be readable");

    assert!(
        !hotkeys_mod.contains("pub struct KeyPress"),
        "KeyPress is a backend-neutral input primitive, not a Sempal app-contract DTO"
    );
    assert!(
        hotkeys_mod.contains("pub use crate::gui::input::KeyPress;"),
        "the compatibility app contract should alias generic input KeyPress"
    );
    assert!(
        input_mod.contains("pub struct KeyPress"),
        "generic input module should own the KeyPress value type"
    );
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
