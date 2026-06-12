/// Integer item viewport for timeline, waveform, list, and canvas ranges.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndexViewport {
    /// First visible item index.
    pub start: usize,
    /// Exclusive end item index.
    pub end: usize,
}

impl IndexViewport {
    /// Build a viewport covering all available items.
    pub fn full(total_items: usize) -> Self {
        Self {
            start: 0,
            end: total_items.max(1),
        }
    }

    /// Return the visible item count, never less than one.
    pub fn visible_items(self) -> usize {
        self.end.saturating_sub(self.start).max(1)
    }

    /// Return the fraction of total items currently visible.
    pub fn visible_fraction(self, total_items: usize) -> f32 {
        self.visible_items() as f32 / total_items.max(1) as f32
    }

    /// Return the scroll offset fraction within the non-visible item span.
    pub fn offset_fraction(self, total_items: usize) -> f32 {
        let total_items = total_items.max(1);
        let free_items = total_items.saturating_sub(self.visible_items());
        if free_items == 0 {
            0.0
        } else {
            self.start as f32 / free_items as f32
        }
    }

    /// Return whether this viewport shows less than the full item span.
    pub fn is_zoomed_in(self, total_items: usize) -> bool {
        self.visible_items() < total_items.max(1)
    }

    /// Clamp this viewport into `total_items` while preserving at least
    /// `min_visible_items` where possible.
    pub fn clamp(self, total_items: usize, min_visible_items: usize) -> Self {
        let total_items = total_items.max(1);
        let min_visible_items = min_visible_items.max(1).min(total_items);
        let visible = self.visible_items().clamp(min_visible_items, total_items);
        let start = self.start.min(total_items.saturating_sub(visible));
        Self {
            start,
            end: start + visible,
        }
    }
}
