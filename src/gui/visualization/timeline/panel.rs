use crate::gui::types::{Point, Rect};

/// Named fields for a standard timeline panel split.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelinePanelLayoutParts {
    /// Full widget or panel bounds.
    pub bounds: Rect,
    /// Width reserved for the left header/track label area.
    pub header_width: f32,
    /// Height reserved for the top ruler area.
    pub ruler_height: f32,
}

impl TimelinePanelLayoutParts {
    /// Build timeline panel layout parts.
    pub const fn new(bounds: Rect, header_width: f32, ruler_height: f32) -> Self {
        Self {
            bounds,
            header_width,
            ruler_height,
        }
    }
}

/// Reusable header, ruler, and lane split for timeline-style editors.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelinePanelLayout {
    /// Left header or track-label area.
    pub header: Rect,
    /// Top ruler area to the right of the header.
    pub ruler: Rect,
    /// Main scrollable or projected lane area below the ruler.
    pub lanes: Rect,
}

impl TimelinePanelLayout {
    /// Build panel layout from named parts.
    pub fn from_parts(parts: TimelinePanelLayoutParts) -> Self {
        let bounds = parts.bounds;
        let header_width = finite_nonnegative(parts.header_width).min(bounds.width().max(0.0));
        let ruler_height = finite_nonnegative(parts.ruler_height).min(bounds.height().max(0.0));
        let split_x = bounds.min.x + header_width;
        let ruler_bottom = bounds.min.y + ruler_height;
        Self {
            header: Rect::from_min_max(bounds.min, Point::new(split_x, bounds.max.y)),
            ruler: Rect::from_min_max(
                Point::new(split_x, bounds.min.y),
                Point::new(bounds.max.x, ruler_bottom),
            ),
            lanes: Rect::from_min_max(Point::new(split_x, ruler_bottom), bounds.max),
        }
    }

    /// Build a standard timeline panel split.
    pub fn new(bounds: Rect, header_width: f32, ruler_height: f32) -> Self {
        Self::from_parts(TimelinePanelLayoutParts::new(
            bounds,
            header_width,
            ruler_height,
        ))
    }
}

fn finite_nonnegative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}
