//! App-facing contracts between the `radiant` runtime and a host application.
//!
//! The host provides one immutable [`AppModel`](crate::app::AppModel) snapshot per frame.
//! `radiant` consumes that snapshot to:
//! 1. derive a frame from the retained shell model,
//! 2. run input hit-testing and command handling,
//! 3. emit [`UiAction`](crate::app::UiAction) values describing intent.
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
//! `radiant` resolves these to deterministic shell targets (hit test -> state
//! transition -> action emission) and does not mutate the host domain state directly.
//!
//! ## Boundary layout
//! The public contract is grouped by responsibility:
//! - shell/source/browser/map/waveform models describe immutable render state.
//! - [`UiAction`](crate::app::UiAction) describes user intent emitted by the runtime.
//! - [`NativeMotionModel`](crate::app::NativeMotionModel) exposes motion-only projection for retained overlays.
//! - [`DirtySegments`](crate::app::DirtySegments) and [`SegmentRevisions`](crate::app::SegmentRevisions) describe incremental rebuild hints.
//! - [`NativeAppBridge`](crate::app::NativeAppBridge) defines the host/runtime integration boundary.

mod actions;
mod automation;
mod bridge;
mod browser;
mod declarative;
mod dirty_segments;
pub(crate) mod hotkeys;
mod motion;
mod shell;
mod sources;
mod waveform;
mod waveform_tempo;

pub use actions::{BrowserTagTarget, UiAction};
pub use automation::{
    AutomationBounds, AutomationNodeId, AutomationNodeSnapshot, AutomationRole,
    GuiAutomationSnapshot,
};
pub use bridge::NativeAppBridge;
pub use browser::{
    BrowserActionsModel, BrowserChromeModel, BrowserPanelModel, BrowserRowModel, MapPanelModel,
    MapPointModel, MapRenderModeModel,
};
pub use declarative::{DeclarativeBridge, declarative_bridge};
pub use dirty_segments::{DirtySegments, FrameBuildResult, SegmentRevisions};
pub use hotkeys::{HotkeyBinding, HotkeyGesture, HotkeyScope, KeyPress, iter_hotkey_bindings};
pub use motion::NativeMotionModel;
pub use shell::{
    AppModel, ConfirmPromptKind, ConfirmPromptModel, DEFAULT_APP_TITLE, DragOverlayModel,
    OptionsPanelModel, ProgressOverlayModel, StatusBarModel, UpdatePanelModel, UpdateStatusModel,
};
pub use sources::{
    ColumnModel, FocusContextModel, FolderActionsModel, FolderRecoveryModel, FolderRowModel,
    SourceRowModel, SourcesPanelModel,
};
pub use waveform::{
    NormalizedRangeModel, WaveformChannelViewModel, WaveformChromeModel, WaveformPanelModel,
};
pub use waveform_tempo::parse_waveform_tempo_number_text;
