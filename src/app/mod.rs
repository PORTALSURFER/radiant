//! App-facing contracts between the `radiant` runtime and a host application.
//!
//! The host provides one immutable [`AppModel`] snapshot per frame.
//! `radiant` consumes that snapshot to:
//! 1. derive a frame from the retained shell model,
//! 2. run input hit-testing and command handling,
//! 3. emit [`UiAction`] values describing intent.
//!
//! Each action is routed back through the host bridge so state updates remain
//! in application code. This keeps GUI-specific input handling and event propagation
//! in `radiant` while preserving a one-way data flow for business logic.
//!
//! ## Diff/update model
//! The update model is explicit and incremental:
//! - `AppModel`: snapshot of current application state for rendering.
//! - action batch: a minimal set of mutations (`UiAction`) produced from input.
//! - host consumes actions and publishes the next model snapshot.
//! - UI repeatedly re-renders from the latest snapshot.
//!
//! ## Event propagation model
//! All backend input is normalized at the runtime boundary into
//! [`KeyCode`](crate::gui::input::KeyCode) and layout-space pointer events.
//! `radiant` resolves these to deterministic shell targets (hit test → state
//! transition → action emission) and does not mutate the host domain state directly.
mod declarative;

use crate::gui::types::ImageRgba;
use std::sync::Arc;

pub use declarative::{DeclarativeBridge, declarative_bridge};

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
    /// Signed keep/trash rating level shown alongside the row label (`-3..=3`).
    pub rating_level: i8,
    /// Optional inline metadata label rendered at the right edge of the sample lane.
    ///
    /// Hosts can use this for secondary metadata such as BPM or loop/length tags.
    /// Keep/trash text should usually stay empty because the shell already renders
    /// signed rating state via the right-edge indicator rectangles.
    pub bucket_label: Option<String>,
    /// Whether this row is currently selected in multi-selection state.
    pub selected: bool,
    /// Whether this row currently has focus/caret.
    pub focused: bool,
    /// Whether the backing sample file is missing on disk.
    pub missing: bool,
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
            rating_level: 0,
            bucket_label: None,
            selected,
            focused,
            missing: false,
        }
    }

    /// Attach a signed keep/trash rating level for inline row indicators.
    pub fn with_rating_level(mut self, rating_level: i8) -> Self {
        self.rating_level = rating_level.clamp(-3, 3);
        self
    }

    /// Attach an explicit inline metadata label for this row.
    pub fn with_bucket_label(mut self, label: impl Into<String>) -> Self {
        self.bucket_label = Some(label.into());
        self
    }

    /// Mark whether the backing sample file is missing on disk.
    pub fn with_missing(mut self, missing: bool) -> Self {
        self.missing = missing;
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

/// Browser chrome copy used by the native shell toolbar and tab strip.
///
/// This separates rendered UI labels from interaction state so hosts can
/// provide layout-specific wording without hardcoded renderer strings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserChromeModel {
    /// Label for the list tab.
    pub samples_tab_label: String,
    /// Label for the map tab.
    pub map_tab_label: String,
    /// Prefix label shown before active search queries.
    pub search_prefix_label: String,
    /// Placeholder label shown when no search query is active.
    pub search_placeholder: String,
    /// Status label shown when browser background work is idle.
    pub activity_ready_label: String,
    /// Status label shown when browser background work is running.
    pub activity_busy_label: String,
    /// Prefix label shown before active sort order labels.
    pub sort_prefix_label: String,
    /// Label describing the active sort order.
    pub sort_order_label: String,
    /// Label describing similarity mode in the map/header chrome.
    pub similarity_toggle_label: String,
    /// Footer/status label for total browser item counts.
    pub item_count_label: String,
}

impl Default for BrowserChromeModel {
    fn default() -> Self {
        Self {
            samples_tab_label: String::from("Samples"),
            map_tab_label: String::from("Similarity map"),
            search_prefix_label: String::from("Search"),
            search_placeholder: String::from("Search samples (Ctrl+F)"),
            activity_ready_label: String::from("Ready"),
            activity_busy_label: String::from("Filtering"),
            sort_prefix_label: String::from("Sort"),
            sort_order_label: String::from("List order"),
            similarity_toggle_label: String::from("points"),
            item_count_label: String::from("0 items"),
        }
    }
}

/// Render mode label for the map panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MapRenderModeModel {
    /// Rendered as a density heatmap.
    Heatmap,
    /// Rendered as individual points.
    #[default]
    Points,
}

