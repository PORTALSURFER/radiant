use crate::gui::range::NormalizedRange;

/// Explicit parts used to build normalized timeline transport state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct TimelineTransportParts {
    /// Cursor position in normalized milli-units.
    pub cursor_milli: Option<u16>,
    /// Playhead position in normalized milli-units.
    pub playhead_milli: Option<u16>,
    /// Playhead position in normalized micro-units.
    pub playhead_micros: Option<u32>,
    /// Selected playback/review range.
    pub selection: Option<NormalizedRange>,
}

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
    /// Build a timeline transport state from named normalized values.
    pub fn from_parts(parts: TimelineTransportParts) -> Self {
        Self {
            cursor_milli: parts.cursor_milli,
            playhead_milli: parts.playhead_milli,
            playhead_micros: parts.playhead_micros,
            selection: parts.selection,
        }
    }

    /// Build a timeline transport state from explicit normalized values.
    pub fn new(
        cursor_milli: Option<u16>,
        playhead_milli: Option<u16>,
        playhead_micros: Option<u32>,
        selection: Option<NormalizedRange>,
    ) -> Self {
        Self::from_parts(TimelineTransportParts {
            cursor_milli,
            playhead_milli,
            playhead_micros,
            selection,
        })
    }

    /// Return the most precise available playhead value in micro-units.
    pub fn resolved_playhead_micros(self) -> Option<u32> {
        self.playhead_micros
            .or_else(|| self.playhead_milli.map(|milli| u32::from(milli) * 1000))
    }
}
