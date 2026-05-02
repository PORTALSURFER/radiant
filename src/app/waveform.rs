//! Waveform-facing models exposed by the `radiant` app contract.

pub use crate::gui::range::NormalizedRange as NormalizedRangeModel;
use crate::gui::types::ImageRgba;
pub use crate::gui::visualization::ChannelViewMode as WaveformChannelViewModel;
pub use crate::gui::visualization::TimelineMarkerPreview as WaveformSlicePreviewModel;
use std::sync::Arc;

/// Waveform preview metadata consumed by the native shell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaveformPanelModel {
    /// Display label for the loaded sample, when any.
    pub loaded_label: Option<String>,
    /// Whether a newly focused sample is still loading waveform data.
    pub loading: bool,
    /// Whether a replacement waveform image is still rendering in the background.
    pub image_rendering: bool,
    /// Cursor position in normalized milli-units.
    pub cursor_milli: Option<u16>,
    /// Playhead position in normalized milli-units.
    pub playhead_milli: Option<u16>,
    /// Playhead position in normalized micro-units (`0..=1_000_000`).
    ///
    /// This preserves sub-milli transport precision for smooth playhead motion
    /// during animation-only redraws and full-model fallback rebuilds.
    pub playhead_micros: Option<u32>,
    /// Current waveform selection bounds.
    pub selection_milli: Option<NormalizedRangeModel>,
    /// Preview slices detected from silence-splitting the loaded waveform.
    pub slices: Vec<WaveformSlicePreviewModel>,
    /// One-shot token incremented when a waveform-selection export is queued.
    ///
    /// Native shells treat each new value as an optimistic event and can run
    /// immediate local flash feedback without depending on controller
    /// wall-clock timestamps.
    pub selection_export_flash_nonce: u64,
    /// One-shot token incremented when a queued waveform-selection export fails.
    ///
    /// Native shells treat each new value as an error event so they can tint a
    /// later flash red after the initial optimistic feedback.
    pub selection_export_failure_flash_nonce: u64,
    /// One-shot token incremented when preview edit fades are committed.
    ///
    /// Native shells treat each new value as a success event so they can
    /// briefly brighten the edit-selection overlay after the write succeeds.
    pub edit_selection_apply_flash_nonce: u64,
    /// Current waveform edit-selection bounds (right-click paint range).
    pub edit_selection_milli: Option<NormalizedRangeModel>,
    /// End position for the edit fade-in region in normalized milli-units.
    ///
    /// When absent, the fade-in handle defaults to the edit-selection start edge.
    pub edit_fade_in_end_milli: Option<u16>,
    /// End position for the edit fade-in region in normalized micro-units.
    pub edit_fade_in_end_micros: Option<u32>,
    /// Start position for the edit fade-in mute region in normalized milli-units.
    ///
    /// When absent, the bottom fade-in handle defaults to the edit-selection start edge.
    pub edit_fade_in_mute_start_milli: Option<u16>,
    /// Start position for the edit fade-in mute region in normalized micro-units.
    pub edit_fade_in_mute_start_micros: Option<u32>,
    /// Fade-in curve tension in normalized milli-units (`0..=1000`).
    pub edit_fade_in_curve_milli: Option<u16>,
    /// Start position for the edit fade-out region in normalized milli-units.
    ///
    /// When absent, the fade-out handle defaults to the edit-selection end edge.
    pub edit_fade_out_start_milli: Option<u16>,
    /// Start position for the edit fade-out region in normalized micro-units.
    pub edit_fade_out_start_micros: Option<u32>,
    /// End position for the edit fade-out mute region in normalized milli-units.
    ///
    /// When absent, the bottom fade-out handle defaults to the edit-selection end edge.
    pub edit_fade_out_mute_end_milli: Option<u16>,
    /// End position for the edit fade-out mute region in normalized micro-units.
    pub edit_fade_out_mute_end_micros: Option<u32>,
    /// Fade-out curve tension in normalized milli-units (`0..=1000`).
    pub edit_fade_out_curve_milli: Option<u16>,
    /// Visible view start in normalized milli-units.
    pub view_start_milli: u16,
    /// Visible view end in normalized milli-units.
    pub view_end_milli: u16,
    /// Visible view start in normalized micro-units (`0..=1_000_000`).
    pub view_start_micros: u32,
    /// Visible view end in normalized micro-units (`0..=1_000_000`).
    pub view_end_micros: u32,
    /// Visible view start in normalized nanounits (`0..=1_000_000_000`).
    ///
    /// Native input uses these fields for deep-zoom pointer-to-time mapping so
    /// click-to-play can preserve exact pixel intent even when the view span is
    /// narrower than one micro-unit.
    pub view_start_nanos: u32,
    /// Visible view end in normalized nanounits (`0..=1_000_000_000`).
    ///
    /// Native input uses these fields for deep-zoom pointer-to-time mapping so
    /// click-to-play can preserve exact pixel intent even when the view span is
    /// narrower than one micro-unit.
    pub view_end_nanos: u32,
    /// Quarter-note beat spacing in normalized micro-units when BPM/grid data is available.
    ///
    /// Native waveform renderers use this to draw a minor line on every beat
    /// and can accent every fourth beat as a bar boundary.
    pub beat_step_micros: Option<u32>,
    /// BPM grid origin in normalized micro-units.
    ///
    /// Native shells use this as the selection-relative anchor for drawing
    /// beat grid lines when no active selection is available. A value of `0`
    /// means no projected origin has been supplied yet.
    pub bpm_grid_origin_micros: u32,
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
            loading: false,
            image_rendering: false,
            cursor_milli: None,
            playhead_milli: None,
            playhead_micros: None,
            selection_milli: None,
            slices: Vec::new(),
            selection_export_flash_nonce: 0,
            selection_export_failure_flash_nonce: 0,
            edit_selection_apply_flash_nonce: 0,
            edit_selection_milli: None,
            edit_fade_in_end_milli: None,
            edit_fade_in_end_micros: None,
            edit_fade_in_mute_start_milli: None,
            edit_fade_in_mute_start_micros: None,
            edit_fade_in_curve_milli: None,
            edit_fade_out_start_milli: None,
            edit_fade_out_start_micros: None,
            edit_fade_out_mute_end_milli: None,
            edit_fade_out_mute_end_micros: None,
            edit_fade_out_curve_milli: None,
            view_start_milli: 0,
            view_end_milli: 1000,
            view_start_micros: 0,
            view_end_micros: 1_000_000,
            view_start_nanos: 0,
            view_end_nanos: 1_000_000_000,
            beat_step_micros: None,
            bpm_grid_origin_micros: 0,
            loop_enabled: false,
            tempo_label: None,
            zoom_label: None,
            waveform_image_signature: None,
            waveform_image: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WaveformPanelModel;

    #[test]
    fn default_bpm_grid_origin_is_zero() {
        assert_eq!(WaveformPanelModel::default().bpm_grid_origin_micros, 0);
    }
}

