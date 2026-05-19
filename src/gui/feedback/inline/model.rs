use crate::gui::types::Rect;

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
