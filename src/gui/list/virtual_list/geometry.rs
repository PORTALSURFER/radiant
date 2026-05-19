use super::item::MaterializedVirtualListItem;
use crate::gui::types::Point;

/// Axis-aligned metrics for a stacked virtual list.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VirtualListStackMetrics {
    /// Logical item extent on the scrolling axis.
    pub item_extent: f32,
    /// Logical gap between adjacent items.
    pub item_gap: f32,
    /// Optional cap for visible items in one viewport.
    pub max_viewport_len: Option<usize>,
}

/// Named fields for constructing stacked virtual-list metrics.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VirtualListStackMetricsParts {
    /// Logical item extent on the scrolling axis.
    pub item_extent: f32,
    /// Logical gap between adjacent items.
    pub item_gap: f32,
    /// Optional cap for visible items in one viewport.
    pub max_viewport_len: Option<usize>,
}

impl VirtualListStackMetrics {
    /// Build normalized stacked-list metrics from named parts.
    pub fn from_parts(parts: VirtualListStackMetricsParts) -> Self {
        Self {
            item_extent: parts.item_extent.max(1.0),
            item_gap: parts.item_gap.max(0.0),
            max_viewport_len: parts.max_viewport_len.map(|len| len.max(1)),
        }
    }

    /// Build normalized stacked-list metrics.
    pub fn new(item_extent: f32, item_gap: f32) -> Self {
        Self::from_parts(VirtualListStackMetricsParts {
            item_extent,
            item_gap,
            max_viewport_len: None,
        })
    }

    /// Apply a maximum viewport length cap.
    pub fn with_max_viewport_len(mut self, max_viewport_len: usize) -> Self {
        self.max_viewport_len = Some(max_viewport_len.max(1));
        self
    }

    /// Return the scrolling-axis stride between adjacent items.
    pub fn stride(self) -> f32 {
        (self.item_extent + self.item_gap).max(1.0)
    }
}

/// Resolve the number of stacked items visible in a viewport extent.
pub fn virtual_list_viewport_len_for_extent(
    viewport_extent: f32,
    metrics: VirtualListStackMetrics,
) -> usize {
    let geometric = ((viewport_extent.max(0.0) + metrics.item_gap) / metrics.stride())
        .floor()
        .max(1.0) as usize;
    metrics
        .max_viewport_len
        .map_or(geometric, |limit| geometric.min(limit))
        .max(1)
}

/// Resolve one item index from stacked virtual-list row geometry in O(1).
pub fn virtual_list_stacked_item_at_point(
    items: &[MaterializedVirtualListItem],
    point: Point,
) -> Option<usize> {
    let first = items.first()?;
    if point.x < first.rect.min.x || point.x > first.rect.max.x {
        return None;
    }

    let item_extent = first.rect.height().max(0.0);
    let stride = if items.len() > 1 {
        (items[1].rect.min.y - first.rect.min.y).max(1.0)
    } else {
        item_extent.max(1.0)
    };
    let relative_y = point.y - first.rect.min.y;
    if relative_y < 0.0 {
        return None;
    }

    let candidate = (relative_y / stride).floor() as usize;
    if candidate >= items.len() {
        return None;
    }
    let item_start = first.rect.min.y + (candidate as f32 * stride);
    let item_end = item_start + item_extent;
    if candidate > 0 {
        let previous_end = item_start - stride + item_extent;
        if point.y <= previous_end {
            return Some(items[candidate - 1].index);
        }
    }
    ((point.y >= item_start) && (point.y <= item_end)).then_some(items[candidate].index)
}
