use crate::TOTAL_BEATS;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TimelineEditorState {
    pub(crate) playing: bool,
    pub(crate) repeat_enabled: bool,
    pub(crate) playhead_beat: u32,
    pub(crate) selected_clip: Option<u32>,
    pub(crate) selection: Option<BeatRange>,
    pub(crate) next_clip_id: u32,
    pub(crate) revision: u64,
    pub(crate) feedback_nonce: u64,
    pub(crate) status: String,
    pub(crate) clips: Vec<TimelineClip>,
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
pub(crate) struct TimelineClip {
    pub(crate) id: u32,
    pub(crate) name: &'static str,
    pub(crate) lane: usize,
    pub(crate) range: BeatRange,
}

impl TimelineClip {
    pub(crate) fn new(id: u32, name: &'static str, lane: usize, start: u32, end: u32) -> Self {
        Self {
            id,
            name,
            lane,
            range: BeatRange { start, end },
        }
    }
}

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
