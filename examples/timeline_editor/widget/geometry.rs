//! Timeline geometry mapping for the timeline editor example.

use super::super::{
    CLIP_HEIGHT, HEADER_WIDTH, LANE_COUNT, LANE_HEIGHT, RULER_HEIGHT, TOTAL_BEATS, TRACK_PAD,
    model::{BeatRange, TimelineClip},
};
use radiant::layout::{Point, Rect};

#[derive(Clone, Copy)]
pub(crate) struct TimelineGeometry {
    pub(crate) header: Rect,
    pub(crate) ruler: Rect,
    pub(crate) lanes: Rect,
}

impl TimelineGeometry {
    pub(crate) fn new(bounds: Rect) -> Self {
        let header = Rect::from_min_max(
            bounds.min,
            Point::new(bounds.min.x + HEADER_WIDTH, bounds.max.y),
        );
        let ruler = Rect::from_min_max(
            Point::new(bounds.min.x + HEADER_WIDTH, bounds.min.y),
            Point::new(bounds.max.x, bounds.min.y + RULER_HEIGHT),
        );
        let lanes = Rect::from_min_max(
            Point::new(bounds.min.x + HEADER_WIDTH, bounds.min.y + RULER_HEIGHT),
            bounds.max,
        );
        Self {
            header,
            ruler,
            lanes,
        }
    }

    pub(crate) fn lane_rect(self, lane: usize) -> Rect {
        let y = self.lanes.min.y + lane as f32 * LANE_HEIGHT;
        Rect::from_min_max(
            Point::new(self.lanes.min.x, y),
            Point::new(self.lanes.max.x, (y + LANE_HEIGHT).min(self.lanes.max.y)),
        )
    }

    pub(crate) fn clip_rect(self, clip: &TimelineClip) -> Rect {
        self.clip_rect_for_range(clip.lane, clip.range)
    }

    pub(crate) fn clip_rect_for_range(self, lane: usize, range: BeatRange) -> Rect {
        let lane_rect = self.lane_rect(lane);
        let y = lane_rect.min.y + (lane_rect.height() - CLIP_HEIGHT) * 0.5;
        Rect::from_min_max(
            Point::new(self.x_for_beat(range.start) + 2.0, y),
            Point::new(self.x_for_beat(range.end) - 2.0, y + CLIP_HEIGHT),
        )
    }

    pub(crate) fn x_for_beat(self, beat: u32) -> f32 {
        self.lanes.min.x + self.beat_width() * beat.min(TOTAL_BEATS) as f32
    }

    pub(crate) fn cursor_x_at(self, position: Point) -> Option<f32> {
        if position.x < self.lanes.min.x || position.x > self.lanes.max.x {
            return None;
        }
        Some(position.x.clamp(self.lanes.min.x, self.lanes.max.x))
    }

    pub(crate) fn beat_at(self, position: Point) -> Option<u32> {
        self.cursor_x_at(position)?;
        Some(((position.x - self.lanes.min.x) / self.beat_width()).round() as u32)
    }

    pub(crate) fn lane_at(self, position: Point) -> Option<usize> {
        if !self.lanes.contains(position) {
            return None;
        }
        Some(((position.y - self.lanes.min.y) / LANE_HEIGHT).floor() as usize)
            .map(|lane| lane.min(LANE_COUNT - 1))
    }

    fn beat_width(self) -> f32 {
        ((self.lanes.width() - TRACK_PAD).max(1.0)) / TOTAL_BEATS as f32
    }
}
