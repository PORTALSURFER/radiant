//! App-facing model/action contracts for native runtime integrations.
//!
//! The native runtime pulls an [`AppModel`] each frame and emits [`UiAction`] events
//! back to the host bridge. This keeps `radiant` rendering/runtime logic decoupled
//! from application-specific controller implementations.

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
        }
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
    /// Whether clearing folder delete-recovery logs is allowed.
    pub can_clear_recovery_log: bool,
}

/// Delete-recovery status for staged folder delete recovery in the sidebar.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct FolderRecoveryModel {
    /// Whether delete recovery is still running in the background.
    pub in_progress: bool,
    /// Number of recovery log entries currently visible.
    pub entry_count: usize,
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

/// Summary of browser/list state consumed by the native shell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserRowModel {
    /// Visible row index in the filtered browser list.
    pub visible_row: usize,
    /// Display label for the row.
    pub label: String,
    /// Triage column index (`0..=2`) that currently owns the row.
    pub column: usize,
    /// Optional badge/chip label rendered in the browser bucket column.
    ///
    /// Hosts can use this for metadata such as BPM (`"150 BPM"`). When absent,
    /// the shell falls back to a column-derived label.
    pub bucket_label: Option<String>,
    /// Whether this row is currently selected in multi-selection state.
    pub selected: bool,
    /// Whether this row currently has focus/caret.
    pub focused: bool,
}

impl BrowserRowModel {
    /// Build a row model, clamping the column into `0..=2`.
    pub fn new(
        visible_row: usize,
        label: impl Into<String>,
        column: usize,
        selected: bool,
        focused: bool,
    ) -> Self {
        Self {
            visible_row,
            label: label.into(),
            column: column.min(2),
            bucket_label: None,
            selected,
            focused,
        }
    }

    /// Attach an explicit bucket-column label for this row.
    pub fn with_bucket_label(mut self, label: impl Into<String>) -> Self {
        self.bucket_label = Some(label.into());
        self
    }
}

/// Summary of browser/list state consumed by the native shell.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct BrowserPanelModel {
    /// Number of rows currently visible in the browser.
    pub visible_count: usize,
    /// Focused visible row index, if any.
    pub selected_visible_row: Option<usize>,
    /// Number of rows currently in multi-selection.
    pub selected_path_count: usize,
    /// Active browser search query.
    pub search_query: String,
    /// Placeholder shown when the browser search query is empty.
    pub search_placeholder: Option<String>,
    /// Whether browser search/filter work is still running in the background.
    pub busy: bool,
    /// Display label for the active browser sort mode.
    pub sort_label: Option<String>,
    /// Display label for the currently active browser tab.
    pub active_tab_label: Option<String>,
    /// Display label for the currently focused sample, when known.
    pub focused_sample_label: Option<String>,
    /// Selection anchor in visible-row space.
    pub anchor_visible_row: Option<usize>,
    /// Visible rows rendered by the native browser panel.
    pub rows: Vec<BrowserRowModel>,
}

/// Triage targets used by native browser action surfaces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowserTagTarget {
    /// Move selected/focused rows to trash.
    Trash,
    /// Set selected/focused rows to neutral.
    Neutral,
    /// Mark selected/focused rows as keep.
    Keep,
}

/// Browser action availability consumed by the native shell action strip.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct BrowserActionsModel {
    /// Whether rename can be started for the focused row.
    pub can_rename: bool,
    /// Whether delete can be applied to focused/selected rows.
    pub can_delete: bool,
    /// Whether tag actions can be applied to focused/selected rows.
    pub can_tag: bool,
}

/// Normalized range in milli-units (`0..=1000`) for deterministic UI contracts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NormalizedRangeModel {
    /// Start position in normalized milli-units.
    pub start_milli: u16,
    /// End position in normalized milli-units.
    pub end_milli: u16,
}

