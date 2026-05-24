use super::{BeatRange, TimelineEditorState};
use crate::TOTAL_BEATS;
use radiant::gui::{
    range::NormalizedRange,
    visualization::{
        ChannelViewMode, SignalChromeState, SignalRasterPreview, SignalToolFlags, SignalToolState,
        TimelineEditPreview, TimelineEditPreviewParts, TimelineFeedbackEvents,
        TimelineFeedbackParts, TimelineMarkerPreview, TimelineMotionState,
        TimelinePresentationParts, TimelinePresentationState, TimelineSurfaceParts,
        TimelineSurfaceState, TimelineTransportParts, TimelineTransportState, TimelineViewport,
        TimelineViewportParts,
    },
};

pub(crate) fn timeline_surface(state: &TimelineEditorState) -> TimelineMotionState {
    let selection = state.edit.selection.map(|range| {
        NormalizedRange::from_micros(beat_to_micros(range.start), beat_to_micros(range.end))
    });
    let surface = TimelineSurfaceState::from_parts(TimelineSurfaceParts {
        viewport: TimelineViewport::from_parts(TimelineViewportParts::default()),
        transport: TimelineTransportState::from_parts(TimelineTransportParts {
            cursor_milli: Some(beat_to_normalized(state.playback.playhead_beat)),
            playhead_milli: None,
            playhead_micros: Some(beat_to_micros(state.playback.playhead_beat)),
            selection,
        }),
        edit_preview: TimelineEditPreview::from_parts(TimelineEditPreviewParts {
            selection,
            leading_end_milli: selection.map(|range| range.start_milli),
            leading_end_micros: selection.map(|range| range.start_micros),
            leading_inner_start_milli: selection.map(|range| range.start_milli.saturating_add(1)),
            leading_inner_start_micros: selection
                .map(|range| range.start_micros.saturating_add(1_000)),
            leading_curve_milli: selection.map(|range| range.end_milli),
            trailing_start_milli: selection.map(|range| range.end_milli.saturating_sub(1)),
            trailing_start_micros: selection.map(|range| range.end_micros.saturating_sub(1_000)),
            trailing_inner_end_milli: selection.map(|range| range.end_milli),
            trailing_inner_end_micros: selection.map(|range| range.end_micros),
            trailing_curve_milli: None,
        }),
        feedback_events: TimelineFeedbackEvents::from_parts(TimelineFeedbackParts {
            primary_success_nonce: state.feedback.feedback_nonce,
            primary_failure_nonce: 0,
            secondary_success_nonce: state.feedback.revision,
        }),
        presentation: TimelinePresentationState::from_parts(TimelinePresentationParts {
            guide_step_micros: Some(beat_to_micros(4)),
            guide_origin_micros: 0,
            repeat_enabled: state.playback.repeat_enabled,
            primary_label: Some("Arrangement".to_string()),
            viewport_label: Some(format!("{} beats", TOTAL_BEATS)),
        }),
        raster_preview: SignalRasterPreview::new(
            Some("arrangement timeline atlas".to_string()),
            false,
            false,
            Some(state.feedback.revision),
            None,
        ),
        markers: state
            .clip_store
            .clips
            .iter()
            .map(|clip| {
                marker(
                    beat_to_normalized(clip.range.start),
                    beat_to_normalized(clip.range.end),
                    state.edit.selected_clip == Some(clip.id),
                )
            })
            .collect(),
    });

    TimelineMotionState::new(
        state.playback.playing,
        surface,
        SignalChromeState::new(
            if state.playback.playing {
                "playing"
            } else {
                "idle"
            },
            true,
            Some(format!("beat {}", state.playback.playhead_beat)),
            ChannelViewMode::Stereo,
        ),
        SignalToolState::from_flags(SignalToolFlags {
            lock_enabled: false,
            alternate_preview_enabled: true,
            primary_snap_enabled: true,
            relative_grid_enabled: true,
            secondary_snap_enabled: true,
            markers_visible: true,
            marker_mode_enabled: true,
            batch_action_available: true,
        }),
    )
}

fn marker(start: u16, end: u16, selected: bool) -> TimelineMarkerPreview {
    TimelineMarkerPreview {
        range: NormalizedRange::new(start, end),
        selected,
        focused: selected,
    }
}

pub(crate) fn clip_range(state: &TimelineEditorState, clip_id: u32) -> Option<BeatRange> {
    state
        .clip_store
        .clips
        .iter()
        .find(|clip| clip.id == clip_id)
        .map(|clip| clip.range)
}

pub(crate) fn beat_to_normalized(beat: u32) -> u16 {
    ((beat.min(TOTAL_BEATS) as f32 / TOTAL_BEATS as f32) * 1_000.0).round() as u16
}

fn beat_to_micros(beat: u32) -> u32 {
    beat.min(TOTAL_BEATS) * 125_000
}

pub(crate) fn timeline_label(
    state: &TimelineEditorState,
    timeline: &TimelineMotionState,
) -> String {
    format!(
        "{} / clips {} / playhead beat {} / {}",
        timeline.chrome.status_hint,
        timeline.surface.markers.len(),
        state.playback.playhead_beat,
        state.feedback.status
    )
}
