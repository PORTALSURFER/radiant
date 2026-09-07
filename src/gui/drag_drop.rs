//! UI-local typed drag data. These values never grant capture or target authority.
use super::{
    pointer_ingress::PointerSequenceToken,
    types::{Point, Vector2},
};
use std::{any::Any, fmt, rc::Rc};

/// Operation negotiated between an in-application drag source and target.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DragOperation {
    /// The target copies the payload's application-owned data.
    Copy,
    /// The application moves the payload's data after a successful drop.
    Move,
    /// The target creates a reference to the payload's data.
    Link,
}
impl DragOperation {
    const fn bit(self) -> u8 {
        match self {
            Self::Copy => 1,
            Self::Move => 2,
            Self::Link => 4,
        }
    }
}
/// A checked, nonempty set of allowed drag operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DragOperations(u8);
impl DragOperations {
    /// Construct a nonempty operation set; duplicates have no effect.
    pub fn new(
        operations: impl IntoIterator<Item = DragOperation>,
    ) -> Result<Self, DragDescriptorError> {
        let mut bits = 0;
        for operation in operations {
            bits |= operation.bit();
        }
        if bits == 0 {
            Err(DragDescriptorError::EmptyOperations)
        } else {
            Ok(Self(bits))
        }
    }
    /// A set accepting every supported operation.
    pub const fn all() -> Self {
        Self(7)
    }
    /// A set containing one operation.
    pub const fn only(operation: DragOperation) -> Self {
        Self(operation.bit())
    }
    /// Return whether an operation is allowed.
    pub const fn contains(self, operation: DragOperation) -> bool {
        self.0 & operation.bit() != 0
    }
}
impl Default for DragOperations {
    fn default() -> Self {
        Self::only(DragOperation::Copy)
    }
}
/// Invalid declarative drag metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragDescriptorError {
    /// At least one operation is required.
    EmptyOperations,
    /// Recognition thresholds must be finite and nonnegative.
    InvalidThreshold,
    /// Preview dimensions must be finite, positive and at most 1024 logical pixels per axis.
    InvalidPreview,
    /// Preview labels are bounded to 4096 UTF-8 bytes.
    PreviewLabelTooLong,
    /// Drag autoscroll geometry and velocity must be finite and positive.
    InvalidAutoscrollPolicy,
}
impl fmt::Display for DragDescriptorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::EmptyOperations => "a drag requires an allowed operation",
            Self::InvalidThreshold => "drag thresholds must be finite and nonnegative",
            Self::InvalidPreview => "drag preview dimensions must be finite and in (0, 1024]",
            Self::PreviewLabelTooLong => "drag preview labels are limited to 4096 bytes",
            Self::InvalidAutoscrollPolicy => {
                "drag autoscroll edge zone and maximum speed must be finite and positive"
            }
        })
    }
}

/// Opt-in logical edge autoscroll for a typed in-process drag source.
///
/// The edge zone is measured inside the current scroll viewport. Maximum speed
/// is measured in logical pixels per second; the runtime clamps both elapsed
/// time and the resulting per-tick movement.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragAutoscrollPolicy {
    edge_zone: f32,
    max_speed: f32,
}
impl Eq for DragAutoscrollPolicy {}
impl DragAutoscrollPolicy {
    /// Construct a checked policy with explicit logical geometry and speed.
    pub fn new(edge_zone: f32, max_speed: f32) -> Result<Self, DragDescriptorError> {
        if !edge_zone.is_finite() || !max_speed.is_finite() || edge_zone <= 0.0 || max_speed <= 0.0
        {
            return Err(DragDescriptorError::InvalidAutoscrollPolicy);
        }
        Ok(Self {
            edge_zone,
            max_speed,
        })
    }
    /// Return the logical edge zone.
    pub const fn edge_zone(self) -> f32 {
        self.edge_zone
    }
    /// Return the bounded maximum logical speed in pixels per second.
    pub const fn max_speed(self) -> f32 {
        self.max_speed
    }
}
impl Default for DragAutoscrollPolicy {
    fn default() -> Self {
        // Deliberately theme-free: this is input geometry, not visual chrome.
        Self {
            edge_zone: 32.0,
            max_speed: 800.0,
        }
    }
}
impl std::error::Error for DragDescriptorError {}

