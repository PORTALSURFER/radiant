//! Stable hotkey catalog assembly for the shared Radiant contract.
//!
//! Each sibling module owns one focus scope's bindings. This module keeps the
//! single flat presentation order that overlays, help surfaces, and routing
//! tests depend on.

mod browser;
mod folders;
mod global;
mod sources;
mod waveform;

use super::HotkeyBinding;

/// Flat hotkey catalog in stable presentation order.
pub(crate) const HOTKEY_BINDINGS: &[HotkeyBinding] = &[
    global::UNDO_CTRL_Z,
    global::UNDO_U,
    global::REDO_CTRL_Y,
    global::REDO_SHIFT_U,
    global::SHOW_HOTKEYS,
    global::COPY_STATUS_LOG,
    global::SUBMIT_GITHUB_ISSUE,
    global::FOCUS_WAVEFORM,
    global::FOCUS_BROWSER,
    global::FOCUS_FOLDER_TREE,
    global::FOCUS_SOURCES_LIST,
    global::PLAY_FROM_START,
    global::PLAY_COMPARE_ANCHOR,
    global::PLAY_FROM_CURRENT_PLAYHEAD,
    global::TOGGLE_LOOP,
    global::TOGGLE_LOOP_LOCK,
    global::RATE_DECREMENT,
    global::RATE_INCREMENT,
    global::TAG_NEUTRAL,
    global::TAG_KEEP,
    global::TAG_TRASH,
    browser::SEARCH_BROWSER,
    browser::FOCUS_LOADED_SAMPLE,
    browser::COPY_BROWSER_SELECTION,
    browser::SET_COMPARE_ANCHOR,
    browser::FIND_SIMILAR,
    browser::TOGGLE_RANDOM_NAVIGATION_MODE,
    browser::PLAY_RANDOM_SAMPLE,
    browser::PLAY_PREVIOUS_RANDOM_SAMPLE,
    browser::MOVE_TRASHED_TO_FOLDER,
    browser::MOVE_TRASHED_TO_FOLDER_SHIFT,
    browser::TOGGLE_SELECT,
    browser::TOGGLE_BROWSER_SAMPLE_MARK,
    browser::MOVE_BROWSER_FOCUS_UP,
    browser::MOVE_BROWSER_FOCUS_DOWN,
    browser::FOCUS_HISTORY_PREVIOUS,
    browser::FOCUS_HISTORY_NEXT,
    browser::RENAME_SAMPLE,
    browser::SELECT_ALL_BROWSER,
    browser::NORMALIZE_BROWSER,
    browser::DELETE_BROWSER,
    folders::TOGGLE_FOLDER_SELECT,
    folders::MOVE_FOLDER_FOCUS_UP,
    folders::MOVE_FOLDER_FOCUS_DOWN,
    folders::COLLAPSE_FOCUSED_FOLDER,
    folders::EXPAND_FOCUSED_FOLDER,
    folders::DELETE_FOLDER,
    folders::RENAME_FOLDER,
    folders::NEW_FOLDER,
    folders::SEARCH_FOLDERS,
    sources::MOVE_SOURCE_FOCUS_UP,
    sources::MOVE_SOURCE_FOCUS_DOWN,
    sources::RELOAD_FOCUSED_SOURCE,
    sources::HARD_SYNC_FOCUSED_SOURCE,
    sources::OPEN_FOCUSED_SOURCE_FOLDER,
    sources::REMOVE_FOCUSED_SOURCE,
    waveform::NORMALIZE_WAVEFORM,
    waveform::ALIGN_WAVEFORM_START,
    waveform::CROP_SELECTION,
    waveform::COPY_WAVEFORM_SELECTION,
    waveform::CROP_SELECTION_NEW_SAMPLE,
    waveform::SAVE_SELECTION_TO_BROWSER,
    waveform::SAVE_SELECTION_TO_BROWSER_KEEP2,
    waveform::COMMIT_WAVEFORM_EDIT_FADES,
    waveform::TOGGLE_FOCUSED_SLICE_EXPORT_MARK,
    waveform::TRIM_SELECTION,
    waveform::TOGGLE_BPM_SNAP,
    waveform::TOGGLE_TRANSIENTS,
    waveform::REVERSE_SELECTION,
    waveform::FADE_SELECTION_LEFT_TO_RIGHT,
    waveform::FADE_SELECTION_RIGHT_TO_LEFT,
    waveform::DELETE_SLICE_MARKERS,
    waveform::DELETE_LOADED_SAMPLE,
    waveform::MUTE_SELECTION,
    waveform::ZOOM_IN_SELECTION,
    waveform::ZOOM_OUT_SELECTION,
    waveform::SLIDE_SELECTION_LEFT,
    waveform::SLIDE_SELECTION_RIGHT,
    waveform::MICRO_SLIDE_SELECTION_LEFT,
    waveform::MICRO_SLIDE_SELECTION_RIGHT,
    waveform::NUDGE_SELECTION_LEFT,
    waveform::NUDGE_SELECTION_RIGHT,
];

#[cfg(test)]
mod tests;
