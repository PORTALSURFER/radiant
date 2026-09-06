//! Immutable animation declarations sampled by the window runtime.
//!
//! Capabilities emit presentation from runtime-owned samples. They must not
//! create timers or mutate application state. Paint animation rebuilds base
//! paint without application projection; geometry uses normal layout.

use crate::{
    gui::types::Rect,
    layout::{Constraints, LayoutPolicy, MeasureChildren, PlaceChildren, SizeHint},
    runtime::{PaintPrimitive, ResolvedEnvironment},
    theme::ThemeTokens,
};
use std::time::Duration;

/// The narrowest safe invalidation required by a property.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimationImpact {
    /// Rebuild paint while retaining geometry and application projection.
    Paint,
    /// Remeasure and place the current accepted surface, then paint.
    Geometry,
}
/// Interpolation curve for a finite transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimationEasing {
    /// Constant interpolation speed.
    Linear,
    /// Quadratic deceleration toward the target.
    EaseOut,
}
/// Finite transition policy. Zero duration applies the target immediately.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Transition {
    duration: Duration,
    easing: AnimationEasing,
}
impl Transition {
    /// Interpolate at constant speed.
    pub const fn linear(duration: Duration) -> Self {
        Self {
            duration,
            easing: AnimationEasing::Linear,
        }
    }
    /// Interpolate with quadratic deceleration.
    pub const fn ease_out(duration: Duration) -> Self {
        Self {
            duration,
            easing: AnimationEasing::EaseOut,
        }
    }
    /// Transition duration.
    pub const fn duration(self) -> Duration {
        self.duration
    }
    /// Transition curve.
    pub const fn easing(self) -> AnimationEasing {
        self.easing
    }
}
/// Invalid declarative animation input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimationError {
    /// An endpoint or static fallback is not finite.
    NonFiniteValue,
    /// A shared feedback clock has no positive period.
    ZeroPeriod,
}
impl std::fmt::Display for AnimationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NonFiniteValue => "animation values must be finite",
            Self::ZeroPeriod => "feedback period must be positive",
        })
    }
}
impl std::error::Error for AnimationError {}
/// One scalar property, identified within its accepted container owner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimationTarget {
    property: u64,
    initial: f64,
    target: f64,
    transition: Transition,
    impact: AnimationImpact,
}
impl AnimationTarget {
    /// Declare a property. A new owner starts at `initial`; retargeting starts
    /// at its current interpolated value instead.
    pub fn new(
        property: u64,
        initial: f64,
        target: f64,
        transition: Transition,
        impact: AnimationImpact,
    ) -> Result<Self, AnimationError> {
        if !initial.is_finite() || !target.is_finite() {
            return Err(AnimationError::NonFiniteValue);
        }
        Ok(Self {
            property,
            initial,
            target,
            transition,
            impact,
        })
    }
    /// Stable property identifier within this owner.
    pub const fn property(self) -> u64 {
        self.property
    }
    /// Initial value for a newly accepted owner.
    pub const fn initial(self) -> f64 {
        self.initial
    }
    /// Desired terminal value.
    pub const fn target(self) -> f64 {
        self.target
    }
    /// Interpolation policy.
    pub const fn transition(self) -> Transition {
        self.transition
    }
    /// Required invalidation.
    pub const fn impact(self) -> AnimationImpact {
        self.impact
    }
}
/// A bounded shared phase for indeterminate feedback.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FeedbackAnimation {
    property: u64,
    group: u64,
    period: Duration,
    static_value: f64,
}
impl FeedbackAnimation {
    /// Declare a shared phase in `[0, 1)`. Equal group identifiers must use the
    /// same period. Reduced motion and rejected admission use `static_value`.
    pub fn new(
        property: u64,
        group: u64,
        period: Duration,
        static_value: f64,
    ) -> Result<Self, AnimationError> {
        if period.is_zero() {
            return Err(AnimationError::ZeroPeriod);
        }
        if !static_value.is_finite() {
            return Err(AnimationError::NonFiniteValue);
        }
        Ok(Self {
            property,
            group,
            period,
            static_value,
        })
    }
    /// Property receiving the shared phase.
    pub const fn property(self) -> u64 {
        self.property
    }
    /// Window-local shared group identifier.
    pub const fn group(self) -> u64 {
        self.group
    }
    /// Shared cycle duration.
    pub const fn period(self) -> Duration {
        self.period
    }
    /// Readable static fallback.
    pub const fn static_value(self) -> f64 {
        self.static_value
    }
}
/// Borrowed runtime-owned values for one accepted animation owner.
#[derive(Clone, Copy, Debug)]
pub struct AnimationValues<'a> {
    values: &'a [(u64, f64)],
}
impl<'a> AnimationValues<'a> {
    /// Build a read-only sample view, useful in isolated capability tests.
    pub const fn new(values: &'a [(u64, f64)]) -> Self {
        Self { values }
    }
    /// Look up a property. Missing or rejected properties have no value.
    pub fn get(self, property: u64) -> Option<f64> {
        self.values
            .iter()
            .find_map(|(id, v)| (*id == property).then_some(*v))
    }
}
/// Immutable context for a container's animated paint contribution.
pub struct AnimationPaintContext<'a> {
    /// Current accepted bounds.
    pub bounds: Rect,
    /// Current theme tokens.
    pub theme: &'a ThemeTokens,
    /// Resolved environment, including reduced-motion policy.
    pub environment: &'a ResolvedEnvironment,
}
/// Optional pure presentation capability for an animated container.
///
/// Declarations remain immutable during their projection lifetime. Identity is
/// supplied by the accepted container and property IDs. Runtime samples are
/// passed explicitly; this capability never owns a clock or mutable samples.
pub trait Animatable: std::any::Any {
    /// Finite target declarations, with unique property IDs.
    fn targets(&self) -> &[AnimationTarget];
    /// Shared feedback declarations. Property IDs must also be unique across
    /// finite targets and feedback.
    fn feedback(&self) -> &[FeedbackAnimation] {
        &[]
    }
    /// Append only this container's animated chrome using the sampled values.
    fn append_paint(
        &self,
        _values: AnimationValues<'_>,
        _context: AnimationPaintContext<'_>,
        _output: &mut Vec<PaintPrimitive>,
    ) {
    }
    /// Measure using sampled geometry. Paint-only capabilities use the fallback.
    fn measure(
        &self,
        _values: AnimationValues<'_>,
        fallback: &dyn LayoutPolicy,
        children: &mut MeasureChildren<'_>,
        constraints: Constraints,
    ) -> SizeHint {
        fallback.measure(children, constraints)
    }
    /// Place children using the same samples used for measurement and paint.
    fn place(
        &self,
        _values: AnimationValues<'_>,
        fallback: &dyn LayoutPolicy,
        children: &mut PlaceChildren<'_>,
        bounds: Rect,
    ) {
        fallback.place(children, bounds);
    }
}