/// Checked text presentation for the existing runtime drag-preview overlay.
#[derive(Clone, Debug, PartialEq)]
pub struct DragPreviewInfo {
    label: Rc<str>,
    size: Vector2,
}
// All floating-point fields are private and checked, including the default.
impl Eq for DragPreviewInfo {}
impl DragPreviewInfo {
    /// Construct bounded preview metadata without allocating a render surface.
    pub fn new(label: impl Into<String>, size: Vector2) -> Result<Self, DragDescriptorError> {
        let label = label.into();
        if label.len() > 4096 {
            return Err(DragDescriptorError::PreviewLabelTooLong);
        }
        if !size.x.is_finite()
            || !size.y.is_finite()
            || size.x <= 0.0
            || size.y <= 0.0
            || size.x > 1024.0
            || size.y > 1024.0
        {
            return Err(DragDescriptorError::InvalidPreview);
        }
        Ok(Self {
            label: Rc::from(label),
            size,
        })
    }
    /// Read the preview label.
    pub fn label(&self) -> &str {
        &self.label
    }
    /// Read the logical preview dimensions.
    pub const fn size(&self) -> Vector2 {
        self.size
    }
}
impl Default for DragPreviewInfo {
    fn default() -> Self {
        Self {
            label: Rc::from("Item"),
            size: Vector2::new(128.0, 24.0),
        }
    }
}

/// An immutable, UI-local payload offer. Constructing an offer does not start a drag.
#[derive(Clone)]
pub struct DragOffer {
    payload: Rc<dyn Any>,
    operations: DragOperations,
    preview: DragPreviewInfo,
}
impl fmt::Debug for DragOffer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DragOffer")
            .field("operations", &self.operations)
            .field("preview", &self.preview)
            .finish_non_exhaustive()
    }
}
impl DragOffer {
    /// Construct a data-only offer retaining an application-owned payload.
    pub fn new<T: 'static>(
        payload: Rc<T>,
        operations: DragOperations,
        preview: DragPreviewInfo,
    ) -> Self {
        Self {
            payload,
            operations,
            preview,
        }
    }
    /// Borrow shared ownership of the payload only when its concrete type matches.
    pub fn payload<T: 'static>(&self) -> Option<Rc<T>> {
        self.payload.clone().downcast().ok()
    }
    /// Check the concrete payload type without invoking any callback.
    pub fn is<T: 'static>(&self) -> bool {
        self.payload.is::<T>()
    }
    /// Read the source's allowed operations.
    pub const fn operations(&self) -> DragOperations {
        self.operations
    }
    /// Read the bounded preview descriptor.
    pub fn preview(&self) -> &DragPreviewInfo {
        &self.preview
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drag_autoscroll_policy_is_checked_and_has_a_theme_free_default() {
        assert_eq!(DragAutoscrollPolicy::default().edge_zone(), 32.0);
        assert_eq!(DragAutoscrollPolicy::default().max_speed(), 800.0);
        for (zone, speed) in [
            (0.0, 1.0),
            (1.0, 0.0),
            (f32::NAN, 1.0),
            (1.0, f32::INFINITY),
        ] {
            assert_eq!(
                DragAutoscrollPolicy::new(zone, speed),
                Err(DragDescriptorError::InvalidAutoscrollPolicy)
            );
        }
    }
}