impl NormalizedRangeModel {
    /// Build a normalized range, clamping bounds to `0..=1000` and ordering them.
    pub fn new(start_milli: u16, end_milli: u16) -> Self {
        let start = start_milli.min(1000);
        let end = end_milli.min(1000);
        Self {
            start_milli: start.min(end),
            end_milli: end.max(start),
        }
    }
}

/// Waveform preview metadata consumed by the native shell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaveformPanelModel {
    /// Display label for the loaded sample, when any.
    pub loaded_label: Option<String>,
    /// Cursor position in normalized milli-units.
    pub cursor_milli: Option<u16>,
    /// Playhead position in normalized milli-units.
    pub playhead_milli: Option<u16>,
    /// Current waveform selection bounds.
    pub selection_milli: Option<NormalizedRangeModel>,
    /// Visible view start in normalized milli-units.
    pub view_start_milli: u16,
    /// Visible view end in normalized milli-units.
    pub view_end_milli: u16,
    /// Whether loop playback is enabled.
    pub loop_enabled: bool,
    /// Optional tempo label rendered in waveform metadata.
    pub tempo_label: Option<String>,
    /// Optional zoom label rendered in waveform metadata.
    pub zoom_label: Option<String>,
}

impl Default for WaveformPanelModel {
    fn default() -> Self {
        Self {
            loaded_label: None,
            cursor_milli: None,
            playhead_milli: None,
            selection_milli: None,
            view_start_milli: 0,
            view_end_milli: 1000,
            loop_enabled: false,
            tempo_label: None,
            zoom_label: None,
        }
    }
}

/// Structured footer status content for left/center/right status segments.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct StatusBarModel {
    /// Left-aligned status segment.
    pub left: String,
    /// Center-aligned status segment.
    pub center: String,
    /// Right-aligned status segment.
    pub right: String,
}

/// Progress overlay state projected into the native shell.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ProgressOverlayModel {
    /// Whether the overlay is currently visible.
    pub visible: bool,
    /// Whether the overlay is modal.
    pub modal: bool,
    /// Title text for the progress surface.
    pub title: String,
    /// Optional detail line.
    pub detail: Option<String>,
    /// Completed steps.
    pub completed: usize,
    /// Total steps.
    pub total: usize,
    /// Whether the running operation supports cancel.
    pub cancelable: bool,
    /// Whether cancel has already been requested.
    pub cancel_requested: bool,
}

/// Prompt types that can block interaction in the native shell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfirmPromptKind {
    /// Pending destructive waveform edit prompt.
    DestructiveEdit,
    /// Pending browser rename prompt.
    BrowserRename,
    /// Pending folder rename prompt.
    FolderRename,
    /// Pending folder creation prompt.
    FolderCreate,
}

/// Modal confirmation prompt projected into the native shell.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ConfirmPromptModel {
    /// Whether the prompt is currently visible.
    pub visible: bool,
    /// Prompt kind used by the bridge to resolve confirm/cancel behavior.
    pub kind: Option<ConfirmPromptKind>,
    /// Prompt title text.
    pub title: String,
    /// Prompt body text.
    pub message: String,
    /// Confirm action label.
    pub confirm_label: String,
    /// Cancel action label.
    pub cancel_label: String,
    /// Optional target label shown as supplemental metadata.
    pub target_label: Option<String>,
    /// Optional editable prompt input value.
    pub input_value: Option<String>,
    /// Placeholder text for editable prompt input fields.
    pub input_placeholder: Option<String>,
    /// Optional validation error shown below editable prompt input.
    pub input_error: Option<String>,
}

/// Drag/drop overlay content for native-shell feedback.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct DragOverlayModel {
    /// Whether a drag payload is currently active.
    pub active: bool,
    /// Human-friendly payload label.
    pub label: String,
    /// Current hover target label.
    pub target_label: String,
    /// Whether the current target is a valid drop.
    pub valid_target: bool,
}