/// Render data for one map point shown in the native map canvas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapPointModel {
    /// Stable sample id used to route click actions back to the host.
    pub sample_id: String,
    /// X position normalized to milli-units (`0..=1000`) across map bounds.
    pub x_milli: u16,
    /// Y position normalized to milli-units (`0..=1000`) across map bounds.
    pub y_milli: u16,
    /// Optional cluster id for color grouping.
    pub cluster_id: Option<i32>,
    /// Whether this point is currently selected in map state.
    pub selected: bool,
    /// Whether this point corresponds to the focused browser sample.
    pub focused: bool,
}

/// Summary of map state consumed by the native shell map tab.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct MapPanelModel {
    /// Whether the map tab is currently active in the browser panel.
    pub active: bool,
    /// Human-readable map summary line.
    pub summary: String,
    /// Legend/status label for map render mode and point density.
    pub legend_label: String,
    /// Selection/focus label for the currently highlighted map sample.
    pub selection_label: String,
    /// Hover label for the currently hovered map sample, when any.
    pub hover_label: String,
    /// Cluster summary label for projected map points.
    pub cluster_label: String,
    /// Viewport label describing zoom/pan state.
    pub viewport_label: String,
    /// Optional error text shown when map data cannot be loaded.
    pub error: Option<String>,
    /// Current map render mode.
    pub render_mode: MapRenderModeModel,
    /// Points available for rendering in normalized map space.
    pub points: Vec<MapPointModel>,
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
    /// Current waveform edit-selection bounds (right-click paint range).
    pub edit_selection_milli: Option<NormalizedRangeModel>,
    /// End position for the edit fade-in region in normalized milli-units.
    ///
    /// When absent, the fade-in handle defaults to the edit-selection start edge.
    pub edit_fade_in_end_milli: Option<u16>,
    /// Start position for the edit fade-in mute region in normalized milli-units.
    ///
    /// When absent, the bottom fade-in handle defaults to the edit-selection start edge.
    pub edit_fade_in_mute_start_milli: Option<u16>,
    /// Fade-in curve tension in normalized milli-units (`0..=1000`).
    pub edit_fade_in_curve_milli: Option<u16>,
    /// Start position for the edit fade-out region in normalized milli-units.
    ///
    /// When absent, the fade-out handle defaults to the edit-selection end edge.
    pub edit_fade_out_start_milli: Option<u16>,
    /// End position for the edit fade-out mute region in normalized milli-units.
    ///
    /// When absent, the bottom fade-out handle defaults to the edit-selection end edge.
    pub edit_fade_out_mute_end_milli: Option<u16>,
    /// Fade-out curve tension in normalized milli-units (`0..=1000`).
    pub edit_fade_out_curve_milli: Option<u16>,
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
    /// Cached signature for waveform image updates.
    pub waveform_image_signature: Option<u64>,
    /// Optional rasterized waveform payload for rendering the waveform preview.
    ///
    /// Hosts render this image inside the waveform plot area and keep overlays on top.
    /// The payload is shared so projection cache hits stay allocation-free.
    pub waveform_image: Option<Arc<ImageRgba>>,
}

impl Default for WaveformPanelModel {
    fn default() -> Self {
        Self {
            loaded_label: None,
            cursor_milli: None,
            playhead_milli: None,
            selection_milli: None,
            edit_selection_milli: None,
            edit_fade_in_end_milli: None,
            edit_fade_in_mute_start_milli: None,
            edit_fade_in_curve_milli: None,
            edit_fade_out_start_milli: None,
            edit_fade_out_mute_end_milli: None,
            edit_fade_out_curve_milli: None,
            view_start_milli: 0,
            view_end_milli: 1000,
            loop_enabled: false,
            tempo_label: None,
            zoom_label: None,
            waveform_image_signature: None,
            waveform_image: None,
        }
    }
}

/// Waveform chrome copy used by metadata lines in the native shell header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaveformChannelViewModel {
    /// Collapse channels into one mono envelope.
    Mono,
    /// Render left/right channels in split stereo mode.
    Stereo,
}

/// Waveform chrome copy used by metadata lines and control surfaces.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaveformChromeModel {
    /// Extra transport metadata hint shown alongside waveform labels.
    pub transport_hint: String,
    /// Current channel-view mode used by waveform rendering.
    pub channel_view: WaveformChannelViewModel,
    /// Whether normalized audition playback is enabled.
    pub normalized_audition_enabled: bool,
    /// Whether BPM snapping is enabled for waveform edits.
    pub bpm_snap_enabled: bool,
    /// Whether transient snapping is enabled for waveform edits.
    pub transient_snap_enabled: bool,
    /// Whether transient markers are visible on the waveform.
    pub transient_markers_enabled: bool,
    /// Whether slice mode is currently active.
    pub slice_mode_enabled: bool,
}

