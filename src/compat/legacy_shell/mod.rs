//! Compatibility-facing contracts between `radiant` and an existing host shell.
//!
//! The host provides one immutable [`AppModel`](crate::compat::legacy_shell::AppModel) snapshot per frame.
//! `radiant` consumes that snapshot to:
//! 1. derive a frame from the retained shell model,
//! 2. run input hit-testing and command handling,
//! 3. emit [`UiAction`](crate::compat::legacy_shell::UiAction) values describing intent.
//!
//! Each action is routed back through the host bridge so state updates remain
//! in application code. This keeps GUI-specific input handling and event propagation
//! in `radiant` while preserving a one-way data flow for business logic.
//!
//! New host applications should prefer [`crate::runtime`], which exposes a
//! generic declarative view tree plus host-defined message reduction without
//! depending on host-shaped top-level models or action enums.
//!
//! This module remains as the legacy path for existing callers. The preferred
//! compatibility entry point for shell-specific code is now
//! [`crate::compat::legacy_shell`], which re-exports these contracts together
//! with the matching native runtime helpers.
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
//! - [`UiAction`](crate::compat::legacy_shell::UiAction) describes user intent emitted by the runtime.
//! - [`NativeMotionModel`](crate::compat::legacy_shell::NativeMotionModel) exposes motion-only projection for retained overlays.
//! - [`DirtySegments`](crate::compat::legacy_shell::DirtySegments) and [`SegmentRevisions`](crate::compat::legacy_shell::SegmentRevisions) describe incremental rebuild hints.
//! - [`NativeAppBridge`](crate::compat::legacy_shell::NativeAppBridge) defines the host/runtime integration boundary.

mod actions;
mod aliases;
mod bridge;
mod dirty_segments;
mod motion;
mod native_vello;
mod shell;
mod sources;
mod waveform;

pub use actions::{BrowserTriageTarget, UiAction};
pub use aliases::{
    AutomationBounds, AutomationNodeId, AutomationNodeSnapshot, AutomationRole,
    BrowserActionsModel, BrowserChromeModel, BrowserPanelModel, BrowserPillEditorModel,
    BrowserPillModel, BrowserPillState, BrowserRowModel, BrowserRowProcessingState, ColumnModel,
    FocusContextModel, FolderActionsModel, FolderPaneIdModel, FolderPaneModel, FolderRecoveryModel,
    FolderRowKind, FolderRowModel, FrameBuildResult, GuiAutomationSnapshot, HotkeyResolution,
    KeyPress, MapPanelModel, MapPointModel, MapRenderModeModel, PlaybackAgeBucket,
    PlaybackAgeFilterChip, RetainedVec, ShortcutResolution, SourceRowModel,
};
pub use bridge::NativeAppBridge;
pub use dirty_segments::{DirtySegments, SegmentRevisions};
pub use motion::NativeMotionModel;
pub use native_vello::run_native_vello_app_with_artifacts;
pub use shell::{
    AppModel, ConfirmPromptKind, ConfirmPromptModel, DragOverlayModel, OptionsPanelModel,
    PairedDevicePanelModel, PairedPickerOptionModel, PairedPickerTargetModel,
    PairedPickerValueModel, ProgressOverlayModel, StatusBarModel, StatusChipStateModel,
    SummaryFieldModel, UpdatePanelModel, UpdateStatusModel,
};
pub use sources::SourcesPanelModel;
pub use waveform::{NormalizedRangeModel, WaveformChromeModel, WaveformPanelModel};
