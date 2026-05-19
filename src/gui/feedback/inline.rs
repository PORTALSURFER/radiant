mod geometry;
mod model;
mod sanitize;

pub use geometry::{inline_indicator_layout, inline_indicator_reserved_width};
pub use model::{InlineIndicatorAnchor, InlineIndicatorLayout, InlineIndicatorMetrics};

#[cfg(test)]
mod tests {
    use super::{
        InlineIndicatorAnchor, InlineIndicatorMetrics, inline_indicator_layout,
        inline_indicator_reserved_width,
    };
    use crate::gui::types::{Point, Rect};

    #[test]
    fn inline_indicator_reserved_width_includes_text_gap_and_unit_gaps() {
        let metrics = InlineIndicatorMetrics {
            unit_width: 6.0,
            unit_height: 5.0,
            unit_gap: 2.0,
            text_gap: 4.0,
            max_count: 3,
        };

        assert_eq!(inline_indicator_reserved_width(0, metrics), 0.0);
        assert_eq!(inline_indicator_reserved_width(2, metrics), 18.0);
        assert_eq!(inline_indicator_reserved_width(9, metrics), 26.0);
    }

    #[test]
    fn inline_indicator_reserved_width_sanitizes_nonfinite_metrics() {
        let metrics = InlineIndicatorMetrics {
            unit_width: 6.0,
            unit_height: 5.0,
            unit_gap: f32::NAN,
            text_gap: f32::INFINITY,
            max_count: 3,
        };

        assert_eq!(inline_indicator_reserved_width(2, metrics), 12.0);
    }

    #[test]
    fn inline_indicator_layout_places_segments_after_text_and_clamps_to_right_limit() {
        let metrics = InlineIndicatorMetrics {
            unit_width: 6.0,
            unit_height: 5.0,
            unit_gap: 2.0,
            text_gap: 4.0,
            max_count: 3,
        };
        let anchor = InlineIndicatorAnchor {
            content_rect: Rect::from_min_max(Point::new(10.0, 20.0), Point::new(60.0, 30.0)),
            text_origin_x: 16.0,
            text_width: 14.0,
            right_limit_x: 44.0,
        };

        let layout = inline_indicator_layout(anchor, 3, metrics).expect("indicator layout");

        assert_eq!(layout.count, 3);
        assert_eq!(
            &layout.rects[..layout.count],
            &[
                Rect::from_min_max(Point::new(22.0, 22.0), Point::new(28.0, 27.0)),
                Rect::from_min_max(Point::new(30.0, 22.0), Point::new(36.0, 27.0)),
                Rect::from_min_max(Point::new(38.0, 22.0), Point::new(44.0, 27.0)),
            ]
        );
    }

    #[test]
    fn inline_indicator_layout_sanitizes_text_measurement_inputs() {
        let metrics = InlineIndicatorMetrics {
            unit_width: 5.0,
            unit_height: 4.0,
            unit_gap: f32::NAN,
            text_gap: f32::INFINITY,
            max_count: 3,
        };
        let anchor = InlineIndicatorAnchor {
            content_rect: Rect::from_min_max(Point::new(10.0, 20.0), Point::new(40.0, 30.0)),
            text_origin_x: f32::NAN,
            text_width: f32::INFINITY,
            right_limit_x: f32::NAN,
        };

        let layout = inline_indicator_layout(anchor, 3, metrics).expect("indicator layout");

        assert_eq!(layout.count, 3);
        assert_eq!(
            &layout.rects[..layout.count],
            &[
                Rect::from_min_max(Point::new(10.0, 23.0), Point::new(15.0, 27.0)),
                Rect::from_min_max(Point::new(15.0, 23.0), Point::new(20.0, 27.0)),
                Rect::from_min_max(Point::new(20.0, 23.0), Point::new(25.0, 27.0)),
            ]
        );
    }

    #[test]
    fn inline_indicator_layout_rejects_nonfinite_content_rect() {
        let metrics = InlineIndicatorMetrics {
            unit_width: 5.0,
            unit_height: 4.0,
            unit_gap: 1.0,
            text_gap: 2.0,
            max_count: 3,
        };
        let anchor = InlineIndicatorAnchor {
            content_rect: Rect::from_min_max(Point::new(10.0, 20.0), Point::new(f32::NAN, 30.0)),
            text_origin_x: 10.0,
            text_width: 5.0,
            right_limit_x: 30.0,
        };

        assert_eq!(inline_indicator_layout(anchor, 3, metrics), None);
    }
}
