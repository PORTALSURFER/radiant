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

use crate::gui::invalidation::{RetainedSegmentMask, RetainedSegmentRevisions};

#[path = "../../../../../src/app_core/native_shell/composition/runtime/actions/mod.rs"]
mod actions;
#[path = "../../../../../src/app_core/native_shell/composition/runtime/bridge.rs"]
mod bridge;
#[path = "../../../../../src/app_core/native_shell/composition/runtime/motion.rs"]
mod motion;
#[path = "../../../../../src/app_core/native_shell/composition/runtime/native_vello.rs"]
mod native_vello;
#[path = "../../../../../src/app_core/native_shell/composition/runtime/runtime_artifacts.rs"]
mod runtime_artifacts;
#[path = "../../../../../src/app_core/native_shell/composition/runtime/shell.rs"]
mod shell;
#[path = "../../../../../src/app_core/native_shell/composition/runtime/shell_snapshot.rs"]
mod shell_snapshot;
#[path = "../../../../../src/app_core/native_shell/composition/runtime/sources.rs"]
mod sources;
#[path = "../../../../../src/app_core/native_shell/composition/runtime/waveform.rs"]
mod waveform;

pub use crate::gui::automation::{
    AutomationBounds, AutomationNodeId, AutomationNodeSnapshot, AutomationRole,
    GuiAutomationSnapshot,
};
pub use crate::gui::chrome::ContentViewChrome as BrowserChromeModel;
pub use crate::gui::feedback::RecoverySummary as FolderRecoveryModel;
pub use crate::gui::focus::FocusSurface as FocusContextModel;
pub use crate::gui::frame::FrameBuildResult;
pub use crate::gui::input::KeyPress;
pub use crate::gui::list::ColumnSummary as ColumnModel;
pub use crate::gui::list::ContentListActions as BrowserActionsModel;
pub use crate::gui::list::ContentListRow as BrowserRowModel;
pub use crate::gui::list::EditableRowKind as FolderRowKind;
pub use crate::gui::list::EditableTreeActions as FolderActionsModel;
pub use crate::gui::list::EditableTreeRow as FolderRowModel;
pub use crate::gui::list::RecencyBucket as PlaybackAgeBucket;
pub use crate::gui::list::RecencyFilterChip as PlaybackAgeFilterChip;
pub use crate::gui::list::RowProcessingState as BrowserRowProcessingState;
pub use crate::gui::panel::SplitPaneAssignedRow as SourceRowModel;
pub use crate::gui::panel::SplitPaneSlot as FolderPaneIdModel;
pub use crate::gui::retained::RetainedVec;
pub use crate::gui::selection::TriState as BrowserPillState;
pub use crate::gui::shortcuts::ShortcutResolution;
pub use crate::gui::visualization::PointRenderMode as MapRenderModeModel;
pub use crate::gui::visualization::SpatialPanel as MapPanelModel;
pub use crate::gui::visualization::SpatialPoint as MapPointModel;
pub use actions::{BrowserTriageTarget, UiAction};
pub use bridge::NativeAppBridge;
/// Compatibility alias for the generic shortcut resolution DTO.
pub type HotkeyResolution = ShortcutResolution<UiAction>;
pub use motion::NativeMotionModel;
#[cfg(test)]
pub(crate) use native_vello::PreviewBridge;
pub use native_vello::{
    capture_gui_automation_snapshot, run_native_vello_app, run_native_vello_app_declarative,
    run_native_vello_app_declarative_with_artifacts, run_native_vello_app_with_artifacts,
    run_native_vello_preview,
};
pub use runtime_artifacts::{NativeRunReport, NativeRuntimeArtifacts};
pub use shell::{
    AppModel, ConfirmPromptKind, ConfirmPromptModel, DragOverlayModel, OptionsPanelModel,
    PairedDevicePanelModel, PairedPickerOptionModel, PairedPickerTargetModel,
    PairedPickerValueModel, ProgressOverlayModel, StatusBarModel, StatusChipStateModel,
    SummaryFieldModel, UpdatePanelModel, UpdateStatusModel,
};
pub use shell_snapshot::capture_native_shell_shot_snapshot;
pub use sources::SourcesPanelModel;
pub use waveform::{NormalizedRangeModel, WaveformChromeModel, WaveformPanelModel};

