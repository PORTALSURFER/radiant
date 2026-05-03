//! Ownership guardrails for primitives already extracted into generic Radiant modules.

use std::fs;

fn host_aliases_mod() -> String {
    fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/aliases.rs"
    ))
    .expect("host compatibility aliases module should be readable")
}

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
    let aliases_mod = host_aliases_mod();

    assert!(!app_mod.contains("pub struct KeyPress"));
    assert!(app_mod.contains("HotkeyResolution"));
    assert!(aliases_mod.contains("pub use crate::gui::input::KeyPress;"));
    assert!(aliases_mod.contains("pub use crate::gui::shortcuts::ShortcutResolution;"));
    assert!(aliases_mod.contains("pub type HotkeyResolution = ShortcutResolution<UiAction>;"));
    assert!(input_mod.contains("pub struct KeyPress"));
    assert!(shortcuts_mod.contains("pub struct ShortcutResolution<Action>"));
}

#[test]
fn pointer_coordinate_quantization_is_owned_by_generic_input_module() {
    let input_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/input.rs"))
        .expect("input module should be readable");
    let legacy_config_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui_runtime/native_vello/legacy_shell_config.rs"
    ))
    .expect("legacy shell config module should be readable");
    let waveform_press_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui_runtime/native_vello/input/waveform_routing/press.rs"
    ))
    .expect("waveform press routing module should be readable");

    assert!(input_mod.contains("pub fn logical_point_to_u16_coords"));
    assert!(legacy_config_mod.contains("logical_point_to_u16_coords"));
    assert!(waveform_press_mod.contains("ui_action_pointer_coords(point)"));
    assert!(
        !legacy_config_mod.contains("point.x.clamp(0.0"),
        "legacy config should delegate pointer coordinate quantization to gui::input"
    );
    assert!(
        !waveform_press_mod.contains("point.x.max(0.0).round() as u16"),
        "legacy waveform press routing should delegate pointer coordinate quantization"
    );
}

#[test]
fn inline_badge_cluster_layout_is_owned_by_generic_badge_module() {
    let badge_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/badge.rs"))
        .expect("badge module should be readable");
    let inline_metadata_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/toolbar_helpers/browser_row_decor/inline_metadata.rs"
    ))
    .expect("inline metadata module should be readable");

    assert!(badge_mod.contains("pub struct InlineBadgeMetrics"));
    assert!(badge_mod.contains("pub fn inline_badge_rects_for_labels"));
    assert!(badge_mod.contains("pub fn inline_badge_text_origin"));
    assert!(inline_metadata_mod.contains("InlineBadgeMetrics::new"));
    assert!(inline_metadata_mod.contains("inline_badge_rects_for_labels"));
    assert!(inline_metadata_mod.contains("inline_badge_text_origin"));
    assert!(
        !inline_metadata_mod.contains("text.split(\" · \")"),
        "legacy browser metadata wrappers should delegate label splitting to gui::badge"
    );
    assert!(
        !inline_metadata_mod.contains("let mut x = start_x"),
        "legacy browser metadata wrappers should delegate rect placement to gui::badge"
    );
}

#[test]
fn fixed_width_toolbar_row_helpers_are_owned_by_generic_layout_module() {
    let layout_helpers_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/layout_core/row_helpers.rs"
    ))
    .expect("layout row helpers module should be readable");
    let native_controls_shared = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/layout_adapter/controls/shared.rs"
    ))
    .expect("native controls shared module should be readable");
    let top_bar_surface = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/top_bar_surface.rs"
    ))
    .expect("top bar surface module should be readable");

    assert!(layout_helpers_mod.contains("pub fn fixed_width_row_rects_start"));
    assert!(layout_helpers_mod.contains("pub fn fixed_width_row_rects_end"));
    assert!(layout_helpers_mod.contains("pub fn visible_suffix_widths"));
    assert!(native_controls_shared.contains("fixed_width_row_rects_start"));
    assert!(native_controls_shared.contains("fixed_width_row_rects_end"));
    assert!(native_controls_shared.contains("generic_visible_suffix_widths"));
    assert!(top_bar_surface.contains("visible_suffix_widths"));
    assert!(
        !native_controls_shared.contains("LayoutNode::container"),
        "native toolbar wrappers should delegate fixed-width row layout to gui::layout_core"
    );
    assert!(
        !top_bar_surface.contains("let mut reversed = Vec::new()"),
        "top-bar compatibility surface should reuse generic suffix fitting"
    );
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
        "/src/compat/legacy_shell/mod.rs"
    ))
    .expect("browser module should be readable");
    let retained_mod =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/retained.rs"))
            .expect("retained module should be readable");
    let aliases_mod = host_aliases_mod();

    assert!(!browser_mod.contains("pub struct RetainedVec"));
    assert!(app_mod.contains("RetainedVec"));
    assert!(aliases_mod.contains("pub use crate::gui::retained::RetainedVec;"));
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
        "/src/compat/legacy_shell/mod.rs"
    ))
    .expect("browser module should be readable");
    let list_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/list.rs"))
        .expect("list module should be readable");
    let aliases_mod = host_aliases_mod();

    assert!(!browser_mod.contains("pub struct BrowserRowModel"));
    assert!(aliases_mod.contains("pub use crate::gui::list::ContentListRow as BrowserRowModel;"));
    assert!(!browser_mod.contains("pub struct BrowserPanelModel"));
    assert!(aliases_mod.contains(
        "pub type BrowserPanelModel =\n    crate::gui::list::ContentListPanel<BrowserRowModel, BrowserPillEditorModel>;"
    ));
    assert!(!browser_mod.contains("pub enum BrowserRowProcessingState"));
    assert!(
        aliases_mod
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
        "/src/compat/legacy_shell/mod.rs"
    ))
    .expect("browser module should be readable");
    let list_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/list.rs"))
        .expect("list module should be readable");
    let aliases_mod = host_aliases_mod();

    assert!(!browser_mod.contains("pub enum PlaybackAgeFilterChip"));
    assert!(!browser_mod.contains("pub enum PlaybackAgeBucket"));
    assert!(
        aliases_mod
            .contains("pub use crate::gui::list::RecencyFilterChip as PlaybackAgeFilterChip;")
    );
    assert!(aliases_mod.contains("pub use crate::gui::list::RecencyBucket as PlaybackAgeBucket;"));
    assert!(list_mod.contains("pub enum RecencyFilterChip"));
    assert!(list_mod.contains("pub enum RecencyBucket"));
}

