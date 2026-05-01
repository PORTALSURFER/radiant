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
fn hotkey_resolution_is_typed_generic_shortcut_resolution() {
    let hotkeys_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/app/hotkeys/mod.rs"
    ))
    .expect("hotkeys module should be readable");
    let shortcuts_mod =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/shortcuts.rs"))
            .expect("shortcuts module should be readable");

    assert!(
        hotkeys_mod.contains(
            "pub type HotkeyResolution = ShortcutResolution<crate::sempal_app::UiAction>;"
        ),
        "HotkeyResolution should be a compatibility alias over generic shortcut resolution"
    );
    assert!(
        shortcuts_mod.contains("pub struct ShortcutResolution<Action>"),
        "generic shortcuts module should own reusable shortcut resolution behavior"
    );
}

#[test]
fn retained_vec_is_owned_by_generic_retained_module() {
    let app_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/mod.rs"))
        .expect("app module should be readable");
    let browser_mod =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/browser.rs"))
            .expect("browser module should be readable");
    let retained_mod =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/retained.rs"))
            .expect("retained module should be readable");

    assert!(
        !browser_mod.contains("pub struct RetainedVec"),
        "RetainedVec is generic retained storage, not a Sempal browser DTO"
    );
    assert!(
        app_mod.contains("pub use crate::gui::retained::RetainedVec;"),
        "the compatibility app contract should alias generic RetainedVec"
    );
    assert!(
        retained_mod.contains("pub struct RetainedVec"),
        "generic retained module should own RetainedVec"
    );
}

#[test]
fn normalized_range_model_is_owned_by_generic_range_module() {
    let waveform_mod =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/waveform.rs"))
            .expect("waveform module should be readable");
    let range_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/range.rs"))
        .expect("range module should be readable");

    assert!(
        !waveform_mod.contains("pub struct NormalizedRangeModel"),
        "NormalizedRangeModel is a generic interval primitive, not a waveform DTO"
    );
    assert!(
        waveform_mod
            .contains("pub use crate::gui::range::NormalizedRange as NormalizedRangeModel;"),
        "the compatibility app contract should alias generic NormalizedRange"
    );
    assert!(
        range_mod.contains("pub struct NormalizedRange"),
        "generic range module should own normalized interval behavior"
    );
}

#[test]
fn browser_row_processing_state_is_owned_by_generic_list_module() {
    let browser_mod =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/browser.rs"))
            .expect("browser module should be readable");
    let list_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/list.rs"))
        .expect("list module should be readable");

    assert!(
        !browser_mod.contains("pub enum BrowserRowProcessingState"),
        "BrowserRowProcessingState is generic row operation state, not a browser DTO"
    );
    assert!(
        browser_mod
            .contains("pub use crate::gui::list::RowProcessingState as BrowserRowProcessingState;"),
        "the compatibility app contract should alias generic RowProcessingState"
    );
    assert!(
        list_mod.contains("pub enum RowProcessingState"),
        "generic list module should own reusable row processing state"
    );
}

#[test]
fn column_model_is_owned_by_generic_list_module() {
    let sources_mod =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/sources.rs"))
            .expect("sources module should be readable");
    let list_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/list.rs"))
        .expect("list module should be readable");

    assert!(
        !sources_mod.contains("pub struct ColumnModel"),
        "ColumnModel is generic list/table column summary state, not a source DTO"
    );
    assert!(
        sources_mod.contains("pub use crate::gui::list::ColumnSummary as ColumnModel;"),
        "the compatibility app contract should alias generic ColumnSummary"
    );
    assert!(
        list_mod.contains("pub struct ColumnSummary"),
        "generic list module should own reusable column summary behavior"
    );
}

#[test]
fn folder_row_kind_is_owned_by_generic_list_module() {
    let sources_mod =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/sources.rs"))
            .expect("sources module should be readable");
    let list_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/list.rs"))
        .expect("list module should be readable");

    assert!(
        !sources_mod.contains("pub enum FolderRowKind"),
        "FolderRowKind is generic editable row state, not a source DTO"
    );
    assert!(
        sources_mod.contains("pub use crate::gui::list::EditableRowKind as FolderRowKind;"),
        "the compatibility app contract should alias generic EditableRowKind"
    );
    assert!(
        list_mod.contains("pub enum EditableRowKind"),
        "generic list module should own reusable editable row kind behavior"
    );
}

#[test]
fn status_bar_model_is_owned_by_generic_chrome_module() {
    let shell_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/shell.rs"))
        .expect("shell module should be readable");
    let chrome_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/chrome.rs"))
        .expect("chrome module should be readable");

    assert!(
        !shell_mod.contains("pub struct StatusBarModel"),
        "StatusBarModel is generic status chrome state, not a shell DTO"
    );
    assert!(
        shell_mod.contains("pub use crate::gui::chrome::StatusSegments as StatusBarModel;"),
        "the compatibility app contract should alias generic StatusSegments"
    );
    assert!(
        chrome_mod.contains("pub struct StatusSegments"),
        "generic chrome module should own reusable status segment behavior"
    );
}

