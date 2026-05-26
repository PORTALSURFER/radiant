//! Timeline geometry mapping for the timeline editor example.

use super::super::{
    CLIP_HEIGHT, HEADER_WIDTH, LANE_COUNT, LANE_HEIGHT, RULER_HEIGHT, TOTAL_BEATS, TRACK_PAD,
    model::{BeatRange, TimelineClip},
};
use radiant::gui::visualization::{TimelineAxis, TimelineLaneLayout};
use radiant::layout::{Point, Rect};

#[derive(Clone, Copy)]
pub(crate) struct TimelineGeometry {
    pub(crate) header: Rect,
    pub(crate) ruler: Rect,
    pub(crate) lanes: Rect,
    axis: TimelineAxis,
    lane_layout: TimelineLaneLayout,
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
        let axis =
            TimelineAxis::new(lanes, 0.0, TOTAL_BEATS as f32).with_trailing_padding(TRACK_PAD);
        let lane_layout = TimelineLaneLayout::fixed_height(lanes, LANE_COUNT, LANE_HEIGHT);
        Self {
            header,
            ruler,
            lanes,
            axis,
            lane_layout,
        }
    }

    pub(crate) fn lane_rect(self, lane: usize) -> Rect {
        self.lane_layout.lane_rect(lane)
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

    pub(crate) fn beat_range_rect(self, range: BeatRange) -> Rect {
        self.axis.range_rect(range.start as f32, range.end as f32)
    }

    pub(crate) fn x_for_beat(self, beat: u32) -> f32 {
        self.axis.x_for_value(beat as f32)
    }

    pub(crate) fn cursor_x_at(self, position: Point) -> Option<f32> {
        if position.x < self.lanes.min.x || position.x > self.lanes.max.x {
            return None;
        }
        Some(position.x.clamp(self.lanes.min.x, self.lanes.max.x))
    }

    pub(crate) fn beat_at(self, position: Point) -> Option<u32> {
        self.cursor_x_at(position)?;
        Some(self.axis.value_for_x(position.x).round() as u32)
    }

    pub(crate) fn lane_at(self, position: Point) -> Option<usize> {
        self.lane_layout.lane_at(position)
    }
}