#[test]
fn column_summary_is_owned_by_generic_list_module() {
    let sources_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/mod.rs"
    ))
    .expect("sources module should be readable");
    let list_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/list.rs"))
        .expect("list module should be readable");
    let aliases_mod = host_aliases_mod();

    assert!(!sources_mod.contains("pub struct ColumnModel"));
    assert!(aliases_mod.contains("pub use crate::gui::list::ColumnSummary as ColumnModel;"));
    assert!(list_mod.contains("pub struct ColumnSummary"));
}

#[test]
fn editable_row_kind_is_owned_by_generic_list_module() {
    let sources_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/mod.rs"
    ))
    .expect("sources module should be readable");
    let list_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/list.rs"))
        .expect("list module should be readable");
    let aliases_mod = host_aliases_mod();

    assert!(!sources_mod.contains("pub enum FolderRowKind"));
    assert!(aliases_mod.contains("pub use crate::gui::list::EditableRowKind as FolderRowKind;"));
    assert!(list_mod.contains("pub enum EditableRowKind"));
}

#[test]
fn editable_tree_actions_are_owned_by_generic_list_module() {
    let sources_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/mod.rs"
    ))
    .expect("sources module should be readable");
    let list_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/list.rs"))
        .expect("list module should be readable");
    let aliases_mod = host_aliases_mod();

    assert!(!sources_mod.contains("pub struct FolderActionsModel"));
    assert!(
        aliases_mod
            .contains("pub use crate::gui::list::EditableTreeActions as FolderActionsModel;")
    );
    assert!(list_mod.contains("pub struct EditableTreeActions"));
}

#[test]
fn editable_tree_row_is_owned_by_generic_list_module() {
    let sources_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/mod.rs"
    ))
    .expect("sources module should be readable");
    let list_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/list.rs"))
        .expect("list module should be readable");
    let aliases_mod = host_aliases_mod();

    assert!(!sources_mod.contains("pub struct FolderRowModel"));
    assert!(aliases_mod.contains("pub use crate::gui::list::EditableTreeRow as FolderRowModel;"));
    assert!(list_mod.contains("pub struct EditableTreeRow"));
    assert!(list_mod.contains("pub backing_index: Option<usize>"));
    assert!(!list_mod.contains("source_index"));
}

#[test]
fn split_pane_slot_is_owned_by_generic_panel_module() {
    let compat_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/mod.rs"
    ))
    .expect("compat shell module should be readable");
    let sources_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/sources.rs"
    ))
    .expect("host source sidebar module should be readable");
    let panel_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/panel.rs"))
        .expect("panel module should be readable");
    let aliases_mod = host_aliases_mod();

    assert!(!compat_mod.contains("pub struct SourcesPanelModel"));
    assert!(compat_mod.contains("pub use sources::SourcesPanelModel;"));
    assert!(!compat_mod.contains("pub enum FolderPaneIdModel"));
    assert!(!compat_mod.contains("pub struct SourceRowModel"));
    assert!(!compat_mod.contains("pub struct FolderPaneModel"));
    assert!(
        aliases_mod.contains("pub use crate::gui::panel::SplitPaneAssignedRow as SourceRowModel;")
    );
    assert!(aliases_mod.contains("pub use crate::gui::panel::SplitPaneSlot as FolderPaneIdModel;"));
    assert!(aliases_mod.contains(
        "pub type FolderPaneModel = crate::gui::panel::SplitPaneTreePanel<FolderRowModel>;"
    ));
    assert!(panel_mod.contains("pub enum SplitPaneSlot"));
    assert!(panel_mod.contains("pub fn select<'a, T>"));
    assert!(panel_mod.contains("pub fn select_mut<'a, T>"));
    assert!(panel_mod.contains("pub struct SplitPaneAssignedRow"));
    assert!(panel_mod.contains("pub struct SplitPaneTreePanel<Row = EditableTreeRow>"));
    assert!(panel_mod.contains("pub struct SplitPaneSidebarState"));
    assert!(sources_mod.contains("split_pane_sidebar"));
    assert!(!sources_mod.contains("GenericSourcesPanelModel"));
}