#[test]
fn progress_overlay_model_is_owned_by_generic_feedback_module() {
    let shell_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/shell.rs"))
        .expect("shell module should be readable");
    let feedback_mod =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/feedback.rs"))
            .expect("feedback module should be readable");

    assert!(
        !shell_mod.contains("pub struct ProgressOverlayModel"),
        "ProgressOverlayModel is generic progress feedback state, not a shell DTO"
    );
    assert!(
        shell_mod
            .contains("pub use crate::gui::feedback::ProgressOverlay as ProgressOverlayModel;"),
        "the compatibility app contract should alias generic ProgressOverlay"
    );
    assert!(
        feedback_mod.contains("pub struct ProgressOverlay"),
        "generic feedback module should own reusable progress overlay behavior"
    );
}

#[test]
fn drag_overlay_model_is_owned_by_generic_feedback_module() {
    let shell_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/shell.rs"))
        .expect("shell module should be readable");
    let feedback_mod =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/feedback.rs"))
            .expect("feedback module should be readable");

    assert!(
        !shell_mod.contains("pub struct DragOverlayModel"),
        "DragOverlayModel is generic pointer feedback state, not a shell DTO"
    );
    assert!(
        shell_mod.contains("pub use crate::gui::feedback::DragOverlay as DragOverlayModel;"),
        "the compatibility app contract should alias generic DragOverlay"
    );
    assert!(
        feedback_mod.contains("pub struct DragOverlay"),
        "generic feedback module should own reusable drag overlay behavior"
    );
}

#[test]
fn update_panel_model_is_owned_by_generic_feedback_module() {
    let shell_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/shell.rs"))
        .expect("shell module should be readable");
    let feedback_mod =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/feedback.rs"))
            .expect("feedback module should be readable");

    assert!(
        !shell_mod.contains("pub enum UpdateStatusModel")
            && !shell_mod.contains("pub struct UpdatePanelModel"),
        "UpdateStatusModel and UpdatePanelModel are generic update feedback state, not shell DTOs"
    );
    assert!(
        shell_mod.contains("pub use crate::gui::feedback::UpdatePanel as UpdatePanelModel;")
            && shell_mod
                .contains("pub use crate::gui::feedback::UpdateStatus as UpdateStatusModel;"),
        "the compatibility app contract should alias generic update feedback types"
    );
    assert!(
        feedback_mod.contains("pub enum UpdateStatus")
            && feedback_mod.contains("pub struct UpdatePanel"),
        "generic feedback module should own reusable update feedback behavior"
    );
}

#[test]
fn confirm_prompt_model_is_owned_by_generic_feedback_module() {
    let shell_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/shell.rs"))
        .expect("shell module should be readable");
    let feedback_mod =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/feedback.rs"))
            .expect("feedback module should be readable");

    assert!(
        !shell_mod.contains("pub struct ConfirmPromptModel"),
        "ConfirmPromptModel is generic modal prompt state parameterized by a host kind"
    );
    assert!(
        shell_mod.contains(
            "pub type ConfirmPromptModel = crate::gui::feedback::ConfirmPrompt<ConfirmPromptKind>;"
        ),
        "the compatibility app contract should alias generic ConfirmPrompt with the legacy kind"
    );
    assert!(
        feedback_mod.contains("pub struct ConfirmPrompt<Kind>"),
        "generic feedback module should own reusable confirmation prompt behavior"
    );
}

#[test]
fn browser_tag_state_is_owned_by_generic_selection_module() {
    let browser_mod =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/browser.rs"))
            .expect("browser module should be readable");
    let selection_mod =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/selection.rs"))
            .expect("selection module should be readable");

    assert!(
        !browser_mod.contains("pub enum BrowserTagState"),
        "BrowserTagState is generic tri-state selection state, not a browser DTO"
    );
    assert!(
        browser_mod.contains("pub use crate::gui::selection::TriState as BrowserTagState;"),
        "the compatibility app contract should alias generic TriState"
    );
    assert!(
        selection_mod.contains("pub enum TriState"),
        "generic selection module should own reusable tri-state behavior"
    );
}

#[test]
fn frame_build_result_is_owned_by_generic_frame_module() {
    let app_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/mod.rs"))
        .expect("app module should be readable");
    let dirty_segments_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/app/dirty_segments.rs"
    ))
    .expect("dirty segments module should be readable");
    let frame_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/frame.rs"))
        .expect("frame module should be readable");

    assert!(
        !dirty_segments_mod.contains("pub struct FrameBuildResult"),
        "FrameBuildResult is generic frame feedback, not a Sempal dirty-segment DTO"
    );
    assert!(
        app_mod.contains("pub use crate::gui::frame::FrameBuildResult;"),
        "the compatibility app contract should alias generic FrameBuildResult"
    );
    assert!(
        frame_mod.contains("pub struct FrameBuildResult"),
        "generic frame module should own FrameBuildResult"
    );
}

#[test]
fn dirty_segments_use_generic_invalidation_mask() {
    let dirty_segments_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/app/dirty_segments.rs"
    ))
    .expect("dirty segments module should be readable");
    let invalidation_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/invalidation.rs"
    ))
    .expect("invalidation module should be readable");

    assert!(
        dirty_segments_mod.contains("use crate::gui::invalidation::InvalidationMask;"),
        "DirtySegments should delegate bitmask behavior to generic invalidation primitives"
    );
    assert!(
        invalidation_mod.contains("pub struct InvalidationMask"),
        "generic invalidation module should own reusable mask behavior"
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
