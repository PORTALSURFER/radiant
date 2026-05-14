use super::{LANE_COUNT, MIN_CLIP_BEATS, TOTAL_BEATS};
use radiant::gui::{
    range::NormalizedRange,
    visualization::{
        ChannelViewMode, SignalChromeState, SignalRasterPreview, SignalToolFlags, SignalToolState,
        TimelineEditPreview, TimelineFeedbackEvents, TimelineMarkerPreview, TimelineMotionState,
        TimelinePresentationState, TimelineSurfaceState, TimelineTransportState, TimelineViewport,
    },
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct TimelineEditorState {
    pub(super) playing: bool,
    pub(super) repeat_enabled: bool,
    pub(super) playhead_beat: u32,
    pub(super) selected_clip: Option<u32>,
    pub(super) selection: Option<BeatRange>,
    pub(super) next_clip_id: u32,
    pub(super) revision: u64,
    pub(super) feedback_nonce: u64,
    pub(super) status: String,
    pub(super) clips: Vec<TimelineClip>,
}

impl Default for TimelineEditorState {
    fn default() -> Self {
        Self {
            playing: false,
            repeat_enabled: true,
            playhead_beat: 18,
            selected_clip: Some(2),
            selection: Some(BeatRange { start: 16, end: 28 }),
            next_clip_id: 5,
            revision: 1,
            feedback_nonce: 0,
            status: "ready".to_string(),
            clips: vec![
                TimelineClip::new(1, "Kick loop", 0, 0, 16),
                TimelineClip::new(2, "Bass phrase", 1, 12, 28),
                TimelineClip::new(3, "Chord stab", 2, 28, 44),
                TimelineClip::new(4, "Vocal chop", 3, 42, 58),
            ],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct TimelineClip {
    pub(super) id: u32,
    pub(super) name: &'static str,
    pub(super) lane: usize,
    pub(super) range: BeatRange,
}

impl TimelineClip {
    pub(super) fn new(id: u32, name: &'static str, lane: usize, start: u32, end: u32) -> Self {
        Self {
            id,
            name,
            lane,
            range: BeatRange { start, end },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct BeatRange {
    pub(super) start: u32,
    pub(super) end: u32,
}

impl BeatRange {
    pub(super) fn normalized(start: u32, end: u32) -> Self {
        Self {
            start: start.min(end).min(TOTAL_BEATS),
            end: start.max(end).min(TOTAL_BEATS),
        }
    }

    pub(super) fn duration(self) -> u32 {
        self.end.saturating_sub(self.start)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum TimelineMessage {
    TogglePlay,
    ToggleRepeat(bool),
    Rewind,
    DuplicateSelection,
    DeleteSelection,
    Surface(TimelineSurfaceMessage),
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum TimelineSurfaceMessage {
    Seek {
        beat: u32,
    },
    SelectClip {
        clip_id: u32,
        beat: u32,
    },
    MoveClip {
        clip_id: u32,
        lane: usize,
        start: u32,
    },
    ResizeClip {
        clip_id: u32,
        range: BeatRange,
    },
    SelectRange {
        range: BeatRange,
    },
    CreateClip {
        lane: usize,
        range: BeatRange,
    },
    DeleteSelected,
}

pub(super) fn update(state: &mut TimelineEditorState, message: TimelineMessage) {
    match message {
        TimelineMessage::TogglePlay => {
            state.playing = !state.playing;
            state.feedback_nonce += 1;
            state.status = if state.playing { "playing" } else { "paused" }.to_string();
        }
        TimelineMessage::ToggleRepeat(enabled) => {
            state.repeat_enabled = enabled;
            state.status = if enabled {
                "loop enabled"
            } else {
                "loop disabled"
            }
            .to_string();
            state.revision += 1;
        }
        TimelineMessage::Rewind => {
            state.playhead_beat = 0;
            state.status = "rewound to bar 1".to_string();
            state.revision += 1;
        }
        TimelineMessage::DuplicateSelection => duplicate_selected_clip(state),
        TimelineMessage::DeleteSelection => delete_selected_clip(state),
        TimelineMessage::Surface(message) => update_surface(state, message),
    }
}

pub(super) fn update_surface(state: &mut TimelineEditorState, message: TimelineSurfaceMessage) {
    match message {
        TimelineSurfaceMessage::Seek { beat } => {
            state.playhead_beat = beat.min(TOTAL_BEATS);
            state.selection = None;
            state.status = format!("playhead at beat {}", state.playhead_beat);
            state.revision += 1;
        }
        TimelineSurfaceMessage::SelectClip { clip_id, beat } => {
            state.selected_clip = Some(clip_id);
            state.playhead_beat = beat.min(TOTAL_BEATS);
            state.selection = clip_range(state, clip_id);
            state.status = format!("clip {} selected", clip_id);
            state.revision += 1;
        }
        TimelineSurfaceMessage::MoveClip {
            clip_id,
            lane,
            start,
        } => {
            let updated = if let Some(clip) = state.clips.iter_mut().find(|clip| clip.id == clip_id)
            {
                let duration = clip.range.duration();
                let start = start.min(TOTAL_BEATS.saturating_sub(duration));
                clip.lane = lane.min(LANE_COUNT - 1);
                clip.range = BeatRange {
                    start,
                    end: start + duration,
                };
                state.selected_clip = Some(clip_id);
                state.selection = Some(clip.range);
                state.status = format!("{} moved to track {}", clip.name, clip.lane + 1);
                state.revision += 1;
                Some((clip.id, clip.lane, clip.range))
            } else {
                None
            };
            if let Some((clip_id, lane, range)) = updated {
                cut_overlapping_clips(state, Some(clip_id), lane, range);
            }
        }
        TimelineSurfaceMessage::ResizeClip { clip_id, range } => {
            let updated = if let Some(clip) = state.clips.iter_mut().find(|clip| clip.id == clip_id)
            {
                clip.range = range;
                state.selected_clip = Some(clip_id);
                state.selection = Some(range);
                state.status = format!(
                    "{} resized to beats {}-{}",
                    clip.name, clip.range.start, clip.range.end
                );
                state.revision += 1;
                Some((clip.id, clip.lane, clip.range))
            } else {
                None
            };
            if let Some((clip_id, lane, range)) = updated {
                cut_overlapping_clips(state, Some(clip_id), lane, range);
            }
        }
        TimelineSurfaceMessage::SelectRange { range } => {
            state.selection = Some(range);
            state.selected_clip = None;
            state.status = format!("selected beats {}-{}", range.start, range.end);
            state.revision += 1;
        }
        TimelineSurfaceMessage::CreateClip { lane, range } => {
            create_clip(state, lane, range);
        }
        TimelineSurfaceMessage::DeleteSelected => delete_selected_clip(state),
    }
}

pub(super) fn create_clip(state: &mut TimelineEditorState, lane: usize, range: BeatRange) {
    if range.duration() < MIN_CLIP_BEATS {
        return;
    }
    let id = state.next_clip_id;
    state.next_clip_id += 1;
    state.clips.push(TimelineClip {
        id,
        name: "New clip",
        lane: lane.min(LANE_COUNT - 1),
        range,
    });
    cut_overlapping_clips(state, Some(id), lane.min(LANE_COUNT - 1), range);
    state.selected_clip = Some(id);
    state.selection = Some(range);
    state.playhead_beat = range.start;
    state.status = format!("created clip {} on track {}", id, lane + 1);
    state.feedback_nonce += 1;
    state.revision += 1;
}

fn cut_overlapping_clips(
    state: &mut TimelineEditorState,
    protected_clip: Option<u32>,
    lane: usize,
    priority: BeatRange,
) {
    if priority.duration() == 0 {
        return;
    }

    let mut next_split_id = state.next_clip_id;
    let mut cut = Vec::with_capacity(state.clips.len() + 1);
    for clip in state.clips.drain(..) {
        if Some(clip.id) == protected_clip || clip.lane != lane {
            cut.push(clip);
            continue;
        }
        append_cut_clip_segments(&mut cut, &mut next_split_id, clip, priority);
    }
    state.next_clip_id = next_split_id;
    cut.sort_by_key(|clip| (clip.lane, clip.range.start, clip.range.end, clip.id));
    state.clips = cut;
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

pub(super) fn duplicate_selected_clip(state: &mut TimelineEditorState) {
    let Some(source_id) = state.selected_clip else {
        state.status = "select a clip first".to_string();
        return;
    };
    let Some(source) = state
        .clips
        .iter()
        .find(|clip| clip.id == source_id)
        .cloned()
    else {
        return;
    };
    let duration = source.range.duration();
    let start = (source.range.end + 2).min(TOTAL_BEATS.saturating_sub(duration));
    let id = state.next_clip_id;
    state.next_clip_id += 1;
    state.clips.push(TimelineClip {
        id,
        name: "Copy",
        lane: source.lane,
        range: BeatRange {
            start,
            end: start + duration,
        },
    });
    state.selected_clip = Some(id);
    state.selection = Some(BeatRange {
        start,
        end: start + duration,
    });
    state.status = format!("duplicated clip {}", source_id);
    state.revision += 1;
}

pub(super) fn delete_selected_clip(state: &mut TimelineEditorState) {
    let Some(clip_id) = state.selected_clip else {
        state.status = "select a clip first".to_string();
        return;
    };
    let before = state.clips.len();
    state.clips.retain(|clip| clip.id != clip_id);
    if state.clips.len() == before {
        state.status = format!("clip {} was already gone", clip_id);
        state.selected_clip = None;
        state.selection = None;
        state.revision += 1;
        return;
    }
    state.selected_clip = None;
    state.selection = None;
    state.status = format!("deleted clip {}", clip_id);
    state.feedback_nonce += 1;
    state.revision += 1;
}

pub(super) fn timeline_surface(state: &TimelineEditorState) -> TimelineMotionState {
    let selection = state.selection.map(|range| {
        NormalizedRange::from_micros(beat_to_micros(range.start), beat_to_micros(range.end))
    });
    let surface = TimelineSurfaceState::new(
        TimelineViewport::new(0, 1_000, 0, 1_000_000, 0, 1_000_000_000),
        TimelineTransportState::new(
            Some(beat_to_normalized(state.playhead_beat)),
            None,
            Some(beat_to_micros(state.playhead_beat)),
            selection,
        ),
        TimelineEditPreview::new(
            selection,
            selection.map(|range| range.start_milli),
            selection.map(|range| range.start_micros),
            selection.map(|range| range.start_milli.saturating_add(1)),
            selection.map(|range| range.start_micros.saturating_add(1_000)),
            selection.map(|range| range.end_milli),
            selection.map(|range| range.end_milli.saturating_sub(1)),
            selection.map(|range| range.end_micros.saturating_sub(1_000)),
            selection.map(|range| range.end_milli),
            selection.map(|range| range.end_micros),
            None,
        ),
        TimelineFeedbackEvents::new(state.feedback_nonce, 0, state.revision),
        TimelinePresentationState::new(
            Some(beat_to_micros(4)),
            0,
            state.repeat_enabled,
            Some("Arrangement".to_string()),
            Some(format!("{} beats", TOTAL_BEATS)),
        ),
        SignalRasterPreview::new(
            Some("arrangement timeline atlas".to_string()),
            false,
            false,
            Some(state.revision),
            None,
        ),
        state
            .clips
            .iter()
            .map(|clip| {
                marker(
                    beat_to_normalized(clip.range.start),
                    beat_to_normalized(clip.range.end),
                    state.selected_clip == Some(clip.id),
                )
            })
            .collect(),
    );

    TimelineMotionState::new(
        state.playing,
        surface,
        SignalChromeState::new(
            if state.playing { "playing" } else { "idle" },
            true,
            Some(format!("beat {}", state.playhead_beat)),
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

pub(super) fn clip_range(state: &TimelineEditorState, clip_id: u32) -> Option<BeatRange> {
    state
        .clips
        .iter()
        .find(|clip| clip.id == clip_id)
        .map(|clip| clip.range)
}

pub(super) fn beat_to_normalized(beat: u32) -> u16 {
    ((beat.min(TOTAL_BEATS) as f32 / TOTAL_BEATS as f32) * 1_000.0).round() as u16
}

fn beat_to_micros(beat: u32) -> u32 {
    beat.min(TOTAL_BEATS) * 125_000
}

pub(super) fn timeline_label(
    state: &TimelineEditorState,
    timeline: &TimelineMotionState,
) -> String {
    format!(
        "{} / clips {} / playhead beat {} / {}",
        timeline.chrome.status_hint,
        timeline.surface.markers.len(),
        state.playhead_beat,
        state.status
    )
}
