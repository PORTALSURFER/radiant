//! Typed, backend-neutral pointer and gesture ingress.
//!
//! This module deliberately stops at the controller boundary.  It carries the
//! evidence needed to validate a native sample without exposing native handles
//! or allowing a caller to manufacture a sequence token.

use std::num::NonZeroU64;

use super::{
    input::{InputSequenceRange, InputTimestamp},
    types::{Point, Vector2},
};
use crate::widgets::interaction::{PointerButton, PointerModifiers};

/// The physical family that produced a pointer sample.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DeviceKind {
    /// A conventional mouse.
    Mouse,
    /// A precision trackpad.
    Trackpad,
    /// A touch contact.
    Touch,
    /// A pen or stylus.
    Pen,
    /// A platform source that cannot be classified reliably.
    Unknown,
}

/// Error returned when a host identity is zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InvalidPointerIdentity;

/// An opaque nonzero identity assigned by a host to a physical input device.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InputDeviceId(NonZeroU64);

impl InputDeviceId {
    /// Construct an identity from a host value, rejecting zero.
    pub const fn from_host(value: u64) -> Result<Self, InvalidPointerIdentity> {
        match NonZeroU64::new(value) {
            Some(value) => Ok(Self(value)),
            None => Err(InvalidPointerIdentity),
        }
    }

    /// Alias for [`Self::from_host`].
    pub const fn new_checked(value: u64) -> Result<Self, InvalidPointerIdentity> {
        Self::from_host(value)
    }

    /// Construct an identity when zero is represented as `None`.
    pub const fn new(value: u64) -> Option<Self> {
        match NonZeroU64::new(value) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

/// An opaque nonzero identity assigned to one native contact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PointerContactId(NonZeroU64);

impl PointerContactId {
    /// Construct an identity from a host value, rejecting zero.
    pub const fn from_host(value: u64) -> Result<Self, InvalidPointerIdentity> {
        match NonZeroU64::new(value) {
            Some(value) => Ok(Self(value)),
            None => Err(InvalidPointerIdentity),
        }
    }

    /// Alias for [`Self::from_host`].
    pub const fn new_checked(value: u64) -> Result<Self, InvalidPointerIdentity> {
        Self::from_host(value)
    }

    /// Construct an identity when zero is represented as `None`.
    pub const fn new(value: u64) -> Option<Self> {
        match NonZeroU64::new(value) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

/// The three pointer buttons represented by a normalized sample.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PointerButtons(u8);

impl PointerButtons {
    /// Primary/left button bit.
    pub const PRIMARY: Self = Self(1);
    /// Secondary/right button bit.
    pub const SECONDARY: Self = Self(2);
    /// Auxiliary/middle button bit.
    pub const AUXILIARY: Self = Self(4);

    /// Construct an empty button set.
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Validate the three-bit host representation.
    pub const fn from_bits(bits: u8) -> Result<Self, InvalidPointerButtons> {
        if bits & !0b111 == 0 {
            Ok(Self(bits))
        } else {
            Err(InvalidPointerButtons)
        }
    }

    /// Return the raw three-bit representation.
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// Test whether all bits in `button` are present.
    pub const fn contains(self, button: Self) -> bool {
        self.0 & button.0 == button.0
    }

    /// Add one button to this set.
    pub const fn with(self, button: Self) -> Self {
        Self(self.0 | button.0)
    }
}

/// Error returned for a button bit outside the normalized three-button set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InvalidPointerButtons;

/// A finite normalized pressure value in the inclusive range `0..=1`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerPressure(f32);

impl PointerPressure {
    /// Validate a normalized pressure value.
    pub fn new(value: f32) -> Result<Self, InvalidPointerPressure> {
        (value.is_finite() && (0.0..=1.0).contains(&value))
            .then_some(Self(value))
            .ok_or(InvalidPointerPressure)
    }

    /// Read the validated pressure.
    pub const fn get(self) -> f32 {
        self.0
    }
}

impl Eq for PointerPressure {}

/// Error returned for a nonfinite or out-of-range pressure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InvalidPointerPressure;

/// Finite pen tilt on the two logical axes, in degrees.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerTilt {
    x: f32,
    y: f32,
}

impl PointerTilt {
    /// Validate both tilt axes in the inclusive `-90..=90` range.
    pub fn new(x: f32, y: f32) -> Result<Self, InvalidPointerTilt> {
        (x.is_finite()
            && y.is_finite()
            && (-90.0..=90.0).contains(&x)
            && (-90.0..=90.0).contains(&y))
        .then_some(Self { x, y })
        .ok_or(InvalidPointerTilt)
    }

    /// Return the x-axis tilt in degrees.
    pub const fn x(self) -> f32 {
        self.x
    }

    /// Return the y-axis tilt in degrees.
    pub const fn y(self) -> f32 {
        self.y
    }
}

impl Eq for PointerTilt {}

/// Error returned for a nonfinite or out-of-range tilt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InvalidPointerTilt;

/// Pointer lifecycle phase.  A hover has no sequence token or capture record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PointerPhase {
    /// A non-captured hover sample.
    Hover,
    /// A new captured sequence began.
    Started {
        /// Button that initiated the sequence.
        button: PointerButton,
    },
    /// A captured sequence moved.
    Moved,
    /// A captured sequence ended normally.
    Ended {
        /// Button that ended the sequence.
        button: PointerButton,
    },
    /// A captured sequence was cancelled.
    Cancelled,
}

impl PointerPhase {
    /// Return whether this phase terminates a sequence.
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Ended { .. } | Self::Cancelled)
    }
}