/// Waveform chrome copy used by metadata lines and control surfaces.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaveformChromeModel {
    /// Extra transport metadata hint shown alongside waveform labels.
    pub transport_hint: String,
    /// Whether compare-anchor replay is currently available.
    pub compare_anchor_available: bool,
    /// Label for the stored compare anchor, when available.
    pub compare_anchor_label: Option<String>,
    /// Whether loop state is locked against sample-driven auto-updates.
    pub loop_lock_enabled: bool,
    /// Current channel-view mode used by waveform rendering.
    pub channel_view: WaveformChannelViewModel,
    /// Whether normalized audition playback is enabled.
    pub normalized_audition_enabled: bool,
    /// Whether BPM snapping is enabled for waveform edits.
    pub bpm_snap_enabled: bool,
    /// Whether playback BPM grids and snapping use selection-relative anchors.
    pub relative_bpm_grid_enabled: bool,
    /// Whether transient snapping is enabled for waveform edits.
    pub transient_snap_enabled: bool,
    /// Whether transient markers are visible on the waveform.
    pub transient_markers_enabled: bool,
    /// Whether slice mode is currently active.
    pub slice_mode_enabled: bool,
    /// Whether the current slice batch is an exact-duplicate cleanup preview.
    ///
    /// Native shells use this to enable cleanup-only actions without exposing
    /// generic slice workflows such as silence-split export review.
    pub exact_duplicate_cleanup_available: bool,
}

impl Default for WaveformChromeModel {
    fn default() -> Self {
        Self {
            transport_hint: String::from("transport idle"),
            compare_anchor_available: false,
            compare_anchor_label: None,
            loop_lock_enabled: false,
            channel_view: WaveformChannelViewModel::Mono,
            normalized_audition_enabled: false,
            bpm_snap_enabled: false,
            relative_bpm_grid_enabled: false,
            transient_snap_enabled: false,
            transient_markers_enabled: true,
            slice_mode_enabled: false,
            exact_duplicate_cleanup_available: false,
        }
    }
}