/// One clickable pill projected into the browser metadata sidebar.
pub type BrowserPillModel = crate::gui::badge::SelectablePill<BrowserPillState>;
/// Browser-local metadata sidebar shown beside the content list.
pub type BrowserPillEditorModel = crate::gui::badge::PillEditorPanel<BrowserPillState>;

/// Summary of browser/list state consumed by the native shell.
pub type BrowserPanelModel =
    crate::gui::list::ContentListPanel<BrowserRowModel, BrowserPillEditorModel>;
/// Projected data for one fixed folder pane shown in the sidebar.
pub type FolderPaneModel = crate::gui::panel::SplitPaneTreePanel<FolderRowModel>;

/// Bitmask describing which projection segments changed during the last model pull.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DirtySegments {
    mask: RetainedSegmentMask<0x00ff, 0x003f, 0x00c0>,
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

    /// Return an empty segment mask.
    pub const fn empty() -> Self {
        Self {
            mask: RetainedSegmentMask::empty(),
        }
    }

    /// Return a full segment mask.
    pub const fn all() -> Self {
        Self {
            mask: RetainedSegmentMask::all(),
        }
    }

    /// Construct a segment mask from raw bits.
    pub const fn from_bits(bits: u16) -> Self {
        Self {
            mask: RetainedSegmentMask::from_bits(bits),
        }
    }

    /// Return raw bit contents for diagnostics and tests.
    pub const fn bits(self) -> u16 {
        self.mask.bits()
    }

    /// Return `true` when the mask contains no segments.
    pub const fn is_empty(self) -> bool {
        self.mask.is_empty()
    }

    /// Return `true` when any static segment requires rebuild.
    pub const fn requires_static_rebuild(self) -> bool {
        self.mask.requires_static_rebuild()
    }

    /// Return `true` when any overlay segment requires rebuild.
    pub const fn requires_overlay_rebuild(self) -> bool {
        self.mask.requires_overlay_rebuild()
    }

    /// Insert one or more segment bits into this mask.
    pub fn insert(&mut self, bits: u16) {
        self.mask.insert(bits);
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
    /// Return these named compatibility revisions as a generic retained segment array.
    pub const fn retained_revisions(self) -> RetainedSegmentRevisions<6> {
        RetainedSegmentRevisions::new([
            self.status_bar,
            self.browser_frame,
            self.browser_rows_window,
            self.map_panel,
            self.waveform_overlay,
            self.global_static,
        ])
    }

    /// Return whether any static-segment revision is non-zero.
    pub fn has_static_revisions(self) -> bool {
        self.retained_revisions().has_revisions()
    }

    /// Bump revisions for the static segments flagged in `dirty_segments`.
    pub fn bump_for_dirty_segments(&mut self, dirty_segments: DirtySegments) {
        let bits = dirty_segments.bits();
        let mut revisions = self.retained_revisions();
        revisions.bump_for_bits(
            bits,
            [
                DirtySegments::STATUS_BAR,
                DirtySegments::BROWSER_FRAME,
                DirtySegments::BROWSER_ROWS_WINDOW,
                DirtySegments::MAP_PANEL,
                DirtySegments::WAVEFORM_OVERLAY,
                DirtySegments::GLOBAL_STATIC,
            ],
        );
        let [
            status_bar,
            browser_frame,
            browser_rows_window,
            map_panel,
            waveform_overlay,
            global_static,
        ] = revisions.revisions;
        self.status_bar = status_bar;
        self.browser_frame = browser_frame;
        self.browser_rows_window = browser_rows_window;
        self.map_panel = map_panel;
        self.waveform_overlay = waveform_overlay;
        self.global_static = global_static;
    }
}
