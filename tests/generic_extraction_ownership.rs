//! Ownership guardrails for primitives already extracted into generic Radiant modules.

use std::fs;

#[test]
fn keypress_value_type_is_owned_by_generic_input_module() {
    let app_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/mod.rs"
    ))
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
    let app_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/mod.rs"
    ))
    .expect("app module should be readable");
    let browser_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/browser.rs"
    ))
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
    let waveform_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/waveform.rs"
    ))
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
    let browser_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/browser.rs"
    ))
    .expect("browser module should be readable");
    let list_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/list.rs"))
        .expect("list module should be readable");

    assert!(!browser_mod.contains("pub struct BrowserRowModel"));
    assert!(browser_mod.contains("pub use crate::gui::list::ContentListRow as BrowserRowModel;"));
    assert!(!browser_mod.contains("pub struct BrowserPanelModel"));
    assert!(browser_mod.contains(
        "pub type BrowserPanelModel =\n    crate::gui::list::ContentListPanel<BrowserRowModel, BrowserPillEditorModel>;"
    ));
    assert!(!browser_mod.contains("pub enum BrowserRowProcessingState"));
    assert!(
        browser_mod
            .contains("pub use crate::gui::list::RowProcessingState as BrowserRowProcessingState;")
    );
    assert!(list_mod.contains("pub struct ContentListPanel<Row, Editor>"));
    assert!(list_mod.contains("pub fn pill_editor(&self) -> &Editor"));
    assert!(!list_mod.contains("source_loading"));
    assert!(!list_mod.contains("active_playback_age_filters"));
    assert!(!list_mod.contains("selected_path_count"));
    assert!(!list_mod.contains("file_op_pending"));
    assert!(list_mod.contains("pub data_loading: bool"));
    assert!(list_mod.contains("pub active_recency_filters: [bool; 3]"));
    assert!(list_mod.contains("pub selected_item_count: usize"));
    assert!(list_mod.contains("pub mutation_pending: bool"));
    assert!(list_mod.contains("pub struct ContentListRow"));
    assert!(list_mod.contains("pub fn encode_similarity_display_strength"));
    assert!(!list_mod.contains("sample"));
    assert!(!list_mod.contains("Sample"));
    assert!(list_mod.contains("pub enum RowProcessingState"));
}

#[test]
fn recency_state_is_owned_by_generic_list_module() {
    let browser_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/browser.rs"
    ))
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
    let sources_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/sources.rs"
    ))
    .expect("sources module should be readable");
    let list_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/list.rs"))
        .expect("list module should be readable");

    assert!(!sources_mod.contains("pub struct ColumnModel"));
    assert!(sources_mod.contains("pub use crate::gui::list::ColumnSummary as ColumnModel;"));
    assert!(list_mod.contains("pub struct ColumnSummary"));
}

#[test]
fn editable_row_kind_is_owned_by_generic_list_module() {
    let sources_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/sources.rs"
    ))
    .expect("sources module should be readable");
    let list_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/list.rs"))
        .expect("list module should be readable");

    assert!(!sources_mod.contains("pub enum FolderRowKind"));
    assert!(sources_mod.contains("pub use crate::gui::list::EditableRowKind as FolderRowKind;"));
    assert!(list_mod.contains("pub enum EditableRowKind"));
}

#[test]
fn editable_tree_actions_are_owned_by_generic_list_module() {
    let sources_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/sources.rs"
    ))
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
    let sources_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/sources.rs"
    ))
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
    let sources_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/sources.rs"
    ))
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
    assert!(panel_mod.contains("pub fn select<'a, T>"));
    assert!(panel_mod.contains("pub fn select_mut<'a, T>"));
    assert!(panel_mod.contains("pub struct SplitPaneAssignedRow"));
    assert!(panel_mod.contains("pub struct SplitPaneTreePanel<Row = EditableTreeRow>"));
}

#[test]
fn focus_context_model_is_owned_by_generic_focus_module() {
    let sources_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/sources.rs"
    ))
    .expect("sources module should be readable");
    let focus_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/focus.rs"))
        .expect("focus module should be readable");

    assert!(!sources_mod.contains("pub enum FocusContextModel"));
    assert!(sources_mod.contains("pub use crate::gui::focus::FocusSurface as FocusContextModel;"));
    assert!(focus_mod.contains("pub enum FocusSurface"));
    assert!(focus_mod.contains("ContentList"));
    assert!(focus_mod.contains("NavigationTree"));
    assert!(!focus_mod.contains("SampleBrowser"));
    assert!(!focus_mod.contains("SourceFolders"));
}

