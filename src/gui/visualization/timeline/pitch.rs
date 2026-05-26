use super::TimelineAxis;
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

/// Named fields for reusable pitch-row item geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelinePitchItemLayoutParts {
    /// Horizontal timeline projection.
    pub axis: TimelineAxis,
    /// Vertical pitch-row projection.
    pub pitches: TimelinePitchLayout,
    /// Horizontal inset applied inside the projected value range.
    pub horizontal_inset: f32,
    /// Vertical inset applied inside the projected pitch row.
    pub vertical_inset: f32,
}

impl TimelinePitchItemLayoutParts {
    /// Build pitch item-layout parts with no insets.
    pub const fn new(axis: TimelineAxis, pitches: TimelinePitchLayout) -> Self {
        Self {
            axis,
            pitches,
            horizontal_inset: 0.0,
            vertical_inset: 0.0,
        }
    }
}

/// Reusable item geometry for notes, events, and regions on pitch timelines.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelinePitchItemLayout {
    /// Horizontal timeline projection.
    pub axis: TimelineAxis,
    /// Vertical pitch-row projection.
    pub pitches: TimelinePitchLayout,
    /// Horizontal inset applied inside the projected value range.
    pub horizontal_inset: f32,
    /// Vertical inset applied inside the projected pitch row.
    pub vertical_inset: f32,
}

impl TimelinePitchItemLayout {
    /// Build pitch item layout from named parts.
    pub const fn from_parts(parts: TimelinePitchItemLayoutParts) -> Self {
        Self {
            axis: parts.axis,
            pitches: parts.pitches,
            horizontal_inset: parts.horizontal_inset,
            vertical_inset: parts.vertical_inset,
        }
    }

    /// Build pitch item layout with no insets.
    pub const fn new(axis: TimelineAxis, pitches: TimelinePitchLayout) -> Self {
        Self::from_parts(TimelinePitchItemLayoutParts::new(axis, pitches))
    }

    /// Return this item layout with horizontal padding inside each projected range.
    pub const fn with_horizontal_inset(mut self, inset: f32) -> Self {
        self.horizontal_inset = inset;
        self
    }

    /// Return this item layout with vertical padding inside each pitch row.
    pub const fn with_vertical_inset(mut self, inset: f32) -> Self {
        self.vertical_inset = inset;
        self
    }

    /// Project a value range into an item rect on a pitch row, clamped to the visible value span.
    pub fn item_rect(self, pitch: i32, start: f32, end: f32) -> Rect {
        self.project_pitch_item_rect(pitch, self.axis.range_rect(start, end))
    }

    /// Project a value range into an item rect on a pitch row without clamping horizontal values.
    pub fn item_rect_unclamped(self, pitch: i32, start: f32, end: f32) -> Rect {
        self.project_pitch_item_rect(pitch, self.axis.range_rect_unclamped(start, end))
    }

    fn project_pitch_item_rect(self, pitch: i32, range_rect: Rect) -> Rect {
        let row = self.pitches.pitch_rect(pitch).inset_vertical(
            finite_nonnegative(self.vertical_inset),
            finite_nonnegative(self.vertical_inset),
        );
        let horizontal_inset = finite_nonnegative(self.horizontal_inset);
        let range = range_rect.inset_horizontal(horizontal_inset, horizontal_inset);
        Rect::from_min_max(
            Point::new(range.min.x, row.min.y),
            Point::new(range.max.x, row.max.y),
        )
    }
}

fn finite_nonnegative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}