/// Runtime identity plus monotonic counter for one window runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PointerSequenceToken {
    runtime: NonZeroU64,
    counter: NonZeroU64,
}

impl PointerSequenceToken {
    pub(crate) const fn new(runtime: NonZeroU64, counter: NonZeroU64) -> Self {
        Self { runtime, counter }
    }
}

/// Errors returned by the bounded sequence allocator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum PointerSequenceAllocationError {
    /// A runtime identity of zero cannot fence tokens.
    InvalidRuntimeIdentity,
    /// The allocator reached the terminal counter value.
    Exhausted,
}

/// Runtime-owned token allocator.  Tokens are never reused after exhaustion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PointerSequenceAllocator {
    runtime: Option<NonZeroU64>,
    next: NonZeroU64,
    exhausted: bool,
}

impl Default for PointerSequenceAllocator {
    fn default() -> Self {
        Self {
            runtime: Some(NonZeroU64::MIN),
            next: NonZeroU64::MIN,
            exhausted: false,
        }
    }
}

impl PointerSequenceAllocator {
    pub(crate) const fn invalid() -> Self {
        Self {
            runtime: None,
            next: NonZeroU64::MIN,
            exhausted: false,
        }
    }

    /// Create an allocator for one nonzero runtime identity.
    pub(crate) const fn new(runtime_identity: u64) -> Result<Self, PointerSequenceAllocationError> {
        let Some(runtime) = NonZeroU64::new(runtime_identity) else {
            return Err(PointerSequenceAllocationError::InvalidRuntimeIdentity);
        };
        Ok(Self {
            runtime: Some(runtime),
            next: NonZeroU64::MIN,
            exhausted: false,
        })
    }

    /// Issue one token, permanently entering exhaustion after `u64::MAX`.
    pub(crate) fn issue(&mut self) -> Result<PointerSequenceToken, PointerSequenceAllocationError> {
        if self.exhausted {
            return Err(PointerSequenceAllocationError::Exhausted);
        }
        let Some(runtime) = self.runtime else {
            self.exhausted = true;
            return Err(PointerSequenceAllocationError::Exhausted);
        };
        let counter = self.next;
        if counter.get() == u64::MAX {
            self.exhausted = true;
        } else {
            self.next = match NonZeroU64::new(counter.get() + 1) {
                Some(next) => next,
                None => {
                    self.exhausted = true;
                    counter
                }
            };
        }
        Ok(PointerSequenceToken::new(runtime, counter))
    }
}