#[test]
fn status_segments_are_owned_by_generic_chrome_module() {
    let shell_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/shell.rs"
    ))
    .expect("shell module should be readable");
    let chrome_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/chrome.rs"))
        .expect("chrome module should be readable");

    assert!(!shell_mod.contains("pub struct StatusBarModel"));
    assert!(shell_mod.contains("pub use crate::gui::chrome::StatusSegments as StatusBarModel;"));
    assert!(chrome_mod.contains("pub struct StatusSegments"));
}

#[test]
fn feedback_models_are_owned_by_generic_feedback_module() {
    let shell_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/shell.rs"
    ))
    .expect("shell module should be readable");
    let sources_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/sources.rs"
    ))
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
    assert!(!shell_mod.contains("pub enum ConfirmPromptKind"));
    assert!(!shell_mod.contains("AudioEngineChipStateModel"));
    assert!(!shell_mod.contains("AudioEngineModel"));
    assert!(!shell_mod.contains("audio_engine"));
    assert!(!shell_mod.contains("output_host"));
    assert!(!shell_mod.contains("input_sample_rate"));
    assert!(!sources_mod.contains("pub struct FolderRecoveryModel"));
    assert!(
        shell_mod
            .contains("pub use crate::gui::feedback::ProgressOverlay as ProgressOverlayModel;")
    );
    assert!(shell_mod.contains("pub use crate::gui::feedback::DragOverlay as DragOverlayModel;"));
    assert!(shell_mod.contains("pub use crate::gui::feedback::UpdatePanel as UpdatePanelModel;"));
    assert!(shell_mod.contains("pub use crate::gui::feedback::UpdateStatus as UpdateStatusModel;"));
    assert!(
        shell_mod.contains("pub use crate::gui::feedback::HealthState as StatusChipStateModel;")
    );
    assert!(!shell_mod.contains("pub struct PairedDevicePanelModel"));
    assert!(
        shell_mod
            .contains("pub use crate::gui::form::PairedStatusPanel as PairedDevicePanelModel;")
    );
    assert!(shell_mod.contains("pub paired_device: PairedDevicePanelModel"));
    assert!(shell_mod.contains("pub use crate::gui::feedback::PromptIntent as ConfirmPromptKind;"));
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
    assert!(feedback_mod.contains("pub enum PromptIntent"));
    assert!(feedback_mod.contains("DestructiveOperation"));
    assert!(feedback_mod.contains("RenameContent"));
    assert!(feedback_mod.contains("CreateNavigationItem"));
    assert!(!feedback_mod.contains("BrowserRename"));
    assert!(!feedback_mod.contains("FolderRename"));
    assert!(feedback_mod.contains("pub struct ConfirmPrompt<Kind>"));
}

#[test]
fn paired_picker_models_are_owned_by_generic_form_module() {
    let shell_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/shell.rs"
    ))
    .expect("shell module should be readable");
    let form_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/form.rs"))
        .expect("form module should be readable");

    assert!(!shell_mod.contains("AudioPickerTargetModel"));
    assert!(!shell_mod.contains("AudioOptionValueModel"));
    assert!(!shell_mod.contains("AudioOptionItemModel"));
    assert!(!shell_mod.contains("AudioFieldModel"));
    assert!(
        shell_mod
            .contains("pub use crate::gui::form::PairedPickerTarget as PairedPickerTargetModel;")
    );
    assert!(shell_mod.contains(
        "pub type PairedPickerValueModel = crate::gui::form::PairedPickerValue<String, u32>;"
    ));
    assert!(shell_mod.contains(
        "pub type PairedPickerOptionModel = crate::gui::form::OptionItem<PairedPickerValueModel>;"
    ));
    assert!(shell_mod.contains("pub use crate::gui::form::SummaryField as SummaryFieldModel;"));
    assert!(form_mod.contains("pub enum PairedPickerTarget"));
    assert!(form_mod.contains("pub enum PairedPickerValue"));
    assert!(form_mod.contains("pub struct PairedStatusPanel"));
    assert!(form_mod.contains("pub fn options_for(&self, target: PairedPickerTarget)"));
}

#[test]
fn paired_picker_actions_use_generic_device_terms() {
    let actions_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/actions/mod.rs"
    ))
    .expect("actions module should be readable");
    let automation_helpers = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/automation/helpers.rs"
    ))
    .expect("automation helpers should be readable");
    let shared_options_actions = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../src/app_core/native_shell/composition/state/options_panel/actions.rs"
    ))
    .expect("shared options-panel actions should be readable");

    for source in [&actions_mod, &automation_helpers, &shared_options_actions] {
        assert!(!source.contains("OpenAudio"));
        assert!(!source.contains("SetAudio"));
        assert!(!source.contains("open_audio"));
        assert!(!source.contains("set_audio"));
    }
    assert!(actions_mod.contains("OpenPrimaryGroupPicker"));
    assert!(actions_mod.contains("SetSecondaryNumber"));
    assert!(automation_helpers.contains("open_primary_group_picker"));
    assert!(automation_helpers.contains("set_secondary_number"));
    assert!(shared_options_actions.contains("UiAction::OpenPrimaryGroupPicker"));
    assert!(shared_options_actions.contains("UiAction::SetSecondaryNumber"));
}

