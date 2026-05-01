//! Ownership guardrails for primitives already extracted into generic Radiant modules.

use std::fs;

#[test]
fn keypress_value_type_is_owned_by_generic_input_module() {
    let app_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/mod.rs"))
        .expect("app module should be readable");
    let input_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/input.rs"))
        .expect("input module should be readable");
    let shortcuts_mod =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/shortcuts.rs"))
            .expect("shortcuts module should be readable");

    assert!(!app_mod.contains("pub struct KeyPress"));
    assert!(app_mod.contains("pub use crate::gui::input::KeyPress;"));
    assert!(app_mod.contains("pub use crate::gui::shortcuts::ShortcutResolution;"));
    assert!(app_mod.contains("pub type HotkeyResolution = ShortcutResolution<UiAction>;"));
    assert!(input_mod.contains("pub struct KeyPress"));
    assert!(shortcuts_mod.contains("pub struct ShortcutResolution<Action>"));
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

    assert!(!browser_mod.contains("pub struct RetainedVec"));
    assert!(app_mod.contains("pub use crate::gui::retained::RetainedVec;"));
    assert!(retained_mod.contains("pub struct RetainedVec"));
}

#[test]
fn normalized_range_model_is_owned_by_generic_range_module() {
    let waveform_mod =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/waveform.rs"))
            .expect("waveform module should be readable");
    let range_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/range.rs"))
        .expect("range module should be readable");

    assert!(!waveform_mod.contains("pub struct NormalizedRangeModel"));
    assert!(
        waveform_mod
            .contains("pub use crate::gui::range::NormalizedRange as NormalizedRangeModel;")
    );
    assert!(range_mod.contains("pub struct NormalizedRange"));
}

#[test]
fn row_processing_state_is_owned_by_generic_list_module() {
    let browser_mod =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/browser.rs"))
            .expect("browser module should be readable");
    let list_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/list.rs"))
        .expect("list module should be readable");

    assert!(!browser_mod.contains("pub enum BrowserRowProcessingState"));
    assert!(
        browser_mod
            .contains("pub use crate::gui::list::RowProcessingState as BrowserRowProcessingState;")
    );
    assert!(list_mod.contains("pub enum RowProcessingState"));
}

#[test]
fn recency_state_is_owned_by_generic_list_module() {
    let browser_mod =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/browser.rs"))
            .expect("browser module should be readable");
    let list_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/list.rs"))
        .expect("list module should be readable");

    assert!(!browser_mod.contains("pub enum PlaybackAgeFilterChip"));
    assert!(!browser_mod.contains("pub enum PlaybackAgeBucket"));
    assert!(
        browser_mod
            .contains("pub use crate::gui::list::RecencyFilterChip as PlaybackAgeFilterChip;")
    );
    assert!(browser_mod.contains("pub use crate::gui::list::RecencyBucket as PlaybackAgeBucket;"));
    assert!(list_mod.contains("pub enum RecencyFilterChip"));
    assert!(list_mod.contains("pub enum RecencyBucket"));
}

#[test]
fn column_summary_is_owned_by_generic_list_module() {
    let sources_mod =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/sources.rs"))
            .expect("sources module should be readable");
    let list_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/list.rs"))
        .expect("list module should be readable");

    assert!(!sources_mod.contains("pub struct ColumnModel"));
    assert!(sources_mod.contains("pub use crate::gui::list::ColumnSummary as ColumnModel;"));
    assert!(list_mod.contains("pub struct ColumnSummary"));
}

#[test]
fn editable_row_kind_is_owned_by_generic_list_module() {
    let sources_mod =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/sources.rs"))
            .expect("sources module should be readable");
    let list_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/list.rs"))
        .expect("list module should be readable");

    assert!(!sources_mod.contains("pub enum FolderRowKind"));
    assert!(sources_mod.contains("pub use crate::gui::list::EditableRowKind as FolderRowKind;"));
    assert!(list_mod.contains("pub enum EditableRowKind"));
}

#[test]
fn editable_tree_actions_are_owned_by_generic_list_module() {
    let sources_mod =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/sources.rs"))
            .expect("sources module should be readable");
    let list_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/list.rs"))
        .expect("list module should be readable");

    assert!(!sources_mod.contains("pub struct FolderActionsModel"));
    assert!(
        sources_mod
            .contains("pub use crate::gui::list::EditableTreeActions as FolderActionsModel;")
    );
    assert!(list_mod.contains("pub struct EditableTreeActions"));
}

