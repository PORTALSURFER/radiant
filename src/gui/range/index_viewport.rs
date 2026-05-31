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

    /// Convert a local visible ratio into an absolute normalized ratio.
    pub fn absolute_ratio_from_visible(
        self,
        total_items: usize,
        min_visible_items: usize,
        visible_ratio: f32,
    ) -> f32 {
        let total_items = total_items.max(1);
        let viewport = self.clamp(total_items, min_visible_items);
        let visible_ratio = finite_unit_or(visible_ratio, 0.0);
        let item = viewport.start as f64 + viewport.visible_items() as f64 * visible_ratio as f64;
        ((item / total_items as f64) as f32).clamp(0.0, 1.0)
    }

    /// Convert an absolute normalized ratio into a local visible ratio.
    ///
    /// Returns `None` when the absolute ratio falls outside this viewport.
    pub fn visible_ratio_from_absolute(
        self,
        total_items: usize,
        absolute_ratio: f32,
    ) -> Option<f32> {
        let absolute_ratio = finite_unit(absolute_ratio)?;
        let total_items = total_items.max(1) as f64;
        let item = f64::from(absolute_ratio) * total_items;
        let visible_start = self.start as f64;
        let visible_width = self.visible_items() as f64;
        let visible_ratio = (item - visible_start) / visible_width.max(1.0);
        if !(-RATIO_EPSILON..=1.0 + RATIO_EPSILON).contains(&visible_ratio) {
            return None;
        }
        Some(visible_ratio.clamp(0.0, 1.0) as f32)
    }

    /// Project and clip an absolute normalized range into this viewport.
    ///
    /// The returned pair is ordered and expressed in local visible ratios.
    /// Returns `None` when the range does not intersect the viewport.
    pub fn visible_range_from_absolute(
        self,
        total_items: usize,
        start_ratio: f32,
        end_ratio: f32,
    ) -> Option<(f32, f32)> {
        let start_ratio = finite_unit(start_ratio)?;
        let end_ratio = finite_unit(end_ratio)?;
        let total_items = total_items.max(1) as f64;
        let visible_start = self.start as f64;
        let visible_end = self.end as f64;
        let visible_width = self.visible_items() as f64;
        let start_item = f64::from(start_ratio) * total_items;
        let end_item = f64::from(end_ratio) * total_items;
        let left = start_item.min(end_item).max(visible_start);
        let right = start_item.max(end_item).min(visible_end);
        if right <= left {
            return None;
        }
        Some((
            ((left - visible_start) / visible_width.max(1.0)).clamp(0.0, 1.0) as f32,
            ((right - visible_start) / visible_width.max(1.0)).clamp(0.0, 1.0) as f32,
        ))
    }

    /// Return a viewport zoomed by `factor` around a local anchor ratio.
    pub fn zoom_around_anchor(
        self,
        total_items: usize,
        min_visible_items: usize,
        factor: f32,
        anchor_ratio: f32,
    ) -> Self {
        let total_items = total_items.max(1);
        let viewport = self.clamp(total_items, min_visible_items);
        let anchor_ratio = finite_unit_or(anchor_ratio, 0.5);
        let factor = if factor.is_finite() && factor > f32::EPSILON {
            factor
        } else {
            1.0
        };
        let anchor_item = viewport.start as f32 + viewport.visible_items() as f32 * anchor_ratio;
        let min_visible_items = min_visible_items.max(1).min(total_items);
        let next_visible = ((viewport.visible_items() as f32) * factor)
            .round()
            .clamp(min_visible_items as f32, total_items as f32)
            as usize;
        let start = (anchor_item - next_visible as f32 * anchor_ratio)
            .round()
            .max(0.0) as usize;
        Self {
            start,
            end: start + next_visible,
        }
        .clamp(total_items, min_visible_items)
    }

    /// Return a viewport panned by a fraction of its current visible span.
    pub fn pan_by_visible_fraction(
        self,
        total_items: usize,
        min_visible_items: usize,
        fraction: f32,
    ) -> Self {
        let total_items = total_items.max(1);
        let viewport = self.clamp(total_items, min_visible_items);
        let fraction = if fraction.is_finite() { fraction } else { 0.0 };
        let delta = (viewport.visible_items() as f32 * fraction).round() as isize;
        let start = viewport.start.saturating_add_signed(delta);
        Self {
            start,
            end: start + viewport.visible_items(),
        }
        .clamp(total_items, min_visible_items)
    }

    /// Return a viewport moved to an offset fraction while preserving visible size.
    pub fn with_offset_fraction(
        self,
        total_items: usize,
        min_visible_items: usize,
        offset_fraction: f32,
    ) -> Self {
        let total_items = total_items.max(1);
        let viewport = self.clamp(total_items, min_visible_items);
        let visible = viewport.visible_items();
        let free_items = total_items.saturating_sub(visible);
        let offset_fraction = finite_unit_or(offset_fraction, 0.0);
        let start = (free_items as f32 * offset_fraction).round() as usize;
        Self {
            start,
            end: start + visible,
        }
        .clamp(total_items, min_visible_items)
    }
}

fn finite_unit_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        fallback.clamp(0.0, 1.0)
    }
}

fn finite_unit(value: f32) -> Option<f32> {
    value.is_finite().then(|| value.clamp(0.0, 1.0))
}

const RATIO_EPSILON: f64 = 0.000_001;

#[cfg(test)]
#[path = "index_viewport/tests.rs"]
mod tests;
