use super::IndexViewport;
use super::ratio::finite_unit_or;

impl IndexViewport {
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

    /// Return a viewport panned by the drag delta between two local ratios.
    ///
    /// This is useful for timeline, waveform, canvas, and similar surfaces
    /// where a pointer drag should keep the original anchor item under the
    /// pointer while the current pointer ratio moves inside the visible span.
    pub fn pan_by_visible_ratio_drag(
        self,
        total_items: usize,
        min_visible_items: usize,
        anchor_ratio: f32,
        current_ratio: f32,
    ) -> Self {
        let total_items = total_items.max(1);
        let viewport = self.clamp(total_items, min_visible_items);
        let visible = viewport.visible_items();
        if visible >= total_items {
            return viewport;
        }
        let anchor_ratio = finite_unit_or(anchor_ratio, 0.0);
        let current_ratio = finite_unit_or(current_ratio, anchor_ratio);
        let delta = ((current_ratio - anchor_ratio) * visible as f32).round() as isize;
        let start = viewport.start.saturating_add_signed(-delta);
        Self {
            start,
            end: start + visible,
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
