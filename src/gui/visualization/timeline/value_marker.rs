use super::TimelineAxis;
use crate::gui::visualization::value_marker::{VerticalValueMarker, vertical_value_marker};

/// Named fields for value markers positioned along a timeline axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelineValueMarkerLayoutParts {
    /// Horizontal timeline projection.
    pub axis: TimelineAxis,
    /// Width of the bottom-anchored marker stem.
    pub stem_width: f32,
    /// Width and height of the centered marker handle.
    pub handle_size: f32,
}

impl TimelineValueMarkerLayoutParts {
    /// Build timeline value-marker parts.
    pub const fn new(axis: TimelineAxis, stem_width: f32, handle_size: f32) -> Self {
        Self {
            axis,
            stem_width,
            handle_size,
        }
    }
}

/// Reusable geometry for velocity, automation, and other timeline value markers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelineValueMarkerLayout {
    /// Horizontal timeline projection.
    pub axis: TimelineAxis,
    /// Width of the bottom-anchored marker stem.
    pub stem_width: f32,
    /// Width and height of the centered marker handle.
    pub handle_size: f32,
}

impl TimelineValueMarkerLayout {
    /// Build value-marker layout from named parts.
    pub const fn from_parts(parts: TimelineValueMarkerLayoutParts) -> Self {
        Self {
            axis: parts.axis,
            stem_width: parts.stem_width,
            handle_size: parts.handle_size,
        }
    }

    /// Build value-marker layout for markers positioned along a timeline axis.
    pub const fn new(axis: TimelineAxis, stem_width: f32, handle_size: f32) -> Self {
        Self::from_parts(TimelineValueMarkerLayoutParts::new(
            axis,
            stem_width,
            handle_size,
        ))
    }

    /// Project a timeline position and normalized vertical value into marker geometry.
    pub fn marker(self, timeline_value: f32, value: f32) -> Option<VerticalValueMarker> {
        vertical_value_marker(
            self.axis.rect,
            self.axis.x_for_value(timeline_value),
            value,
            self.stem_width,
            self.handle_size,
        )
    }

    /// Project a timeline position without clamping it to the visible horizontal span.
    pub fn marker_unclamped(self, timeline_value: f32, value: f32) -> Option<VerticalValueMarker> {
        vertical_value_marker(
            self.axis.rect,
            self.axis.x_for_value_unclamped(timeline_value),
            value,
            self.stem_width,
            self.handle_size,
        )
    }
}
