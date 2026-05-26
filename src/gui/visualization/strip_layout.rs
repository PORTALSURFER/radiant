use crate::gui::types::{Point, Rect, Vector2};

/// Named fields for constructing reusable horizontal strip geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HorizontalStripLayoutParts {
    /// Rect that contains all strips.
    pub rect: Rect,
    /// Number of strips in the layout.
    pub strip_count: usize,
    /// Gap between adjacent strips in logical pixels.
    pub gap: f32,
}

impl HorizontalStripLayoutParts {
    /// Build horizontal strip layout parts.
    pub const fn new(rect: Rect, strip_count: usize, gap: f32) -> Self {
        Self {
            rect,
            strip_count,
            gap,
        }
    }
}

/// Reusable horizontal strip geometry for dense editor panels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HorizontalStripLayout {
    /// Rect that contains all strips.
    pub rect: Rect,
    /// Number of strips in the layout.
    pub strip_count: usize,
    /// Gap between adjacent strips in logical pixels.
    pub gap: f32,
}

impl HorizontalStripLayout {
    /// Build a horizontal strip layout from named parts.
    pub const fn from_parts(parts: HorizontalStripLayoutParts) -> Self {
        Self {
            rect: parts.rect,
            strip_count: parts.strip_count,
            gap: parts.gap,
        }
    }

    /// Build horizontal strip geometry from a rect, strip count, and gap.
    pub const fn new(rect: Rect, strip_count: usize, gap: f32) -> Self {
        Self::from_parts(HorizontalStripLayoutParts::new(rect, strip_count, gap))
    }

    /// Return whether this layout can produce finite strip geometry.
    pub fn is_valid(self) -> bool {
        self.strip_count > 0 && self.rect.has_finite_positive_area() && self.strip_width() > 0.0
    }

    /// Return the sanitized gap between adjacent strips.
    pub fn gap(self) -> f32 {
        if self.gap.is_finite() {
            self.gap.max(0.0)
        } else {
            0.0
        }
    }

    /// Return the resolved strip width.
    pub fn strip_width(self) -> f32 {
        if self.strip_count == 0 || !self.rect.has_finite_positive_area() {
            return 0.0;
        }
        let total_gap = self.gap() * self.strip_count.saturating_sub(1) as f32;
        ((self.rect.width() - total_gap) / self.strip_count as f32).max(0.0)
    }

    /// Return the rect for a strip index.
    pub fn strip_rect(self, strip: usize) -> Option<Rect> {
        if !self.is_valid() || strip >= self.strip_count {
            return None;
        }
        let strip_width = self.strip_width();
        let x = self.rect.min.x + strip as f32 * (strip_width + self.gap());
        Some(Rect::from_min_size(
            Point::new(x, self.rect.min.y),
            Vector2::new(strip_width, self.rect.height()),
        ))
    }

    /// Return the strip containing a point.
    pub fn strip_at_position(self, position: Point) -> Option<usize> {
        if !self.is_valid() || !self.rect.contains(position) {
            return None;
        }
        (0..self.strip_count).find(|strip| {
            self.strip_rect(*strip)
                .is_some_and(|rect| rect.contains(position))
        })
    }

    /// Return the insertion index nearest a horizontal pointer position.
    pub fn insertion_index_at(self, position: Point) -> usize {
        if !self.is_valid() || position.x <= self.rect.min.x {
            return 0;
        }
        if position.x >= self.rect.max.x {
            return self.strip_count;
        }
        for strip in 0..self.strip_count {
            if let Some(rect) = self.strip_rect(strip)
                && position.x < rect.center().x
            {
                return strip;
            }
        }
        self.strip_count
    }

    /// Return a vertical insertion marker for a strip insertion index.
    pub fn insertion_line_rect(
        self,
        insertion_index: usize,
        width: f32,
        vertical_inset: f32,
    ) -> Option<Rect> {
        if !self.is_valid() {
            return None;
        }
        let width = finite_nonnegative(width);
        if width <= 0.0 {
            return None;
        }
        let inset = finite_nonnegative(vertical_inset).min(self.rect.height() * 0.5);
        let insertion_index = insertion_index.min(self.strip_count);
        let x = if insertion_index == 0 {
            self.strip_rect(0)?.min.x
        } else if insertion_index == self.strip_count {
            self.strip_rect(self.strip_count - 1)?.max.x
        } else {
            let left = self.strip_rect(insertion_index - 1)?;
            let right = self.strip_rect(insertion_index)?;
            (left.max.x + right.min.x) * 0.5
        };
        Some(Rect::from_min_max(
            Point::new(x - width * 0.5, self.rect.min.y + inset),
            Point::new(x + width * 0.5, self.rect.max.y - inset),
        ))
    }
}

fn finite_nonnegative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}