/// A validated normalized pointer sample.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerIngress {
    kind: DeviceKind,
    device: InputDeviceId,
    contact: PointerContactId,
    phase: PointerPhase,
    logical_position: Point,
    buttons: PointerButtons,
    modifiers: PointerModifiers,
    pressure: Option<PointerPressure>,
    tilt: Option<PointerTilt>,
    timestamp: Option<InputTimestamp>,
    sequence_range: Option<InputSequenceRange>,
    token: Option<PointerSequenceToken>,
}

impl PointerIngress {
    /// Build a checked pointer sample.  Started and Hover samples must omit a
    /// token; a controller supplies the token after admission.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        kind: DeviceKind,
        device: InputDeviceId,
        contact: PointerContactId,
        phase: PointerPhase,
        logical_position: Point,
        buttons: PointerButtons,
        modifiers: PointerModifiers,
        pressure: Option<PointerPressure>,
        tilt: Option<PointerTilt>,
        timestamp: Option<InputTimestamp>,
        sequence_range: Option<InputSequenceRange>,
    ) -> Result<Self, PointerIngressError> {
        if !logical_position.is_finite() {
            return Err(PointerIngressError::NonfinitePosition);
        }
        if matches!(phase, PointerPhase::Hover | PointerPhase::Started { .. }) {
            Ok(Self {
                kind,
                device,
                contact,
                phase,
                logical_position,
                buttons,
                modifiers,
                pressure,
                tilt,
                timestamp,
                sequence_range,
                token: None,
            })
        } else {
            Err(PointerIngressError::MissingSequenceToken)
        }
    }

    pub(crate) fn with_token(mut self, token: PointerSequenceToken) -> Self {
        self.token = Some(token);
        self
    }

    #[allow(clippy::too_many_arguments)]
    /// Build a continuation from a token previously returned by runtime
    /// admission. The token is opaque and cannot be minted by a host.
    pub fn from_runtime(
        kind: DeviceKind,
        device: InputDeviceId,
        contact: PointerContactId,
        phase: PointerPhase,
        logical_position: Point,
        buttons: PointerButtons,
        modifiers: PointerModifiers,
        pressure: Option<PointerPressure>,
        tilt: Option<PointerTilt>,
        timestamp: Option<InputTimestamp>,
        sequence_range: Option<InputSequenceRange>,
        token: PointerSequenceToken,
    ) -> Result<Self, PointerIngressError> {
        if matches!(phase, PointerPhase::Hover | PointerPhase::Started { .. }) {
            return Err(PointerIngressError::UnexpectedSequenceToken);
        }
        if !logical_position.is_finite() {
            return Err(PointerIngressError::NonfinitePosition);
        }
        Ok(Self {
            kind,
            device,
            contact,
            phase,
            logical_position,
            buttons,
            modifiers,
            pressure,
            tilt,
            timestamp,
            sequence_range,
            token: Some(token),
        })
    }

    /// The opaque continuation token carried by a runtime-created sample.
    pub const fn token(self) -> Option<PointerSequenceToken> {
        self.token
    }

    /// Build a continuation carrying the opaque token from an admitted event.
    #[allow(clippy::too_many_arguments)]
    pub fn continuation(
        event: &PointerEvent,
        phase: PointerPhase,
        logical_position: Point,
        buttons: PointerButtons,
        modifiers: PointerModifiers,
        pressure: Option<PointerPressure>,
        tilt: Option<PointerTilt>,
        timestamp: Option<InputTimestamp>,
        sequence_range: Option<InputSequenceRange>,
    ) -> Result<Self, PointerIngressError> {
        if matches!(phase, PointerPhase::Hover | PointerPhase::Started { .. }) {
            return Err(PointerIngressError::UnexpectedSequenceToken);
        }
        if !logical_position.is_finite() {
            return Err(PointerIngressError::NonfinitePosition);
        }
        Ok(Self {
            kind: event.kind,
            device: event.device,
            contact: event.contact,
            phase,
            logical_position,
            buttons,
            modifiers,
            pressure,
            tilt,
            timestamp,
            sequence_range,
            token: event.token,
        })
    }

    /// Device kind evidence.
    pub const fn kind(self) -> DeviceKind {
        self.kind
    }
    /// Device identity evidence.
    pub const fn device(self) -> InputDeviceId {
        self.device
    }
    /// Contact identity evidence.
    pub const fn contact(self) -> PointerContactId {
        self.contact
    }
    /// Pointer phase evidence.
    pub const fn phase(self) -> PointerPhase {
        self.phase
    }
    /// Surface logical position.
    pub const fn logical_position(self) -> Point {
        self.logical_position
    }
    /// Buttons held in this sample.
    pub const fn buttons(self) -> PointerButtons {
        self.buttons
    }
    /// Modifiers captured at arrival.
    pub const fn modifiers(self) -> PointerModifiers {
        self.modifiers
    }
    /// Optional checked pressure.
    pub const fn pressure(self) -> Option<PointerPressure> {
        self.pressure
    }
    /// Optional checked tilt.
    pub const fn tilt(self) -> Option<PointerTilt> {
        self.tilt
    }
    /// Optional arrival timestamp.
    pub const fn timestamp(self) -> Option<InputTimestamp> {
        self.timestamp
    }
    /// Optional native sample range.
    pub const fn sequence_range(self) -> Option<InputSequenceRange> {
        self.sequence_range
    }
}