impl Default for WaveformChromeModel {
    fn default() -> Self {
        Self {
            transport_hint: String::from("transport idle"),
            channel_view: WaveformChannelViewModel::Mono,
            normalized_audition_enabled: false,
            bpm_snap_enabled: false,
            transient_snap_enabled: false,
            transient_markers_enabled: true,
            slice_mode_enabled: false,
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

/// Update-check status projected into the native shell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum UpdateStatusModel {
    /// No update activity in progress.
    #[default]
    Idle,
    /// Update check is running.
    Checking,
    /// A newer update is available.
    Available,
    /// Update check failed.
    Error,
}

/// Update panel state used by native top-bar actions.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct UpdatePanelModel {
    /// Current update-check status.
    pub status: UpdateStatusModel,
    /// Status label rendered in native top-bar chrome.
    pub status_label: String,
    /// Action hint label rendered near update controls.
    pub action_hint_label: String,
    /// Supplemental release-notes label rendered under update hints.
    pub release_notes_label: String,
    /// Available release tag, when present.
    pub available_tag: Option<String>,
    /// Available release URL, when present.
    pub available_url: Option<String>,
    /// Last error message from update checks, if any.
    pub last_error: Option<String>,
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
#[derive(Clone, Debug, PartialEq)]
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
    /// Master output volume normalized to `0.0..=1.0`.
    pub volume: f32,
    /// Whether transport/animation should be considered running.
    pub transport_running: bool,
    /// Source panel model consumed by the native renderer.
    pub sources: SourcesPanelModel,
    /// Browser panel summary consumed by the native renderer.
    pub browser: BrowserPanelModel,
    /// Browser chrome labels consumed by native tabs/toolbar/footer text.
    pub browser_chrome: BrowserChromeModel,
    /// Map panel summary consumed by the native renderer.
    pub map: MapPanelModel,
    /// Waveform panel summary consumed by the native renderer.
    pub waveform: WaveformPanelModel,
    /// Waveform chrome labels consumed by the native waveform header.
    pub waveform_chrome: WaveformChromeModel,
    /// Update surface summary consumed by the native top bar.
    pub update: UpdatePanelModel,
    /// Current keyboard focus bucket used for contextual native key routing.
    pub focus_context: FocusContextModel,
}

impl Default for AppModel {
    fn default() -> Self {
        Self {
            title: String::from("Sempal"),
            backend_label: String::from("backend: native_vello"),
            sources_label: String::from("Sources"),
            status_text: String::new(),
            status: StatusBarModel {
                left: String::new(),
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
            volume: 1.0,
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
            browser_chrome: BrowserChromeModel::default(),
            map: MapPanelModel::default(),
            waveform: WaveformPanelModel::default(),
            waveform_chrome: WaveformChromeModel::default(),
            update: UpdatePanelModel::default(),
            focus_context: FocusContextModel::None,
        }
    }
}

/// Action emitted by the native runtime input layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UiAction {
    /// Select a target triage/browser column.
    SelectColumn {
        /// Target column index in the visible triage column set.
        index: usize,
    },
    /// Move column focus left/right.
    MoveColumn {
        /// Signed column delta (`-1` for left, `+1` for right).
        delta: i8,
    },
    /// Toggle transport playback state.
    ToggleTransport,
    /// Start playback from the saved play-start marker (or cursor fallback).
    ReplayFromLastStart,
    /// Handle Escape key behavior for playback, selection, and cursor cleanup.
    HandleEscape,
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
    /// Clear browser-search focus while preserving the current query text.
    BlurBrowserSearch,
    /// Open the native options menu.
    OpenOptionsMenu,
    /// Focus the source-folder search field.
    FocusFolderSearch,
    /// Set folder search query.
    SetFolderSearch {
        /// Full folder-search query text.
        query: String,
    },
    /// Select a source row by index.
    SelectSourceRow {
        /// Target source row index.
        index: usize,
    },
    /// Reload wav entries for one source row.
    ReloadSourceRow {
        /// Target source row index.
        index: usize,
    },
    /// Run a hard sync/rescan for one source row.
    HardSyncSourceRow {
        /// Target source row index.
        index: usize,
    },
    /// Open one source row folder in the system file manager.
    OpenSourceFolderRow {
        /// Target source row index.
        index: usize,
    },
    /// Remove one configured source row.
    RemoveSourceRow {
        /// Target source row index.
        index: usize,
    },
    /// Remove missing/dead-link rows for one source row.
    RemoveDeadLinksForSourceRow {
        /// Target source row index.
        index: usize,
    },
    /// Focus a folder row by index.
    FocusFolderRow {
        /// Target folder row index.
        index: usize,
    },
    /// Move folder focus by row delta.
    MoveFolderFocus {
        /// Signed row delta applied to focused folder selection.
        delta: i8,
    },
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
    MoveBrowserFocus {
        /// Signed visible-row delta for browser focus movement.
        delta: i8,
    },
    /// Focus a browser row by visible index.
    FocusBrowserRow {
        /// Target visible row index in the browser list.
        visible_row: usize,
    },
    /// Commit the currently focused browser row as the active loaded sample.
    CommitFocusedBrowserRow,
    /// Toggle browser-row selection by visible index.
    ToggleBrowserRowSelection {
        /// Target visible row index in the browser list.
        visible_row: usize,
    },
    /// Extend selection from the anchor to the target visible row.
    ExtendBrowserSelectionToRow {
        /// Target visible row index used as selection endpoint.
        visible_row: usize,
    },
    /// Extend selection additively from the anchor to the target visible row.
    AddRangeBrowserSelection {
        /// Target visible row index used as additive selection endpoint.
        visible_row: usize,
    },
    /// Move browser focus and extend selection by a visible-row delta.
    ExtendBrowserSelectionFromFocus {
        /// Signed visible-row delta from current focus.
        delta: i8,
    },
    /// Move browser focus and extend selection additively by a visible-row delta.
    AddRangeBrowserSelectionFromFocus {
        /// Signed visible-row delta from current focus.
        delta: i8,
    },
    /// Toggle selection state for the currently focused browser row.
    ToggleFocusedBrowserRowSelection,
    /// Select every row in the current visible browser list.
    SelectAllBrowserRows,
    /// Set browser search query.
    SetBrowserSearch {
        /// Full browser-search query text.
        query: String,
    },
    /// Set active browser tab (`map = true` selects map; otherwise list).
    SetBrowserTab {
        /// Whether to switch to map tab (`true`) or list tab (`false`).
        map: bool,
    },
    /// Focus a specific map sample by stable sample id.
    FocusMapSample {
        /// Stable sample identifier used by map hit-testing.
        sample_id: String,
    },
    /// Set editable text for the active prompt input field.
    SetPromptInput {
        /// Prompt input text after edit.
        value: String,
    },
    /// Start inline rename flow for the focused browser row.
    StartBrowserRename,
    /// Confirm the currently pending browser rename prompt.
    ConfirmBrowserRename,
    /// Cancel the currently pending browser rename prompt.
    CancelBrowserRename,
    /// Apply a triage tag to focused/selected browser rows.
    TagBrowserSelection {
        /// Triage bucket applied to focused/selected browser rows.
        target: BrowserTagTarget,
    },
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
    /// Set waveform channel view mode.
    SetWaveformChannelView {
        /// When true, uses split stereo mode; otherwise mono mode.
        stereo: bool,
    },
    /// Enable/disable normalized audition playback.
    SetNormalizedAuditionEnabled {
        /// Target enabled state.
        enabled: bool,
    },
    /// Enable/disable BPM snapping for waveform edits.
    SetBpmSnapEnabled {
        /// Target enabled state.
        enabled: bool,
    },
    /// Adjust waveform BPM by a signed whole-number delta.
    AdjustWaveformBpm {
        /// Signed BPM delta applied to the current value.
        delta: i8,
    },
    /// Set waveform BPM to an explicit positive numeric value.
    SetWaveformBpmValue {
        /// Absolute BPM value in tenths (`1200` = `120.0 BPM`).
        value_tenths: u16,
    },
    /// Enable/disable transient snapping for waveform edits.
    SetTransientSnapEnabled {
        /// Target enabled state.
        enabled: bool,
    },
    /// Enable/disable transient marker visibility.
    SetTransientMarkersEnabled {
        /// Target enabled state.
        enabled: bool,
    },
    /// Enable/disable slice mode.
    SetSliceModeEnabled {
        /// Target enabled state.
        enabled: bool,
    },
    /// Set output volume to a normalized milli value (`0..=1000`).
    SetVolume {
        /// Normalized milli volume value (`0..=1000`).
        value_milli: u16,
    },
    /// Persist the current volume setting after a drag/continuous edit.
    CommitVolumeSetting,
    /// Seek waveform/playhead to a normalized milli position (`0..=1000`).
    SeekWaveform {
        /// Normalized milli target position (`0..=1000`).
        position_milli: u16,
    },
    /// Set waveform cursor to a normalized milli position (`0..=1000`).
    SetWaveformCursor {
        /// Normalized milli cursor position (`0..=1000`).
        position_milli: u16,
    },
    /// Set waveform selection bounds in normalized milli space (`0..=1000`).
    SetWaveformSelectionRange {
        /// Selection start position in normalized milli-units.
        start_milli: u16,
        /// Selection end position in normalized milli-units.
        end_milli: u16,
    },
    /// Set waveform edit-selection bounds in normalized milli space (`0..=1000`).
    SetWaveformEditSelectionRange {
        /// Edit-selection start position in normalized milli-units.
        start_milli: u16,
        /// Edit-selection end position in normalized milli-units.
        end_milli: u16,
    },
    /// Set the edit fade-in end handle in normalized milli space (`0..=1000`).
    SetWaveformEditFadeInEnd {
        /// Fade-in end handle position in normalized milli-units.
        position_milli: u16,
    },
    /// Set the edit fade-in mute start handle in normalized milli space (`0..=1000`).
    SetWaveformEditFadeInMuteStart {
        /// Fade-in mute-start handle position in normalized milli-units.
        position_milli: u16,
    },
    /// Set the edit fade-in curve tension in normalized milli space (`0..=1000`).
    SetWaveformEditFadeInCurve {
        /// Fade-in curve value in normalized milli-units.
        curve_milli: u16,
    },
    /// Set the edit fade-out start handle in normalized milli space (`0..=1000`).
    SetWaveformEditFadeOutStart {
        /// Fade-out start handle position in normalized milli-units.
        position_milli: u16,
    },
    /// Set the edit fade-out mute end handle in normalized milli space (`0..=1000`).
    SetWaveformEditFadeOutMuteEnd {
        /// Fade-out mute-end handle position in normalized milli-units.
        position_milli: u16,
    },
    /// Set the edit fade-out curve tension in normalized milli space (`0..=1000`).
    SetWaveformEditFadeOutCurve {
        /// Fade-out curve value in normalized milli-units.
        curve_milli: u16,
    },
    /// Finish an active waveform edit-fade drag gesture.
    FinishWaveformEditFadeDrag,
    /// Start dragging the current waveform playback selection from its drag handle.
    StartWaveformSelectionDrag {
        /// Pointer x-position in logical UI coordinates.
        pointer_x: u16,
        /// Pointer y-position in logical UI coordinates.
        pointer_y: u16,
    },
    /// Update the active waveform-selection drag with the latest pointer position.
    UpdateWaveformSelectionDrag {
        /// Pointer x-position in logical UI coordinates.
        pointer_x: u16,
        /// Pointer y-position in logical UI coordinates.
        pointer_y: u16,
        /// Whether the pointer currently hovers the sample browser list.
        over_browser_list: bool,
        /// Whether Shift is currently held.
        shift_down: bool,
        /// Whether Alt is currently held.
        alt_down: bool,
    },
    /// Finish the active waveform-selection drag gesture.
    FinishWaveformSelectionDrag,
    /// Arm a playback-selection translate gesture from the bottom-center handle.
    BeginWaveformSelectionShift {
        /// Pointer milli position captured at press time.
        pointer_milli: u16,
        /// Selection start preserved across the translate gesture.
        start_milli: u16,
        /// Selection end preserved across the translate gesture.
        end_milli: u16,
    },
    /// Arm an edit-selection translate gesture from the bottom-center handle.
    BeginWaveformEditSelectionShift {
        /// Pointer milli position captured at press time.
        pointer_milli: u16,
        /// Edit-selection start preserved across the translate gesture.
        start_milli: u16,
        /// Edit-selection end preserved across the translate gesture.
        end_milli: u16,
    },
    /// Clear active waveform selection.
    ClearWaveformSelection,
    /// Clear active waveform edit selection.
    ClearWaveformEditSelection,
    /// Zoom waveform view by discrete steps.
    ZoomWaveform {
        /// When true, zooms in; otherwise zooms out.
        zoom_in: bool,
        /// Number of discrete zoom steps to apply.
        steps: u8,
        /// Optional high-precision hover anchor ratio within current waveform view.
        ///
        /// Values are stored in micros (`0..=1_000_000`) to preserve deterministic
        /// equality semantics while keeping pointer-anchored zoom stable at deep zoom.
        anchor_ratio_micros: Option<u32>,
    },
    /// Fit waveform view to the active selection.
    ZoomWaveformToSelection,
    /// Reset waveform view to full-range (`0..=1000`).
    ZoomWaveformFull,
    /// Trigger undo.
    Undo,
    /// Trigger redo.
    Redo,
    /// Trigger an explicit update check.
    CheckForUpdates,
    /// Open the available update URL.
    OpenUpdateLink,
    /// Install update and exit where supported.
    InstallUpdate,
    /// Dismiss current update notification.
    DismissUpdate,
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
    /// End-to-end frame time in microseconds for the redraw pass.
    pub frame_total_us: u32,
    /// Present-stage duration in microseconds for the redraw pass.
    pub present_us: u32,
    /// Frame-time budget used to classify redraw jank.
    pub frame_budget_us: u32,
    /// Whether the frame exceeded the configured frame-time budget.
    pub jank: bool,
    /// Whether the redraw produced a successful surface present.
    pub presented: bool,
    /// Whether a present was expected but not completed for this redraw.
    pub missed_present: bool,
}

/// Bitmask describing which projection segments changed during the last model pull.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DirtySegments {
    bits: u16,
}

