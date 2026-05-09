use crate::gui::range::NormalizedRange;

/// Cursor, playhead, and selected range for a normalized timeline.
///
/// The playhead can carry both milli and micro precision so render passes can
/// use a coarse label while preserving smoother motion when hosts provide it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct TimelineTransportState {
    /// Cursor position in normalized milli-units.
    pub cursor_milli: Option<u16>,
    /// Playhead position in normalized milli-units.
    pub playhead_milli: Option<u16>,
    /// Playhead position in normalized micro-units.
    pub playhead_micros: Option<u32>,
    /// Selected playback/review range.
    pub selection: Option<NormalizedRange>,
}

impl TimelineTransportState {
    /// Build a timeline transport state from explicit normalized values.
    pub fn new(
        cursor_milli: Option<u16>,
        playhead_milli: Option<u16>,
        playhead_micros: Option<u32>,
        selection: Option<NormalizedRange>,
    ) -> Self {
        Self {
            cursor_milli,
            playhead_milli,
            playhead_micros,
            selection,
        }
    }

    /// Return the most precise available playhead value in micro-units.
    pub fn resolved_playhead_micros(self) -> Option<u32> {
        self.playhead_micros
            .or_else(|| self.playhead_milli.map(|milli| u32::from(milli) * 1000))
    }
}