#[test]
fn grouped_toolbar_cluster_width_is_owned_by_generic_layout_module() {
    let toolbar_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/layout_adapter/controls/browser_toolbar.rs"
    ))
    .expect("browser toolbar layout adapter should be readable");
    let row_helpers_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/layout_core/row_helpers.rs"
    ))
    .expect("generic row helper module should be readable");

    assert!(row_helpers_mod.contains("pub fn fixed_width_group_width"));
    assert!(row_helpers_mod.contains("pub fn grouped_fixed_width_row_width"));
    assert!(row_helpers_mod.contains("pub fn fixed_width_item_extent_for_available_width"));
    assert!(toolbar_mod.contains("fixed_width_group_width"));
    assert!(toolbar_mod.contains("grouped_fixed_width_row_width"));
    assert!(toolbar_mod.contains("fixed_width_item_extent_for_available_width"));
    assert!(
        !toolbar_mod.contains("chip_side * RATING_FILTER_CHIP_COUNT"),
        "legacy browser toolbar adapter should delegate grouped chip cluster widths to layout_core"
    );
    assert!(
        !toolbar_mod.contains("let raw_side ="),
        "legacy browser toolbar adapter should delegate fixed item sizing to layout_core"
    );
}

#[test]
fn focus_context_model_is_owned_by_generic_focus_module() {
    let sources_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/mod.rs"
    ))
    .expect("sources module should be readable");
    let focus_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/focus.rs"))
        .expect("focus module should be readable");
    let aliases_mod = host_aliases_mod();

    assert!(!sources_mod.contains("pub enum FocusContextModel"));
    assert!(aliases_mod.contains("pub use crate::gui::focus::FocusSurface as FocusContextModel;"));
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
        "/src/compat/legacy_shell/mod.rs"
    ))
    .expect("sources module should be readable");
    let feedback_mod =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/feedback.rs"))
            .expect("feedback module should be readable");
    let aliases_mod = host_aliases_mod();

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
        aliases_mod
            .contains("pub use crate::gui::feedback::RecoverySummary as FolderRecoveryModel;")
    );
    assert!(feedback_mod.contains("pub struct ProgressOverlay"));
    assert!(feedback_mod.contains("pub fn horizontal_progress_fill_rect"));
    assert!(feedback_mod.contains("pub fn horizontal_progress_activity_rect"));
    assert!(feedback_mod.contains("pub fn horizontal_meter_fill_rect"));
    assert!(feedback_mod.contains("pub fn horizontal_discrete_meter_fill_rect"));
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
fn progress_fill_geometry_is_owned_by_generic_feedback_module() {
    let feedback_mod =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/feedback.rs"))
            .expect("feedback module should be readable");
    let overlay_visuals_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/layout_adapter/overlay_visuals.rs"
    ))
    .expect("overlay visuals module should be readable");
    let status_bar_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/frame_build/status_bar.rs"
    ))
    .expect("status bar module should be readable");
    let similarity_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/toolbar_helpers/browser_row_decor/similarity.rs"
    ))
    .expect("similarity module should be readable");
    let top_bar_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/frame_build/chrome/top_bar.rs"
    ))
    .expect("top bar module should be readable");

    assert!(feedback_mod.contains("pub fn horizontal_progress_fill_rect"));
    assert!(feedback_mod.contains("pub fn horizontal_progress_activity_rect"));
    assert!(feedback_mod.contains("pub fn horizontal_progress_track_rect"));
    assert!(feedback_mod.contains("pub fn horizontal_meter_fill_rect"));
    assert!(feedback_mod.contains("pub fn horizontal_discrete_meter_fill_rect"));
    assert!(overlay_visuals_mod.contains("horizontal_progress_fill_rect"));
    assert!(status_bar_mod.contains("horizontal_progress_track_rect"));
    assert!(similarity_mod.contains("horizontal_discrete_meter_fill_rect"));
    assert!(top_bar_mod.contains("horizontal_meter_fill_rect"));
    assert!(
        !overlay_visuals_mod.contains("fn compute_progress_fill_rect"),
        "compat overlay geometry should delegate horizontal progress fill math to gui::feedback"
    );
    assert!(
        !status_bar_mod.contains("let fill_width ="),
        "compat status bar should delegate determinate progress fill math to gui::feedback"
    );
    assert!(
        !status_bar_mod.contains("let segment_width ="),
        "compat status bar should delegate indeterminate progress segment math to gui::feedback"
    );
    assert!(
        !status_bar_mod.contains("if model.progress_overlay.total == 0"),
        "compat status bar should delegate progress mode switching to gui::feedback"
    );
    assert!(
        !similarity_mod.contains("let fill_width ="),
        "compat similarity meter should delegate discrete fill math to gui::feedback"
    );
    assert!(
        !top_bar_mod.contains("let fill_width ="),
        "compat volume meter should delegate continuous fill math to gui::feedback"
    );
}

#[test]
fn pixel_centered_icon_geometry_is_owned_by_generic_rect_type() {
    let types_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/types.rs"))
        .expect("types module should be readable");
    let folder_rows_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/frame_build/chrome/sidebar_parts/folders/rows.rs"
    ))
    .expect("folder rows module should be readable");
    let folder_header_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/frame_build/chrome/sidebar_parts/folders/header.rs"
    ))
    .expect("folder header module should be readable");

    assert!(types_mod.contains("pub fn centered_pixel_square"));
    assert!(types_mod.contains("pub fn centered_odd_pixel_square"));
    assert!(folder_rows_mod.contains("centered_odd_pixel_square"));
    assert!(folder_header_mod.contains("centered_pixel_square"));
    assert!(
        !folder_rows_mod.contains("let mut size ="),
        "native folder row disclosure icons should delegate odd centered-square geometry to gui::types"
    );
    assert!(
        !folder_header_mod.contains("let size ="),
        "native folder header icons should delegate pixel centered-square geometry to gui::types"
    );
}

