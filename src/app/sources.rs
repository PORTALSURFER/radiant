//! Source/sidebar-facing models exposed by the `radiant` app contract.

/// Render data for one triage/browser column.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColumnModel {
    /// Display label for the column header.
    pub title: String,
    /// Number of rows/items represented by the column.
    pub item_count: usize,
}

impl ColumnModel {
    /// Build a new column model.
    pub fn new(title: impl Into<String>, item_count: usize) -> Self {
        Self {
            title: title.into(),
            item_count,
        }
    }
}

/// Render data for one source row shown in the sidebar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceRowModel {
    /// Primary label shown for the source.
    pub label: String,
    /// Optional secondary detail text (usually a path or status).
    pub detail: String,
    /// Whether the row is currently selected.
    pub selected: bool,
    /// Whether the source is missing from disk.
    pub missing: bool,
}

impl SourceRowModel {
    /// Build a new source row model.
    pub fn new(
        label: impl Into<String>,
        detail: impl Into<String>,
        selected: bool,
        missing: bool,
    ) -> Self {
        Self {
            label: label.into(),
            detail: detail.into(),
            selected,
            missing,
        }
    }
}

/// Render data for one folder row shown in the sidebar folder tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FolderRowKind {
    /// Standard existing folder row projected from host state.
    #[default]
    Existing,
    /// Inline draft row used while creating a new folder in place.
    CreateDraft,
    /// Inline draft row used while renaming an existing folder in place.
    RenameDraft,
}

/// Render data for one folder row shown in the sidebar folder tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FolderRowModel {
    /// Display label for the folder row.
    pub label: String,
    /// Optional secondary detail text for the folder row.
    pub detail: String,
    /// Tree depth used for indentation.
    pub depth: usize,
    /// Whether this row is currently selected.
    pub selected: bool,
    /// Whether this row currently has keyboard focus.
    pub focused: bool,
    /// Whether this row represents the synthetic source root.
    pub is_root: bool,
    /// Whether this row has child folders.
    pub has_children: bool,
    /// Whether this row is expanded in the folder tree.
    pub expanded: bool,
    /// Row kind used by the shell for inline draft rendering and hit testing.
    pub kind: FolderRowKind,
    /// Source/controller row index backing this projected row, when applicable.
    pub source_index: Option<usize>,
    /// Editable input value for inline draft rows.
    pub input_value: Option<String>,
    /// Placeholder text for inline draft rows.
    pub input_placeholder: Option<String>,
    /// Validation error for inline draft rows.
    pub input_error: Option<String>,
    /// Whether the inline draft input should own keyboard focus.
    pub input_focused: bool,
    /// Whether the next focus transition should select the full input text once.
    pub select_all_on_focus: bool,
}

impl FolderRowModel {
    /// Build a new folder row model.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        label: impl Into<String>,
        detail: impl Into<String>,
        depth: usize,
        selected: bool,
        focused: bool,
        is_root: bool,
        has_children: bool,
        expanded: bool,
    ) -> Self {
        Self {
            label: label.into(),
            detail: detail.into(),
            depth,
            selected,
            focused,
            is_root,
            has_children,
            expanded,
            kind: FolderRowKind::Existing,
            source_index: None,
            input_value: None,
            input_placeholder: None,
            input_error: None,
            input_focused: false,
            select_all_on_focus: false,
        }
    }

    /// Attach the backing source/controller row index for one existing row.
    pub fn with_source_index(mut self, source_index: usize) -> Self {
        self.source_index = Some(source_index);
        self
    }

    /// Build one inline create-draft row embedded in the folder tree.
    pub fn create_draft(
        depth: usize,
        input_value: impl Into<String>,
        input_placeholder: impl Into<String>,
        input_error: Option<String>,
        input_focused: bool,
    ) -> Self {
        Self {
            label: String::new(),
            detail: String::new(),
            depth,
            selected: false,
            focused: false,
            is_root: false,
            has_children: false,
            expanded: false,
            kind: FolderRowKind::CreateDraft,
            source_index: None,
            input_value: Some(input_value.into()),
            input_placeholder: Some(input_placeholder.into()),
            input_error,
            input_focused,
            select_all_on_focus: false,
        }
    }

    /// Build one inline rename-draft row embedded in the folder tree.
    pub fn rename_draft(
        depth: usize,
        input_value: impl Into<String>,
        input_placeholder: impl Into<String>,
        input_error: Option<String>,
        input_focused: bool,
    ) -> Self {
        let input_value = input_value.into();
        Self {
            label: input_value.clone(),
            detail: String::new(),
            depth,
            selected: false,
            focused: false,
            is_root: false,
            has_children: false,
            expanded: false,
            kind: FolderRowKind::RenameDraft,
            source_index: None,
            input_value: Some(input_value),
            input_placeholder: Some(input_placeholder.into()),
            input_error,
            input_focused,
            select_all_on_focus: true,
        }
    }

    /// Set whether the inline input should select all text the next time it receives focus.
    pub fn with_select_all_on_focus(mut self, select_all_on_focus: bool) -> Self {
        self.select_all_on_focus = select_all_on_focus;
        self
    }
}

/// Native folder-action availability consumed by sidebar action surfaces.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct FolderActionsModel {
    /// Whether creating a folder at the focused parent is allowed.
    pub can_create_folder: bool,
    /// Whether creating a folder at source root is allowed.
    pub can_create_folder_at_root: bool,
    /// Whether renaming the focused folder is allowed.
    pub can_rename_folder: bool,
    /// Whether deleting the focused folder is allowed.
    pub can_delete_folder: bool,
    /// Whether explicit restore for retained folder deletes is allowed.
    pub can_restore_retained_deletes: bool,
    /// Whether explicit purge for retained folder deletes is allowed.
    pub can_purge_retained_deletes: bool,
    /// Whether clearing folder delete-recovery logs is allowed.
    pub can_clear_recovery_log: bool,
}

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

/// Delete-recovery status for staged folder delete recovery in the sidebar.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct FolderRecoveryModel {
    /// Whether delete recovery is still running in the background.
    pub in_progress: bool,
    /// Number of completed recovery log entries currently visible.
    pub entry_count: usize,
    /// Number of retained deletes currently awaiting explicit restore or purge.
    pub retained_count: usize,
}

/// Sidebar model for source browsing controls.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SourcesPanelModel {
    /// Header text for the source panel.
    pub header: String,
    /// Active source-search query.
    pub search_query: String,
    /// Active folder-search query.
    pub folder_search_query: String,
    /// Whether the folder browser currently includes empty on-disk folders.
    pub show_all_folders: bool,
    /// Whether the folder-visibility toggle is currently actionable.
    pub can_toggle_show_all_folders: bool,
    /// Selected row index, if any.
    pub selected_row: Option<usize>,
    /// Focused folder row index, if any.
    pub focused_folder_row: Option<usize>,
    /// Rows to render in the source panel.
    pub rows: Vec<SourceRowModel>,
    /// Folder rows to render in the folder browser section.
    pub folder_rows: Vec<FolderRowModel>,
    /// Folder action availability for native sidebar controls.
    pub folder_actions: FolderActionsModel,
    /// Folder delete-recovery summary for native sidebar status.
    pub folder_recovery: FolderRecoveryModel,
}
