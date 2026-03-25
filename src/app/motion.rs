//! Motion-only projection types exposed by the `radiant` app contract.

use super::{AppModel, NormalizedRangeModel, WaveformChannelViewModel, WaveformSlicePreviewModel};

/// Motion-sensitive slice of the app model used for incremental overlay rendering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeMotionModel {
    /// Transport animation state used by motion overlays.
    pub transport_running: bool,
    /// Whether map mode is active for tab overlay tinting.
    pub map_active: bool,
    /// Active browser rating-filter chip states for levels `-3..=3`, plus `4` for locked keeps.
    pub active_rating_filters: [bool; 8],
    /// Waveform selected playback window with milli and micro precision.
    pub waveform_selection_milli: Option<NormalizedRangeModel>,
    /// Preview slices detected from silence-splitting the loaded waveform.
    pub waveform_slices: Vec<WaveformSlicePreviewModel>,
    /// One-shot token incremented when a waveform-selection export is queued.
    pub waveform_selection_export_flash_nonce: u64,
    /// One-shot token incremented when a queued waveform-selection export fails.
    pub waveform_selection_export_failure_flash_nonce: u64,
    /// Waveform edit-selection window with milli and micro precision.
    pub waveform_edit_selection_milli: Option<NormalizedRangeModel>,
    /// Waveform edit fade-in end handle in normalized milliseconds.
    pub waveform_edit_fade_in_end_milli: Option<u16>,
    /// Waveform edit fade-in end handle in normalized micro-units.
    pub waveform_edit_fade_in_end_micros: Option<u32>,
    /// Waveform edit fade-in mute-start handle in normalized milliseconds.
    pub waveform_edit_fade_in_mute_start_milli: Option<u16>,
    /// Waveform edit fade-in mute-start handle in normalized micro-units.
    pub waveform_edit_fade_in_mute_start_micros: Option<u32>,
    /// Waveform edit fade-in curve tension in normalized milliseconds.
    pub waveform_edit_fade_in_curve_milli: Option<u16>,
    /// Waveform edit fade-out start handle in normalized milliseconds.
    pub waveform_edit_fade_out_start_milli: Option<u16>,
    /// Waveform edit fade-out start handle in normalized micro-units.
    pub waveform_edit_fade_out_start_micros: Option<u32>,
    /// Waveform edit fade-out mute-end handle in normalized milliseconds.
    pub waveform_edit_fade_out_mute_end_milli: Option<u16>,
    /// Waveform edit fade-out mute-end handle in normalized micro-units.
    pub waveform_edit_fade_out_mute_end_micros: Option<u32>,
    /// Waveform edit fade-out curve tension in normalized milliseconds.
    pub waveform_edit_fade_out_curve_milli: Option<u16>,
    /// Whether loop playback is enabled for the active waveform selection.
    pub waveform_loop_enabled: bool,
    /// Waveform cursor position in normalized milliseconds.
    pub waveform_cursor_milli: Option<u16>,
    /// Waveform playhead position in normalized milliseconds.
    pub waveform_playhead_milli: Option<u16>,
    /// Waveform playhead position in normalized micro-units (`0..=1_000_000`).
    pub waveform_playhead_micros: Option<u32>,
    /// Current waveform view start in normalized milliseconds.
    pub waveform_view_start_milli: u16,
    /// Current waveform view end in normalized milliseconds.
    pub waveform_view_end_milli: u16,
    /// Current waveform view start in normalized micro-units (`0..=1_000_000`).
    pub waveform_view_start_micros: u32,
    /// Current waveform view end in normalized micro-units (`0..=1_000_000`).
    pub waveform_view_end_micros: u32,
    /// Human-readable tempo metadata.
    pub waveform_tempo_label: Option<String>,
    /// Human-readable zoom metadata.
    pub waveform_zoom_label: Option<String>,
    /// Loaded waveform label shown in the waveform overlay header.
    pub waveform_loaded_label: Option<String>,
    /// Whether the waveform plot is currently waiting for a new sample to load.
    pub waveform_loading: bool,
    /// Stable image signature for detecting waveform image updates during motion-only frames.
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
            active_rating_filters: model.browser.active_rating_filters,
            waveform_selection_milli: model.waveform.selection_milli,
            waveform_slices: model.waveform.slices.clone(),
            waveform_selection_export_flash_nonce: model.waveform.selection_export_flash_nonce,
            waveform_selection_export_failure_flash_nonce: model
                .waveform
                .selection_export_failure_flash_nonce,
            waveform_edit_selection_milli: model.waveform.edit_selection_milli,
            waveform_edit_fade_in_end_milli: model.waveform.edit_fade_in_end_milli,
            waveform_edit_fade_in_end_micros: model.waveform.edit_fade_in_end_micros,
            waveform_edit_fade_in_mute_start_milli: model.waveform.edit_fade_in_mute_start_milli,
            waveform_edit_fade_in_mute_start_micros: model.waveform.edit_fade_in_mute_start_micros,
            waveform_edit_fade_in_curve_milli: model.waveform.edit_fade_in_curve_milli,
            waveform_edit_fade_out_start_milli: model.waveform.edit_fade_out_start_milli,
            waveform_edit_fade_out_start_micros: model.waveform.edit_fade_out_start_micros,
            waveform_edit_fade_out_mute_end_milli: model.waveform.edit_fade_out_mute_end_milli,
            waveform_edit_fade_out_mute_end_micros: model.waveform.edit_fade_out_mute_end_micros,
            waveform_edit_fade_out_curve_milli: model.waveform.edit_fade_out_curve_milli,
            waveform_loop_enabled: model.waveform.loop_enabled,
            waveform_cursor_milli: model.waveform.cursor_milli,
            waveform_playhead_milli: model.waveform.playhead_milli,
            waveform_playhead_micros: model.waveform.playhead_micros.or_else(|| {
                model
                    .waveform
                    .playhead_milli
                    .map(|milli| u32::from(milli) * 1000)
            }),
            waveform_view_start_milli: model.waveform.view_start_milli,
            waveform_view_end_milli: model.waveform.view_end_milli,
            waveform_view_start_micros: model.waveform.view_start_micros,
            waveform_view_end_micros: model.waveform.view_end_micros,
            waveform_tempo_label: model.waveform.tempo_label.clone(),
            waveform_zoom_label: model.waveform.zoom_label.clone(),
            waveform_loaded_label: model.waveform.loaded_label.clone(),
            waveform_loading: model.waveform.loading,
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