#[test]
fn centered_toolbar_icon_geometry_reuses_generic_rect_type() {
    let similarity_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/toolbar_helpers/browser_row_decor/similarity.rs"
    ))
    .expect("similarity module should be readable");
    let browser_panel_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/frame_build/browser/panel.rs"
    ))
    .expect("browser panel module should be readable");
    let waveform_visuals_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/toolbar_helpers/waveform_visuals.rs"
    ))
    .expect("waveform visuals module should be readable");

    assert!(similarity_mod.contains("button_rect.centered_square(side)"));
    assert!(browser_panel_mod.contains("button_rect.centered_square(side)"));
    assert!(waveform_visuals_mod.contains("button_rect.centered_square(icon_side)"));
    for source in [&similarity_mod, &browser_panel_mod, &waveform_visuals_mod] {
        assert!(
            !source.contains("let min_x = button_rect.min.x + ((button_rect.width() -"),
            "native toolbar icon wrappers should delegate centered-square geometry to gui::types"
        );
    }
}

#[test]
fn stroke_aligned_border_geometry_is_owned_by_generic_rect_type() {
    let types_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/types.rs"))
        .expect("types module should be readable");
    let markers_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/toolbar_helpers/browser_row_decor/markers.rs"
    ))
    .expect("browser row marker module should be readable");

    assert!(types_mod.contains("pub fn stroke_aligned_rect"));
    assert!(markers_mod.contains("rect.stroke_aligned_rect(stroke)"));
    assert!(
        !markers_mod.contains("let snap ="),
        "native browser row borders should delegate stroke-grid snapping to gui::types"
    );
}

#[test]
fn top_right_overlay_icon_geometry_is_owned_by_generic_rect_type() {
    let types_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/types.rs"))
        .expect("types module should be readable");
    let waveform_visuals_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/toolbar_helpers/waveform_visuals.rs"
    ))
    .expect("waveform visuals module should be readable");

    assert!(types_mod.contains("pub fn top_right_square"));
    assert!(waveform_visuals_mod.contains("base.top_right_square(side, inset)"));
    assert!(
        !waveform_visuals_mod.contains("base.max.x - side - inset"),
        "native waveform overlay icon geometry should delegate top-right square placement to gui::types"
    );
}

#[test]
fn focus_border_edge_geometry_is_owned_by_generic_rect_type() {
    let types_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/types.rs"))
        .expect("types module should be readable");
    let focus_shared_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/frame_build/overlay/focus/shared.rs"
    ))
    .expect("focus overlay shared module should be readable");

    for method in [
        "pub fn top_edge_strip",
        "pub fn bottom_edge_strip",
        "pub fn left_edge_strip",
        "pub fn right_edge_strip",
    ] {
        assert!(types_mod.contains(method));
    }
    for call in [
        "rect.top_edge_strip(stroke)",
        "rect.bottom_edge_strip(stroke)",
        "rect.left_edge_strip(stroke)",
        "rect.right_edge_strip(stroke)",
    ] {
        assert!(focus_shared_mod.contains(call));
    }
    assert!(
        !focus_shared_mod.contains("rect.max.y - stroke"),
        "native focus border rendering should delegate edge strip geometry to gui::types"
    );
}

#[test]
fn union_rect_geometry_is_owned_by_generic_rect_type() {
    let types_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/types.rs"))
        .expect("types module should be readable");
    let focus_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/frame_build/overlay/focus.rs"
    ))
    .expect("focus overlay module should be readable");
    let focus_shared_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/frame_build/overlay/focus/shared.rs"
    ))
    .expect("focus overlay shared module should be readable");
    let sidebar_automation_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/automation/sidebar.rs"
    ))
    .expect("sidebar automation module should be readable");

    assert!(types_mod.contains("pub fn union"));
    assert!(focus_mod.contains(".union(sections.tree_rows(active_pane))"));
    assert!(sidebar_automation_mod.contains("header_rect.union(tree_rows_band)"));
    assert!(!focus_shared_mod.contains("fn union_rect"));
    assert!(!sidebar_automation_mod.contains("fn union_rect"));
}

#[test]
fn text_layout_clamping_reuses_generic_rect_methods() {
    let text_layout_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/text_layout.rs"
    ))
    .expect("text layout module should be readable");
    let browser_text_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/layout_adapter/browser_text.rs"
    ))
    .expect("browser text adapter should be readable");
    let map_header_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/layout_adapter/map_header.rs"
    ))
    .expect("map header adapter should be readable");

    for source in [&text_layout_mod, &browser_text_mod, &map_header_mod] {
        assert!(!source.contains("fn clamp_rect_to_bounds"));
        assert!(!source.contains("fn empty_rect"));
        assert!(!source.contains("Rect::from_min_max(bounds.min, bounds.min)"));
    }
    assert!(text_layout_mod.contains(".empty_at_min()"));
    assert!(text_layout_mod.contains(".clamp_to(inner)"));
    assert!(browser_text_mod.contains("output.rect_for_clamped("));
    assert!(map_header_mod.contains("output.rect_for_clamped("));
}

