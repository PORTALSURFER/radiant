//! Shared waveform drag-mode types and hit-test constants.

/// Drag-mode state carried across waveform pointer interactions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) enum WaveformPointerDragMode {
    /// Drag updates seek/playhead position.
    Seek,
    /// Drag updates cursor position.
    Cursor,
    /// Drag updates circular waveform-slide preview and commit state.
    CircularSlide {
        /// Fixed anchor micro position captured at drag start.
        anchor_micros: u32,
    },
    /// Drag extends playback selection from a fixed anchor micro value.
    Selection {
        /// Fixed anchor micro position captured at drag start.
        anchor_micros: u32,
        /// Optional stable clamp captured while the pointer remains off-plot.
        boundary_lock: Option<WaveformSelectionBoundaryLock>,
    },
    /// Drag resizes a playback selection without snapping and recomputes BPM from a 4-beat span.
    SelectionSmartScale {
        /// Fixed anchor micro position captured at drag start.
        anchor_micros: u32,
        /// Optional stable clamp captured while the pointer remains off-plot.
        boundary_lock: Option<WaveformSelectionBoundaryLock>,
    },
    /// Drag shifts the playback selection while preserving its width.
    SelectionShift {
        /// Pointer micro position captured at drag start.
        pointer_micros: u32,
        /// Original playback-selection start micro position.
        start_micros: u32,
        /// Original playback-selection end micro position.
        end_micros: u32,
    },
    /// Drag extends edit selection from a fixed anchor micro value.
    EditSelection {
        /// Fixed anchor micro position captured at drag start.
        anchor_micros: u32,
        /// Optional stable clamp captured while the pointer remains off-plot.
        boundary_lock: Option<WaveformSelectionBoundaryLock>,
    },
    /// Drag shifts the edit selection while preserving its width.
    EditSelectionShift {
        /// Pointer micro position captured at drag start.
        pointer_micros: u32,
        /// Original edit-selection start micro position.
        start_micros: u32,
        /// Original edit-selection end micro position.
        end_micros: u32,
    },
    /// Drag updates the edit fade-in end handle.
    EditFadeInEnd,
    /// Drag updates the edit fade-in mute-start handle.
    EditFadeInMuteStart,
    /// Drag updates the edit fade-in curve.
    EditFadeInCurve,
    /// Drag updates the edit fade-out start handle.
    EditFadeOutStart,
    /// Drag updates the edit fade-out mute-end handle.
    EditFadeOutMuteEnd,
    /// Drag updates the edit fade-out curve.
    EditFadeOutCurve,
}

/// Horizontal waveform plot edge used by out-of-bounds drag locks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) enum WaveformOutsidePlotSide {
    /// Pointer sits left of the waveform plot.
    Left,
    /// Pointer sits right of the waveform plot.
    Right,
}

/// Stable absolute clamp for anchor-based selection drags outside the waveform plot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) struct WaveformSelectionBoundaryLock {
    /// Horizontal plot edge the pointer is currently beyond.
    pub(in crate::gui_runtime::native_vello) side: WaveformOutsidePlotSide,
    /// Absolute waveform micro position captured for the drag.
    pub(in crate::gui_runtime::native_vello) position_micros: u32,
}

/// Half-width in pixels used for fade-handle hit testing.
pub(in crate::gui_runtime::native_vello) const WAVEFORM_EDIT_FADE_HANDLE_HIT_HALF_WIDTH: f32 = 7.0;
pub(in crate::gui_runtime::native_vello) const WAVEFORM_EDIT_FADE_TOP_TAB_SIZE: f32 = 10.0;
/// Horizontal drag distance required before a new playback selection counts as intentional.
pub(in crate::gui_runtime::native_vello) const WAVEFORM_SELECTION_CLICK_SLOP_PX: f32 = 3.0;
/// Half-width in pixels used for waveform edge-resize hit testing.
pub(in crate::gui_runtime::native_vello) const WAVEFORM_RESIZE_EDGE_HIT_HALF_WIDTH: f32 = 7.0;
/// Fraction of waveform height used by centered resize-edge hit regions.
pub(in crate::gui_runtime::native_vello) const WAVEFORM_RESIZE_EDGE_HEIGHT_RATIO: f32 = 0.34;
/// Width/height in logical pixels for the playback-selection drag handle.
pub(in crate::gui_runtime::native_vello) const WAVEFORM_SELECTION_DRAG_HANDLE_SIZE: f32 = 12.0;
/// Extra hit slop around the playback-selection drag handle.
pub(in crate::gui_runtime::native_vello) const WAVEFORM_SELECTION_DRAG_HANDLE_HIT_INSET: f32 = 4.0;
/// Width in logical pixels for bottom-center selection shift handles.
pub(in crate::gui_runtime::native_vello) const WAVEFORM_SELECTION_SHIFT_HANDLE_WIDTH: f32 = 14.0;
/// Height in logical pixels for bottom-center selection shift handles.
pub(in crate::gui_runtime::native_vello) const WAVEFORM_SELECTION_SHIFT_HANDLE_HEIGHT: f32 = 7.0;
/// Extra hit slop around bottom-center selection shift handles.
pub(in crate::gui_runtime::native_vello) const WAVEFORM_SELECTION_SHIFT_HANDLE_HIT_INSET: f32 = 4.0;
/// Pixel-delta normalization factor for wheel-driven waveform zoom steps.
pub(in crate::gui_runtime::native_vello) const WAVEFORM_WHEEL_ZOOM_PIXEL_STEP: f32 = 48.0;
/// Integer precision used by pointer-anchored zoom ratios (`0..=1_000_000`).
pub(in crate::gui_runtime::native_vello) const WAVEFORM_ANCHOR_RATIO_MICROS_SCALE: u32 = 1_000_000;
