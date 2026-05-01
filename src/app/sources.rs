//! Source/sidebar-facing models exposed by the `radiant` app contract.

use super::RetainedVec;
pub use crate::gui::feedback::RecoverySummary as FolderRecoveryModel;
pub use crate::gui::list::ColumnSummary as ColumnModel;
pub use crate::gui::list::EditableRowKind as FolderRowKind;
pub use crate::gui::list::EditableTreeActions as FolderActionsModel;
pub use crate::gui::list::EditableTreeRow as FolderRowModel;
pub use crate::gui::panel::SplitPaneAssignedRow as SourceRowModel;
pub use crate::gui::panel::SplitPaneSlot as FolderPaneIdModel;

/// Logical focus buckets projected into the native runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FocusContextModel {
    /// No UI surface currently owns keyboard focus.
    #[default]
    None,
    /// The waveform viewer handles navigation and shortcuts.
    Waveform,
    /// The sample browser handles row navigation and browser shortcuts.
    SampleBrowser,
    /// The folder tree handles folder navigation and folder shortcuts.
    SourceFolders,
    /// The source list handles source-row navigation and shortcuts.
    SourcesList,
}

/// Projected data for one fixed folder pane shown in the sidebar.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct FolderPaneModel {
    /// Stable pane identity used by native routing.
    pub pane: FolderPaneIdModel,
    /// Short title shown in the pane header.
    pub title: String,
    /// Primary source label currently assigned to the pane.
    pub source_label: String,
    /// Secondary source detail text, usually the source path.
    pub source_detail: String,
    /// Whether this pane currently drives browser and waveform state.
    pub active: bool,
    /// Whether a source is assigned to this pane.
    pub has_source: bool,
    /// Whether this pane is hydrating its assigned source snapshot.
    pub loading: bool,
    /// Whether this pane is asynchronously rebuilding its folder-tree rows.
    pub projecting: bool,
    /// Whether this pane's source currently owns a background file or folder mutation.
    pub mutation_busy: bool,
    /// Active folder-search query for this pane.
    pub folder_search_query: String,
    /// Whether the folder browser currently includes empty on-disk folders.
    pub show_all_folders: bool,
    /// Whether the folder-visibility toggle is currently actionable.
    pub can_toggle_show_all_folders: bool,
    /// Whether folder filtering includes descendant files in a flattened list.
    pub flattened_view: bool,
    /// Whether the folder flattened-view toggle is currently actionable.
    pub can_toggle_flattened_view: bool,
    /// Focused folder row index, if any.
    pub focused_folder_row: Option<usize>,
    /// Folder rows to render in this pane.
    pub folder_rows: RetainedVec<FolderRowModel>,
    /// Folder action availability projected for this pane.
    pub folder_actions: FolderActionsModel,
    /// Folder delete-recovery summary projected for this pane.
    pub folder_recovery: FolderRecoveryModel,
}

/// Sidebar model for source browsing controls.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SourcesPanelModel {
    /// Header text for the source panel.
    pub header: String,
    /// Active source-search query.
    pub search_query: String,
    /// Pane that currently drives browser and waveform state.
    pub active_folder_pane: FolderPaneIdModel,
    /// Upper fixed folder pane.
    pub upper_folder_pane: FolderPaneModel,
    /// Lower fixed folder pane.
    pub lower_folder_pane: FolderPaneModel,
    /// Active folder-search query.
    pub folder_search_query: String,
    /// Whether the folder browser currently includes empty on-disk folders.
    pub show_all_folders: bool,
    /// Whether the folder-visibility toggle is currently actionable.
    pub can_toggle_show_all_folders: bool,
    /// Whether folder filtering includes descendant files in a flattened list.
    pub flattened_view: bool,
    /// Whether the folder flattened-view toggle is currently actionable.
    pub can_toggle_flattened_view: bool,
    /// Selected row index, if any.
    pub selected_row: Option<usize>,
    /// Source row currently hydrating in the background, if any.
    pub loading_row: Option<usize>,
    /// Source row currently running a background file or folder mutation, if any.
    pub mutation_busy_row: Option<usize>,
    /// Focused folder row index, if any.
    pub focused_folder_row: Option<usize>,
    /// Rows to render in the source panel.
    pub rows: RetainedVec<SourceRowModel>,
    /// Folder rows to render in the folder browser section.
    pub folder_rows: RetainedVec<FolderRowModel>,
    /// Folder action availability for native sidebar controls.
    pub folder_actions: FolderActionsModel,
    /// Folder delete-recovery summary for native sidebar status.
    pub folder_recovery: FolderRecoveryModel,
}

impl SourcesPanelModel {
    /// Borrow one pane model by id.
    pub fn folder_pane(&self, pane: FolderPaneIdModel) -> &FolderPaneModel {
        match pane {
            FolderPaneIdModel::Upper => &self.upper_folder_pane,
            FolderPaneIdModel::Lower => &self.lower_folder_pane,
        }
    }

    /// Borrow the pane that currently drives browser and waveform state.
    pub fn active_folder_pane_model(&self) -> &FolderPaneModel {
        self.folder_pane(self.active_folder_pane)
    }
}
