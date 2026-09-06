//! Explicit gesture recognition policy and optional widget consumer.
use crate::{
    gui::pointer_ingress::{GestureIngress, GestureKind, GesturePhase},
    gui::types::{Point, Vector2},
    widgets::{WidgetOutput, WidgetSemanticsRevision},
};

/// Checked per-family thresholds. Pan uses logical distance, pinch scale deviation,
/// and rotation radians. Thresholds apply to accumulation since Started.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GesturePolicy {
    thresholds: [Option<f32>; 3],
}
impl Eq for GesturePolicy {}
impl GesturePolicy {
    /// A policy with no recognizers.
    pub const fn none() -> Self {
        Self {
            thresholds: [None; 3],
        }
    }
    /// Enable a recognizer with a finite nonnegative threshold.
    pub fn recognize(
        mut self,
        kind: GestureKind,
        threshold: f32,
    ) -> Result<Self, InvalidGestureThreshold> {
        if !threshold.is_finite() || threshold < 0.0 {
            return Err(InvalidGestureThreshold);
        }
        self.thresholds[index(kind)] = Some(threshold);
        Ok(self)
    }
    /// Read the configured recognition threshold.
    pub const fn threshold(self, kind: GestureKind) -> Option<f32> {
        self.thresholds[index(kind)]
    }
}
const fn index(kind: GestureKind) -> usize {
    match kind {
        GestureKind::Pan => 0,
        GestureKind::Pinch => 1,
        GestureKind::Rotate => 2,
    }
}
/// A threshold was negative or nonfinite.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidGestureThreshold;

/// Why an admitted gesture ended without completion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GestureCancellation {
    /// The native source explicitly cancelled.
    Source,
    /// Focus, capture, or lifecycle teardown cancelled the current owner.
    CaptureLost,
    /// Current identity or gesture policy no longer matches the owner.
    Retired,
    /// Accumulation exceeded finite representable coordinates.
    InvalidSample,
}

/// One admitted lifecycle event. Native sample evidence is never synthesized.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GestureEvent {
    pub(crate) sample: GestureIngress,
    pub(crate) anchor: Point,
    pub(crate) phase: GesturePhase,
    pub(crate) accumulated: Vector2,
    pub(crate) cancellation: Option<GestureCancellation>,
}
impl GestureEvent {
    /// Exact latest source sample, including timestamp, sequence range and modifiers.
    pub const fn sample(self) -> GestureIngress {
        self.sample
    }
    /// Finite logical anchor latched at sequence admission, including native fallback.
    pub const fn anchor(self) -> Point {
        self.anchor
    }
    /// Recognized lifecycle phase. A threshold crossing emits Started exactly once.
    pub const fn phase(self) -> GesturePhase {
        self.phase
    }
    /// Total pan displacement, multiplicative pinch scale, or summed rotation radians.
    pub const fn accumulated(self) -> Vector2 {
        self.accumulated
    }
    /// Present only for a cancelled gesture.
    pub const fn cancellation(self) -> Option<GestureCancellation> {
        self.cancellation
    }
}

/// Optional gesture consumer. Policy observation cannot invoke application callbacks.
pub trait WidgetGestures {
    /// Exact evidence for recognition and execution policy, conservative by default.
    fn revision(&self) -> WidgetSemanticsRevision {
        WidgetSemanticsRevision::conservative()
    }
    /// Read the current configured recognizers.
    fn policy(&self) -> GesturePolicy;
    /// Emit zero or one ordinary typed widget output for an admitted lifecycle event.
    fn dispatch(&mut self, event: GestureEvent) -> Option<WidgetOutput>;
}
