//! Normalized interval primitives for reusable UI models.

mod index_viewport;
mod scrollbar;
mod viewport;

pub use index_viewport::IndexViewport;
pub use scrollbar::{
    NormalizedScrollbar, NormalizedScrollbarRequest, normalized_scrollbar_center_at_point,
    normalized_scrollbar_center_for_pointer, normalized_scrollbar_thumb_offset_at_point,
    normalized_scrollbar_thumb_ratio_at_point, resolve_normalized_scrollbar,
};
pub use viewport::{NormalizedPixelSnap, NormalizedViewport, NormalizedViewportParts};

/// Normalized range with deterministic milli, micro, and nano projections.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NormalizedRange {
    /// Start position in normalized milli-units.
    pub start_milli: u16,
    /// End position in normalized milli-units.
    pub end_milli: u16,
    /// Start position in normalized micro-units (`0..=1_000_000`).
    pub start_micros: u32,
    /// End position in normalized micro-units (`0..=1_000_000`).
    pub end_micros: u32,
    /// Start position in normalized nanounits (`0..=1_000_000_000`).
    pub start_nanos: u32,
    /// End position in normalized nanounits (`0..=1_000_000_000`).
    pub end_nanos: u32,
}

/// Named milli-unit bounds for constructing a normalized range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NormalizedRangeParts {
    /// Start position in normalized milli-units.
    pub start_milli: u16,
    /// End position in normalized milli-units.
    pub end_milli: u16,
}

impl NormalizedRange {
    /// Build a normalized range from named milli-unit bounds.
    pub fn from_parts(parts: NormalizedRangeParts) -> Self {
        Self::from_micros(
            u32::from(parts.start_milli.min(1000)) * 1000,
            u32::from(parts.end_milli.min(1000)) * 1000,
        )
    }

    /// Build a normalized range, clamping bounds to `0..=1000` and ordering them.
    pub fn new(start_milli: u16, end_milli: u16) -> Self {
        Self::from_parts(NormalizedRangeParts {
            start_milli,
            end_milli,
        })
    }

    /// Build a normalized range from micro precision while preserving ordered milli mirrors.
    pub fn from_micros(start_micros: u32, end_micros: u32) -> Self {
        Self::from_nanos(
            start_micros.min(1_000_000).saturating_mul(1000),
            end_micros.min(1_000_000).saturating_mul(1000),
        )
    }

    /// Build a normalized range from nano precision while preserving ordered mirrors.
    pub fn from_nanos(start_nanos: u32, end_nanos: u32) -> Self {
        let start = start_nanos.min(1_000_000_000);
        let end = end_nanos.min(1_000_000_000);
        let ordered_start = start.min(end);
        let ordered_end = end.max(start);
        Self {
            start_milli: nanos_to_milli(ordered_start),
            end_milli: nanos_to_milli(ordered_end),
            start_micros: nanos_to_micros(ordered_start),
            end_micros: nanos_to_micros(ordered_end),
            start_nanos: ordered_start,
            end_nanos: ordered_end,
        }
    }
}

fn micros_to_milli(value_micros: u32) -> u16 {
    ((value_micros.min(1_000_000) + 500) / 1000) as u16
}

fn nanos_to_micros(value_nanos: u32) -> u32 {
    ((value_nanos.min(1_000_000_000) + 500) / 1000).min(1_000_000)
}

fn nanos_to_milli(value_nanos: u32) -> u16 {
    micros_to_milli(nanos_to_micros(value_nanos))
}

pub(crate) fn micros_matches_projected_nanos(value_micros: u32, value_nanos: u32) -> bool {
    let projected_micros = nanos_to_micros(value_nanos);
    projected_micros.abs_diff(value_micros.min(1_000_000)) <= 1
}

#[cfg(test)]
mod tests {
    use super::{
        NormalizedPixelSnap, NormalizedRange, NormalizedRangeParts, NormalizedScrollbar,
        NormalizedScrollbarRequest, NormalizedViewport, NormalizedViewportParts,
        normalized_scrollbar_center_at_point, normalized_scrollbar_center_for_pointer,
        normalized_scrollbar_thumb_offset_at_point, normalized_scrollbar_thumb_ratio_at_point,
        resolve_normalized_scrollbar,
    };
    use crate::gui::types::{Point, Rect};

    #[test]
    fn normalized_range_orders_and_clamps_nano_bounds() {
        let range = NormalizedRange::from_nanos(1_200_000_000, 125_600_000);

        assert_eq!(range.start_nanos, 125_600_000);
        assert_eq!(range.end_nanos, 1_000_000_000);
        assert_eq!(range.start_micros, 125_600);
        assert_eq!(range.end_micros, 1_000_000);
        assert_eq!(range.start_milli, 126);
        assert_eq!(range.end_milli, 1000);
    }

    #[test]
    fn normalized_range_from_milli_preserves_mirror_fields() {
        let range = NormalizedRange::new(800, 200);

        assert_eq!(range.start_milli, 200);
        assert_eq!(range.end_milli, 800);
        assert_eq!(range.start_micros, 200_000);
        assert_eq!(range.end_micros, 800_000);
        assert_eq!(range.start_nanos, 200_000_000);
        assert_eq!(range.end_nanos, 800_000_000);
    }

    #[test]
    fn normalized_range_supports_named_parts_construction() {
        let range = NormalizedRange::from_parts(NormalizedRangeParts {
            start_milli: 1_200,
            end_milli: 250,
        });

        assert_eq!(range.start_milli, 250);
        assert_eq!(range.end_milli, 1000);
        assert_eq!(range.start_micros, 250_000);
        assert_eq!(range.end_micros, 1_000_000);
    }