#[test]
fn editable_tree_row_is_owned_by_generic_list_module() {
    let sources_mod =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/sources.rs"))
            .expect("sources module should be readable");
    let list_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/list.rs"))
        .expect("list module should be readable");

    assert!(!sources_mod.contains("pub struct FolderRowModel"));
    assert!(sources_mod.contains("pub use crate::gui::list::EditableTreeRow as FolderRowModel;"));
    assert!(list_mod.contains("pub struct EditableTreeRow"));
    assert!(list_mod.contains("pub backing_index: Option<usize>"));
    assert!(!list_mod.contains("source_index"));
}

#[test]
fn split_pane_slot_is_owned_by_generic_panel_module() {
    let sources_mod =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/sources.rs"))
            .expect("sources module should be readable");
    let panel_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/panel.rs"))
        .expect("panel module should be readable");

    assert!(!sources_mod.contains("pub enum FolderPaneIdModel"));
    assert!(!sources_mod.contains("pub struct SourceRowModel"));
    assert!(!sources_mod.contains("pub struct FolderPaneModel"));
    assert!(
        sources_mod.contains("pub use crate::gui::panel::SplitPaneAssignedRow as SourceRowModel;")
    );
    assert!(sources_mod.contains("pub use crate::gui::panel::SplitPaneSlot as FolderPaneIdModel;"));
    assert!(sources_mod.contains(
        "pub type FolderPaneModel = crate::gui::panel::SplitPaneTreePanel<FolderRowModel>;"
    ));
    assert!(panel_mod.contains("pub enum SplitPaneSlot"));
    assert!(panel_mod.contains("pub struct SplitPaneAssignedRow"));
    assert!(panel_mod.contains("pub struct SplitPaneTreePanel<Row = EditableTreeRow>"));
}

#[test]
fn status_segments_are_owned_by_generic_chrome_module() {
    let shell_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/shell.rs"))
        .expect("shell module should be readable");
    let chrome_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/chrome.rs"))
        .expect("chrome module should be readable");

    assert!(!shell_mod.contains("pub struct StatusBarModel"));
    assert!(shell_mod.contains("pub use crate::gui::chrome::StatusSegments as StatusBarModel;"));
    assert!(chrome_mod.contains("pub struct StatusSegments"));
}

#[test]
fn feedback_models_are_owned_by_generic_feedback_module() {
    let shell_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/shell.rs"))
        .expect("shell module should be readable");
    let sources_mod =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/sources.rs"))
            .expect("sources module should be readable");
    let feedback_mod =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/feedback.rs"))
            .expect("feedback module should be readable");

    assert!(!shell_mod.contains("pub struct ProgressOverlayModel"));
    assert!(!shell_mod.contains("pub struct DragOverlayModel"));
    assert!(
        !shell_mod.contains("pub enum UpdateStatusModel")
            && !shell_mod.contains("pub struct UpdatePanelModel")
    );
    assert!(!shell_mod.contains("pub struct ConfirmPromptModel"));
    assert!(!shell_mod.contains("pub enum AudioEngineChipStateModel"));
    assert!(!sources_mod.contains("pub struct FolderRecoveryModel"));
    assert!(
        shell_mod
            .contains("pub use crate::gui::feedback::ProgressOverlay as ProgressOverlayModel;")
    );
    assert!(shell_mod.contains("pub use crate::gui::feedback::DragOverlay as DragOverlayModel;"));
    assert!(shell_mod.contains("pub use crate::gui::feedback::UpdatePanel as UpdatePanelModel;"));
    assert!(shell_mod.contains("pub use crate::gui::feedback::UpdateStatus as UpdateStatusModel;"));
    assert!(
        shell_mod
            .contains("pub use crate::gui::feedback::HealthState as AudioEngineChipStateModel;")
    );
    assert!(shell_mod.contains(
        "pub type ConfirmPromptModel = crate::gui::feedback::ConfirmPrompt<ConfirmPromptKind>;"
    ));
    assert!(
        sources_mod
            .contains("pub use crate::gui::feedback::RecoverySummary as FolderRecoveryModel;")
    );
    assert!(feedback_mod.contains("pub struct ProgressOverlay"));
    assert!(feedback_mod.contains("pub struct RecoverySummary"));
    assert!(feedback_mod.contains("pub enum HealthState"));
    assert!(feedback_mod.contains("pub struct DragOverlay"));
    assert!(feedback_mod.contains("pub enum UpdateStatus"));
    assert!(feedback_mod.contains("pub struct UpdatePanel"));
    assert!(feedback_mod.contains("pub struct ConfirmPrompt<Kind>"));
}

#[test]
fn paired_picker_models_are_owned_by_generic_form_module() {
    let shell_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/shell.rs"))
        .expect("shell module should be readable");
    let form_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/form.rs"))
        .expect("form module should be readable");

    assert!(!shell_mod.contains("pub enum AudioPickerTargetModel"));
    assert!(!shell_mod.contains("pub enum AudioOptionValueModel"));
    assert!(
        shell_mod
            .contains("pub use crate::gui::form::PairedPickerTarget as AudioPickerTargetModel;")
    );
    assert!(shell_mod.contains(
        "pub type AudioOptionValueModel = crate::gui::form::PairedPickerValue<String, u32>;"
    ));
    assert!(form_mod.contains("pub enum PairedPickerTarget"));
    assert!(form_mod.contains("pub enum PairedPickerValue"));
}