#[test]
fn horizontal_rect_insets_are_owned_by_generic_rect_type() {
    let types_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/types.rs"))
        .expect("generic types module should be readable");
    let control_text_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/layout_adapter/control_text.rs"
    ))
    .expect("control text adapter should be readable");
    let sidebar_text_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/layout_adapter/sidebar_text.rs"
    ))
    .expect("sidebar text adapter should be readable");
    let shell_geometry_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/layout/geometry.rs"
    ))
    .expect("shell geometry adapter should be readable");
    let browser_bands_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/layout_adapter/bands.rs"
    ))
    .expect("browser bands adapter should be readable");
    let sidebar_header_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/layout_adapter/sidebar_header/helpers.rs"
    ))
    .expect("sidebar header adapter should be readable");

    assert!(types_mod.contains("pub fn inset_horizontal"));
    assert!(types_mod.contains("pub fn inset_horizontal_saturating"));
    assert!(control_text_mod.contains(".inset_horizontal("));
    assert!(sidebar_text_mod.contains(".inset_horizontal("));
    assert!(shell_geometry_mod.contains(".inset_horizontal_saturating("));
    assert!(browser_bands_mod.contains(".inset_horizontal_saturating("));
    assert!(sidebar_header_mod.contains(".inset_horizontal_saturating("));
    assert!(!control_text_mod.contains("fn inset_horizontal"));
    assert!(!sidebar_text_mod.contains("fn inset_rect_horizontal"));
    assert!(!shell_geometry_mod.contains("fn inset_horizontal"));
    assert!(!browser_bands_mod.contains("fn inset_horizontal"));
    assert!(!sidebar_header_mod.contains("fn inset_horizontal"));
}

#[test]
fn measured_rect_lookup_is_owned_by_generic_layout_output() {
    let layout_types = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/layout_core/engine/types.rs"
    ))
    .expect("layout engine types module should be readable");
    let layout_adapter = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/layout_adapter.rs"
    ))
    .expect("layout adapter should be readable");
    let browser_chrome_surface = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/browser_chrome_surface.rs"
    ))
    .expect("browser chrome surface should be readable");
    let waveform_header_surface = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/waveform_header_surface.rs"
    ))
    .expect("waveform header surface should be readable");
    let layout_adapter_files = [
        "/src/gui/native_shell/layout_adapter/bands.rs",
        "/src/gui/native_shell/layout_adapter/browser_tabs.rs",
        "/src/gui/native_shell/layout_adapter/browser_text.rs",
        "/src/gui/native_shell/layout_adapter/map_canvas.rs",
        "/src/gui/native_shell/layout_adapter/map_header.rs",
        "/src/gui/native_shell/layout_adapter/sidebar_bands.rs",
    ]
    .map(|path| {
        fs::read_to_string(format!("{}{}", env!("CARGO_MANIFEST_DIR"), path))
            .unwrap_or_else(|err| panic!("layout adapter file {path} should be readable: {err}"))
    });

    assert!(layout_types.contains("pub fn rect_for(&self, node_id: NodeId, fallback: Rect)"));
    assert!(layout_types.contains("pub fn rect_for_clamped"));
    assert!(layout_adapter.contains("output.rect_for("));
    assert!(browser_chrome_surface.contains("output.rect_for_clamped("));
    assert!(waveform_header_surface.contains("output.rect_for_clamped("));
    assert!(
        layout_adapter_files
            .iter()
            .all(|source| source.contains("output.rect_for_clamped("))
    );
    assert!(!layout_adapter.contains("fn rect_for"));
    assert!(!browser_chrome_surface.contains("fn rect_for"));
    assert!(!browser_chrome_surface.contains("fn clamp_rect_to_bounds"));
    assert!(!waveform_header_surface.contains("fn rect_for"));
    assert!(!waveform_header_surface.contains("fn clamp_rect_to_bounds"));
    for source in layout_adapter_files {
        assert!(!source.contains("fn rect_for"));
        assert!(!source.contains("fn clamp_rect_to_bounds"));
    }
}

#[test]
fn text_baseline_snapping_is_owned_by_generic_text_layout() {
    let text_layout_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/text_layout.rs"
    ))
    .expect("text layout module should be readable");
    let browser_text_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/layout_adapter/browser_text.rs"
    ))
    .expect("browser text adapter should be readable");

    assert!(text_layout_mod.contains("pub fn snap_text_baseline_to_pixel"));
    assert!(browser_text_mod.contains("snap_text_baseline_to_pixel("));
    assert!(!browser_text_mod.contains("fn snap_browser_row_text_baseline"));
    assert!(
        !browser_text_mod.contains("let baseline = (line.min.y + height).round()"),
        "native browser text should delegate baseline snapping to gui::text_layout"
    );
}