/// Validation failures for a pointer sample.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PointerIngressError {
    /// Logical position contained a NaN or infinity.
    NonfinitePosition,
    /// A continuation omitted its sequence token.
    MissingSequenceToken,
    /// A token was supplied for a new sequence or hover.
    UnexpectedSequenceToken,
}

/// A consumer-facing pointer event admitted by a runtime sequence record.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerEvent {
    kind: DeviceKind,
    device: InputDeviceId,
    contact: PointerContactId,
    phase: PointerPhase,
    logical_position: Point,
    buttons: PointerButtons,
    modifiers: PointerModifiers,
    pressure: Option<PointerPressure>,
    tilt: Option<PointerTilt>,
    timestamp: Option<InputTimestamp>,
    sequence_range: Option<InputSequenceRange>,
    token: Option<PointerSequenceToken>,
}

impl PointerEvent {
    pub(crate) const fn from_ingress(
        ingress: PointerIngress,
        token: Option<PointerSequenceToken>,
    ) -> Self {
        Self {
            kind: ingress.kind,
            device: ingress.device,
            contact: ingress.contact,
            phase: ingress.phase,
            logical_position: ingress.logical_position,
            buttons: ingress.buttons,
            modifiers: ingress.modifiers,
            pressure: ingress.pressure,
            tilt: ingress.tilt,
            timestamp: ingress.timestamp,
            sequence_range: ingress.sequence_range,
            token,
        }
    }

    /// Device kind evidence.
    pub const fn kind(self) -> DeviceKind {
        self.kind
    }
    /// Device identity evidence.
    pub const fn device(self) -> InputDeviceId {
        self.device
    }
    /// Contact identity evidence.
    pub const fn contact(self) -> PointerContactId {
        self.contact
    }
    /// Pointer phase evidence.
    pub const fn phase(self) -> PointerPhase {
        self.phase
    }
    /// Surface logical position.
    pub const fn logical_position(self) -> Point {
        self.logical_position
    }
    /// Buttons held in this sample.
    pub const fn buttons(self) -> PointerButtons {
        self.buttons
    }
    /// Modifiers captured at arrival.
    pub const fn modifiers(self) -> PointerModifiers {
        self.modifiers
    }
    /// Optional pressure.
    pub const fn pressure(self) -> Option<PointerPressure> {
        self.pressure
    }
    /// Optional tilt.
    pub const fn tilt(self) -> Option<PointerTilt> {
        self.tilt
    }
    /// Optional timestamp.
    pub const fn timestamp(self) -> Option<InputTimestamp> {
        self.timestamp
    }
    /// Optional sample range.
    pub const fn sequence_range(self) -> Option<InputSequenceRange> {
        self.sequence_range
    }
    /// Copy the opaque token into a continuation without exposing its value.
    pub const fn sequence_token(self) -> Option<PointerSequenceToken> {
        self.token
    }
}

/// Normalized gesture family.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GestureKind {
    Pan,
    Pinch,
    Rotate,
}

/// Gesture lifecycle phase.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GesturePhase {
    Started,
    Changed,
    Ended,
    Cancelled,
}

