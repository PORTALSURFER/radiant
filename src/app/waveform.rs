//! Waveform-facing models exposed by the `radiant` app contract.

use crate::gui::types::ImageRgba;
use std::sync::Arc;

/// Normalized range with deterministic milli and micro projections.
///
/// The native shell keeps milli fields for coarse equality checks and legacy
/// tests, while the micro fields preserve enough precision for smooth waveform
/// selection, edit-selection, and fade-handle motion at deep zoom levels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NormalizedRangeModel {
    /// Start position in normalized milli-units.
    pub start_milli: u16,
    /// End position in normalized milli-units.
    pub end_milli: u16,
    /// Start position in normalized micro-units (`0..=1_000_000`).
    pub start_micros: u32,
    /// End position in normalized micro-units (`0..=1_000_000`).
    pub end_micros: u32,
}

impl NormalizedRangeModel {
    /// Build a normalized range, clamping bounds to `0..=1000` and ordering them.
    pub fn new(start_milli: u16, end_milli: u16) -> Self {
        Self::from_micros(
            u32::from(start_milli.min(1000)) * 1000,
            u32::from(end_milli.min(1000)) * 1000,
        )
    }

    /// Build a normalized range from micro precision while preserving ordered milli mirrors.
    pub fn from_micros(start_micros: u32, end_micros: u32) -> Self {
        let start = start_micros.min(1_000_000);
        let end = end_micros.min(1_000_000);
        let ordered_start = start.min(end);
        let ordered_end = end.max(start);
        Self {
            start_milli: micros_to_milli(ordered_start),
            end_milli: micros_to_milli(ordered_end),
            start_micros: ordered_start,
            end_micros: ordered_end,
        }
    }
}

fn micros_to_milli(value_micros: u32) -> u16 {
    ((value_micros.min(1_000_000) + 500) / 1000) as u16
}

/// Waveform preview metadata consumed by the native shell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaveformPanelModel {
    /// Display label for the loaded sample, when any.
    pub loaded_label: Option<String>,
    /// Whether a newly focused sample is still loading waveform data.
    pub loading: bool,
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
            cursor_milli: None,
            playhead_milli: None,
            playhead_micros: None,
            selection_milli: None,
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
            loop_enabled: false,
            tempo_label: None,
            zoom_label: None,
            waveform_image_signature: None,
            waveform_image: None,
        }
    }
}

/// Waveform chrome copy used by metadata lines in the native shell header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaveformChannelViewModel {
    /// Collapse channels into one mono envelope.
    Mono,
    /// Render left/right channels in split stereo mode.
    Stereo,
}

/// Waveform chrome copy used by metadata lines and control surfaces.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaveformChromeModel {
    /// Extra transport metadata hint shown alongside waveform labels.
    pub transport_hint: String,
    /// Current channel-view mode used by waveform rendering.
    pub channel_view: WaveformChannelViewModel,
    /// Whether normalized audition playback is enabled.
    pub normalized_audition_enabled: bool,
    /// Whether BPM snapping is enabled for waveform edits.
    pub bpm_snap_enabled: bool,
    /// Whether transient snapping is enabled for waveform edits.
    pub transient_snap_enabled: bool,
    /// Whether transient markers are visible on the waveform.
    pub transient_markers_enabled: bool,
    /// Whether slice mode is currently active.
    pub slice_mode_enabled: bool,
}

impl Default for WaveformChromeModel {
    fn default() -> Self {
        Self {
            transport_hint: String::from("transport idle"),
            channel_view: WaveformChannelViewModel::Mono,
            normalized_audition_enabled: false,
            bpm_snap_enabled: false,
            transient_snap_enabled: false,
            transient_markers_enabled: true,
            slice_mode_enabled: false,
        }
    }
}
