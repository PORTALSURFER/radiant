//! Object-safe custom measure/place policies for layout containers.

use super::constraints::Constraints;
use crate::gui::types::{Rect, Vector2};
use std::rc::Rc;

/// An immutable, UI-local policy for measuring and placing a container's
/// declared children.
pub trait LayoutPolicy: 'static {
    /// Measure the container under normalized constraints.
    fn measure(&self, children: &mut MeasureChildren<'_>, constraints: Constraints) -> SizeHint;

    /// Assign each declared child one rectangle or an explicit omission.
    fn place(&self, children: &mut PlaceChildren<'_>, bounds: Rect);
}

impl<Policy: LayoutPolicy + ?Sized> LayoutPolicy for Rc<Policy> {
    fn measure(&self, children: &mut MeasureChildren<'_>, constraints: Constraints) -> SizeHint {
        self.as_ref().measure(children, constraints)
    }

    fn place(&self, children: &mut PlaceChildren<'_>, bounds: Rect) {
        self.as_ref().place(children, bounds)
    }
}

/// A normalized container size hint returned by [`LayoutPolicy::measure`].
///
/// The representation is intentionally private. Constructors retain enough
/// validation state for the engine to diagnose malformed policy output while
/// accessors always expose non-negative, ordered values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SizeHint {
    intrinsic_minimum: Vector2,
    preferred_extent: Vector2,
    maximum: Option<Vector2>,
    baseline: Option<f32>,
    validation: SizeHintValidation,
}

impl Default for SizeHint {
    fn default() -> Self {
        Self::new(Vector2::default(), Vector2::default())
    }
}

impl SizeHint {
    /// Build a size hint with an intrinsic minimum and preferred extent.
    pub fn new(intrinsic_minimum: Vector2, preferred_extent: Vector2) -> Self {
        Self::normalized(
            intrinsic_minimum,
            preferred_extent,
            None,
            None,
            SizeHintValidation::default(),
        )
    }

    /// Build a size hint whose preferred extent is the supplied value.
    pub fn preferred(preferred_extent: Vector2) -> Self {
        Self::new(Vector2::default(), preferred_extent)
    }

    /// Add a maximum extent to this size hint.
    pub fn with_maximum(self, maximum: Vector2) -> Self {
        Self::normalized(
            self.intrinsic_minimum,
            self.preferred_extent,
            Some(maximum),
            self.baseline,
            self.validation,
        )
    }

    /// Set or clear the optional maximum extent.
    pub fn with_optional_maximum(self, maximum: Option<Vector2>) -> Self {
        Self::normalized(
            self.intrinsic_minimum,
            self.preferred_extent,
            maximum,
            self.baseline,
            self.validation,
        )
    }

    /// Remove the optional maximum extent.
    pub fn without_maximum(self) -> Self {
        self.with_optional_maximum(None)
    }

    /// Add a baseline measured from the top edge of the preferred extent.
    pub fn with_baseline(self, baseline: f32) -> Self {
        Self::normalized(
            self.intrinsic_minimum,
            self.preferred_extent,
            self.maximum,
            Some(baseline),
            self.validation,
        )
    }

    /// Set or clear the optional baseline.
    pub fn with_optional_baseline(self, baseline: Option<f32>) -> Self {
        Self::normalized(
            self.intrinsic_minimum,
            self.preferred_extent,
            self.maximum,
            baseline,
            self.validation,
        )
    }

    /// Remove the optional baseline.
    pub fn without_baseline(self) -> Self {
        self.with_optional_baseline(None)
    }

    /// Return the intrinsic minimum extent.
    pub fn intrinsic_minimum(self) -> Vector2 {
        self.intrinsic_minimum
    }

    /// Return the preferred extent.
    pub fn preferred_extent(self) -> Vector2 {
        self.preferred_extent
    }

    /// Return the optional maximum extent.
    pub fn maximum(self) -> Option<Vector2> {
        self.maximum
    }

    /// Return the optional baseline.
    pub fn baseline(self) -> Option<f32> {
        self.baseline
    }