/// Explicit units carried by a gesture sample.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GestureUnit {
    LogicalPixels,
    Scale,
    Radians,
}

/// A checked normalized gesture sample.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GestureIngress {
    kind: GestureKind,
    phase: GesturePhase,
    unit: GestureUnit,
    value: Vector2,
    device: InputDeviceId,
    anchor: Option<Point>,
    modifiers: PointerModifiers,
    timestamp: Option<InputTimestamp>,
    sequence_range: Option<InputSequenceRange>,
}

impl GestureIngress {
    /// Construct a checked gesture sample. Pan uses pixels, pinch uses scale,
    /// and rotate uses the x component in radians.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        kind: GestureKind,
        phase: GesturePhase,
        unit: GestureUnit,
        value: Vector2,
        device: InputDeviceId,
        anchor: Option<Point>,
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
        sequence_range: Option<InputSequenceRange>,
    ) -> Result<Self, GestureIngressError> {
        let expected = match kind {
            GestureKind::Pan => GestureUnit::LogicalPixels,
            GestureKind::Pinch => GestureUnit::Scale,
            GestureKind::Rotate => GestureUnit::Radians,
        };
        if unit != expected || !value.x.is_finite() || !value.y.is_finite() {
            return Err(GestureIngressError::InvalidUnitOrValue);
        }
        if anchor.is_some_and(|point| !point.is_finite()) {
            return Err(GestureIngressError::NonfiniteAnchor);
        }
        if matches!(kind, GestureKind::Pinch) && value.x <= 0.0 {
            return Err(GestureIngressError::InvalidUnitOrValue);
        }
        Ok(Self {
            kind,
            phase,
            unit,
            value,
            device,
            anchor,
            modifiers,
            timestamp,
            sequence_range,
        })
    }

    /// Construct a pan sample in logical pixels.
    pub fn pan(
        phase: GesturePhase,
        delta: Vector2,
        device: InputDeviceId,
        anchor: Option<Point>,
        modifiers: PointerModifiers,
    ) -> Result<Self, GestureIngressError> {
        Self::new(
            GestureKind::Pan,
            phase,
            GestureUnit::LogicalPixels,
            delta,
            device,
            anchor,
            modifiers,
            None,
            None,
        )
    }

    /// Construct a pinch sample as a positive scale factor.
    pub fn pinch(
        phase: GesturePhase,
        scale: f32,
        device: InputDeviceId,
        anchor: Option<Point>,
        modifiers: PointerModifiers,
    ) -> Result<Self, GestureIngressError> {
        Self::new(
            GestureKind::Pinch,
            phase,
            GestureUnit::Scale,
            Vector2::new(scale, 0.0),
            device,
            anchor,
            modifiers,
            None,
            None,
        )
    }

    /// Construct a rotate sample in radians.
    pub fn rotate(
        phase: GesturePhase,
        radians: f32,
        device: InputDeviceId,
        anchor: Option<Point>,
        modifiers: PointerModifiers,
    ) -> Result<Self, GestureIngressError> {
        Self::new(
            GestureKind::Rotate,
            phase,
            GestureUnit::Radians,
            Vector2::new(radians, 0.0),
            device,
            anchor,
            modifiers,
            None,
            None,
        )
    }

    /// Return the gesture family.
    pub const fn kind(self) -> GestureKind {
        self.kind
    }
    /// Return the lifecycle phase.
    pub const fn phase(self) -> GesturePhase {
        self.phase
    }
    /// Return the explicit unit.
    pub const fn unit(self) -> GestureUnit {
        self.unit
    }
    /// Return the checked value.
    pub const fn value(self) -> Vector2 {
        self.value
    }
    /// Return the source device identity.
    pub const fn device(self) -> InputDeviceId {
        self.device
    }
    /// Return the optional logical anchor.
    pub const fn anchor(self) -> Option<Point> {
        self.anchor
    }
    /// Return captured modifiers.
    pub const fn modifiers(self) -> PointerModifiers {
        self.modifiers
    }
    /// Return the optional arrival timestamp.
    pub const fn timestamp(self) -> Option<InputTimestamp> {
        self.timestamp
    }
    /// Return the optional native sample range.
    pub const fn sequence_range(self) -> Option<InputSequenceRange> {
        self.sequence_range
    }
}