impl DirtySegments {
    /// Status-bar content segment.
    pub const STATUS_BAR: u16 = 1 << 0;
    /// Browser metadata/chrome segment.
    pub const BROWSER_FRAME: u16 = 1 << 1;
    /// Browser row-window segment.
    pub const BROWSER_ROWS_WINDOW: u16 = 1 << 2;
    /// Map-panel segment.
    pub const MAP_PANEL: u16 = 1 << 3;
    /// Waveform panel/chrome segment.
    pub const WAVEFORM_OVERLAY: u16 = 1 << 4;
    /// Static content that is outside explicit segment buckets.
    pub const GLOBAL_STATIC: u16 = 1 << 5;
    /// State-overlay model fields.
    pub const STATE_OVERLAY: u16 = 1 << 6;
    /// Motion-overlay model fields.
    pub const MOTION_OVERLAY: u16 = 1 << 7;

    const STATIC_MASK: u16 = Self::STATUS_BAR
        | Self::BROWSER_FRAME
        | Self::BROWSER_ROWS_WINDOW
        | Self::MAP_PANEL
        | Self::WAVEFORM_OVERLAY
        | Self::GLOBAL_STATIC;
    const OVERLAY_MASK: u16 = Self::STATE_OVERLAY | Self::MOTION_OVERLAY;

