use super::{IndexViewport, NormalizedRange};

/// Viewport operations bound to one item domain.
///
/// This keeps application code from repeatedly threading the same item count
/// and minimum visible span through every projection, zoom, pan, and scrollbar
/// calculation. It is useful for waveform, timeline, image-strip, document, and
/// other canvas-like surfaces backed by integer item ranges.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndexViewportScope {
    viewport: IndexViewport,
    total_items: usize,
    min_visible_items: usize,
}

impl IndexViewportScope {
    /// Bind a viewport to one item domain and clamp it into that domain.
    pub fn new(viewport: IndexViewport, total_items: usize, min_visible_items: usize) -> Self {
        let total_items = total_items.max(1);
        let min_visible_items = min_visible_items.max(1).min(total_items);
        Self {
            viewport: viewport.clamp(total_items, min_visible_items),
            total_items,
            min_visible_items,
        }
    }

    /// Return the clamped viewport.
    pub fn viewport(self) -> IndexViewport {
        self.viewport
    }

    /// Return the total item count, never less than one.
    pub fn total_items(self) -> usize {
        self.total_items
    }

    /// Return the minimum visible item count, clamped into this domain.
    pub fn min_visible_items(self) -> usize {
        self.min_visible_items
    }

    /// Return the visible item count, never less than one.
    pub fn visible_items(self) -> usize {
        self.viewport.visible_items()
    }

    /// Return the fraction of total items currently visible.
    pub fn visible_fraction(self) -> f32 {
        self.viewport.visible_fraction(self.total_items)
    }

    /// Return the scroll offset fraction within the non-visible item span.
    pub fn offset_fraction(self) -> f32 {
        self.viewport.offset_fraction(self.total_items)
    }

    /// Return whether this viewport shows less than the full item span.
    pub fn is_zoomed_in(self) -> bool {
        self.viewport.is_zoomed_in(self.total_items)
    }

    /// Convert a local visible ratio into an absolute normalized ratio.
    pub fn absolute_ratio_from_visible(self, visible_ratio: f32) -> f32 {
        self.viewport.absolute_ratio_from_visible(
            self.total_items,
            self.min_visible_items,
            visible_ratio,
        )
    }

    /// Convert an absolute normalized ratio into a local visible ratio.
    ///
    /// Returns `None` when the absolute ratio falls outside this viewport.
    pub fn visible_ratio_from_absolute(self, absolute_ratio: f32) -> Option<f32> {
        self.viewport
            .visible_ratio_from_absolute(self.total_items, absolute_ratio)
    }

    /// Project and clip an absolute normalized range into this viewport.
    ///
    /// The returned pair is ordered and expressed in local visible ratios.
    /// Returns `None` when the range does not intersect the viewport.
    pub fn visible_range_from_absolute(
        self,
        start_ratio: f32,
        end_ratio: f32,
    ) -> Option<(f32, f32)> {
        self.viewport
            .visible_range_from_absolute(self.total_items, start_ratio, end_ratio)
    }

    /// Project and clip an absolute normalized range into this viewport.
    ///
    /// Returns an ordered local normalized range, or `None` when the source
    /// range does not intersect the viewport.
    pub fn visible_normalized_range(self, range: NormalizedRange) -> Option<NormalizedRange> {
        self.viewport
            .visible_normalized_range(self.total_items, range)
    }

    /// Return a viewport zoomed by `factor` around a local anchor ratio.
    pub fn zoom_around_anchor(self, factor: f32, anchor_ratio: f32) -> IndexViewport {
        self.viewport.zoom_around_anchor(
            self.total_items,
            self.min_visible_items,
            factor,
            anchor_ratio,
        )
    }

    /// Return a viewport panned by a fraction of its current visible span.
    pub fn pan_by_visible_fraction(self, fraction: f32) -> IndexViewport {
        self.viewport
            .pan_by_visible_fraction(self.total_items, self.min_visible_items, fraction)
    }

    /// Return a viewport panned by the drag delta between two local ratios.
    pub fn pan_by_visible_ratio_drag(self, anchor_ratio: f32, current_ratio: f32) -> IndexViewport {
        self.viewport.pan_by_visible_ratio_drag(
            self.total_items,
            self.min_visible_items,
            anchor_ratio,
            current_ratio,
        )
    }

    /// Return a viewport moved to an offset fraction while preserving visible size.
    pub fn with_offset_fraction(self, offset_fraction: f32) -> IndexViewport {
        self.viewport.with_offset_fraction(
            self.total_items,
            self.min_visible_items,
            offset_fraction,
        )
    }
}
