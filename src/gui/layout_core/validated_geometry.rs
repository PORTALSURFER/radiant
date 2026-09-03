//! Crate-private validation boundary for layout geometry.

use crate::gui::layout_core::model::Insets;
use crate::gui::types::{Point, Rect, Vector2};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ValidatedRect(Rect);

impl ValidatedRect {
    pub(crate) fn new(rect: Rect) -> Option<Self> {
        (rect.is_finite() && rect.width() >= 0.0 && rect.height() >= 0.0).then_some(Self(rect))
    }

    pub(crate) fn rounded(rect: Rect) -> Option<Self> {
        let rect = Self::new(rect)?.0;
        let width = rect.width().round();
        let height = rect.height().round();
        if !width.is_finite() || !height.is_finite() || width < 0.0 || height < 0.0 {
            return None;
        }
        Self::new(Rect::from_min_size(
            Point::new(rect.min.x.floor(), rect.min.y.floor()),
            Vector2::new(width, height),
        ))
    }

    pub(crate) const fn rect(self) -> Rect {
        self.0
    }
}

/// Derive a rectangle by applying finite insets to each edge.
pub(crate) fn checked_inset_rect(rect: Rect, insets: Insets) -> Option<Rect> {
    if !rect.is_finite()
        || ![insets.left, insets.right, insets.top, insets.bottom]
            .iter()
            .all(|value| value.is_finite())
    {
        return None;
    }
    let derived = Rect::from_min_max(
        Point::new(rect.min.x + insets.left, rect.min.y + insets.top),
        Point::new(rect.max.x - insets.right, rect.max.y - insets.bottom),
    );
    ValidatedRect::new(derived).map(ValidatedRect::rect)
}

pub(crate) fn normalize_constraint_axis(minimum: f32, maximum: f32) -> (f32, f32) {
    let minimum = if minimum.is_finite() && minimum >= 0.0 {
        minimum
    } else {
        0.0
    };
    let maximum = if maximum == f32::INFINITY || (maximum.is_finite() && maximum >= minimum) {
        maximum
    } else {
        minimum
    };
    (minimum, maximum)
}

#[cfg(test)]
mod tests {
    use super::{ValidatedRect, checked_inset_rect, normalize_constraint_axis};
    use crate::gui::layout_core::model::Insets;
    use crate::gui::types::{Point, Rect, Vector2};

    #[test]
    fn constraint_normalization_accepts_only_explicit_positive_infinity() {
        assert_eq!(normalize_constraint_axis(4.0, 9.0), (4.0, 9.0));
        assert_eq!(normalize_constraint_axis(f32::NAN, f32::NAN), (0.0, 0.0));
        assert_eq!(normalize_constraint_axis(-2.0, -1.0), (0.0, 0.0));
        assert_eq!(normalize_constraint_axis(8.0, 3.0), (8.0, 8.0));
        assert_eq!(
            normalize_constraint_axis(8.0, f32::NEG_INFINITY),
            (8.0, 8.0)
        );
        assert_eq!(
            normalize_constraint_axis(8.0, f32::INFINITY),
            (8.0, f32::INFINITY)
        );
    }

    #[test]
    fn validated_rect_preserves_negative_origins_and_rejects_bad_geometry() {
        let rect = Rect::from_min_size(Point::new(-3.0, -2.0), Vector2::new(0.0, 4.0));
        assert_eq!(ValidatedRect::rounded(rect).unwrap().rect(), rect);
        assert!(ValidatedRect::new(Rect::from_xy_size(0.0, 0.0, f32::NAN, 1.0)).is_none());
        assert!(
            ValidatedRect::new(Rect::from_min_max(
                Point::new(1.0, 0.0),
                Point::new(0.0, 1.0),
            ))
            .is_none()
        );
    }

    #[test]
    fn checked_insets_preserve_zero_sizes_and_reject_bad_derivations() {
        let rect = Rect::from_min_size(Point::new(-3.0, -2.0), Vector2::new(4.0, 4.0));
        assert_eq!(
            checked_inset_rect(rect, Insets::all(2.0)),
            Some(Rect::from_min_size(
                Point::new(-1.0, 0.0),
                Vector2::new(0.0, 0.0)
            ))
        );
        assert!(checked_inset_rect(rect, Insets::all(3.0)).is_none());
        assert!(checked_inset_rect(rect, Insets::all(f32::NAN)).is_none());
        assert!(
            checked_inset_rect(
                Rect::from_min_max(
                    Point::new(f32::MAX - 4.0, 0.0),
                    Point::new(f32::MAX - 2.0, 1.0),
                ),
                Insets::all(-4.0),
            )
            .is_none()
        );
    }
}