    /// Return an empty segment mask.
    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    /// Return a full segment mask.
    pub const fn all() -> Self {
        Self {
            bits: Self::STATIC_MASK | Self::OVERLAY_MASK,
        }
    }

    /// Construct a segment mask from raw bits.
    pub const fn from_bits(bits: u16) -> Self {
        Self {
            bits: bits & (Self::STATIC_MASK | Self::OVERLAY_MASK),
        }
    }

    /// Return raw bit contents for diagnostics and tests.
    pub const fn bits(self) -> u16 {
        self.bits
    }

    /// Return `true` when the mask contains no segments.
    pub const fn is_empty(self) -> bool {
        self.bits == 0
    }

    /// Return `true` when any static segment requires rebuild.
    pub const fn requires_static_rebuild(self) -> bool {
        (self.bits & Self::STATIC_MASK) != 0
    }

    /// Return `true` when any overlay segment requires rebuild.
    pub const fn requires_overlay_rebuild(self) -> bool {
        (self.bits & Self::OVERLAY_MASK) != 0
    }

    /// Insert one or more segment bits into this mask.
    pub fn insert(&mut self, bits: u16) {
        self.bits |= bits & (Self::STATIC_MASK | Self::OVERLAY_MASK);
    }
}

/// Monotonic revision counters for static projection segments.
///
/// Bridges bump the counters for segments whose projected model slices changed on
/// the most recent `pull_model`. Runtimes use these revisions in retained-scene
/// cache keys to avoid expensive segment hashing on every frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SegmentRevisions {
    /// Status-bar projection revision.
    pub status_bar: u64,
    /// Browser metadata/chrome projection revision.
    pub browser_frame: u64,
    /// Browser visible-row window projection revision.
    pub browser_rows_window: u64,
    /// Map-panel projection revision.
    pub map_panel: u64,
    /// Waveform panel/chrome projection revision.
    pub waveform_overlay: u64,
    /// Global static fields projection revision.
    pub global_static: u64,
}

