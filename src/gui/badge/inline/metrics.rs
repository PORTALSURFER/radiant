/// Explicit geometry tokens used to build inline badge metrics.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InlineBadgeMetricsParts {
    /// Font size used for label measurement and vertical text placement.
    pub font_size: f32,
    /// Horizontal inset inside each badge.
    pub padding_x: f32,
    /// Vertical inset inside each badge.
    pub padding_y: f32,
    /// Horizontal gap between adjacent badges.
    pub badge_gap: f32,
    /// Gap between the host item label and the badge cluster.
    pub cluster_gap: f32,
    /// Minimum desired badge height before clamping to the available row height.
    pub min_height: f32,
}

/// Layout metrics for a compact inline badge cluster.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InlineBadgeMetrics {
    /// Font size used for label measurement and vertical text placement.
    pub font_size: f32,
    /// Horizontal inset inside each badge.
    pub padding_x: f32,
    /// Vertical inset inside each badge.
    pub padding_y: f32,
    /// Horizontal gap between adjacent badges.
    pub badge_gap: f32,
    /// Gap between the host item label and the badge cluster.
    pub cluster_gap: f32,
    /// Minimum desired badge height before clamping to the available row height.
    pub min_height: f32,
}

impl InlineBadgeMetrics {
    /// Construct metrics from named already-resolved geometry tokens.
    pub fn from_parts(parts: InlineBadgeMetricsParts) -> Self {
        Self {
            font_size: parts.font_size,
            padding_x: parts.padding_x,
            padding_y: parts.padding_y,
            badge_gap: parts.badge_gap,
            cluster_gap: parts.cluster_gap,
            min_height: parts.min_height,
        }
    }

    /// Construct metrics from already-resolved geometry tokens.
    pub fn new(
        font_size: f32,
        padding_x: f32,
        padding_y: f32,
        badge_gap: f32,
        cluster_gap: f32,
        min_height: f32,
    ) -> Self {
        Self::from_parts(InlineBadgeMetricsParts {
            font_size,
            padding_x,
            padding_y,
            badge_gap,
            cluster_gap,
            min_height,
        })
    }
}
