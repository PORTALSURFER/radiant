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
        }
    }
}

/// Action emitted by the native runtime input layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiAction {
    /// Select a target triage/browser column.
    SelectColumn { index: usize },
    /// Move column focus left/right.
    MoveColumn { delta: i8 },
    /// Toggle transport playback state.
    ToggleTransport,
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