impl SegmentRevisions {
    /// Return whether any static-segment revision is non-zero.
    pub const fn has_static_revisions(self) -> bool {
        self.status_bar != 0
            || self.browser_frame != 0
            || self.browser_rows_window != 0
            || self.map_panel != 0
            || self.waveform_overlay != 0
            || self.global_static != 0
    }

    /// Bump revisions for the static segments flagged in `dirty_segments`.
    pub fn bump_for_dirty_segments(&mut self, dirty_segments: DirtySegments) {
        let bits = dirty_segments.bits();
        if (bits & DirtySegments::STATUS_BAR) != 0 {
            self.status_bar = self.status_bar.saturating_add(1);
        }
        if (bits & DirtySegments::BROWSER_FRAME) != 0 {
            self.browser_frame = self.browser_frame.saturating_add(1);
        }
        if (bits & DirtySegments::BROWSER_ROWS_WINDOW) != 0 {
            self.browser_rows_window = self.browser_rows_window.saturating_add(1);
        }
        if (bits & DirtySegments::MAP_PANEL) != 0 {
            self.map_panel = self.map_panel.saturating_add(1);
        }
        if (bits & DirtySegments::WAVEFORM_OVERLAY) != 0 {
            self.waveform_overlay = self.waveform_overlay.saturating_add(1);
        }
        if (bits & DirtySegments::GLOBAL_STATIC) != 0 {
            self.global_static = self.global_static.saturating_add(1);
        }
    }
}

