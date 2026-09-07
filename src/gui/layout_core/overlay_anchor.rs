//! Current-layout placement policy for a transient overlay.
use super::{NodeId, Point, Rect, Vector2};

/// Preferred vertical side of a current widget anchor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OverlayAnchorSide {
    /// Place below the trigger.
    Below,
    /// Place above the trigger.
    Above,
}

/// Declarative trigger identity and logical-pixel placement policy.
///
/// The layout engine resolves the unique trigger from the same complete layout
/// pass. Missing, ambiguous, omitted, offscreen, or invalid anchors hide the
/// complete overlay, including its input shield. This value stores no geometry
/// receipt, widget reference, or runtime owner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OverlayAnchor {
    pub(crate) target: NodeId,
    pub(crate) size: Vector2,
    pub(crate) gap: f32,
    pub(crate) side: OverlayAnchorSide,
    pub(crate) flip: bool,
    pub(crate) clamp: bool,
}
impl OverlayAnchor {
    /// Prefer placement below a current trigger, flipping and clamping as needed.
    pub fn below(target: NodeId, size: Vector2) -> Self {
        Self {
            target,
            size,
            gap: 0.0,
            side: OverlayAnchorSide::Below,
            flip: true,
            clamp: true,
        }
    }
    /// Prefer placement above a current trigger, flipping and clamping as needed.
    pub fn above(target: NodeId, size: Vector2) -> Self {
        Self {
            side: OverlayAnchorSide::Above,
            ..Self::below(target, size)
        }
    }
    /// Set a non-negative logical-pixel gap. Invalid values fail closed at layout.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }
    /// Select whether placement may flip to the opposite side when clipped.
    pub fn flip_when_clipped(mut self, enabled: bool) -> Self {
        self.flip = enabled;
        self
    }
    /// Select viewport clamping, including shrinking an oversized overlay to fit.
    pub fn clamp_to_viewport(mut self, enabled: bool) -> Self {
        self.clamp = enabled;
        self
    }
    /// Return the requested current trigger identity.
    pub const fn target(self) -> NodeId {
        self.target
    }

    pub(crate) fn resolve(self, trigger: Rect, viewport: Rect) -> Option<Rect> {
        if !trigger.has_finite_positive_area()
            || !viewport.has_finite_positive_area()
            || !self.size.x.is_finite()
            || !self.size.y.is_finite()
            || self.size.x <= 0.0
            || self.size.y <= 0.0
            || !self.gap.is_finite()
            || self.gap < 0.0
            || trigger.max.x <= viewport.min.x
            || trigger.min.x >= viewport.max.x
            || trigger.max.y <= viewport.min.y
            || trigger.min.y >= viewport.max.y
        {
            return None;
        }
        let size = if self.clamp {
            Vector2::new(
                self.size.x.min(viewport.width()),
                self.size.y.min(viewport.height()),
            )
        } else {
            self.size
        };
        let below = trigger.max.y + self.gap;
        let above = trigger.min.y - self.gap - size.y;
        if !below.is_finite() || !above.is_finite() {
            return None;
        }
        let fits = |y: f32| y >= viewport.min.y && y + size.y <= viewport.max.y;
        let (preferred, opposite) = match self.side {
            OverlayAnchorSide::Below => (below, above),
            OverlayAnchorSide::Above => (above, below),
        };
        let mut y = if self.flip && !fits(preferred) && fits(opposite) {
            opposite
        } else {
            preferred
        };
        let mut x = trigger.min.x;
        if self.clamp {
            x = x.clamp(viewport.min.x, viewport.max.x - size.x);
            y = y.clamp(viewport.min.y, viewport.max.y - size.y);
        }
        let rect = Rect::from_min_size(Point::new(x, y), size);
        rect.has_finite_positive_area().then_some(rect)
    }

    pub(crate) fn hash_into(self, hasher: &mut impl std::hash::Hasher) {
        use std::hash::Hash;
        self.target.hash(hasher);
        self.size.x.to_bits().hash(hasher);
        self.size.y.to_bits().hash(hasher);
        self.gap.to_bits().hash(hasher);
        self.side.hash(hasher);
        self.flip.hash(hasher);
        self.clamp.hash(hasher);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AnchoredOverlayLayout {
    pub(crate) anchor: OverlayAnchor,
    pub(crate) has_input: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
        Rect::from_min_size(Point::new(x, y), Vector2::new(width, height))
    }
    #[test]
    fn placement_flips_both_directions_and_clamps_to_current_viewport() {
        let viewport = rect(10.0, 20.0, 200.0, 120.0);
        let anchor = OverlayAnchor::below(1, Vector2::new(80.0, 40.0)).gap(4.0);
        assert_eq!(
            anchor.resolve(rect(180.0, 115.0, 20.0, 20.0), viewport),
            Some(rect(130.0, 71.0, 80.0, 40.0))
        );
        let above = OverlayAnchor::above(1, Vector2::new(80.0, 40.0)).gap(4.0);
        assert_eq!(
            above.resolve(rect(15.0, 25.0, 20.0, 20.0), viewport),
            Some(rect(15.0, 49.0, 80.0, 40.0))
        );
    }
    #[test]
    fn oversized_overlay_shrinks_and_invalid_inputs_are_omitted() {
        let viewport = rect(0.0, 0.0, 100.0, 80.0);
        let trigger = rect(20.0, 20.0, 10.0, 10.0);
        assert_eq!(
            OverlayAnchor::below(1, Vector2::new(1000.0, 1000.0)).resolve(trigger, viewport),
            Some(viewport)
        );
        for gap in [f32::NAN, f32::INFINITY, -1.0] {
            assert!(
                OverlayAnchor::below(1, Vector2::new(20.0, 20.0))
                    .gap(gap)
                    .resolve(trigger, viewport)
                    .is_none()
            );
        }
        assert!(
            OverlayAnchor::below(1, Vector2::new(20.0, 20.0))
                .resolve(rect(200.0, 0.0, 10.0, 10.0), viewport)
                .is_none()
        );
    }
}
