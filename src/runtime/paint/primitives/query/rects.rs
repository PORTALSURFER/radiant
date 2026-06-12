use super::super::{PaintPrimitive, SurfacePaintPlan};
use crate::gui::types::Rect;

impl SurfacePaintPlan {
    /// Iterate over rectangular regions directly carried by primitives in paint order.
    ///
    /// Batched rectangle primitives contribute every carried rectangle, while
    /// [`PaintPrimitive::rect`] remains the first-rectangle anchor helper for
    /// overlay placement code.
    pub fn rects(&self) -> impl Iterator<Item = Rect> + '_ {
        self.primitives.iter().flat_map(PaintPrimitive::rects)
    }

    /// Return whether any rectangle-bearing primitive matches `predicate`.
    pub fn contains_rect_matching(&self, predicate: impl FnMut(Rect) -> bool) -> bool {
        self.rects().any(predicate)
    }

    /// Iterate over rectangular regions carried by non-clip paint primitives.
    pub fn paint_rects(&self) -> impl Iterator<Item = Rect> + '_ {
        self.paint_primitives().flat_map(PaintPrimitive::rects)
    }

    /// Return whether any non-clip paint rectangle matches `predicate`.
    pub fn contains_paint_rect_matching(&self, predicate: impl FnMut(Rect) -> bool) -> bool {
        self.paint_rects().any(predicate)
    }
}
