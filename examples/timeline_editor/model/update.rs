#[path = "update/clips.rs"]
mod clips;

use super::{BeatRange, TimelineEditorState, TimelineMessage, TimelineSurfaceMessage};
use crate::{LANE_COUNT, TOTAL_BEATS};
use clips::{create_clip, cut_overlapping_clips};
pub(crate) use clips::{delete_selected_clip, duplicate_selected_clip};

pub(crate) fn update(state: &mut TimelineEditorState, message: TimelineMessage) {
    match message {
        TimelineMessage::TogglePlay => {
            state.playback.playing = !state.playback.playing;
            state.feedback.feedback_nonce += 1;
            state.feedback.status = if state.playback.playing {
                "playing"
            } else {
                "paused"
            }
            .to_string();
        }
        TimelineMessage::ToggleRepeat(enabled) => {
            state.playback.repeat_enabled = enabled;
            state.feedback.status = if enabled {
                "loop enabled"
            } else {
                "loop disabled"
            }
            .to_string();
            state.feedback.revision += 1;
        }
        TimelineMessage::Rewind => {
            state.playback.playhead_beat = 0;
            state.feedback.status = "rewound to bar 1".to_string();
            state.feedback.revision += 1;
        }
        TimelineMessage::DuplicateSelection => duplicate_selected_clip(state),
        TimelineMessage::DeleteSelection => delete_selected_clip(state),
        TimelineMessage::Surface(message) => update_surface(state, message),
    }
}

pub(crate) fn update_surface(state: &mut TimelineEditorState, message: TimelineSurfaceMessage) {
    match message {
        TimelineSurfaceMessage::Seek { beat } => {
            state.playback.playhead_beat = beat.min(TOTAL_BEATS);
            state.edit.selection = None;
            state.feedback.status = format!("playhead at beat {}", state.playback.playhead_beat);
            state.feedback.revision += 1;
        }
        TimelineSurfaceMessage::SelectClip { clip_id, beat } => {
            state.edit.selected_clip = Some(clip_id);
            state.playback.playhead_beat = beat.min(TOTAL_BEATS);
            state.edit.selection = super::projection::clip_range(state, clip_id);
            state.feedback.status = format!("clip {} selected", clip_id);
            state.feedback.revision += 1;
        }
        TimelineSurfaceMessage::MoveClip {
            clip_id,
            lane,
            start,
        } => {
            let updated = if let Some(clip) = state
                .clip_store
                .clips
                .iter_mut()
                .find(|clip| clip.id == clip_id)
            {
                let duration = clip.range.duration();
                let start = start.min(TOTAL_BEATS.saturating_sub(duration));
                clip.lane = lane.min(LANE_COUNT - 1);
                clip.range = BeatRange {
                    start,
                    end: start + duration,
                };
                state.edit.selected_clip = Some(clip_id);
                state.edit.selection = Some(clip.range);
                state.feedback.status = format!("{} moved to track {}", clip.name, clip.lane + 1);
                state.feedback.revision += 1;
                Some((clip.id, clip.lane, clip.range))
            } else {
                None
            };
            if let Some((clip_id, lane, range)) = updated {
                cut_overlapping_clips(state, Some(clip_id), lane, range);
            }
        }
        TimelineSurfaceMessage::ResizeClip { clip_id, range } => {
            let updated = if let Some(clip) = state
                .clip_store
                .clips
                .iter_mut()
                .find(|clip| clip.id == clip_id)
            {
                clip.range = range;
                state.edit.selected_clip = Some(clip_id);
                state.edit.selection = Some(range);
                state.feedback.status = format!(
                    "{} resized to beats {}-{}",
                    clip.name, clip.range.start, clip.range.end
                );
                state.feedback.revision += 1;
                Some((clip.id, clip.lane, clip.range))
            } else {
                None
            };
            if let Some((clip_id, lane, range)) = updated {
                cut_overlapping_clips(state, Some(clip_id), lane, range);
            }
        }
        TimelineSurfaceMessage::SelectRange { range } => {
            state.edit.selection = Some(range);
            state.edit.selected_clip = None;
            state.feedback.status = format!("selected beats {}-{}", range.start, range.end);
            state.feedback.revision += 1;
        }
        TimelineSurfaceMessage::CreateClip { lane, range } => {
            create_clip(state, lane, range);
        }
        TimelineSurfaceMessage::DeleteSelected => delete_selected_clip(state),
    }
}