/// Opaque identity of a drag admitted through an exact gesture sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DragSessionToken(PointerSequenceToken);
impl DragSessionToken {
    pub(crate) const fn new(sequence: PointerSequenceToken) -> Self {
        Self(sequence)
    }
}
/// Current read-only context attached to one qualified drag lifecycle event.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragEventContext {
    pub(crate) token: DragSessionToken,
    pub(crate) source: u64,
    pub(crate) target: Option<u64>,
    pub(crate) position: Point,
    pub(crate) modifiers: crate::widgets::PointerModifiers,
}
impl DragEventContext {
    /// Read the originating session identity; this alone grants no action authority.
    pub const fn token(self) -> DragSessionToken {
        self.token
    }
    /// Read the originating source node identity.
    pub const fn source(self) -> u64 {
        self.source
    }
    /// Read the target associated with this event, if any.
    pub const fn target(self) -> Option<u64> {
        self.target
    }
    /// Read modifiers from the current checked gesture or pointer sample.
    pub const fn modifiers(self) -> crate::widgets::PointerModifiers {
        self.modifiers
    }
    /// Read the checked logical pointer position in the receiving surface.
    pub const fn position(self) -> Point {
        self.position
    }
}
/// Explicit target negotiation result. Pending and rejected offers cannot be dropped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropDecision {
    /// Application-owned validation has not completed.
    Pending,
    /// This target rejects the offered payload.
    Rejected,
    /// This target accepts one operation allowed by the source.
    Accepted(DragOperation),
}
/// Opt-in theme styles for transient drop-target outlines.
/// Colors resolve from the current theme; this descriptor owns no hover state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DropTargetFeedback {
    /// Style while an operation is accepted.
    pub accepted: crate::widgets::WidgetStyle,
    /// Style while application validation is pending.
    pub pending: crate::widgets::WidgetStyle,
    /// Style while the offered operation is rejected.
    pub rejected: crate::widgets::WidgetStyle,
}
impl DropTargetFeedback {
    /// Use success, warning, and danger tones from the active theme.
    pub const fn themed() -> Self {
        use crate::widgets::{WidgetStyle, WidgetTone};
        Self {
            accepted: WidgetStyle::strong(WidgetTone::Success),
            pending: WidgetStyle::normal(WidgetTone::Warning),
            rejected: WidgetStyle::strong(WidgetTone::Danger),
        }
    }
    pub(crate) const fn style(self, decision: DropDecision) -> crate::widgets::WidgetStyle {
        match decision {
            DropDecision::Accepted(_) => self.accepted,
            DropDecision::Pending => self.pending,
            DropDecision::Rejected => self.rejected,
        }
    }
}
/// Terminal reason for an in-application drag cancellation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragCancelReason {
    /// The source's gesture capture ended or was explicitly cancelled.
    CaptureLost,
    /// Source identity or behavior was retired during projection.
    SourceRetired,
    /// No current accepting target existed at release.
    NoTarget,
    /// Coordinates or continuation evidence became invalid.
    InvalidSample,
}
/// Source lifecycle. Completed and Cancelled are mutually exclusive terminal events.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragSourcePhase {
    /// The source crossed its recognition threshold and owns the drag.
    Started,
    /// Its checked pointer position changed.
    Moved,
    /// A qualified target accepted the terminal drop.
    Completed(DragOperation),
    /// The drag ended without a drop.
    Cancelled(DragCancelReason),
}
/// Target lifecycle delivered through the normal application update path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropPhase {
    /// This became the current typed target.
    Entered,
    /// Position or operation negotiation changed on the same target.
    Over,
    /// The pointer or target authority left this target.
    Left,
    /// The target accepted the terminal drop exactly once.
    Dropped,
    /// The source was cancelled while this target was current.
    Cancelled,
}
/// Typed source event retaining the original application payload.
pub struct DragSourceEvent<T> {
    pub(crate) payload: Rc<T>,
    pub(crate) context: DragEventContext,
    pub(crate) phase: DragSourcePhase,
}
impl<T> Clone for DragSourceEvent<T> {
    fn clone(&self) -> Self {
        Self {
            payload: self.payload.clone(),
            context: self.context,
            phase: self.phase,
        }
    }
}
impl<T> DragSourceEvent<T> {
    /// Borrow the original immutable payload.
    pub fn payload(&self) -> &T {
        &self.payload
    }
    /// Read source and session context.
    pub const fn context(&self) -> DragEventContext {
        self.context
    }
    /// Read the source lifecycle phase.
    pub const fn phase(&self) -> DragSourcePhase {
        self.phase
    }
}
/// Typed target event retaining the offered application payload.
pub struct DropEvent<T> {
    pub(crate) payload: Rc<T>,
    pub(crate) context: DragEventContext,
    pub(crate) phase: DropPhase,
    pub(crate) decision: DropDecision,
}
impl<T> Clone for DropEvent<T> {
    fn clone(&self) -> Self {
        Self {
            payload: self.payload.clone(),
            context: self.context,
            phase: self.phase,
            decision: self.decision,
        }
    }
}
impl<T> DropEvent<T> {
    /// Borrow the immutable offered payload.
    pub fn payload(&self) -> &T {
        &self.payload
    }
    /// Read qualified source, target and position context.
    pub const fn context(&self) -> DragEventContext {
        self.context
    }
    /// Read the target lifecycle phase.
    pub const fn phase(&self) -> DropPhase {
        self.phase
    }
    /// Read the checked operation negotiation result for this event.
    pub const fn decision(&self) -> DropDecision {
        self.decision
    }
}