#[test]
fn selection_badge_and_visualization_models_are_owned_by_generic_modules() {
    let browser_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/browser.rs"
    ))
    .expect("browser module should be readable");
    let waveform_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/waveform.rs"
    ))
    .expect("waveform module should be readable");
    let actions_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/actions/mod.rs"
    ))
    .expect("actions module should be readable");
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

    assert!(!browser_mod.contains("pub enum BrowserPillState"));
    assert!(!browser_mod.contains("pub struct BrowserPillModel"));
    assert!(!browser_mod.contains("pub struct BrowserPillEditorModel"));
    assert!(!browser_mod.contains("BrowserTagState"));
    assert!(!browser_mod.contains("BrowserTagPillModel"));
    assert!(!browser_mod.contains("BrowserTagSidebarModel"));
    assert!(!actions_mod.contains("BrowserTagTarget"));
    assert!(!actions_mod.contains("pub enum BrowserTriageTarget"));
    assert!(!browser_mod.contains("pub enum MapRenderModeModel"));
    assert!(!browser_mod.contains("pub struct MapPointModel"));
    assert!(!browser_mod.contains("pub struct MapPanelModel"));
    assert!(!waveform_mod.contains("pub enum WaveformChannelViewModel"));
    assert!(!waveform_mod.contains("pub struct WaveformSlicePreviewModel"));
    assert!(!waveform_mod.contains("pub struct WaveformViewportModel"));
    assert!(!waveform_mod.contains("pub struct WaveformEditPreviewModel"));
    assert!(!waveform_mod.contains("pub struct WaveformImagePreviewModel"));
    assert!(!waveform_mod.contains("pub struct WaveformChromeStateModel"));
    assert!(browser_mod.contains("pub use crate::gui::selection::TriState as BrowserPillState;"));
    assert!(
        actions_mod.contains("pub type BrowserTriageTarget = crate::gui::selection::TriageTarget;")
    );
    assert!(selection_mod.contains("pub enum TriageTarget"));
    assert!(browser_mod.contains(
        "pub type BrowserPillModel = crate::gui::badge::SelectablePill<BrowserPillState>;"
    ));
    assert!(browser_mod.contains(
        "pub type BrowserPillEditorModel = crate::gui::badge::PillEditorPanel<BrowserPillState>;"
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
    assert!(waveform_mod.contains(
        "pub use crate::gui::visualization::TimelineMarkerPreview as WaveformSlicePreviewModel;"
    ));
    assert!(
        waveform_mod.contains(
            "pub use crate::gui::visualization::TimelineViewport as WaveformViewportModel;"
        )
    );
    assert!(waveform_mod.contains(
        "pub use crate::gui::visualization::TimelineEditPreview as WaveformEditPreviewModel;"
    ));
    assert!(waveform_mod.contains(
        "pub use crate::gui::visualization::SignalRasterPreview as WaveformImagePreviewModel;"
    ));
    assert!(waveform_mod.contains(
        "pub use crate::gui::visualization::SignalChromeState as WaveformChromeStateModel;"
    ));
    assert!(selection_mod.contains("pub enum TriState"));
    assert!(badge_mod.contains("pub struct SelectablePill<State>"));
    assert!(badge_mod.contains("pub struct PillEditorPanel<State>"));
    assert!(visualization_mod.contains("pub enum PointRenderMode"));
    assert!(visualization_mod.contains("pub struct SpatialPoint"));
    assert!(visualization_mod.contains("pub struct SpatialPanel"));
    assert!(visualization_mod.contains("pub enum ChannelViewMode"));
    assert!(visualization_mod.contains("pub struct TimelineMarkerPreview"));
    assert!(visualization_mod.contains("pub struct TimelineViewport"));
    assert!(visualization_mod.contains("pub struct TimelineEditPreview"));
    assert!(visualization_mod.contains("pub struct SignalRasterPreview"));
    assert!(visualization_mod.contains("pub struct SignalChromeState"));
    assert!(!visualization_mod.contains("waveform"));
    assert!(!visualization_mod.contains("Waveform"));
}

#[test]
fn compat_shell_defaults_do_not_bake_in_sample_browser_copy() {
    let browser_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/browser.rs"
    ))
    .expect("browser module should be readable");
    let shell_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/shell.rs"
    ))
    .expect("shell module should be readable");
    let chrome_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/chrome.rs"))
        .expect("chrome module should be readable");

    for source in [&browser_mod, &shell_mod, &chrome_mod] {
        assert!(
            !source.contains("Search samples")
                && !source.contains("Similarity map")
                && !source.contains("ColumnModel::new(\"Samples\""),
            "Radiant compatibility defaults must stay product-neutral; host projections supply product labels"
        );
    }

    assert!(!browser_mod.contains("pub struct BrowserChromeModel"));
    assert!(
        browser_mod
            .contains("pub use crate::gui::chrome::ContentViewChrome as BrowserChromeModel;")
    );
    assert!(chrome_mod.contains("pub struct ContentViewChrome"));
    assert!(chrome_mod.contains("String::from(\"Search items (Ctrl+F)\")"));
    assert!(chrome_mod.contains("items_tab_label: String::from(\"Items\")"));
    assert!(chrome_mod.contains("map_tab_label: String::from(\"Map\")"));
    assert!(shell_mod.contains("ColumnModel::new(\"Items\", 0)"));
}

