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

/// Sidebar model for source browsing controls.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SourcesPanelModel {
    /// Header text for the source panel.
    pub header: String,
    /// Active source-search query.
    pub search_query: String,
    /// Selected row index, if any.
    pub selected_row: Option<usize>,
    /// Rows to render in the source panel.
    pub rows: Vec<SourceRowModel>,
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
            selected,
            focused,
        }
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
    /// Whether browser search/filter work is still running in the background.
    pub busy: bool,
    /// Display label for the currently focused sample, when known.
    pub focused_sample_label: Option<String>,
    /// Selection anchor in visible-row space.
    pub anchor_visible_row: Option<usize>,
    /// Visible rows rendered by the native browser panel.
    pub rows: Vec<BrowserRowModel>,
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
        }
    }
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
                selected_row: None,
                rows: Vec::new(),
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
    /// Select a source row by index.
    SelectSourceRow { index: usize },
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
    /// Toggle loop-playback state.
    ToggleLoopPlayback,
    /// Seek waveform/playhead to a normalized milli position (`0..=1000`).
    SeekWaveform { position_milli: u16 },
    /// Set waveform cursor to a normalized milli position (`0..=1000`).
    SetWaveformCursor { position_milli: u16 },
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
