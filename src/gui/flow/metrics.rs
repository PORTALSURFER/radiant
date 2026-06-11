/// Geometry policy for a wrapped inline flow.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlowLayoutMetricsParts {
    /// Horizontal gap between adjacent items in one row.
    pub item_gap: f32,
    /// Vertical gap between rows.
    pub line_gap: f32,
    /// Height of one row.
    pub item_height: f32,
}

/// Geometry policy for a wrapped inline flow.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlowLayoutMetrics {
    /// Horizontal gap between adjacent items in one row.
    pub item_gap: f32,
    /// Vertical gap between rows.
    pub line_gap: f32,
    /// Height of one row.
    pub item_height: f32,
}

impl FlowLayoutMetrics {
    /// Construct metrics from named already-resolved logical-pixel values.
    pub const fn from_parts(parts: FlowLayoutMetricsParts) -> Self {
        Self {
            item_gap: parts.item_gap,
            line_gap: parts.line_gap,
            item_height: parts.item_height,
        }
    }

    /// Construct metrics from resolved logical-pixel values.
    pub const fn new(item_gap: f32, line_gap: f32, item_height: f32) -> Self {
        Self::from_parts(FlowLayoutMetricsParts {
            item_gap,
            line_gap,
            item_height,
        })
    }
}

/// Return the total height for a known number of flow rows.
pub fn flow_rows_height(row_count: usize, metrics: FlowLayoutMetrics) -> f32 {
    if row_count == 0 {
        return 0.0;
    }
    row_count as f32 * metrics.item_height.max(0.0)
        + row_count.saturating_sub(1) as f32 * metrics.line_gap.max(0.0)
}