#[test]
fn selection_badge_and_visualization_models_are_owned_by_generic_modules() {
    let browser_mod =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/browser.rs"))
            .expect("browser module should be readable");
    let waveform_mod =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/waveform.rs"))
            .expect("waveform module should be readable");
    let selection_mod =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/selection.rs"))
            .expect("selection module should be readable");
    let badge_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/badge.rs"))
        .expect("badge module should be readable");
    let visualization_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/visualization.rs"
    ))
    .expect("visualization module should be readable");

    assert!(!browser_mod.contains("pub enum BrowserTagState"));
    assert!(!browser_mod.contains("pub struct BrowserTagPillModel"));
    assert!(!browser_mod.contains("pub struct BrowserTagSidebarModel"));
    assert!(!browser_mod.contains("pub enum MapRenderModeModel"));
    assert!(!browser_mod.contains("pub struct MapPointModel"));
    assert!(!browser_mod.contains("pub struct MapPanelModel"));
    assert!(!waveform_mod.contains("pub enum WaveformChannelViewModel"));
    assert!(browser_mod.contains("pub use crate::gui::selection::TriState as BrowserTagState;"));
    assert!(browser_mod.contains(
        "pub type BrowserTagPillModel = crate::gui::badge::SelectablePill<BrowserTagState>;"
    ));
    assert!(browser_mod.contains(
        "pub type BrowserTagSidebarModel = crate::gui::badge::PillEditorPanel<BrowserTagState>;"
    ));
    assert!(
        browser_mod
            .contains("pub use crate::gui::visualization::PointRenderMode as MapRenderModeModel;")
    );
    assert!(
        browser_mod.contains("pub use crate::gui::visualization::SpatialPoint as MapPointModel;")
    );
    assert!(
        browser_mod.contains("pub use crate::gui::visualization::SpatialPanel as MapPanelModel;")
    );
    assert!(waveform_mod.contains(
        "pub use crate::gui::visualization::ChannelViewMode as WaveformChannelViewModel;"
    ));
    assert!(selection_mod.contains("pub enum TriState"));
    assert!(badge_mod.contains("pub struct SelectablePill<State>"));
    assert!(badge_mod.contains("pub struct PillEditorPanel<State>"));
    assert!(visualization_mod.contains("pub enum PointRenderMode"));
    assert!(visualization_mod.contains("pub struct SpatialPoint"));
    assert!(visualization_mod.contains("pub struct SpatialPanel"));
    assert!(visualization_mod.contains("pub enum ChannelViewMode"));
}

#[test]
fn frame_and_invalidation_models_are_owned_by_generic_modules() {
    let app_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/mod.rs"))
        .expect("app module should be readable");
    let dirty_segments_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/app/dirty_segments.rs"
    ))
    .expect("dirty segments module should be readable");
    let frame_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/frame.rs"))
        .expect("frame module should be readable");
    let invalidation_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/invalidation.rs"
    ))
    .expect("invalidation module should be readable");

    assert!(!dirty_segments_mod.contains("pub struct FrameBuildResult"));
    assert!(app_mod.contains("pub use crate::gui::frame::FrameBuildResult;"));
    assert!(frame_mod.contains("pub struct FrameBuildResult"));
    assert!(dirty_segments_mod.contains("use crate::gui::invalidation::InvalidationMask;"));
    assert!(invalidation_mod.contains("pub struct InvalidationMask"));
}

#[test]
fn automation_snapshot_primitives_are_owned_by_generic_gui_module() {
    let app_automation_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/app/automation.rs"
    ))
    .expect("app automation module should be readable");
    let gui_automation_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/automation.rs"
    ))
    .expect("generic automation module should be readable");

    assert!(
        app_automation_mod
            .contains("pub use crate::gui::automation::{AutomationBounds, AutomationNodeId};")
    );
    assert!(gui_automation_mod.contains("pub struct AutomationNodeSnapshot"));
    assert!(gui_automation_mod.contains("pub enum AutomationRole"));
    assert!(gui_automation_mod.contains("pub struct GuiAutomationSnapshot"));
    assert!(gui_automation_mod.contains("TimelineRegion"));
    assert!(gui_automation_mod.contains("SpatialCanvas"));
    assert!(!gui_automation_mod.contains("WaveformRegion"));
    assert!(!gui_automation_mod.contains("MapCanvas"));
}