/// Snapshot of app state required by the native shell renderer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppModel {
    /// Main title rendered in the top bar.
    pub title: String,
    /// Backend description shown in top-bar metadata.
    pub backend_label: String,
    /// Sidebar header label.
    pub sources_label: String,
    /// Footer status text.
    pub status_text: String,
    /// Structured footer status segments used by the native shell footer.
    pub status: StatusBarModel,
    /// Browser action availability for native action surfaces.
    pub browser_actions: BrowserActionsModel,
    /// Progress overlay projection.
    pub progress_overlay: ProgressOverlayModel,
    /// Modal confirm prompt projection.
    pub confirm_prompt: ConfirmPromptModel,
    /// Drag/drop overlay projection.
    pub drag_overlay: DragOverlayModel,
    /// Logical triage/browser columns.
    pub columns: [ColumnModel; 3],
    /// Selected column index (0..=2).
    pub selected_column: usize,
    /// Whether transport/animation should be considered running.
    pub transport_running: bool,
    /// Source panel model consumed by the native renderer.
    pub sources: SourcesPanelModel,
    /// Browser panel summary consumed by the native renderer.
    pub browser: BrowserPanelModel,
    /// Waveform panel summary consumed by the native renderer.
    pub waveform: WaveformPanelModel,
}

impl Default for AppModel {
    fn default() -> Self {
        Self {
            title: String::from("Sempal Native Shell"),
            backend_label: String::from("backend: native_vello"),
            sources_label: String::from("Sources"),
            status_text: String::from("Native shell preview"),
            status: StatusBarModel {
                left: String::from("Native shell preview"),
                center: String::from("rows: 0 | selected: 0 | anchor: — | search: —"),
                right: String::from("col: 2/3"),
            },
            browser_actions: BrowserActionsModel::default(),
            progress_overlay: ProgressOverlayModel::default(),
            confirm_prompt: ConfirmPromptModel::default(),
            drag_overlay: DragOverlayModel::default(),
            columns: [
                ColumnModel::new("Trash", 0),
                ColumnModel::new("Samples", 0),
                ColumnModel::new("Keep", 0),
            ],
            selected_column: 1,
            transport_running: true,
            sources: SourcesPanelModel {
                header: String::from("Sources"),
                search_query: String::new(),
                folder_search_query: String::new(),
                selected_row: None,
                focused_folder_row: None,
                rows: Vec::new(),
                folder_rows: Vec::new(),
                folder_actions: FolderActionsModel::default(),
                folder_recovery: FolderRecoveryModel::default(),
            },
            browser: BrowserPanelModel::default(),
            waveform: WaveformPanelModel::default(),
        }
    }
}

