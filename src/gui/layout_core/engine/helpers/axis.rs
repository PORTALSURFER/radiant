//! Internal main/cross-axis helpers for layout algorithms.

use crate::gui::types::Rect;

#[cfg(test)]
#[path = "axis/tests.rs"]
mod tests;

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
