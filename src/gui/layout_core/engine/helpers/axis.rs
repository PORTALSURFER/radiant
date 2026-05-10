//! Internal main/cross-axis helpers for layout algorithms.

use crate::gui::types::Rect;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::gui::layout_core::engine) enum LayoutAxis {
    Horizontal,
    Vertical,
}

impl LayoutAxis {
    pub fn from_horizontal(horizontal: bool) -> Self {
        if horizontal {
            Self::Horizontal
        } else {
            Self::Vertical
        }
    }

    pub fn is_horizontal(self) -> bool {
        matches!(self, Self::Horizontal)
    }

    pub fn main_extent(self, rect: Rect) -> f32 {
        if self.is_horizontal() {
            rect.width()
        } else {
            rect.height()
        }
    }

    pub fn cross_extent(self, rect: Rect) -> f32 {
        if self.is_horizontal() {
            rect.height()
        } else {
            rect.width()
        }
    }

    pub fn overflow_flags(self) -> (bool, bool) {
        (self.is_horizontal(), !self.is_horizontal())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::types::{Point, Vector2};

    #[test]
    fn layout_axis_resolves_main_and_cross_extents() {
        let rect = Rect::from_min_size(Point::new(4.0, 8.0), Vector2::new(120.0, 48.0));

        assert_eq!(LayoutAxis::Horizontal.main_extent(rect), 120.0);
        assert_eq!(LayoutAxis::Horizontal.cross_extent(rect), 48.0);
        assert_eq!(LayoutAxis::Vertical.main_extent(rect), 48.0);
        assert_eq!(LayoutAxis::Vertical.cross_extent(rect), 120.0);
    }

    #[test]
    fn layout_axis_reports_overflow_direction() {
        assert_eq!(LayoutAxis::Horizontal.overflow_flags(), (true, false));
        assert_eq!(LayoutAxis::Vertical.overflow_flags(), (false, true));
    }
}