/// Action emitted by the native runtime input layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UiAction {
    /// Select a target triage/browser column.
    SelectColumn { index: usize },
    /// Move column focus left/right.
    MoveColumn { delta: i8 },
    /// Toggle transport playback state.
    ToggleTransport,
    /// Focus the browser/list panel.
    FocusBrowserPanel,
    /// Focus the sources panel.
    FocusSourcesPanel,
    /// Focus the waveform panel.
    FocusWaveformPanel,
    /// Focus the currently loaded sample in the browser.
    FocusLoadedSampleInBrowser,
    /// Focus the browser search field.
    FocusBrowserSearch,
    /// Focus the source-folder search field.
    FocusFolderSearch,
    /// Set folder search query.
    SetFolderSearch { query: String },
    /// Select a source row by index.
    SelectSourceRow { index: usize },
    /// Focus a folder row by index.
    FocusFolderRow { index: usize },
    /// Move folder focus by row delta.
    MoveFolderFocus { delta: i8 },
    /// Create a folder relative to the focused folder.
    StartNewFolder,
    /// Create a folder at the source root.
    StartNewFolderAtRoot,
    /// Start folder rename flow for the focused folder.
    StartFolderRename,
    /// Delete the currently focused folder.
    DeleteFocusedFolder,
    /// Clear staged delete recovery log entries.
    ClearFolderDeleteRecoveryLog,
    /// Move browser focus by a row delta in the visible list.
    MoveBrowserFocus { delta: i8 },
    /// Focus a browser row by visible index.
    FocusBrowserRow { visible_row: usize },
    /// Toggle browser-row selection by visible index.
    ToggleBrowserRowSelection { visible_row: usize },
    /// Extend selection from the anchor to the target visible row.
    ExtendBrowserSelectionToRow { visible_row: usize },
    /// Extend selection additively from the anchor to the target visible row.
    AddRangeBrowserSelection { visible_row: usize },
    /// Move browser focus and extend selection by a visible-row delta.
    ExtendBrowserSelectionFromFocus { delta: i8 },
    /// Move browser focus and extend selection additively by a visible-row delta.
    AddRangeBrowserSelectionFromFocus { delta: i8 },
    /// Toggle selection state for the currently focused browser row.
    ToggleFocusedBrowserRowSelection,
    /// Select every row in the current visible browser list.
    SelectAllBrowserRows,
    /// Set browser search query.
    SetBrowserSearch { query: String },
    /// Set editable text for the active prompt input field.
    SetPromptInput { value: String },
    /// Start inline rename flow for the focused browser row.
    StartBrowserRename,
    /// Confirm the currently pending browser rename prompt.
    ConfirmBrowserRename,
    /// Cancel the currently pending browser rename prompt.
    CancelBrowserRename,
    /// Apply a triage tag to focused/selected browser rows.
    TagBrowserSelection { target: BrowserTagTarget },
    /// Delete focused/selected browser rows.
    DeleteBrowserSelection,
    /// Confirm the currently visible modal prompt.
    ConfirmPrompt,
    /// Cancel the currently visible modal prompt.
    CancelPrompt,
    /// Request cancellation of the active progress operation.
    CancelProgress,
    /// Toggle loop-playback state.
    ToggleLoopPlayback,
    /// Seek waveform/playhead to a normalized milli position (`0..=1000`).
    SeekWaveform { position_milli: u16 },
    /// Set waveform cursor to a normalized milli position (`0..=1000`).
    SetWaveformCursor { position_milli: u16 },
    /// Set waveform selection bounds in normalized milli space (`0..=1000`).
    SetWaveformSelectionRange {
        /// Selection start position in normalized milli-units.
        start_milli: u16,
        /// Selection end position in normalized milli-units.
        end_milli: u16,
    },
    /// Clear active waveform selection.
    ClearWaveformSelection,
    /// Zoom waveform view by discrete steps.
    ZoomWaveform {
        /// When true, zooms in; otherwise zooms out.
        zoom_in: bool,
        /// Number of discrete zoom steps to apply.
        steps: u8,
    },
    /// Fit waveform view to the active selection.
    ZoomWaveformToSelection,
    /// Reset waveform view to full-range (`0..=1000`).
    ZoomWaveformFull,
    /// Trigger undo.
    Undo,
    /// Trigger redo.
    Redo,
}

/// Frame-level feedback from renderer to host bridge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameBuildResult {
    /// Number of generated shape primitives.
    pub primitive_count: usize,
    /// Number of generated text runs.
    pub text_run_count: usize,
    /// Whether runtime should keep animating while idle.
    pub needs_animation: bool,
}

/// Host bridge consumed by the native runtime.
pub trait NativeAppBridge {
    /// Pull the latest app model snapshot before frame build.
    fn pull_model(&mut self) -> AppModel;

    /// Handle a user action emitted by runtime input processing.
    fn on_action(&mut self, _action: UiAction) {}

    /// Observe a built frame result for diagnostics or telemetry.
    fn on_frame_result(&mut self, _result: FrameBuildResult) {}

    /// Lifecycle hook fired when the runtime is shutting down.
    fn on_exit(&mut self) {}
}
