use super::metrics::{FlowLayoutMetrics, flow_rows_height};
/// Geometry policy for a bounded wrapped inline field.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlowFieldMetricsParts {
    /// Row-packing metrics for the wrapped inline items.
    pub flow: FlowLayoutMetrics,
    /// Width reserved by field chrome outside the packed content area.
    pub horizontal_chrome: f32,
    /// Height reserved by field chrome outside the packed content area.
    pub vertical_chrome: f32,
    /// Minimum usable content width after chrome is removed.
    pub min_content_width: f32,
    /// Maximum number of rows shown before the host should use scrolling.
    pub max_visible_rows: usize,
}

/// Geometry policy for a bounded wrapped inline field.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlowFieldMetrics {
    /// Row-packing metrics for the wrapped inline items.
    pub flow: FlowLayoutMetrics,
    /// Width reserved by field chrome outside the packed content area.
    pub horizontal_chrome: f32,
    /// Height reserved by field chrome outside the packed content area.
    pub vertical_chrome: f32,
    /// Minimum usable content width after chrome is removed.
    pub min_content_width: f32,
    /// Maximum number of rows shown before the host should use scrolling.
    pub max_visible_rows: usize,
}

/// Resolved geometry summary for a bounded wrapped inline field.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlowFieldLayout {
    /// Usable row-packing width inside the field chrome.
    pub content_width: f32,
    /// Number of rows produced by caller-owned flow packing.
    pub row_count: usize,
    /// Number of rows visible before scrolling is needed.
    pub visible_row_count: usize,
    /// Visible content height before adding field chrome.
    pub content_height: f32,
    /// Visible field height including field chrome.
    pub field_height: f32,
    /// Whether the packed rows exceed the visible row limit.
    pub requires_scroll: bool,
}

impl FlowFieldMetrics {
    /// Construct field metrics from named already-resolved logical-pixel values.
    pub const fn from_parts(parts: FlowFieldMetricsParts) -> Self {
        Self {
            flow: parts.flow,
            horizontal_chrome: parts.horizontal_chrome,
            vertical_chrome: parts.vertical_chrome,
            min_content_width: parts.min_content_width,
            max_visible_rows: parts.max_visible_rows,
        }
    }

    /// Construct field metrics from resolved logical-pixel values.
    pub const fn new(
        flow: FlowLayoutMetrics,
        horizontal_chrome: f32,
        vertical_chrome: f32,
        min_content_width: f32,
        max_visible_rows: usize,
    ) -> Self {
        Self::from_parts(FlowFieldMetricsParts {
            flow,
            horizontal_chrome,
            vertical_chrome,
            min_content_width,
            max_visible_rows,
        })
    }

    /// Return the available row-packing width inside a containing field.
    pub fn content_width(self, container_width: f32) -> f32 {
        (container_width - self.horizontal_chrome.max(0.0)).max(self.min_content_width.max(0.0))
    }

    /// Return the resolved field layout for a containing width and row count.
    pub fn layout(self, container_width: f32, row_count: usize) -> FlowFieldLayout {
        self.layout_for_content_width(self.content_width(container_width), row_count)
    }

    /// Return the resolved field layout for an already-computed content width.
    pub fn layout_for_content_width(self, content_width: f32, row_count: usize) -> FlowFieldLayout {
        let visible_row_count = self.visible_row_count(row_count);
        let content_height = flow_rows_height(visible_row_count, self.flow);
        FlowFieldLayout {
            content_width: content_width.max(self.min_content_width.max(0.0)),
            row_count,
            visible_row_count,
            content_height,
            field_height: content_height + self.vertical_chrome.max(0.0),
            requires_scroll: self.requires_scroll(row_count),
        }
    }

    /// Return the row count visible before the field should scroll.
    pub fn visible_row_count(self, row_count: usize) -> usize {
        row_count.clamp(1, self.max_visible_rows.max(1))
    }

    /// Return the visible content height for the supplied packed row count.
    pub fn visible_rows_height(self, row_count: usize) -> f32 {
        flow_rows_height(self.visible_row_count(row_count), self.flow)
    }

    /// Return the full field height for the supplied packed row count.
    pub fn visible_field_height(self, row_count: usize) -> f32 {
        self.visible_rows_height(row_count) + self.vertical_chrome.max(0.0)
    }

    /// Return whether the supplied packed row count exceeds the visible limit.
    pub fn requires_scroll(self, row_count: usize) -> bool {
        row_count > self.max_visible_rows.max(1)
    }
}

/// Return the visible height for a capped wrapped flow plus surrounding chrome.
///
/// This is useful for chip, pill, tag, recipient, and token editors that grow
/// with wrapped rows until a maximum visible row count, then switch the content
/// area to scrolling. `min_visible_rows` keeps empty editors tall enough for
/// their trailing input or placeholder row.
pub fn capped_flow_rows_height(
    row_count: usize,
    min_visible_rows: usize,
    max_visible_rows: usize,
    chrome_height: f32,
    metrics: FlowLayoutMetrics,
) -> f32 {
    let min_visible_rows = min_visible_rows.min(max_visible_rows);
    let visible_rows = row_count.clamp(min_visible_rows, max_visible_rows);
    flow_rows_height(visible_rows, metrics) + chrome_height.max(0.0)
}