/// Motion-sensitive slice of the app model used for incremental overlay rendering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeMotionModel {
    /// Transport animation state used by motion overlays.
    pub transport_running: bool,
    /// Whether map mode is active for tab overlay tinting.
    pub map_active: bool,
    /// Waveform selected playback window in normalized milliseconds.
    pub waveform_selection_milli: Option<NormalizedRangeModel>,
    /// Waveform edit-selection window in normalized milliseconds.
    pub waveform_edit_selection_milli: Option<NormalizedRangeModel>,
    /// Waveform edit fade-in end handle in normalized milliseconds.
    pub waveform_edit_fade_in_end_milli: Option<u16>,
    /// Waveform edit fade-in mute-start handle in normalized milliseconds.
    pub waveform_edit_fade_in_mute_start_milli: Option<u16>,
    /// Waveform edit fade-in curve tension in normalized milliseconds.
    pub waveform_edit_fade_in_curve_milli: Option<u16>,
    /// Waveform edit fade-out start handle in normalized milliseconds.
    pub waveform_edit_fade_out_start_milli: Option<u16>,
    /// Waveform edit fade-out mute-end handle in normalized milliseconds.
    pub waveform_edit_fade_out_mute_end_milli: Option<u16>,
    /// Waveform edit fade-out curve tension in normalized milliseconds.
    pub waveform_edit_fade_out_curve_milli: Option<u16>,
    /// Whether loop playback is enabled for the active waveform selection.
    pub waveform_loop_enabled: bool,
    /// Waveform cursor position in normalized milliseconds.
    pub waveform_cursor_milli: Option<u16>,
    /// Waveform playhead position in normalized milliseconds.
    pub waveform_playhead_milli: Option<u16>,
    /// Waveform playhead position in normalized micro-units (`0..=1_000_000`).
    ///
    /// This complements `waveform_playhead_milli` for high-precision motion
    /// overlays so transport playback can animate smoothly while preserving
    /// deterministic integer contracts across runtime boundaries.
    pub waveform_playhead_micros: Option<u32>,
    /// Current waveform view start in normalized milliseconds.
    pub waveform_view_start_milli: u16,
    /// Current waveform view end in normalized milliseconds.
    pub waveform_view_end_milli: u16,
    /// Human-readable tempo metadata.
    pub waveform_tempo_label: Option<String>,
    /// Human-readable zoom metadata.
    pub waveform_zoom_label: Option<String>,
    /// Loaded waveform label shown in the waveform overlay header.
    pub waveform_loaded_label: Option<String>,
    /// Stable image signature for detecting waveform image updates during motion-only frames.
    ///
    /// Hosts can force static-scene rebuilds when this value changes.
    pub waveform_image_signature: Option<u64>,
    /// Transport hint rendered with waveform metadata.
    pub waveform_transport_hint: String,
    /// Current waveform channel-view mode.
    pub waveform_channel_view: WaveformChannelViewModel,
    /// Whether normalized audition playback is enabled.
    pub waveform_normalized_audition_enabled: bool,
    /// Whether BPM snapping is enabled.
    pub waveform_bpm_snap_enabled: bool,
    /// Whether transient snapping is enabled.
    pub waveform_transient_snap_enabled: bool,
    /// Whether transient markers are visible.
    pub waveform_transient_markers_enabled: bool,
    /// Whether slice mode is active.
    pub waveform_slice_mode_enabled: bool,
    /// Right-aligned status-bar text rendered in the motion overlay.
    pub status_right: String,
}