#[test]
fn compat_browser_contract_uses_generic_item_label_fields() {
    let browser_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/browser.rs"
    ))
    .expect("browser module should be readable");
    let list_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/list.rs"))
        .expect("list module should be readable");
    let chrome_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/chrome.rs"))
        .expect("chrome module should be readable");
    let automation_browser = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/automation/browser.rs"
    ))
    .expect("automation browser should be readable");
    let frame_text_cache = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/frame_text_cache.rs"
    ))
    .expect("frame text cache should be readable");

    for source in [&browser_mod, &automation_browser, &frame_text_cache] {
        assert!(!source.contains("focused_sample_label"));
        assert!(!source.contains("samples_tab_label"));
        assert!(!source.contains("can_normalize_focused_sample"));
        assert!(!source.contains("can_loop_crossfade_focused_sample"));
        assert!(!source.contains("samples_tab_text"));
    }

    assert!(list_mod.contains("focused_item_label"));
    assert!(chrome_mod.contains("items_tab_label"));
    assert!(!browser_mod.contains("can_normalize_focused_item"));
    assert!(!browser_mod.contains("can_loop_crossfade_focused_item"));
    assert!(list_mod_contains_content_actions());
    assert!(automation_browser.contains("focused_item_label"));
    assert!(frame_text_cache.contains("items_tab_text"));
}

fn list_mod_contains_content_actions() -> bool {
    let list_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/list.rs"))
        .expect("list module should be readable");
    list_mod.contains("pub struct ContentListActions")
        && list_mod.contains("pub can_process_focused_item: bool")
        && list_mod.contains("pub can_open_focused_item_flow: bool")
        && !list_mod.contains("can_normalize_focused_item")
        && !list_mod.contains("can_loop_crossfade_focused_item")
}

#[test]
fn compat_action_catalog_uses_generic_item_language_for_discard_flow() {
    let actions_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/actions/mod.rs"
    ))
    .expect("actions module should be readable");
    let automation_helpers = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/automation/helpers.rs"
    ))
    .expect("automation helpers should be readable");

    assert!(!actions_mod.contains("MoveTrashedSamplesToFolder"));
    assert!(actions_mod.contains("MoveDiscardedItemsToFolder"));
    assert!(!automation_helpers.contains("move_trashed_samples_to_folder"));
    assert!(automation_helpers.contains("move_discarded_items_to_folder"));
}

#[test]
fn compat_action_catalog_uses_generic_loaded_content_focus_action() {
    let actions_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/actions/mod.rs"
    ))
    .expect("actions module should be readable");
    let automation_helpers = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/automation/helpers.rs"
    ))
    .expect("automation helpers should be readable");
    let pointer_routing = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui_runtime/native_vello/input/pointer.rs"
    ))
    .expect("pointer routing should be readable");

    for source in [&actions_mod, &automation_helpers, &pointer_routing] {
        assert!(!source.contains("FocusLoadedSampleInBrowser"));
        assert!(!source.contains("focus_loaded_sample_in_browser"));
    }
    assert!(actions_mod.contains("FocusLoadedContentInList"));
    assert!(automation_helpers.contains("focus_loaded_content_in_list"));
    assert!(pointer_routing.contains("UiAction::FocusLoadedContentInList"));
}

#[test]
fn compat_action_catalog_uses_generic_compare_anchor_action() {
    let actions_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/actions/mod.rs"
    ))
    .expect("actions module should be readable");
    let automation_helpers = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/automation/helpers.rs"
    ))
    .expect("automation helpers should be readable");

    for source in [&actions_mod, &automation_helpers] {
        assert!(!source.contains("SetCompareAnchorFromFocusedBrowserSample"));
        assert!(!source.contains("set_compare_anchor_from_focused_browser_sample"));
    }
    assert!(actions_mod.contains("SetCompareAnchorFromFocusedContent"));
    assert!(automation_helpers.contains("set_compare_anchor_from_focused_content"));
}

