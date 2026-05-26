use super::{TimelineAxis, TimelineLaneLayout};
use crate::gui::types::{Point, Rect};

/// Named fields for reusable timeline item geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelineItemLayoutParts {
    /// Horizontal timeline projection.
    pub axis: TimelineAxis,
    /// Vertical lane projection.
    pub lanes: TimelineLaneLayout,
    /// Preferred item height in logical pixels.
    pub item_height: f32,
    /// Horizontal inset applied inside the projected value range.
    pub horizontal_inset: f32,
}

impl TimelineItemLayoutParts {
    /// Build item-layout parts with no horizontal inset.
    pub const fn new(axis: TimelineAxis, lanes: TimelineLaneLayout, item_height: f32) -> Self {
        Self {
            axis,
            lanes,
            item_height,
            horizontal_inset: 0.0,
        }
    }
}

/// Reusable item geometry for clips, notes, events, and regions on timelines.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelineItemLayout {
    /// Horizontal timeline projection.
    pub axis: TimelineAxis,
    /// Vertical lane projection.
    pub lanes: TimelineLaneLayout,
    /// Preferred item height in logical pixels.
    pub item_height: f32,
    /// Horizontal inset applied inside the projected value range.
    pub horizontal_inset: f32,
}

impl TimelineItemLayout {
    /// Build timeline item layout from named parts.
    pub const fn from_parts(parts: TimelineItemLayoutParts) -> Self {
        Self {
            axis: parts.axis,
            lanes: parts.lanes,
            item_height: parts.item_height,
            horizontal_inset: parts.horizontal_inset,
        }
    }

    /// Build timeline item layout with no horizontal inset.
    pub const fn new(axis: TimelineAxis, lanes: TimelineLaneLayout, item_height: f32) -> Self {
        Self::from_parts(TimelineItemLayoutParts::new(axis, lanes, item_height))
    }

    /// Return this item layout with horizontal padding inside each projected range.
    pub const fn with_horizontal_inset(mut self, inset: f32) -> Self {
        self.horizontal_inset = inset;
        self
    }

    /// Project a value range into a vertically centered item rect on a lane.
    pub fn item_rect(self, lane: usize, start: f32, end: f32) -> Rect {
        let lane_rect = self.lanes.lane_rect(lane);
        let inset = finite_nonnegative(self.horizontal_inset);
        let range_rect = self
            .axis
            .range_rect(start, end)
            .inset_horizontal(inset, inset);
        let height = self.resolved_item_height(lane_rect);
        let min_y = lane_rect.min.y + ((lane_rect.height() - height) * 0.5).max(0.0);
        Rect::from_min_max(
            Point::new(range_rect.min.x, min_y),
            Point::new(range_rect.max.x, min_y + height),
        )
    }

    fn resolved_item_height(self, lane_rect: Rect) -> f32 {
        let lane_height = lane_rect.height().max(0.0);
        if self.item_height.is_finite() && self.item_height > 0.0 {
            self.item_height.min(lane_height)
        } else {
            lane_height
        }
    }
}

fn finite_nonnegative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}
