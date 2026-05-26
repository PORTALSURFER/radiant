use crate::gui::types::{Point, Rect};

/// Named fields for reusable piano-roll pitch-row geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelinePitchLayoutParts {
    /// Rect that contains the visible pitch rows.
    pub rect: Rect,
    /// Lowest visible pitch.
    pub pitch_start: i32,
    /// Number of visible pitch rows.
    pub visible_pitches: usize,
}

impl TimelinePitchLayoutParts {
    /// Build pitch layout parts.
    pub const fn new(rect: Rect, pitch_start: i32, visible_pitches: usize) -> Self {
        Self {
            rect,
            pitch_start,
            visible_pitches,
        }
    }
}

/// Reusable vertical pitch-row geometry for piano rolls and note editors.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelinePitchLayout {
    /// Rect that contains the visible pitch rows.
    pub rect: Rect,
    /// Lowest visible pitch.
    pub pitch_start: i32,
    /// Number of visible pitch rows.
    pub visible_pitches: usize,
}

impl TimelinePitchLayout {
    /// Build a pitch layout from named parts.
    pub const fn from_parts(parts: TimelinePitchLayoutParts) -> Self {
        Self {
            rect: parts.rect,
            pitch_start: parts.pitch_start,
            visible_pitches: parts.visible_pitches,
        }
    }

    /// Build a pitch layout.
    pub const fn new(rect: Rect, pitch_start: i32, visible_pitches: usize) -> Self {
        Self::from_parts(TimelinePitchLayoutParts::new(
            rect,
            pitch_start,
            visible_pitches,
        ))
    }

    /// Return the highest visible pitch.
    pub fn pitch_end(self) -> i32 {
        self.pitch_start + self.visible_pitches.saturating_sub(1) as i32
    }

    /// Return the resolved pitch-row height.
    pub fn row_height(self) -> f32 {
        self.rect.height() / self.visible_pitches.max(1) as f32
    }

    /// Return the y coordinate for the top edge of a pitch row.
    pub fn y_for_pitch(self, pitch: i32) -> f32 {
        let row = self.pitch_end() - pitch;
        self.rect.min.y + row as f32 * self.row_height()
    }

    /// Return the visible row rect for a pitch.
    pub fn pitch_rect(self, pitch: i32) -> Rect {
        let y = self.y_for_pitch(pitch);
        Rect::from_min_max(
            Point::new(self.rect.min.x, y),
            Point::new(
                self.rect.max.x,
                (y + self.row_height()).min(self.rect.max.y),
            ),
        )
    }

    /// Return the pitch at a pointer position.
    pub fn pitch_at(self, position: Point) -> Option<i32> {
        if self.visible_pitches == 0 || !self.rect.contains(position) {
            return None;
        }
        let row = ((position.y - self.rect.min.y) / self.row_height().max(1.0)).floor() as i32;
        Some((self.pitch_end() - row).clamp(self.pitch_start, self.pitch_end()))
    }
}