    #[test]
    fn normalized_viewport_projects_absolute_ratios_into_rect() {
        let rect = Rect::from_min_max(Point::new(10.0, 0.0), Point::new(110.0, 20.0));
        let viewport = NormalizedViewport::from_micros(250_000, 750_000);

        assert_eq!(
            viewport.x_for_ratio(rect, 0.25, NormalizedPixelSnap::Nearest),
            10.0
        );
        assert_eq!(
            viewport.x_for_ratio(rect, 0.5, NormalizedPixelSnap::Nearest),
            60.0
        );
        assert_eq!(
            viewport.x_for_ratio(rect, 0.75, NormalizedPixelSnap::Nearest),
            110.0
        );
    }

    #[test]
    fn normalized_viewport_projection_sanitizes_invalid_inputs() {
        let rect = Rect::from_min_max(Point::new(10.0, 0.0), Point::new(110.0, 20.0));
        let viewport = NormalizedViewport::from_micros(250_000, 750_000);

        assert_eq!(
            viewport.x_for_ratio(rect, f64::NAN, NormalizedPixelSnap::Nearest),
            10.0
        );
        assert_eq!(
            viewport.x_for_ratio(
                Rect::from_min_max(Point::new(f32::NAN, 0.0), Point::new(110.0, 20.0)),
                0.5,
                NormalizedPixelSnap::Nearest
            ),
            0.0
        );
        assert_eq!(
            viewport.x_for_ratio(
                Rect::from_min_max(Point::new(10.0, 0.0), Point::new(f32::INFINITY, 20.0)),
                0.5,
                NormalizedPixelSnap::Nearest
            ),
            0.0
        );
    }

    #[test]
    fn normalized_viewport_uses_nanos_only_when_they_match_micro_mirrors() {
        let viewport =
            NormalizedViewport::from_bounds(500_123, 500_124, Some(500_123_000), Some(500_123_200));

        assert_eq!(viewport.start_ratio, 0.500123);
        assert!((viewport.width_ratio - 0.0000002).abs() < f64::EPSILON);

        let fallback =
            NormalizedViewport::from_bounds(500_123, 500_124, Some(400_000_000), Some(400_100_000));

        assert_eq!(fallback, NormalizedViewport::from_micros(500_123, 500_124));
    }

    #[test]
    fn normalized_viewport_supports_named_parts_construction() {
        let viewport = NormalizedViewport::from_parts(NormalizedViewportParts {
            start_micros: 500_123,
            end_micros: 500_124,
            start_nanos: Some(500_123_000),
            end_nanos: Some(500_123_200),
        });

        assert_eq!(viewport.start_ratio, 0.500123);
        assert!((viewport.width_ratio - 0.0000002).abs() < f64::EPSILON);
    }

    #[test]
    fn normalized_scrollbar_maps_viewport_to_horizontal_thumb() {
        let track = Rect::from_min_max(Point::new(10.0, 40.0), Point::new(210.0, 44.0));
        let scrollbar = resolve_normalized_scrollbar(NormalizedScrollbarRequest {
            track,
            start_micros: 250_000,
            end_micros: 500_000,
            min_thumb_width: 28.0,
        })
        .expect("zoomed normalized viewport should show a scrollbar");

        assert_eq!(scrollbar.track, track);
        assert_eq!(scrollbar.thumb.width(), 50.0);
        assert_eq!(scrollbar.thumb.min.x, 60.0);
        assert_eq!(
            resolve_normalized_scrollbar(NormalizedScrollbarRequest {
                track,
                start_micros: 0,
                end_micros: 1_000_000,
                min_thumb_width: 28.0,
            }),
            None
        );
    }

    #[test]
    fn normalized_scrollbar_resolves_thumb_pointer_state() {
        let scrollbar = NormalizedScrollbar {
            track: Rect::from_min_max(Point::new(10.0, 40.0), Point::new(210.0, 44.0)),
            thumb: Rect::from_min_max(Point::new(60.0, 40.0), Point::new(110.0, 44.0)),
        };

        assert_eq!(
            normalized_scrollbar_thumb_offset_at_point(scrollbar, Point::new(85.0, 42.0)),
            Some(25.0)
        );
        assert_eq!(
            normalized_scrollbar_thumb_ratio_at_point(scrollbar, Point::new(85.0, 42.0)),
            Some(0.5)
        );
        assert_eq!(
            normalized_scrollbar_thumb_offset_at_point(scrollbar, Point::new(85.0, 50.0)),
            None
        );
    }

    #[test]
    fn normalized_scrollbar_resolves_drag_and_track_click_center() {
        let scrollbar = NormalizedScrollbar {
            track: Rect::from_min_max(Point::new(10.0, 40.0), Point::new(210.0, 44.0)),
            thumb: Rect::from_min_max(Point::new(60.0, 40.0), Point::new(110.0, 44.0)),
        };

        assert_eq!(
            normalized_scrollbar_center_for_pointer(scrollbar, 250_000, 500_000, 210.0, 0.0),
            Some(875_000)
        );
        assert_eq!(
            normalized_scrollbar_center_at_point(
                scrollbar,
                250_000,
                500_000,
                Point::new(185.0, 42.0)
            ),
            Some(875_000)
        );
        assert_eq!(
            normalized_scrollbar_center_at_point(
                scrollbar,
                250_000,
                500_000,
                Point::new(85.0, 42.0)
            ),
            None
        );
    }
}