#[test]
fn context_menu_panel_placement_is_owned_by_generic_panel_module() {
    let panel_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/panel.rs"))
        .expect("panel module should be readable");
    let sidebar_toolbar_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/toolbar_helpers/sidebar_toolbar.rs"
    ))
    .expect("sidebar toolbar module should be readable");

    assert!(panel_mod.contains("pub fn anchored_panel_rect"));
    assert!(sidebar_toolbar_mod.contains("anchored_panel_rect("));
    assert!(
        !sidebar_toolbar_mod.contains("menu.anchor.x.clamp(min_x, max_x)"),
        "native context menu placement should delegate anchored panel geometry to gui::panel"
    );
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
    assert!(!shell_mod.contains("PreferencePanelStateModel"));
    assert!(shell_mod.contains("crate::gui::form::PreferencePanelState<4>"));
    assert!(shell_mod.contains("crate::gui::form::PreferencePanelState::new("));
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
    assert!(form_mod.contains("pub struct PreferencePanelState"));
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
        "/src/compat/legacy_shell/mod.rs"
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
    let motion_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/motion.rs"
    ))
    .expect("motion module should be readable");
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
    let aliases_mod = host_aliases_mod();

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
    assert!(!waveform_mod.contains("pub struct WaveformTransportModel"));
    assert!(!waveform_mod.contains("pub struct WaveformEditPreviewModel"));
    assert!(!waveform_mod.contains("pub struct WaveformImagePreviewModel"));
    assert!(!waveform_mod.contains("pub struct WaveformChromeStateModel"));
    assert!(!waveform_mod.contains("pub struct WaveformToolStateModel"));
    assert!(aliases_mod.contains("pub use crate::gui::selection::TriState as BrowserPillState;"));
    assert!(
        actions_mod.contains("pub type BrowserTriageTarget = crate::gui::selection::TriageTarget;")
    );
    assert!(selection_mod.contains("pub enum TriageTarget"));
    assert!(aliases_mod.contains(
        "pub type BrowserPillModel = crate::gui::badge::SelectablePill<BrowserPillState>;"
    ));
    assert!(aliases_mod.contains(
        "pub type BrowserPillEditorModel = crate::gui::badge::PillEditorPanel<BrowserPillState>;"
    ));
    assert!(
        aliases_mod
            .contains("pub use crate::gui::visualization::PointRenderMode as MapRenderModeModel;")
    );
    assert!(
        aliases_mod.contains("pub use crate::gui::visualization::SpatialPoint as MapPointModel;")
    );
    assert!(
        aliases_mod.contains("pub use crate::gui::visualization::SpatialPanel as MapPanelModel;")
    );
    assert!(!waveform_mod.contains("WaveformChannelViewModel"));
    assert!(waveform_mod.contains("crate::gui::visualization::ChannelViewMode"));
    assert!(motion_mod.contains("crate::gui::visualization::ChannelViewMode"));
    assert!(!waveform_mod.contains("WaveformSlicePreviewModel"));
    assert!(waveform_mod.contains("crate::gui::visualization::TimelineMarkerPreview"));
    assert!(motion_mod.contains("crate::gui::visualization::TimelineMarkerPreview"));
    assert!(!waveform_mod.contains("WaveformViewportModel"));
    assert!(waveform_mod.contains("crate::gui::visualization::TimelineViewport"));
    assert!(motion_mod.contains("crate::gui::visualization::TimelineViewport"));
    assert!(!waveform_mod.contains("WaveformTransportModel"));
    assert!(waveform_mod.contains("crate::gui::visualization::TimelineTransportState"));
    assert!(motion_mod.contains("crate::gui::visualization::TimelineTransportState"));
    assert!(!waveform_mod.contains("WaveformEditPreviewModel"));
    assert!(waveform_mod.contains("crate::gui::visualization::TimelineEditPreview"));
    assert!(motion_mod.contains("crate::gui::visualization::TimelineEditPreview"));
    assert!(!waveform_mod.contains("WaveformFeedbackEventsModel"));
    assert!(waveform_mod.contains("crate::gui::visualization::TimelineFeedbackEvents"));
    assert!(motion_mod.contains("crate::gui::visualization::TimelineFeedbackEvents"));
    assert!(!waveform_mod.contains("WaveformPresentationModel"));
    assert!(waveform_mod.contains("crate::gui::visualization::TimelinePresentationState"));
    assert!(motion_mod.contains("crate::gui::visualization::TimelinePresentationState"));
    assert!(!waveform_mod.contains("WaveformSurfaceModel"));
    assert!(waveform_mod.contains("crate::gui::visualization::TimelineSurfaceState<"));
    assert!(waveform_mod.contains("crate::gui::visualization::TimelineSurfaceState::new("));
    assert!(motion_mod.contains("crate::gui::visualization::TimelineSurfaceState::new("));
    assert!(!waveform_mod.contains("WaveformMotionModel"));
    assert!(motion_mod.contains("crate::gui::visualization::TimelineMotionState<"));
    assert!(motion_mod.contains("crate::gui::visualization::TimelineMotionState::new("));
    assert!(!waveform_mod.contains("WaveformImagePreviewModel"));
    assert!(waveform_mod.contains("crate::gui::visualization::SignalRasterPreview"));
    assert!(motion_mod.contains("crate::gui::visualization::SignalRasterPreview"));
    assert!(!waveform_mod.contains("WaveformChromeStateModel"));
    assert!(waveform_mod.contains("crate::gui::visualization::SignalChromeState"));
    assert!(motion_mod.contains("crate::gui::visualization::SignalChromeState"));
    assert!(!waveform_mod.contains("WaveformToolStateModel"));
    assert!(waveform_mod.contains("crate::gui::visualization::SignalToolState"));
    assert!(motion_mod.contains("crate::gui::visualization::SignalToolState"));
    assert!(selection_mod.contains("pub enum TriState"));
    assert!(badge_mod.contains("pub struct SelectablePill<State>"));
    assert!(badge_mod.contains("pub struct PillEditorPanel<State>"));
    assert!(visualization_mod.contains("pub enum PointRenderMode"));
    assert!(visualization_mod.contains("pub struct SpatialPoint"));
    assert!(visualization_mod.contains("pub struct SpatialPanel"));
    assert!(visualization_mod.contains("pub fn normalized_milli_point_in_rect"));
    assert!(visualization_mod.contains("pub enum ChannelViewMode"));
    assert!(visualization_mod.contains("pub struct TimelineMarkerPreview"));
    assert!(visualization_mod.contains("pub struct TimelineViewport"));
    assert!(visualization_mod.contains("pub struct TimelineTransportState"));
    assert!(visualization_mod.contains("pub struct TimelineEditPreview"));
    assert!(visualization_mod.contains("pub struct TimelineFeedbackEvents"));
    assert!(visualization_mod.contains("pub struct TimelinePresentationState"));
    assert!(visualization_mod.contains("pub struct SignalRasterPreview"));
    assert!(visualization_mod.contains("pub struct SignalChromeState"));
    assert!(visualization_mod.contains("pub struct SignalToolState"));
    assert!(visualization_mod.contains("pub struct TimelineMotionState"));
    assert!(!visualization_mod.contains("waveform"));
    assert!(!visualization_mod.contains("Waveform"));
    assert!(visualization_mod.contains("pub struct TimelineSurfaceState"));
    assert!(waveform_mod.contains("timeline_surface"));
    assert!(motion_mod.contains("timeline_motion"));
}