#[test]
fn compat_action_catalog_uses_generic_content_mark_action() {
    let actions_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/actions/mod.rs"
    ))
    .expect("actions module should be readable");
    let automation_helpers = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/automation/helpers.rs"
    ))
    .expect("automation helpers should be readable");
    let key_bindings = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui_runtime/native_vello/tests/key_bindings.rs"
    ))
    .expect("key binding tests should be readable");

    for source in [&actions_mod, &automation_helpers, &key_bindings] {
        assert!(!source.contains("ToggleBrowserSampleMark"));
        assert!(!source.contains("toggle_browser_sample_mark"));
    }
    assert!(actions_mod.contains("ToggleContentMark"));
    assert!(automation_helpers.contains("toggle_content_mark"));
    assert!(key_bindings.contains("UiAction::ToggleContentMark"));
}

#[test]
fn compat_action_catalog_uses_generic_browser_triage_mark_action() {
    let actions_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/actions/mod.rs"
    ))
    .expect("actions module should be readable");
    let automation_helpers = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/automation/helpers.rs"
    ))
    .expect("automation helpers should be readable");

    for source in [&actions_mod, &automation_helpers] {
        assert!(!source.contains("TagBrowserSelection"));
        assert!(!source.contains("tag_browser_selection"));
    }
    assert!(actions_mod.contains("SetBrowserTriageMark"));
    assert!(automation_helpers.contains("set_browser_triage_mark"));
}

#[test]
fn compat_action_catalog_uses_generic_pill_editor_input_actions() {
    let actions_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/actions/mod.rs"
    ))
    .expect("actions module should be readable");
    let automation_helpers = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/automation/helpers.rs"
    ))
    .expect("automation helpers should be readable");
    let automation_browser = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/automation/browser.rs"
    ))
    .expect("automation browser should be readable");
    let text_runtime = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui_runtime/native_vello/text_runtime.rs"
    ))
    .expect("text runtime should be readable");
    let runtime_state = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui_runtime/native_vello/runtime_state.rs"
    ))
    .expect("runtime state should be readable");

    for source in [
        &actions_mod,
        &automation_helpers,
        &automation_browser,
        &text_runtime,
    ] {
        assert!(!source.contains("BrowserTagSidebarInput"));
        assert!(!source.contains("browser_tag_sidebar_input"));
    }
    for source in [&runtime_state, &text_runtime] {
        assert!(!source.contains("BrowserTagSidebar"));
        assert!(source.contains("BrowserPillEditor"));
    }
    assert!(actions_mod.contains("FocusBrowserPillEditorInput"));
    assert!(actions_mod.contains("SetBrowserPillEditorInput"));
    assert!(actions_mod.contains("CommitBrowserPillEditorInput"));
    assert!(automation_helpers.contains("focus_browser_pill_editor_input"));
    assert!(automation_helpers.contains("set_browser_pill_editor_input"));
    assert!(automation_helpers.contains("commit_browser_pill_editor_input"));
}

#[test]
fn compat_action_catalog_uses_generic_pill_editor_toggle_actions() {
    let actions_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/actions/mod.rs"
    ))
    .expect("actions module should be readable");
    let automation_helpers = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/automation/helpers.rs"
    ))
    .expect("automation helpers should be readable");

    for source in [&actions_mod, &automation_helpers] {
        assert!(!source.contains("ToggleBrowserTagSidebar"));
        assert!(!source.contains("toggle_browser_tag_sidebar"));
    }
    assert!(actions_mod.contains("ToggleBrowserPillEditor"));
    assert!(actions_mod.contains("ToggleBrowserPillEditorPrimaryAction"));
    assert!(automation_helpers.contains("toggle_browser_pill_editor"));
    assert!(automation_helpers.contains("toggle_browser_pill_editor_primary_action"));
}

#[test]
fn compat_shell_uses_generic_pill_editor_layout_identifiers() {
    let state_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state.rs"
    ))
    .expect("state module should be readable");
    let model_sync = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/model_sync.rs"
    ))
    .expect("model sync should be readable");
    let text_runtime = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui_runtime/native_vello/text_runtime.rs"
    ))
    .expect("text runtime should be readable");

    for source in [&state_mod, &model_sync, &text_runtime] {
        assert!(!source.contains("browser_tag_sidebar_editor_visual"));
        assert!(!source.contains("set_browser_tag_sidebar_editor_state"));
        assert!(!source.contains("sync_browser_tag_sidebar_editor_state"));
    }
    assert!(text_runtime.contains("sync_browser_pill_editor_state"));
    assert!(model_sync.contains("set_browser_pill_editor_visual_state"));
    assert!(state_mod.contains("browser_pill_editor_visual"));
}

