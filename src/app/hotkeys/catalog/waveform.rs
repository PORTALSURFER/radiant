//! Waveform hotkeys for editing, export, and focused selection movement.

use crate::app::UiAction;
use crate::gui::input::KeyCode;

use super::super::{HotkeyBinding, HotkeyGesture, WAVEFORM_SCOPE};

pub(crate) const NORMALIZE_WAVEFORM: HotkeyBinding = HotkeyBinding {
    id: "normalize-waveform",
    label: "Normalize selection/sample",
    gesture: HotkeyGesture::new(KeyCode::N),
    scope: WAVEFORM_SCOPE,
    action: UiAction::NormalizeWaveformSelectionOrSample,
};
pub(crate) const ALIGN_WAVEFORM_START: HotkeyBinding = HotkeyBinding {
    id: "align-waveform-start",
    label: "Set start to hover cursor",
    gesture: HotkeyGesture::new(KeyCode::S),
    scope: WAVEFORM_SCOPE,
    action: UiAction::AlignWaveformStartToMarker,
};
pub(crate) const CROP_SELECTION: HotkeyBinding = HotkeyBinding {
    id: "crop-selection",
    label: "Crop selection",
    gesture: HotkeyGesture::new(KeyCode::C),
    scope: WAVEFORM_SCOPE,
    action: UiAction::CropWaveformSelection,
};
pub(crate) const COPY_WAVEFORM_SELECTION: HotkeyBinding = HotkeyBinding {
    id: "copy-waveform-selection",
    label: "Copy selection clip",
    gesture: HotkeyGesture::with_command(KeyCode::C),
    scope: WAVEFORM_SCOPE,
    action: UiAction::CopySelectionToClipboard,
};
pub(crate) const CROP_SELECTION_NEW_SAMPLE: HotkeyBinding = HotkeyBinding {
    id: "crop-selection-new-sample",
    label: "Crop selection as new sample",
    gesture: HotkeyGesture::with_shift(KeyCode::C),
    scope: WAVEFORM_SCOPE,
    action: UiAction::CropWaveformSelectionToNewSample,
};
pub(crate) const SAVE_SELECTION_TO_BROWSER: HotkeyBinding = HotkeyBinding {
    id: "save-selection-to-browser",
    label: "Save selection/slices to browser",
    gesture: HotkeyGesture::new(KeyCode::E),
    scope: WAVEFORM_SCOPE,
    action: UiAction::SaveWaveformSelectionToBrowser,
};
pub(crate) const SAVE_SELECTION_TO_BROWSER_KEEP2: HotkeyBinding = HotkeyBinding {
    id: "save-selection-to-browser-keep2",
    label: "Save selection/slices to browser (keep x2)",
    gesture: HotkeyGesture::with_shift(KeyCode::E),
    scope: WAVEFORM_SCOPE,
    action: UiAction::SaveWaveformSelectionToBrowserWithKeep2,
};
pub(crate) const COMMIT_WAVEFORM_EDIT_FADES: HotkeyBinding = HotkeyBinding {
    id: "commit-waveform-edit-fades",
    label: "Apply edit fades",
    gesture: HotkeyGesture::new(KeyCode::Enter),
    scope: WAVEFORM_SCOPE,
    action: UiAction::CommitWaveformEditFades,
};
pub(crate) const TOGGLE_FOCUSED_SLICE_EXPORT_MARK: HotkeyBinding = HotkeyBinding {
    id: "toggle-focused-slice-export-mark",
    label: "Mark focused slice for export",
    gesture: HotkeyGesture::new(KeyCode::A),
    scope: WAVEFORM_SCOPE,
    action: UiAction::ToggleFocusedWaveformSliceExportMark,
};
pub(crate) const TRIM_SELECTION: HotkeyBinding = HotkeyBinding {
    id: "trim-selection",
    label: "Trim selection",
    gesture: HotkeyGesture::new(KeyCode::T),
    scope: WAVEFORM_SCOPE,
    action: UiAction::TrimWaveformSelection,
};
pub(crate) const TOGGLE_BPM_SNAP: HotkeyBinding = HotkeyBinding {
    id: "toggle-bpm-snap",
    label: "Toggle BPM snap",
    gesture: HotkeyGesture::new(KeyCode::B),
    scope: WAVEFORM_SCOPE,
    action: UiAction::ToggleBpmSnap,
};
pub(crate) const TOGGLE_TRANSIENTS: HotkeyBinding = HotkeyBinding {
    id: "toggle-transients",
    label: "Show/hide transients",
    gesture: HotkeyGesture::new(KeyCode::I),
    scope: WAVEFORM_SCOPE,
    action: UiAction::ToggleTransientMarkers,
};
pub(crate) const REVERSE_SELECTION: HotkeyBinding = HotkeyBinding {
    id: "reverse-selection",
    label: "Reverse selection",
    gesture: HotkeyGesture::with_shift(KeyCode::R),
    scope: WAVEFORM_SCOPE,
    action: UiAction::ReverseWaveformSelection,
};
pub(crate) const FADE_SELECTION_LEFT_TO_RIGHT: HotkeyBinding = HotkeyBinding {
    id: "fade-selection-left-to-right",
    label: "Fade selection (left to right)",
    gesture: HotkeyGesture::new(KeyCode::Backslash),
    scope: WAVEFORM_SCOPE,
    action: UiAction::FadeWaveformSelectionLeftToRight,
};
pub(crate) const FADE_SELECTION_RIGHT_TO_LEFT: HotkeyBinding = HotkeyBinding {
    id: "fade-selection-right-to-left",
    label: "Fade selection (right to left)",
    gesture: HotkeyGesture::new(KeyCode::Slash),
    scope: WAVEFORM_SCOPE,
    action: UiAction::FadeWaveformSelectionRightToLeft,
};
pub(crate) const DELETE_SLICE_MARKERS: HotkeyBinding = HotkeyBinding {
    id: "delete-slice-markers",
    label: "Delete slice markers (Slice mode)",
    gesture: HotkeyGesture::with_shift(KeyCode::D),
    scope: WAVEFORM_SCOPE,
    action: UiAction::DeleteSelectedSliceMarkers,
};
pub(crate) const DELETE_LOADED_SAMPLE: HotkeyBinding = HotkeyBinding {
    id: "delete-loaded-sample",
    label: "Delete loaded sample",
    gesture: HotkeyGesture::new(KeyCode::D),
    scope: WAVEFORM_SCOPE,
    action: UiAction::DeleteLoadedWaveformSample,
};
pub(crate) const MUTE_SELECTION: HotkeyBinding = HotkeyBinding {
    id: "mute-selection",
    label: "Mute selection / Merge slices (Slice mode)",
    gesture: HotkeyGesture::new(KeyCode::M),
    scope: WAVEFORM_SCOPE,
    action: UiAction::MuteWaveformSelection,
};
pub(crate) const ZOOM_IN_SELECTION: HotkeyBinding = HotkeyBinding {
    id: "zoom-in-selection",
    label: "Zoom to selection",
    gesture: HotkeyGesture::new(KeyCode::Z),
    scope: WAVEFORM_SCOPE,
    action: UiAction::ZoomWaveformToSelection,
};
pub(crate) const ZOOM_OUT_SELECTION: HotkeyBinding = HotkeyBinding {
    id: "zoom-out-selection",
    label: "Zoom out",
    gesture: HotkeyGesture::new(KeyCode::X),
    scope: WAVEFORM_SCOPE,
    action: UiAction::ZoomWaveformFull,
};
pub(crate) const SLIDE_SELECTION_LEFT: HotkeyBinding = HotkeyBinding {
    id: "slide-selection-left",
    label: "Previous slice / Slide selection left",
    gesture: HotkeyGesture::new(KeyCode::ArrowLeft),
    scope: WAVEFORM_SCOPE,
    action: UiAction::MoveWaveformSliceFocus { delta: -1 },
};
pub(crate) const SLIDE_SELECTION_RIGHT: HotkeyBinding = HotkeyBinding {
    id: "slide-selection-right",
    label: "Next slice / Slide selection right",
    gesture: HotkeyGesture::new(KeyCode::ArrowRight),
    scope: WAVEFORM_SCOPE,
    action: UiAction::MoveWaveformSliceFocus { delta: 1 },
};
pub(crate) const MICRO_SLIDE_SELECTION_LEFT: HotkeyBinding = HotkeyBinding {
    id: "micro-slide-selection-left",
    label: "Micro-slide selection left (1 sample)",
    gesture: HotkeyGesture::with_alt(KeyCode::ArrowLeft),
    scope: WAVEFORM_SCOPE,
    action: UiAction::SlideWaveformSelection {
        delta: -1,
        fine: true,
    },
};
pub(crate) const MICRO_SLIDE_SELECTION_RIGHT: HotkeyBinding = HotkeyBinding {
    id: "micro-slide-selection-right",
    label: "Micro-slide selection right (1 sample)",
    gesture: HotkeyGesture::with_alt(KeyCode::ArrowRight),
    scope: WAVEFORM_SCOPE,
    action: UiAction::SlideWaveformSelection {
        delta: 1,
        fine: true,
    },
};
pub(crate) const NUDGE_SELECTION_LEFT: HotkeyBinding = HotkeyBinding {
    id: "nudge-selection-left",
    label: "Nudge selection left (fine)",
    gesture: HotkeyGesture::with_shift(KeyCode::ArrowLeft),
    scope: WAVEFORM_SCOPE,
    action: UiAction::SlideWaveformSelection {
        delta: -1,
        fine: true,
    },
};
pub(crate) const NUDGE_SELECTION_RIGHT: HotkeyBinding = HotkeyBinding {
    id: "nudge-selection-right",
    label: "Nudge selection right (fine)",
    gesture: HotkeyGesture::with_shift(KeyCode::ArrowRight),
    scope: WAVEFORM_SCOPE,
    action: UiAction::SlideWaveformSelection {
        delta: 1,
        fine: true,
    },
};