#[test]
fn spatial_point_projection_is_owned_by_generic_visualization_module() {
    let visualization_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/visualization.rs"
    ))
    .expect("visualization module should be readable");
    let map_canvas_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/layout_adapter/map_canvas.rs"
    ))
    .expect("map canvas module should be readable");

    assert!(visualization_mod.contains("pub fn normalized_milli_point_in_rect"));
    assert!(map_canvas_mod.contains("normalized_milli_point_in_rect(canvas, x_milli, y_milli)"));
    assert!(
        !map_canvas_mod.contains("let x_ratio = f32::from(x_milli.min(1000))"),
        "native map canvas wrapper should delegate normalized point projection to gui::visualization"
    );
}

#[test]
fn compat_shell_defaults_do_not_bake_in_sample_browser_copy() {
    let browser_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/mod.rs"
    ))
    .expect("browser module should be readable");
    let shell_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/shell.rs"
    ))
    .expect("shell module should be readable");
    let chrome_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/chrome.rs"))
        .expect("chrome module should be readable");
    let aliases_mod = host_aliases_mod();

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
        aliases_mod
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
        "/src/compat/legacy_shell/mod.rs"
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
        "/src/compat/legacy_shell/mod.rs"
    ))
    .expect("browser module should be readable");
    let aliases_mod = host_aliases_mod();
    let list_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/list.rs"))
        .expect("generic list module should be readable");
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
    assert!(aliases_mod.contains("pub type BrowserPanelModel ="));
    assert!(aliases_mod.contains("ContentListPanel<BrowserRowModel, BrowserPillEditorModel>"));
    assert!(list_mod.contains("pub pill_editor: Editor"));
    assert!(list_mod.contains("pub pill_editor_open: bool"));
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
        "/src/compat/legacy_shell/mod.rs"
    ))
    .expect("browser module should be readable");
    let aliases_mod = host_aliases_mod();
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
        aliases_mod
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
        "/src/compat/legacy_shell/mod.rs"
    ))
    .expect("app browser model should be readable");
    let list_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/list.rs"))
        .expect("generic list module should be readable");
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
    let aliases_mod = host_aliases_mod();

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
    assert!(aliases_mod.contains("ContentListPanel<BrowserRowModel, BrowserPillEditorModel>"));
    assert!(list_mod.contains("derived_label_filter_active"));
    assert!(list_mod.contains("derived_label_filter_negated"));
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
    let native_hit_testing = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/hit_testing/browser.rs"
    ))
    .expect("native-shell browser hit testing should be readable");

    for source in [
        &actions_mod,
        &automation_helpers,
        &key_bindings,
        &browser_pointer,
        &native_hit_testing,
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
    let native_hit_testing = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/hit_testing/browser.rs"
    ))
    .expect("native-shell browser hit testing should be readable");

    for source in [
        &actions_mod,
        &automation_helpers,
        &automation_browser,
        &runtime_actions,
        &runtime_drag,
        &runtime_pointer,
        &native_hit_testing,
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
    assert!(runtime_drag.contains("spatial_content_action_at_point"));
    assert!(runtime_pointer.contains("spatial_content_action_at_point"));
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
    let legacy_shell_runner = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui_runtime/native_vello/legacy_shell_runner.rs"
    ))
    .expect("legacy shell runner should be readable");
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
        &legacy_shell_runner,
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
    assert!(legacy_shell_runner.contains("content_item_drag: Option<ContentItemDragState>"));
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
    let frame_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/frame.rs"))
        .expect("frame module should be readable");
    let invalidation_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/invalidation.rs"
    ))
    .expect("invalidation module should be readable");
    let compat_runtime_artifacts_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/runtime_artifacts.rs"
    ))
    .expect("compat runtime artifacts module should be readable");
    let host_runtime_artifacts_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/runtime_artifacts.rs"
    ))
    .expect("host runtime artifacts module should be readable");
    let dirty_segments_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/dirty_segments.rs"
    ))
    .expect("host dirty segment module should be readable");
    let aliases_mod = host_aliases_mod();

    assert!(!app_mod.contains("pub struct FrameBuildResult"));
    assert!(app_mod.contains("FrameBuildResult"));
    assert!(aliases_mod.contains("pub use crate::gui::frame::FrameBuildResult;"));
    assert!(frame_mod.contains("pub struct FrameBuildResult"));
    assert!(!app_mod.contains("pub struct NativeRuntimeArtifacts"));
    assert!(app_mod.contains("pub use crate::compat::runtime_artifacts::NativeRunReport;"));
    assert!(!app_mod.contains("runtime/runtime_artifacts.rs"));
    assert!(compat_runtime_artifacts_mod.contains("pub struct NativeRuntimeArtifacts"));
    assert!(compat_runtime_artifacts_mod.contains("pub type NativeRunReport"));
    assert!(host_runtime_artifacts_mod.contains("pub struct NativeRuntimeArtifacts"));
    assert!(host_runtime_artifacts_mod.contains("pub type NativeRunReport"));
    assert!(!app_mod.contains("pub struct DirtySegments"));
    assert!(!app_mod.contains("pub struct SegmentRevisions"));
    assert!(app_mod.contains("pub use dirty_segments::{DirtySegments, SegmentRevisions};"));
    assert!(!app_mod.contains("RetainedSegmentMask"));
    assert!(!app_mod.contains("RetainedSegmentRevisions"));
    assert!(dirty_segments_mod.contains("pub struct DirtySegments"));
    assert!(dirty_segments_mod.contains("pub struct SegmentRevisions"));
    assert!(dirty_segments_mod.contains("RetainedSegmentMask"));
    assert!(dirty_segments_mod.contains("RetainedSegmentRevisions"));
    assert!(invalidation_mod.contains("pub struct InvalidationMask"));
    assert!(invalidation_mod.contains("pub struct RetainedSegmentMask"));
    assert!(invalidation_mod.contains("pub struct RetainedSegmentRevisions"));
    assert!(dirty_segments_mod.contains("retained_revisions"));
}

