use super::ratio::{RATIO_EPSILON, finite_unit, finite_unit_or};
use super::{IndexViewport, NormalizedRange};

impl IndexViewport {
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

    /// Project and clip an absolute normalized range into this viewport.
    ///
    /// Returns an ordered local normalized range, or `None` when the source
    /// range does not intersect the viewport.
    pub fn visible_normalized_range(
        self,
        total_items: usize,
        range: NormalizedRange,
    ) -> Option<NormalizedRange> {
        self.visible_range_from_absolute(total_items, range.start_fraction(), range.end_fraction())
            .map(|(start, end)| NormalizedRange::from_fractions(start, end))
    }
}