#[test]
fn compat_browser_model_uses_generic_pill_editor_fields() {
    let browser_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/browser.rs"
    ))
    .expect("browser module should be readable");
    let text_runtime = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui_runtime/native_vello/text_runtime.rs"
    ))
    .expect("text runtime should be readable");
    let automation_browser = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/automation/browser.rs"
    ))
    .expect("automation browser should be readable");

    for source in [&browser_mod, &text_runtime, &automation_browser] {
        assert!(!source.contains("tag_sidebar: BrowserPillEditorModel"));
        assert!(!source.contains("tag_sidebar_open: bool"));
        assert!(!source.contains("model.browser.tag_sidebar"));
        assert!(!source.contains("browser.tag_sidebar"));
        assert!(!source.contains("normal_tag_labels"));
        assert!(!source.contains("tag_state"));
        assert!(!source.contains("tag_id"));
    }
    assert!(browser_mod.contains("pub pill_editor: BrowserPillEditorModel"));
    assert!(browser_mod.contains("pub pill_editor_open: bool"));
    assert!(text_runtime.contains("self.model.browser.pill_editor.input_value"));
    assert!(automation_browser.contains("model.browser.pill_editor"));
    assert!(automation_browser.contains("browser.pill_editor"));
    assert!(automation_browser.contains("option_pill_labels"));
    assert!(automation_browser.contains("pill_state"));
    assert!(automation_browser.contains("pill_id"));
}

#[test]
fn compat_browser_actions_use_generic_pill_edit_capability() {
    let browser_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/browser.rs"
    ))
    .expect("browser module should be readable");
    let toolbar_layout_tests = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../src/app_core/native_shell/composition/state/tests/browser_toolbar/layout.rs"
    ))
    .expect("toolbar layout tests should be readable");

    for source in [&browser_mod, &toolbar_layout_tests] {
        assert!(!source.contains("can_tag"));
    }
    assert!(!browser_mod.contains("pub struct BrowserActionsModel"));
    assert!(
        browser_mod
            .contains("pub use crate::gui::list::ContentListActions as BrowserActionsModel;")
    );
    assert!(toolbar_layout_tests.contains("can_edit_pills"));
}

#[test]
fn compat_action_catalog_uses_generic_pill_option_action() {
    let actions_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/actions/mod.rs"
    ))
    .expect("actions module should be readable");
    let automation_helpers = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/automation/helpers.rs"
    ))
    .expect("automation helpers should be readable");
    let automation_browser = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/automation/browser.rs"
    ))
    .expect("automation browser should be readable");

    for source in [&actions_mod, &automation_helpers, &automation_browser] {
        assert!(!source.contains("ToggleBrowserSidebarNormalTag"));
        assert!(!source.contains("toggle_browser_sidebar_normal_tag"));
    }
    assert!(actions_mod.contains("ToggleBrowserPillOption"));
    assert!(automation_helpers.contains("toggle_browser_pill_option"));
    assert!(automation_browser.contains("toggle_browser_pill_option"));
}

#[test]
fn compat_action_catalog_uses_generic_derived_label_filter_action() {
    let actions_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/actions/mod.rs"
    ))
    .expect("actions module should be readable");
    let automation_helpers = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/automation/helpers.rs"
    ))
    .expect("automation helpers should be readable");
    let automation_browser = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/automation/browser.rs"
    ))
    .expect("automation browser should be readable");

    for source in [&actions_mod, &automation_helpers, &automation_browser] {
        assert!(!source.contains("ToggleBrowserTagNamedFilter"));
        assert!(!source.contains("toggle_browser_tag_named_filter"));
    }
    assert!(actions_mod.contains("ToggleBrowserDerivedLabelFilter"));
    assert!(automation_helpers.contains("toggle_browser_derived_label_filter"));
    assert!(automation_browser.contains("toggle_browser_derived_label_filter"));
}

#[test]
fn compat_browser_model_uses_generic_derived_label_filter_fields() {
    let app_browser = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/browser.rs"
    ))
    .expect("app browser model should be readable");
    let shared_toolbar = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../src/app_core/native_shell/composition/browser_chrome_surface.rs"
    ))
    .expect("shared browser chrome surface should be readable");
    let shared_hit_testing = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../src/app_core/native_shell/composition/state/hit_testing/browser.rs"
    ))
    .expect("shared browser hit testing should be readable");
    let automation_browser = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/automation/browser.rs"
    ))
    .expect("automation browser should be readable");

    for source in [
        &app_browser,
        &shared_toolbar,
        &shared_hit_testing,
        &automation_browser,
    ] {
        assert!(!source.contains("tag_named_filter_active"));
        assert!(!source.contains("tag_named_filter_negated"));
        assert!(!source.contains("tag_named_filter_chip"));
        assert!(!source.contains("browser.tag_named_filter"));
    }
    assert!(app_browser.contains("derived_label_filter_active"));
    assert!(shared_toolbar.contains("derived_label_filter_chip"));
    assert!(shared_hit_testing.contains("browser_derived_label_filter_chip_rect"));
    assert!(automation_browser.contains("browser.derived_label_filter"));
}

