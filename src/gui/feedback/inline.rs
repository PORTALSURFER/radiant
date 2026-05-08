use crate::gui::types::{Point, Rect};

/// Metrics for compact count-based indicators placed after inline text.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InlineIndicatorMetrics {
    /// Width of each indicator segment.
    pub unit_width: f32,
    /// Height of each indicator segment.
    pub unit_height: f32,
    /// Horizontal gap between adjacent segments.
    pub unit_gap: f32,
    /// Gap between the preceding text and the first indicator segment.
    pub text_gap: f32,
    /// Maximum number of segments to materialize.
    pub max_count: usize,
}

/// Text-relative placement anchor for compact inline indicators.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InlineIndicatorAnchor {
    /// Bounds available to the text and indicator cluster.
    pub content_rect: Rect,
    /// X origin where the preceding text is rendered.
    pub text_origin_x: f32,
    /// Rendered width of the preceding text.
    pub text_width: f32,
    /// Right edge available to the indicator cluster.
    pub right_limit_x: f32,
}

/// Resolved compact indicator segment rects.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InlineIndicatorLayout {
    /// Segment rects, ordered from leading to trailing.
    pub rects: [Rect; 8],
    /// Number of materialized rects in `rects`.
    pub count: usize,
}

/// Return the total width reserved for an inline indicator cluster and text gap.
pub fn inline_indicator_reserved_width(count: usize, metrics: InlineIndicatorMetrics) -> f32 {
    let count = count.min(metrics.max_count).min(8);
    if count == 0 {
        return 0.0;
    }
    let unit_width = metrics.unit_width.max(0.0);
    let unit_gap = metrics.unit_gap.max(0.0);
    (count as f32 * unit_width)
        + ((count.saturating_sub(1)) as f32 * unit_gap)
        + metrics.text_gap.max(0.0)
}

/// Place a compact inline indicator cluster after rendered text.
pub fn inline_indicator_layout(
    anchor: InlineIndicatorAnchor,
    count: usize,
    metrics: InlineIndicatorMetrics,
) -> Option<InlineIndicatorLayout> {
    let count = count.min(metrics.max_count).min(8);
    let content_rect = anchor.content_rect;
    if count == 0 || content_rect.width() <= 0.0 || content_rect.height() <= 0.0 {
        return None;
    }
    let unit_height = metrics
        .unit_height
        .max(0.0)
        .min(content_rect.height().max(1.0));
    let unit_width = metrics
        .unit_width
        .max(0.0)
        .min(content_rect.width().max(1.0));
    if unit_width <= 0.0 || unit_height <= 0.0 {
        return None;
    }
    let unit_gap = metrics.unit_gap.max(0.0);
    let total_width = (count as f32 * unit_width) + ((count.saturating_sub(1)) as f32 * unit_gap);
    let ideal_start_x =
        anchor.text_origin_x + anchor.text_width.max(0.0) + metrics.text_gap.max(0.0);
    let right_limit_x = anchor
        .right_limit_x
        .clamp(content_rect.min.x, content_rect.max.x);
    let max_start_x = (right_limit_x - total_width).max(content_rect.min.x);
    let start_x = ideal_start_x.clamp(content_rect.min.x, max_start_x);
    let min_y = content_rect.min.y + ((content_rect.height() - unit_height) * 0.5).floor();
    let max_y = (min_y + unit_height).min(content_rect.max.y);
    let mut rects = [Rect::from_min_max(content_rect.min, content_rect.min); 8];
    for (index, rect) in rects.iter_mut().enumerate().take(count) {
        let min_x = start_x + index as f32 * (unit_width + unit_gap);
        *rect = Rect::from_min_max(
            Point::new(min_x, min_y),
            Point::new((min_x + unit_width).min(content_rect.max.x), max_y),
        );
    }
    Some(InlineIndicatorLayout { rects, count })
}

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
}