impl NativeMotionModel {
    /// Build a motion model from a full application model snapshot.
    pub fn from_app_model(model: &AppModel) -> Self {
        Self {
            transport_running: model.transport_running,
            map_active: model.map.active,
            waveform_selection_milli: model.waveform.selection_milli,
            waveform_edit_selection_milli: model.waveform.edit_selection_milli,
            waveform_edit_fade_in_end_milli: model.waveform.edit_fade_in_end_milli,
            waveform_edit_fade_in_mute_start_milli: model.waveform.edit_fade_in_mute_start_milli,
            waveform_edit_fade_in_curve_milli: model.waveform.edit_fade_in_curve_milli,
            waveform_edit_fade_out_start_milli: model.waveform.edit_fade_out_start_milli,
            waveform_edit_fade_out_mute_end_milli: model.waveform.edit_fade_out_mute_end_milli,
            waveform_edit_fade_out_curve_milli: model.waveform.edit_fade_out_curve_milli,
            waveform_loop_enabled: model.waveform.loop_enabled,
            waveform_cursor_milli: model.waveform.cursor_milli,
            waveform_playhead_milli: model.waveform.playhead_milli,
            waveform_playhead_micros: model
                .waveform
                .playhead_milli
                .map(|milli| u32::from(milli).saturating_mul(1000)),
            waveform_view_start_milli: model.waveform.view_start_milli,
            waveform_view_end_milli: model.waveform.view_end_milli,
            waveform_tempo_label: model.waveform.tempo_label.clone(),
            waveform_zoom_label: model.waveform.zoom_label.clone(),
            waveform_loaded_label: model.waveform.loaded_label.clone(),
            waveform_image_signature: model.waveform.waveform_image_signature,
            waveform_transport_hint: model.waveform_chrome.transport_hint.clone(),
            waveform_channel_view: model.waveform_chrome.channel_view,
            waveform_normalized_audition_enabled: model.waveform_chrome.normalized_audition_enabled,
            waveform_bpm_snap_enabled: model.waveform_chrome.bpm_snap_enabled,
            waveform_transient_snap_enabled: model.waveform_chrome.transient_snap_enabled,
            waveform_transient_markers_enabled: model.waveform_chrome.transient_markers_enabled,
            waveform_slice_mode_enabled: model.waveform_chrome.slice_mode_enabled,
            status_right: model.status.right.clone(),
        }
    }
}

/// Host bridge consumed by the native runtime.
pub trait NativeAppBridge {
    /// Project the latest app model snapshot before frame build.
    ///
    /// This is the declarative render projection entrypoint:
    /// host state in, immutable view-model snapshot out.
    fn project_model(&mut self) -> Arc<AppModel>;

    /// Pull the latest app model snapshot before frame build.
    ///
    /// This compatibility shim unwraps the projected arc when callers need
    /// owned model values.
    fn pull_model(&mut self) -> AppModel {
        Arc::unwrap_or_clone(self.project_model())
    }

    /// Pull the latest app model snapshot as a shared immutable `Arc`.
    ///
    /// Runtimes can use this to avoid full-model cloning on retained cache hits
    /// when hosts already store projected models behind shared ownership.
    fn pull_model_arc(&mut self) -> Arc<AppModel> {
        self.project_model()
    }

    /// Project motion-sensitive fields only; this allows renderers to avoid
    /// full-model work on animation-only ticks.
    fn project_motion_model(&mut self) -> Option<NativeMotionModel> {
        None
    }

    /// Pull motion-sensitive fields only; this allows renderers to avoid
    /// full-model work on animation-only ticks.
    fn pull_motion_model(&mut self) -> Option<NativeMotionModel> {
        self.project_motion_model()
    }

    /// Return and clear dirty projection segments produced by the latest `pull_model`.
    ///
    /// Implementations that do not track segment deltas may return
    /// [`DirtySegments::all`] to preserve conservative full-rebuild behavior.
    fn take_dirty_segments(&mut self) -> DirtySegments {
        DirtySegments::all()
    }

    /// Return static-segment revisions produced by the latest `pull_model`.
    ///
    /// Bridges that do not track segment revisions may return
    /// [`SegmentRevisions::default`] and runtimes should fall back to conservative
    /// behavior.
    fn take_segment_revisions(&mut self) -> SegmentRevisions {
        SegmentRevisions::default()
    }

    /// Reduce one UI action into host state.
    fn reduce_action(&mut self, _action: UiAction) {}

    /// Install a runtime repaint signal used by background workers.
    ///
    /// Hosts that run background jobs can store this callback and forward it into
    /// worker systems so asynchronous completions can wake the UI runtime.
    fn install_repaint_signal(&mut self, _signal: Arc<dyn crate::gui::repaint::RepaintSignal>) {}

    /// Handle a user action emitted by runtime input processing.
    ///
    /// Compatibility shim that forwards to [`NativeAppBridge::reduce_action`].
    fn on_action(&mut self, action: UiAction) {
        self.reduce_action(action);
    }

    /// Observe one built frame result for diagnostics or telemetry.
    fn observe_frame_result(&mut self, _result: FrameBuildResult) {}

    /// Observe a built frame result for diagnostics or telemetry.
    ///
    /// Compatibility shim that forwards to
    /// [`NativeAppBridge::observe_frame_result`].
    fn on_frame_result(&mut self, result: FrameBuildResult) {
        self.observe_frame_result(result);
    }

    /// Lifecycle hook fired when the runtime is shutting down.
    fn on_runtime_exit(&mut self) {}

    /// Lifecycle hook fired when the runtime is shutting down.
    ///
    /// Compatibility shim that forwards to
    /// [`NativeAppBridge::on_runtime_exit`].
    fn on_exit(&mut self) {
        self.on_runtime_exit();
    }
}
