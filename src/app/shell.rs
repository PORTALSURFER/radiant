//! Top-level shell and overlay models exposed by the `radiant` app contract.

pub use crate::gui::chrome::StatusSegments as StatusBarModel;
pub use crate::gui::feedback::DragOverlay as DragOverlayModel;
pub use crate::gui::feedback::ProgressOverlay as ProgressOverlayModel;
pub use crate::gui::feedback::UpdatePanel as UpdatePanelModel;
pub use crate::gui::feedback::UpdateStatus as UpdateStatusModel;

use super::{
    BrowserActionsModel, BrowserChromeModel, BrowserPanelModel, ColumnModel, FocusContextModel,
    FolderActionsModel, FolderPaneIdModel, FolderPaneModel, FolderRecoveryModel, MapPanelModel,
    SourcesPanelModel, WaveformChromeModel, WaveformPanelModel,
};

/// Health state of the compact audio-engine status chip.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AudioEngineChipStateModel {
    /// Output engine is active and matches the requested configuration.
    #[default]
    Healthy,
    /// Output engine is unavailable or degraded.
    Error,
}

/// Audio field currently expanded into a picker inside the options panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioPickerTargetModel {
    /// Output host/backend picker.
    OutputHost,
    /// Output device picker.
    OutputDevice,
    /// Output sample-rate picker.
    OutputSampleRate,
    /// Input host/backend picker.
    InputHost,
    /// Input device picker.
    InputDevice,
    /// Input sample-rate picker.
    InputSampleRate,
}

/// Raw value carried by one audio picker option.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AudioOptionValueModel {
    /// Output host identifier, or `None` for the system default.
    OutputHost(Option<String>),
    /// Output device name, or `None` for the host default.
    OutputDevice(Option<String>),
    /// Output sample rate in Hz, or `None` for the device default.
    OutputSampleRate(Option<u32>),
    /// Input host identifier, or `None` for the system default.
    InputHost(Option<String>),
    /// Input device name, or `None` for the host default.
    InputDevice(Option<String>),
    /// Input sample rate in Hz, or `None` for the device default.
    InputSampleRate(Option<u32>),
}

/// One selectable item shown inside an audio picker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioOptionItemModel {
    /// Human-readable option label.
    pub label: String,
    /// Whether the option is currently selected.
    pub selected: bool,
    /// Raw value applied when the option is chosen.
    pub value: AudioOptionValueModel,
}

/// Overview row shown for one audio field inside the options panel.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct AudioFieldModel {
    /// Static row label.
    pub label: String,
    /// Current value summary.
    pub value_label: String,
}

/// Output/input audio engine state projected into the native shell.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct AudioEngineModel {
    /// Compact chip health state.
    pub chip_state: AudioEngineChipStateModel,
    /// Compact chip label shown in the top-right chrome.
    pub chip_label: String,
    /// Optional detail or error text shown inside the options overview.
    pub detail_label: Option<String>,
    /// Output host summary row.
    pub output_host: AudioFieldModel,
    /// Output device summary row.
    pub output_device: AudioFieldModel,
    /// Output sample-rate summary row.
    pub output_sample_rate: AudioFieldModel,
    /// Input host summary row.
    pub input_host: AudioFieldModel,
    /// Input device summary row.
    pub input_device: AudioFieldModel,
    /// Input sample-rate summary row.
    pub input_sample_rate: AudioFieldModel,
    /// Currently expanded picker, or `None` for the overview.
    pub active_picker: Option<AudioPickerTargetModel>,
    /// Output host choices.
    pub output_host_options: Vec<AudioOptionItemModel>,
    /// Output device choices.
    pub output_device_options: Vec<AudioOptionItemModel>,
    /// Output sample-rate choices.
    pub output_sample_rate_options: Vec<AudioOptionItemModel>,
    /// Input host choices.
    pub input_host_options: Vec<AudioOptionItemModel>,
    /// Input device choices.
    pub input_device_options: Vec<AudioOptionItemModel>,
    /// Input sample-rate choices.
    pub input_sample_rate_options: Vec<AudioOptionItemModel>,
}

/// Options-panel state projected into the native shell.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct OptionsPanelModel {
    /// Whether the panel is currently visible.
    pub visible: bool,
    /// Current default identifier used by auto rename.
    pub default_identifier: String,
    /// Whether input monitoring is enabled.
    pub input_monitoring_enabled: bool,
    /// Whether rating advances browser focus.
    pub advance_after_rating_enabled: bool,
    /// Whether destructive edits skip confirmation.
    pub destructive_yolo_mode_enabled: bool,
    /// Whether waveform scrolling is inverted.
    pub invert_waveform_scroll_enabled: bool,
    /// Short display label for the configured trash folder, when available.
    pub trash_folder_label: Option<String>,
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
    /// Pending retained folder-delete restore prompt.
    RestoreRetainedFolderDeletes,
    /// Pending retained folder-delete purge prompt.
    PurgeRetainedFolderDeletes,
    /// Pending options-panel default-identifier prompt.
    OptionsDefaultIdentifier,
}

/// Modal confirmation prompt projected into the native shell.
pub type ConfirmPromptModel = crate::gui::feedback::ConfirmPrompt<ConfirmPromptKind>;

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
    /// Output/input audio engine state rendered in the top-right chrome and options panel.
    pub audio_engine: AudioEngineModel,
    /// Browser action availability for native action surfaces.
    pub browser_actions: BrowserActionsModel,
    /// Options-panel overlay projection.
    pub options_panel: OptionsPanelModel,
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
            title: String::from(crate::gui_runtime::DEFAULT_NATIVE_WINDOW_TITLE),
            backend_label: String::from("backend: native_vello"),
            sources_label: String::from("Sources"),
            status_text: String::new(),
            status: StatusBarModel {
                left: String::new(),
                center: String::from("rows: 0 | selected: 0 | anchor: — | search: —"),
                right: String::from("col: 2/3"),
            },
            audio_engine: AudioEngineModel::default(),
            browser_actions: BrowserActionsModel::default(),
            options_panel: OptionsPanelModel::default(),
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
                active_folder_pane: FolderPaneIdModel::Upper,
                upper_folder_pane: FolderPaneModel {
                    pane: FolderPaneIdModel::Upper,
                    title: String::from("Upper"),
                    ..FolderPaneModel::default()
                },
                lower_folder_pane: FolderPaneModel {
                    pane: FolderPaneIdModel::Lower,
                    title: String::from("Lower"),
                    ..FolderPaneModel::default()
                },
                folder_search_query: String::new(),
                show_all_folders: false,
                can_toggle_show_all_folders: false,
                flattened_view: false,
                can_toggle_flattened_view: false,
                selected_row: None,
                loading_row: None,
                mutation_busy_row: None,
                focused_folder_row: None,
                rows: super::RetainedVec::new(),
                folder_rows: super::RetainedVec::new(),
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