    pub(crate) fn resolve(self, constraints: Constraints) -> Vector2 {
        let max_w = self.maximum.map_or(constraints.max_w, |maximum| {
            constraints.max_w.min(maximum.x)
        });
        let max_h = self.maximum.map_or(constraints.max_h, |maximum| {
            constraints.max_h.min(maximum.y)
        });
        let min_w = constraints.min_w.max(self.intrinsic_minimum.x);
        let min_h = constraints.min_h.max(self.intrinsic_minimum.y);
        Vector2::new(
            self.preferred_extent.x.clamp(min_w, max_w.max(min_w)),
            self.preferred_extent.y.clamp(min_h, max_h.max(min_h)),
        )
    }

    pub(crate) fn has_non_finite_values(self) -> bool {
        self.validation.non_finite
    }

    pub(crate) fn has_negative_values(self) -> bool {
        self.validation.negative
    }

    pub(crate) fn has_contradictory_values(self) -> bool {
        self.validation.contradictory
    }

    fn normalized(
        intrinsic_minimum: Vector2,
        preferred_extent: Vector2,
        maximum: Option<Vector2>,
        baseline: Option<f32>,
        mut validation: SizeHintValidation,
    ) -> Self {
        let intrinsic_minimum = sanitize_vector(intrinsic_minimum, &mut validation);
        let mut preferred_extent = sanitize_vector(preferred_extent, &mut validation);
        if preferred_extent.x < intrinsic_minimum.x {
            preferred_extent.x = intrinsic_minimum.x;
            validation.contradictory = true;
        }
        if preferred_extent.y < intrinsic_minimum.y {
            preferred_extent.y = intrinsic_minimum.y;
            validation.contradictory = true;
        }

        let mut maximum = maximum.map(|maximum| sanitize_vector(maximum, &mut validation));
        if let Some(maximum) = &mut maximum {
            if maximum.x < preferred_extent.x {
                maximum.x = preferred_extent.x;
                validation.contradictory = true;
            }
            if maximum.y < preferred_extent.y {
                maximum.y = preferred_extent.y;
                validation.contradictory = true;
            }
        }

        let baseline = baseline.map(|baseline| {
            if !baseline.is_finite() {
                validation.non_finite = true;
                0.0
            } else if baseline < 0.0 {
                validation.negative = true;
                0.0
            } else {
                baseline
            }
        });

        Self {
            intrinsic_minimum,
            preferred_extent,
            maximum,
            baseline,
            validation,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct SizeHintValidation {
    non_finite: bool,
    negative: bool,
    contradictory: bool,
}

fn sanitize_vector(value: Vector2, validation: &mut SizeHintValidation) -> Vector2 {
    Vector2::new(
        sanitize_component(value.x, validation),
        sanitize_component(value.y, validation),
    )
}

fn sanitize_component(value: f32, validation: &mut SizeHintValidation) -> f32 {
    if !value.is_finite() {
        validation.non_finite = true;
        0.0
    } else if value < 0.0 {
        validation.negative = true;
        0.0
    } else {
        value
    }
}

/// A bounded child-measurement context supplied to a layout policy.
pub struct MeasureChildren<'a> {
    count: usize,
    measure: &'a mut dyn FnMut(usize, Constraints) -> Vector2,
    errors: &'a mut Vec<MeasureChildrenError>,
}

impl<'a> MeasureChildren<'a> {
    pub(crate) fn new(
        count: usize,
        measure: &'a mut dyn FnMut(usize, Constraints) -> Vector2,
        errors: &'a mut Vec<MeasureChildrenError>,
    ) -> Self {
        Self {
            count,
            measure,
            errors,
        }
    }

    /// Return the number of declared children.
    pub fn len(&self) -> usize {
        self.count
    }

    /// Return whether the container has no declared children.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Measure one declared child under the requested constraints.
    ///
    /// The engine normalizes and bounds the request by the child's declared
    /// slot constraints before invoking the child measurement.
    pub fn measure(
        &mut self,
        index: usize,
        constraints: Constraints,
    ) -> Result<Vector2, MeasureChildrenError> {
        if index >= self.count {
            let error = MeasureChildrenError::InvalidIndex {
                index,
                child_count: self.count,
            };
            self.errors.push(error);
            return Err(error);
        }
        Ok((self.measure)(index, constraints))
    }

    /// Alias for [`Self::measure`] with an explicit child-oriented name.
    pub fn measure_child(
        &mut self,
        index: usize,
        constraints: Constraints,
    ) -> Result<Vector2, MeasureChildrenError> {
        self.measure(index, constraints)
    }
}

/// A rejected request made through [`MeasureChildren`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeasureChildrenError {
    /// The policy addressed a child outside the declared range.
    InvalidIndex {
        /// Requested child index.
        index: usize,
        /// Number of declared children.
        child_count: usize,
    },
}

/// Short alias for [`MeasureChildrenError`].
pub type MeasureChildError = MeasureChildrenError;

/// A typed reason why a declared child is intentionally omitted from visual
/// placement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutOmissionReason {
    /// The policy conditionally excludes this child from the current layout.
    Conditional,
    /// The child is not materialized for the current layout pass.
    Virtualized,
    /// The child cannot be placed under the current layout inputs.
    Unavailable,
}