#[test]
fn compat_action_catalog_uses_generic_find_similar_action() {
    let actions_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/actions/mod.rs"
    ))
    .expect("actions module should be readable");
    let automation_helpers = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/automation/helpers.rs"
    ))
    .expect("automation helpers should be readable");
    let key_bindings = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui_runtime/native_vello/tests/key_bindings.rs"
    ))
    .expect("key binding tests should be readable");
    let browser_pointer = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui_runtime/native_vello/tests/browser_pointer/browser_rows.rs"
    ))
    .expect("browser pointer tests should be readable");

    for source in [
        &actions_mod,
        &automation_helpers,
        &key_bindings,
        &browser_pointer,
    ] {
        assert!(!source.contains("ToggleFindSimilarFocusedSample"));
        assert!(!source.contains("toggle_find_similar_focused_sample"));
    }
    assert!(actions_mod.contains("ToggleFindSimilarFocusedContent"));
    assert!(automation_helpers.contains("toggle_find_similar_focused_content"));
    assert!(key_bindings.contains("UiAction::ToggleFindSimilarFocusedContent"));
    assert!(browser_pointer.contains("UiAction::ToggleFindSimilarFocusedContent"));
}

#[test]
fn compat_action_catalog_uses_generic_normalize_focused_content_action() {
    let actions_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/actions/mod.rs"
    ))
    .expect("actions module should be readable");
    let automation_helpers = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/automation/helpers.rs"
    ))
    .expect("automation helpers should be readable");
    let key_bindings = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui_runtime/native_vello/tests/key_bindings.rs"
    ))
    .expect("key binding tests should be readable");

    for source in [&actions_mod, &automation_helpers, &key_bindings] {
        assert!(!source.contains("NormalizeFocusedBrowserSample"));
        assert!(!source.contains("normalize_focused_browser_sample"));
    }
    assert!(actions_mod.contains("NormalizeFocusedContentItem"));
    assert!(automation_helpers.contains("normalize_focused_content_item"));
    assert!(key_bindings.contains("UiAction::NormalizeFocusedContentItem"));
}

#[test]
fn compat_action_catalog_uses_generic_random_content_actions() {
    let actions_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/actions/mod.rs"
    ))
    .expect("actions module should be readable");
    let automation_helpers = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/automation/helpers.rs"
    ))
    .expect("automation helpers should be readable");

    for source in [&actions_mod, &automation_helpers] {
        assert!(!source.contains("PlayRandomSample"));
        assert!(!source.contains("PlayPreviousRandomSample"));
        assert!(!source.contains("play_random_sample"));
        assert!(!source.contains("play_previous_random_sample"));
    }
    assert!(actions_mod.contains("PlayRandomContentItem"));
    assert!(actions_mod.contains("PlayPreviousRandomContentItem"));
    assert!(automation_helpers.contains("play_random_content_item"));
    assert!(automation_helpers.contains("play_previous_random_content_item"));
}

#[test]
fn compat_action_catalog_uses_generic_spatial_content_focus_action() {
    let actions_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/actions/mod.rs"
    ))
    .expect("actions module should be readable");
    let automation_helpers = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/automation/helpers.rs"
    ))
    .expect("automation helpers should be readable");
    let automation_browser = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/automation/browser.rs"
    ))
    .expect("automation browser should be readable");
    let runtime_actions = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui_runtime/native_vello/runtime_actions.rs"
    ))
    .expect("runtime actions should be readable");
    let runtime_drag = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui_runtime/native_vello/runtime_input/drag.rs"
    ))
    .expect("runtime drag should be readable");
    let runtime_pointer = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui_runtime/native_vello/input/pointer.rs"
    ))
    .expect("runtime pointer should be readable");

    for source in [
        &actions_mod,
        &automation_helpers,
        &automation_browser,
        &runtime_actions,
        &runtime_drag,
        &runtime_pointer,
    ] {
        assert!(!source.contains("FocusMapSample"));
        assert!(!source.contains("focus_map_sample"));
        assert!(!source.contains("map_sample_action_at_point"));
    }
    assert!(actions_mod.contains("FocusSpatialContentItem"));
    assert!(actions_mod.contains("content_id: String"));
    assert!(automation_helpers.contains("focus_spatial_content_item"));
    assert!(automation_browser.contains("focus_spatial_content_item"));
    assert!(runtime_actions.contains("UiAction::FocusSpatialContentItem"));
    assert!(runtime_drag.contains("UiAction::FocusSpatialContentItem"));
    assert!(runtime_drag.contains("map_content_action_at_point"));
    assert!(runtime_pointer.contains("map_content_action_at_point"));
}