/// Gesture validation failure.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GestureIngressError {
    InvalidUnitOrValue,
    NonfiniteAnchor,
}

/// Typed pointer routing disposition.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PointerIngressDisposition {
    RoutedWidget(crate::widgets::WidgetId),
    HandledLayout,
    HandledScrollbar,
    AdmittedUnsupportedConsumer,
    Blocked,
    Stale,
    CapacityExhausted,
    IdentityExhausted,
    Invalid,
}

/// Result of typed admission, including the opaque token required to submit
/// later move or terminal samples. Layout, scrollbar, and explicitly
/// unsupported consumers return their token here because they have no widget
/// callback through which to observe it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PointerIngressAdmission {
    disposition: PointerIngressDisposition,
    token: Option<PointerSequenceToken>,
}

impl PointerIngressAdmission {
    pub(crate) const fn new(
        disposition: PointerIngressDisposition,
        token: Option<PointerSequenceToken>,
    ) -> Self {
        Self { disposition, token }
    }

    /// The routing result for the admitted sample.
    pub const fn disposition(self) -> PointerIngressDisposition {
        self.disposition
    }

    /// The runtime-issued token, when a started sequence was admitted.
    pub const fn sequence_token(self) -> Option<PointerSequenceToken> {
        self.token
    }
}

/// Typed gesture routing disposition.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GestureIngressDisposition {
    Pending,
    Unrecognized,
    RoutedWidget(crate::widgets::WidgetId),
    RoutedContainer(crate::layout::NodeId),
    HandledLayout,
    HandledScrollbar,
    AdmittedUnsupportedConsumer,
    Blocked,
    Stale,
    Invalid,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids() -> (InputDeviceId, PointerContactId) {
        (
            InputDeviceId::from_host(1).expect("nonzero device"),
            PointerContactId::from_host(1).expect("nonzero contact"),
        )
    }

    #[test]
    fn host_id_and_physical_values_reject_malformed_evidence() {
        assert!(InputDeviceId::from_host(0).is_err());
        assert!(PointerContactId::from_host(0).is_err());
        assert!(PointerPressure::new(-0.1).is_err());
        assert!(PointerPressure::new(f32::NAN).is_err());
        assert!(PointerTilt::new(91.0, 0.0).is_err());
        assert!(PointerTilt::new(0.0, f32::INFINITY).is_err());
    }

    #[test]
    fn phase_validation_keeps_new_sequences_tokenless_and_continuations_opaque() {
        let (device, contact) = ids();
        let started = PointerIngress::new(
            DeviceKind::Mouse,
            device,
            contact,
            PointerPhase::Started {
                button: PointerButton::Primary,
            },
            Point::new(1.0, 2.0),
            PointerButtons::PRIMARY,
            PointerModifiers::default(),
            None,
            None,
            None,
            None,
        )
        .expect("valid started sample");
        assert_eq!(started.token(), None);
        assert!(
            PointerIngress::new(
                DeviceKind::Mouse,
                device,
                contact,
                PointerPhase::Moved,
                Point::new(1.0, 2.0),
                PointerButtons::PRIMARY,
                PointerModifiers::default(),
                None,
                None,
                None,
                None,
            )
            .is_err()
        );
    }

    #[test]
    fn sequence_allocator_fences_runtime_identity_and_exhausts() {
        let mut first = PointerSequenceAllocator::new(11).expect("runtime identity");
        let mut second = PointerSequenceAllocator::new(12).expect("runtime identity");
        let first_token = first.issue().expect("first token");
        let second_token = second.issue().expect("second token");
        assert_ne!(first_token, second_token);
        let mut max = PointerSequenceAllocator {
            runtime: Some(NonZeroU64::new(1).expect("nonzero")),
            next: NonZeroU64::new(u64::MAX).expect("nonzero"),
            exhausted: false,
        };
        assert!(max.issue().is_ok());
        assert_eq!(max.issue(), Err(PointerSequenceAllocationError::Exhausted));
    }
}
