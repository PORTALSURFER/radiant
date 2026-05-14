use super::{Point, Rect};

impl Rect {
    /// Snap rectangle bounds to a stroke-width grid for even retained borders.
    ///
    /// Tiny rectangles keep their original bounds when snapping would leave too
    /// little room for both stroke edges.
    pub fn stroke_aligned_rect(self, stroke: f32) -> Self {
        let stroke = stroke.max(1.0);
        let snap = |value: f32| (value / stroke).round() * stroke;
        let snapped = Self::from_min_max(
            Point::new(snap(self.min.x), snap(self.min.y)),
            Point::new(snap(self.max.x), snap(self.max.y)),
        );
        if snapped.width() <= stroke * 2.0 || snapped.height() <= stroke * 2.0 {
            self
        } else {
            snapped
        }
    }
}
