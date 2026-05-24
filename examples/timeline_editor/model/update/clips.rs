use crate::{LANE_COUNT, MIN_CLIP_BEATS, TOTAL_BEATS};

use super::super::{BeatRange, TimelineClip, TimelineEditorState};

pub(super) fn create_clip(state: &mut TimelineEditorState, lane: usize, range: BeatRange) {
    if range.duration() < MIN_CLIP_BEATS {
        return;
    }
    let id = state.clip_store.next_clip_id;
    state.clip_store.next_clip_id += 1;
    state.clip_store.clips.push(TimelineClip {
        id,
        name: "New clip",
        lane: lane.min(LANE_COUNT - 1),
        range,
    });
    cut_overlapping_clips(state, Some(id), lane.min(LANE_COUNT - 1), range);
    state.edit.selected_clip = Some(id);
    state.edit.selection = Some(range);
    state.playback.playhead_beat = range.start;
    state.feedback.status = format!("created clip {} on track {}", id, lane + 1);
    state.feedback.feedback_nonce += 1;
    state.feedback.revision += 1;
}

pub(super) fn cut_overlapping_clips(
    state: &mut TimelineEditorState,
    protected_clip: Option<u32>,
    lane: usize,
    priority: BeatRange,
) {
    if priority.duration() == 0 {
        return;
    }

    let mut next_split_id = state.clip_store.next_clip_id;
    let mut cut = Vec::with_capacity(state.clip_store.clips.len() + 1);
    for clip in state.clip_store.clips.drain(..) {
        if Some(clip.id) == protected_clip || clip.lane != lane {
            cut.push(clip);
            continue;
        }
        append_cut_clip_segments(&mut cut, &mut next_split_id, clip, priority);
    }
    state.clip_store.next_clip_id = next_split_id;
    cut.sort_by_key(|clip| (clip.lane, clip.range.start, clip.range.end, clip.id));
    state.clip_store.clips = cut;
}

fn append_cut_clip_segments(
    clips: &mut Vec<TimelineClip>,
    next_split_id: &mut u32,
    clip: TimelineClip,
    priority: BeatRange,
) {
    if !ranges_overlap(clip.range, priority) {
        clips.push(clip);
        return;
    }

    let left = BeatRange {
        start: clip.range.start,
        end: priority.start.min(clip.range.end),
    };
    let right = BeatRange {
        start: priority.end.max(clip.range.start),
        end: clip.range.end,
    };
    let keep_left = left.duration() >= MIN_CLIP_BEATS;
    let keep_right = right.duration() >= MIN_CLIP_BEATS;
    if keep_left {
        clips.push(TimelineClip {
            range: left,
            ..clip.clone()
        });
    }
    if keep_right {
        let id = if keep_left {
            let id = *next_split_id;
            *next_split_id += 1;
            id
        } else {
            clip.id
        };
        clips.push(TimelineClip {
            id,
            range: right,
            ..clip
        });
    }
}

fn ranges_overlap(a: BeatRange, b: BeatRange) -> bool {
    a.start < b.end && b.start < a.end
}

pub(crate) fn duplicate_selected_clip(state: &mut TimelineEditorState) {
    let Some(source_id) = state.edit.selected_clip else {
        state.feedback.status = "select a clip first".to_string();
        return;
    };
    let Some(source) = state
        .clip_store
        .clips
        .iter()
        .find(|clip| clip.id == source_id)
        .cloned()
    else {
        return;
    };
    let duration = source.range.duration();
    let start = (source.range.end + 2).min(TOTAL_BEATS.saturating_sub(duration));
    let id = state.clip_store.next_clip_id;
    state.clip_store.next_clip_id += 1;
    state.clip_store.clips.push(TimelineClip {
        id,
        name: "Copy",
        lane: source.lane,
        range: BeatRange {
            start,
            end: start + duration,
        },
    });
    state.edit.selected_clip = Some(id);
    state.edit.selection = Some(BeatRange {
        start,
        end: start + duration,
    });
    state.feedback.status = format!("duplicated clip {}", source_id);
    state.feedback.revision += 1;
}

pub(crate) fn delete_selected_clip(state: &mut TimelineEditorState) {
    let Some(clip_id) = state.edit.selected_clip else {
        state.feedback.status = "select a clip first".to_string();
        return;
    };
    let before = state.clip_store.clips.len();
    state.clip_store.clips.retain(|clip| clip.id != clip_id);
    if state.clip_store.clips.len() == before {
        state.feedback.status = format!("clip {} was already gone", clip_id);
        state.edit.selected_clip = None;
        state.edit.selection = None;
        state.feedback.revision += 1;
        return;
    }
    state.edit.selected_clip = None;
    state.edit.selection = None;
    state.feedback.status = format!("deleted clip {}", clip_id);
    state.feedback.feedback_nonce += 1;
    state.feedback.revision += 1;
}