#[test]
fn automation_snapshot_primitives_are_owned_by_generic_gui_module() {
    let legacy_shell_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/mod.rs"
    ))
    .expect("legacy shell module should be readable");
    let gui_automation_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/automation.rs"
    ))
    .expect("generic automation module should be readable");
    let aliases_mod = host_aliases_mod();

    assert!(!legacy_shell_mod.contains("mod automation;"));
    assert!(!legacy_shell_mod.contains("pub enum AutomationRole"));
    assert!(!legacy_shell_mod.contains("pub struct AutomationNodeSnapshot"));
    assert!(!legacy_shell_mod.contains("pub struct GuiAutomationSnapshot"));
    assert!(
        aliases_mod.contains(
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

#[test]
fn visual_snapshot_serialization_is_owned_by_generic_gui_module() {
    let legacy_shell_snapshot_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/compat/legacy_shell/shell_snapshot.rs"
    ))
    .expect("legacy shell snapshot module should be readable");
    let gui_snapshot_mod =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/snapshot.rs"))
            .expect("generic snapshot module should be readable");

    assert!(gui_snapshot_mod.contains("pub fn visual_snapshot_from_paint_frame"));
    assert!(legacy_shell_snapshot_mod.contains("visual_snapshot_from_paint_frame"));
    for duplicate in [
        "SnapshotPrimitive",
        "SnapshotTextRun",
        "fn snap_primitive",
        "fn snap_rect",
        "fn snap_color",
        "fn snap_align",
    ] {
        assert!(
            !legacy_shell_snapshot_mod.contains(duplicate),
            "legacy shell snapshot capture should delegate `{duplicate}` handling to gui::snapshot"
        );
    }
}

#[test]
fn virtual_list_scroll_clamping_is_owned_by_generic_list_module() {
    let legacy_wheel_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui_runtime/native_vello/input/wheel.rs"
    ))
    .expect("legacy wheel module should be readable");
    let gui_list_mod = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/list.rs"))
        .expect("generic list module should be readable");

    assert!(gui_list_mod.contains("pub fn virtual_list_view_start_after_scroll_delta"));
    assert!(gui_list_mod.contains("pub fn virtual_list_scroll_delta_from_units"));
    assert!(legacy_wheel_mod.contains("virtual_list_view_start_after_scroll_delta"));
    assert!(legacy_wheel_mod.contains("virtual_list_scroll_delta_from_units"));
    assert!(
        !legacy_wheel_mod.contains("let max_start = visible_count.saturating_sub"),
        "legacy wheel input should delegate virtual-list viewport clamping to gui::list"
    );
    assert!(
        !legacy_wheel_mod.contains("let mut steps = raw.round()"),
        "legacy wheel input should delegate logical scroll-step normalization to gui::list"
    );
}

#[test]
fn inline_indicator_layout_is_owned_by_generic_feedback_module() {
    let browser_indicators_mod = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gui/native_shell/state/toolbar_helpers/browser_row_decor/rating_indicators.rs"
    ))
    .expect("browser rating indicator module should be readable");
    let feedback_mod =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gui/feedback.rs"))
            .expect("generic feedback module should be readable");

    assert!(feedback_mod.contains("pub struct InlineIndicatorMetrics"));
    assert!(feedback_mod.contains("pub fn inline_indicator_reserved_width"));
    assert!(feedback_mod.contains("pub fn inline_indicator_layout"));
    assert!(browser_indicators_mod.contains("inline_indicator_reserved_width"));
    assert!(browser_indicators_mod.contains("inline_indicator_layout"));
    assert!(
        !browser_indicators_mod.contains("let total_width = (count as f32 * width)"),
        "legacy browser rating indicators should delegate cluster width and placement to gui::feedback"
    );
}

#[test]
fn native_shell_motion_helpers_do_not_use_sample_manager_terms_for_points() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let files = [
        "/src/gui/native_shell/state/motion_overlay/playhead_trail.rs",
        "/src/gui/native_shell/state/svg_icons.rs",
        "/src/gui/native_shell/state/waveform_segments/trail.rs",
        "/src/gui/native_shell/state/waveform_segments/surface.rs",
    ];

    for file in files {
        let source = fs::read_to_string(format!("{manifest_dir}{file}"))
            .expect("native-shell helper should be readable");
        assert!(
            !source.contains("sample") && !source.contains("Sample"),
            "{file} should describe generic points, offsets, or profile data without sample-manager terms"
        );
    }
}