/// Short alias for [`LayoutOmissionReason`].
pub type LayoutPolicyOmissionReason = LayoutOmissionReason;

/// A bounded child-placement context supplied to a layout policy.
pub struct PlaceChildren<'a> {
    count: usize,
    dispositions: &'a mut [Option<ChildDisposition>],
    errors: &'a mut Vec<PlaceChildrenError>,
}

impl<'a> PlaceChildren<'a> {
    pub(crate) fn new(
        count: usize,
        dispositions: &'a mut [Option<ChildDisposition>],
        errors: &'a mut Vec<PlaceChildrenError>,
    ) -> Self {
        Self {
            count,
            dispositions,
            errors,
        }
    }

    /// Return the number of declared children.
    pub fn len(&self) -> usize {
        self.count
    }

    /// Return whether the container has no declared children.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Place one declared child exactly once.
    pub fn place(&mut self, index: usize, rect: Rect) -> Result<(), PlaceChildrenError> {
        if let Some(error) = self.validate_index(index) {
            return Err(error);
        }
        if !valid_placement_rect(rect) {
            let error = PlaceChildrenError::InvalidRect { index, rect };
            self.errors.push(error);
            return Err(error);
        }
        if let Some(error) = self.validate_available(index) {
            return Err(error);
        }
        self.dispositions[index] = Some(ChildDisposition::Placed(rect));
        Ok(())
    }

    /// Explicitly omit one declared child with a typed reason.
    pub fn omit(
        &mut self,
        index: usize,
        reason: LayoutOmissionReason,
    ) -> Result<(), PlaceChildrenError> {
        if let Some(error) = self.validate_index(index) {
            return Err(error);
        }
        if let Some(error) = self.validate_available(index) {
            return Err(error);
        }
        self.dispositions[index] = Some(ChildDisposition::Omitted(reason));
        Ok(())
    }

    fn validate_index(&mut self, index: usize) -> Option<PlaceChildrenError> {
        if index >= self.count {
            let error = PlaceChildrenError::InvalidIndex {
                index,
                child_count: self.count,
            };
            self.errors.push(error);
            Some(error)
        } else {
            None
        }
    }

    fn validate_available(&mut self, index: usize) -> Option<PlaceChildrenError> {
        if self.dispositions[index].is_some() {
            let error = PlaceChildrenError::DuplicateDisposition { index };
            self.errors.push(error);
            Some(error)
        } else {
            None
        }
    }
}

fn valid_placement_rect(rect: Rect) -> bool {
    crate::gui::layout_core::validated_geometry::ValidatedRect::new(rect).is_some()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ChildDisposition {
    Placed(Rect),
    Omitted(LayoutOmissionReason),
}

/// A rejected request made through [`PlaceChildren`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlaceChildrenError {
    /// The policy addressed a child outside the declared range.
    InvalidIndex {
        /// Requested child index.
        index: usize,
        /// Number of declared children.
        child_count: usize,
    },
    /// The policy supplied non-finite or inverted rectangle geometry.
    InvalidRect {
        /// Child index associated with the invalid rectangle.
        index: usize,
        /// Rejected rectangle.
        rect: Rect,
    },
    /// The policy tried to overwrite an already accepted disposition.
    DuplicateDisposition {
        /// Child index whose first disposition was retained.
        index: usize,
    },
}

/// Short alias for [`PlaceChildrenError`].
pub type LayoutPolicyPlacementError = PlaceChildrenError;