#[test]
fn compat_action_catalog_uses_generic_content_item_drag_actions() {
    let actions_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/actions/mod.rs"
    ))
    .expect("actions module should be readable");
    let automation_helpers = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/automation/helpers.rs"
    ))
    .expect("automation helpers should be readable");
    let runtime_shell = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui_runtime/native_vello.rs"
    ))
    .expect("runtime shell should be readable");
    let runtime_state = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui_runtime/native_vello/runtime_state.rs"
    ))
    .expect("runtime state should be readable");
    let runtime_drag = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui_runtime/native_vello/runtime_input/drag.rs"
    ))
    .expect("runtime drag should be readable");

    for source in [
        &actions_mod,
        &automation_helpers,
        &runtime_shell,
        &runtime_state,
        &runtime_drag,
    ] {
        assert!(!source.contains("StartBrowserSampleDrag"));
        assert!(!source.contains("UpdateBrowserSampleDrag"));
        assert!(!source.contains("FinishBrowserSampleDrag"));
        assert!(!source.contains("start_browser_sample_drag"));
        assert!(!source.contains("update_browser_sample_drag"));
        assert!(!source.contains("finish_browser_sample_drag"));
        assert!(!source.contains("BrowserSampleDrag"));
        assert!(!source.contains("browser_sample_drag"));
        assert!(!source.contains("browser-sample drag"));
    }
    assert!(actions_mod.contains("StartContentItemDrag"));
    assert!(actions_mod.contains("UpdateContentItemDrag"));
    assert!(actions_mod.contains("FinishContentItemDrag"));
    assert!(automation_helpers.contains("start_content_item_drag"));
    assert!(automation_helpers.contains("update_content_item_drag"));
    assert!(automation_helpers.contains("finish_content_item_drag"));
    assert!(runtime_shell.contains("content_item_drag: Option<ContentItemDragState>"));
    assert!(runtime_state.contains("pub(super) struct ContentItemDragState"));
    assert!(runtime_drag.contains("UiAction::StartContentItemDrag"));
}

#[test]
fn compat_action_catalog_uses_generic_waveform_content_actions() {
    let actions_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/actions/mod.rs"
    ))
    .expect("actions module should be readable");
    let automation_helpers = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/automation/helpers.rs"
    ))
    .expect("automation helpers should be readable");

    for source in [&actions_mod, &automation_helpers] {
        assert!(!source.contains("NormalizeWaveformSelectionOrSample"));
        assert!(!source.contains("CropWaveformSelectionToNewSample"));
        assert!(!source.contains("DeleteLoadedWaveformSample"));
        assert!(!source.contains("normalize_waveform_selection_or_sample"));
        assert!(!source.contains("crop_waveform_selection_to_new_sample"));
        assert!(!source.contains("delete_loaded_waveform_sample"));
    }
    assert!(actions_mod.contains("NormalizeWaveformSelectionOrLoadedContent"));
    assert!(actions_mod.contains("CropWaveformSelectionToNewContentItem"));
    assert!(actions_mod.contains("DeleteLoadedWaveformContent"));
    assert!(automation_helpers.contains("normalize_waveform_selection_or_loaded_content"));
    assert!(automation_helpers.contains("crop_waveform_selection_to_new_content_item"));
    assert!(automation_helpers.contains("delete_loaded_waveform_content"));
}

#[test]
fn frame_and_invalidation_models_are_owned_by_generic_modules() {
    let app_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/mod.rs"
    ))
    .expect("app module should be readable");
    let dirty_segments_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/dirty_segments.rs"
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
        "/src/compat/legacy_shell/automation.rs"
    ))
    .expect("app automation module should be readable");
    let gui_automation_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/automation.rs"
    ))
    .expect("generic automation module should be readable");

    assert!(!app_automation_mod.contains("pub enum AutomationRole"));
    assert!(!app_automation_mod.contains("pub struct AutomationNodeSnapshot"));
    assert!(!app_automation_mod.contains("pub struct GuiAutomationSnapshot"));
    assert!(
        app_automation_mod.contains(
            "pub use crate::gui::automation::{\n    AutomationBounds, AutomationNodeId, AutomationNodeSnapshot, AutomationRole,\n    GuiAutomationSnapshot,\n};"
        )
    );
    assert!(gui_automation_mod.contains("pub struct AutomationNodeSnapshot"));
    assert!(gui_automation_mod.contains("pub enum AutomationRole"));
    assert!(gui_automation_mod.contains("pub struct GuiAutomationSnapshot"));
    assert!(gui_automation_mod.contains("TimelineRegion"));
    assert!(gui_automation_mod.contains("SpatialCanvas"));
    assert!(gui_automation_mod.contains("SpatialPoint"));
    assert!(!gui_automation_mod.contains("WaveformRegion"));
    assert!(!gui_automation_mod.contains("MapCanvas"));
    assert!(!gui_automation_mod.contains("MapPoint"));
}
