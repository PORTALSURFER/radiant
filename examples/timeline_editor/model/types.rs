use crate::TOTAL_BEATS;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TimelineEditorState {
    pub(crate) playback: TimelinePlaybackState,
    pub(crate) edit: TimelineEditState,
    pub(crate) clip_store: TimelineClipStore,
    pub(crate) feedback: TimelineFeedbackState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TimelinePlaybackState {
    pub(crate) playing: bool,
    pub(crate) repeat_enabled: bool,
    pub(crate) playhead_beat: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TimelineEditState {
    pub(crate) selected_clip: Option<u32>,
    pub(crate) selection: Option<BeatRange>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TimelineClipStore {
    pub(crate) next_clip_id: u32,
    pub(crate) clips: Vec<TimelineClip>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TimelineFeedbackState {
    pub(crate) revision: u64,
    pub(crate) feedback_nonce: u64,
    pub(crate) status: String,
}

impl Default for TimelineEditorState {
    fn default() -> Self {
        Self {
            playback: TimelinePlaybackState {
                playing: false,
                repeat_enabled: true,
                playhead_beat: 18,
            },
            edit: TimelineEditState {
                selected_clip: Some(2),
                selection: Some(BeatRange { start: 16, end: 28 }),
            },
            clip_store: TimelineClipStore {
                next_clip_id: 5,
                clips: DEFAULT_CLIPS.into_iter().map(TimelineClip::new).collect(),
            },
            feedback: TimelineFeedbackState {
                revision: 1,
                feedback_nonce: 0,
                status: "ready".to_string(),
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TimelineClip {
    pub(crate) id: u32,
    pub(crate) name: &'static str,
    pub(crate) lane: usize,
    pub(crate) range: BeatRange,
}

impl TimelineClip {
    pub(crate) fn new(parts: TimelineClipParts) -> Self {
        Self {
            id: parts.id,
            name: parts.name,
            lane: parts.lane,
            range: parts.range,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TimelineClipParts {
    pub(crate) id: u32,
    pub(crate) name: &'static str,
    pub(crate) lane: usize,
    pub(crate) range: BeatRange,
}

const DEFAULT_CLIPS: [TimelineClipParts; 4] = [
    TimelineClipParts {
        id: 1,
        name: "Kick loop",
        lane: 0,
        range: BeatRange { start: 0, end: 16 },
    },
    TimelineClipParts {
        id: 2,
        name: "Bass phrase",
        lane: 1,
        range: BeatRange { start: 12, end: 28 },
    },
    TimelineClipParts {
        id: 3,
        name: "Chord stab",
        lane: 2,
        range: BeatRange { start: 28, end: 44 },
    },
    TimelineClipParts {
        id: 4,
        name: "Vocal chop",
        lane: 3,
        range: BeatRange { start: 42, end: 58 },
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BeatRange {
    pub(crate) start: u32,
    pub(crate) end: u32,
}

impl BeatRange {
    pub(crate) fn normalized(start: u32, end: u32) -> Self {
        Self {
            start: start.min(end).min(TOTAL_BEATS),
            end: start.max(end).min(TOTAL_BEATS),
        }
    }

    pub(crate) fn duration(self) -> u32 {
        self.end.saturating_sub(self.start)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum TimelineMessage {
    TogglePlay,
    ToggleRepeat(bool),
    Rewind,
    DuplicateSelection,
    DeleteSelection,
    Surface(TimelineSurfaceMessage),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum TimelineSurfaceMessage {
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
