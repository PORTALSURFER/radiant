//! Query-only keyed virtual-layout capability primitives.
//!
//! The public capability portion deliberately stops at a bounded policy query.
//! The private coordinator child adds query-only accepted-window reconciliation
//! and key continuity, but neither layer registers with a runtime, materializes
//! widgets, or schedules follow-up work.
//!
//! The current anchor contract is deliberately conservative: same-key anchor
//! correction requires the key to be present in both accepted bounded windows.
//! Bounded absence leaves an explicit key unresolved; it is not deletion
//! evidence and does not select a successor or predecessor.

use std::{any::Any, fmt, rc::Rc};

use crate::gui::{
    automation::AutomationNodeSemantics,
    types::{Point, Rect, Vector2},
};

use super::tree::NodeId;

mod adapter;
mod coordinator;
mod materialization;

pub(crate) use adapter::VirtualLayoutBatchProjector;
pub(crate) use coordinator::{
    VirtualLayoutCompletion, VirtualLayoutRetainReason, VirtualLayoutWindowCoordinator,
};
pub(crate) use materialization::{
    VirtualLayoutLifecycleAdapter, VirtualLayoutMaterializationError,
    VirtualLayoutMaterializationReentry, VirtualLayoutMaterializationStore,
    VirtualLayoutProjectionEvidence, VirtualLayoutProjectionKind,
};

/// Maximum number of keyed entries that one query may expose.
pub const VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES: usize = 1024;

const VIRTUAL_LAYOUT_MAX_DIAGNOSTICS: usize = 8;

trait ExactValue: Any {
    fn equals(&self, other: &dyn ExactValue) -> bool;
}

impl<T> ExactValue for T
where
    T: Eq + 'static,
{
    fn equals(&self, other: &dyn ExactValue) -> bool {
        other
            .as_any()
            .downcast_ref::<T>()
            .is_some_and(|candidate| self == candidate)
    }
}

impl dyn ExactValue {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Clone)]
struct OpaqueExactValue(Rc<dyn ExactValue>);

impl OpaqueExactValue {
    fn new<T>(value: T) -> Self
    where
        T: Eq + 'static,
    {
        Self(Rc::new(value))
    }

    fn equals(&self, other: &Self) -> bool {
        self.0.equals(&*other.0)
    }

    fn stable_equals(&self, other: &Self) -> Option<bool> {
        let first = self.equals(other);
        let reverse = other.equals(self);
        let second = self.equals(other);
        (first == reverse && first == second).then_some(first)
    }
}

impl PartialEq for OpaqueExactValue {
    fn eq(&self, other: &Self) -> bool {
        self.equals(other)
    }
}

impl Eq for OpaqueExactValue {}

/// Opaque exact identity for one registered virtual-layout policy.
///
/// Values are compared by their concrete type and `Eq` implementation. The
/// value is never exposed, hashed, formatted, or used as a pointer identity.
#[derive(Clone)]
pub struct VirtualLayoutPolicyIdentity {
    value: OpaqueExactValue,
}

impl VirtualLayoutPolicyIdentity {
    /// Construct an exact policy identity from a caller-owned value.
    #[must_use]
    pub fn new<T>(value: T) -> Self
    where
        T: Eq + 'static,
    {
        Self {
            value: OpaqueExactValue::new(value),
        }
    }

    fn stable_equals(&self, other: &Self) -> Option<bool> {
        self.value.stable_equals(&other.value)
    }
}

impl fmt::Debug for VirtualLayoutPolicyIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VirtualLayoutPolicyIdentity(..)")
    }
}

impl PartialEq for VirtualLayoutPolicyIdentity {
    fn eq(&self, other: &Self) -> bool {
        self.stable_equals(other) == Some(true)
    }
}

impl Eq for VirtualLayoutPolicyIdentity {}

/// Opaque exact identity for one logical virtual-layout item.
///
/// Item keys are compared by their concrete type and `Eq` implementation.
/// They are independent of logical index, allocation, frame, and pointer
/// identity.
#[derive(Clone)]
pub struct VirtualLayoutItemKey {
    value: OpaqueExactValue,
}

impl VirtualLayoutItemKey {
    /// Construct an exact item key from a caller-owned value.
    #[must_use]
    pub fn new<T>(value: T) -> Self
    where
        T: Eq + 'static,
    {
        Self {
            value: OpaqueExactValue::new(value),
        }
    }

    fn stable_equals(&self, other: &Self) -> Option<bool> {
        self.value.stable_equals(&other.value)
    }
}

impl fmt::Debug for VirtualLayoutItemKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VirtualLayoutItemKey(..)")
    }
}

impl PartialEq for VirtualLayoutItemKey {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl Eq for VirtualLayoutItemKey {}

/// Typed provider-side unavailable reasons for one semantic lookup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum VirtualLayoutSemanticUnavailableReason {
    NoProvider,
    DataUnavailable,
    Unsupported,
}

/// Typed provider-side deferred reasons for one semantic lookup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum VirtualLayoutSemanticDeferredReason {
    DataPending,
    SemanticPending,
    Retry,
}

/// Typed provider rejection reasons. Structural malformed output is detected
/// by the runtime after the provider returns and is reported separately.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum VirtualLayoutSemanticRejectedReason {
    UnknownContainer,
    Retired,
    ScopeMismatch,
    Stale,
    WrongKey,
    UnstableKey,
    NonFiniteBounds,
    InvertedBounds,
    ProviderRejected,
}

/// One exact, immutable request for a bounded semantic lookup.
///
/// This boundary is crate-private on purpose. Providers receive only the
/// mounted identity and one key; they do not receive runtime, materializer,
/// scheduler, renderer, or lifecycle handles.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct VirtualLayoutSemanticRequest {
    container_id: NodeId,
    policy_identity: VirtualLayoutPolicyIdentity,
    mount_generation: u64,
    semantic_revision: u64,
    key: VirtualLayoutItemKey,
}

#[allow(dead_code)]
impl VirtualLayoutSemanticRequest {
    pub(crate) fn new(
        container_id: NodeId,
        policy_identity: VirtualLayoutPolicyIdentity,
        mount_generation: u64,
        semantic_revision: u64,
        key: VirtualLayoutItemKey,
    ) -> Self {
        Self {
            container_id,
            policy_identity,
            mount_generation,
            semantic_revision,
            key,
        }
    }

    pub(crate) const fn container_id(&self) -> NodeId {
        self.container_id
    }

    pub(crate) fn policy_identity(&self) -> &VirtualLayoutPolicyIdentity {
        &self.policy_identity
    }

    pub(crate) const fn mount_generation(&self) -> u64 {
        self.mount_generation
    }

    pub(crate) const fn semantic_revision(&self) -> u64 {
        self.semantic_revision
    }

    pub(crate) fn key(&self) -> &VirtualLayoutItemKey {
        &self.key
    }

    /// Validate the exact mounted scope and semantic revision without side
    /// effects or access to runtime-owned state.
    pub(crate) fn validate_scope(
        &self,
        container_id: NodeId,
        policy_identity: &VirtualLayoutPolicyIdentity,
        mount_generation: u64,
        semantic_revision: u64,
    ) -> Result<(), VirtualLayoutSemanticRejectedReason> {
        if self.container_id != container_id
            || self.policy_identity.stable_equals(policy_identity) != Some(true)
        {
            return Err(VirtualLayoutSemanticRejectedReason::ScopeMismatch);
        }
        if self.mount_generation != mount_generation || self.semantic_revision != semantic_revision
        {
            return Err(VirtualLayoutSemanticRejectedReason::Stale);
        }
        Ok(())
    }
}

/// One bounded semantic entry returned by a private provider.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct VirtualLayoutSemanticEntry {
    requested_key: VirtualLayoutItemKey,
    logical_index: usize,
    bounds: Rect,
    semantics: AutomationNodeSemantics,
}

#[allow(dead_code)]
impl VirtualLayoutSemanticEntry {
    pub(crate) fn new(
        requested_key: VirtualLayoutItemKey,
        logical_index: usize,
        bounds: Rect,
        semantics: AutomationNodeSemantics,
    ) -> Self {
        Self {
            requested_key,
            logical_index,
            bounds,
            semantics,
        }
    }

    pub(crate) fn requested_key(&self) -> &VirtualLayoutItemKey {
        &self.requested_key
    }

    pub(crate) const fn logical_index(&self) -> usize {
        self.logical_index
    }

    pub(crate) const fn bounds(&self) -> Rect {
        self.bounds
    }

    pub(crate) fn semantics(&self) -> &AutomationNodeSemantics {
        &self.semantics
    }

    /// Validate the requested key and finite, non-inverted semantic bounds.
    pub(crate) fn validate_for(
        &self,
        request: &VirtualLayoutSemanticRequest,
    ) -> Result<(), VirtualLayoutSemanticRejectedReason> {
        match self.requested_key.stable_equals(request.key()) {
            Some(true) => {}
            Some(false) => return Err(VirtualLayoutSemanticRejectedReason::WrongKey),
            None => return Err(VirtualLayoutSemanticRejectedReason::UnstableKey),
        }
        if !self.bounds.is_finite() {
            return Err(VirtualLayoutSemanticRejectedReason::NonFiniteBounds);
        }
        if self.bounds.min.x > self.bounds.max.x || self.bounds.min.y > self.bounds.max.y {
            return Err(VirtualLayoutSemanticRejectedReason::InvertedBounds);
        }
        Ok(())
    }
}

/// Provider result for one bounded semantic request.
#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub(crate) enum VirtualLayoutSemanticQueryOutcome {
    Found(Box<VirtualLayoutSemanticEntry>),
    NotFound,
    Unavailable(VirtualLayoutSemanticUnavailableReason),
    Deferred(VirtualLayoutSemanticDeferredReason),
    Rejected(VirtualLayoutSemanticRejectedReason),
}

/// Crate-private immutable provider boundary for semantic lookup.
#[allow(dead_code)]
pub(crate) trait VirtualLayoutSemanticProvider {
    fn lookup(&self, request: &VirtualLayoutSemanticRequest) -> VirtualLayoutSemanticQueryOutcome;
}

/// One bounded semantic pin retained by a mounted runtime record.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct VirtualLayoutSemanticPin {
    request: VirtualLayoutSemanticRequest,
    entry: VirtualLayoutSemanticEntry,
}

#[allow(dead_code)]
impl VirtualLayoutSemanticPin {
    pub(crate) fn new(
        request: VirtualLayoutSemanticRequest,
        entry: VirtualLayoutSemanticEntry,
    ) -> Self {
        Self { request, entry }
    }

    pub(crate) fn request(&self) -> &VirtualLayoutSemanticRequest {
        &self.request
    }

    pub(crate) fn entry(&self) -> &VirtualLayoutSemanticEntry {
        &self.entry
    }
}

/// Coordinate-space identity included in every query fence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VirtualLayoutCoordinateSpace {
    /// The backend-neutral logical coordinate space.
    Logical,
    /// A caller-defined coordinate space with exact identity.
    Custom(VirtualLayoutPolicyIdentity),
}

impl VirtualLayoutCoordinateSpace {
    /// Return the backend-neutral logical coordinate space.
    #[must_use]
    pub const fn logical() -> Self {
        Self::Logical
    }

    /// Construct a caller-defined coordinate space with exact identity.
    #[must_use]
    pub fn custom(identity: VirtualLayoutPolicyIdentity) -> Self {
        Self::Custom(identity)
    }
}

/// Finite leading and trailing overscan evidence for one query.
#[derive(Clone, Copy, Debug)]
pub struct VirtualLayoutOverscan {
    leading: f32,
    trailing: f32,
}

impl VirtualLayoutOverscan {
    /// Construct non-negative finite leading and trailing overscan.
    pub fn new(leading: f32, trailing: f32) -> Result<Self, VirtualLayoutInputError> {
        if !leading.is_finite() || !trailing.is_finite() {
            return Err(VirtualLayoutInputError::NonFiniteOverscan);
        }
        if leading < 0.0 || trailing < 0.0 {
            return Err(VirtualLayoutInputError::NegativeOverscan);
        }
        Ok(Self { leading, trailing })
    }

    /// Return the leading overscan distance.
    #[must_use]
    pub const fn leading(self) -> f32 {
        self.leading
    }

    /// Return the trailing overscan distance.
    #[must_use]
    pub const fn trailing(self) -> f32 {
        self.trailing
    }

    fn is_valid(self) -> bool {
        self.leading.is_finite()
            && self.trailing.is_finite()
            && self.leading >= 0.0
            && self.trailing >= 0.0
    }
}

impl PartialEq for VirtualLayoutOverscan {
    fn eq(&self, other: &Self) -> bool {
        self.leading.to_bits() == other.leading.to_bits()
            && self.trailing.to_bits() == other.trailing.to_bits()
    }
}

impl Eq for VirtualLayoutOverscan {}

/// Caller-provided maximum number of keyed entries for one query.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VirtualLayoutBudget {
    max_entries: usize,
}

impl VirtualLayoutBudget {
    /// Construct a budget. Zero is valid for an extent-only query.
    #[must_use]
    pub const fn new(max_entries: usize) -> Self {
        Self { max_entries }
    }

    /// Return the caller-provided entry limit before the library hard cap.
    #[must_use]
    pub const fn max_entries(self) -> usize {
        self.max_entries
    }
}

/// Typed input validation failures detected before a policy is called.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VirtualLayoutInputError {
    /// The viewport contains a non-finite coordinate or derived extent.
    NonFiniteViewport,
    /// The viewport minimum exceeds its maximum on at least one axis.
    InvertedViewport,
    /// Overscan contains a non-finite value.
    NonFiniteOverscan,
    /// Overscan contains a negative value.
    NegativeOverscan,
}

impl VirtualLayoutInputError {
    fn diagnostic_code(self) -> VirtualLayoutDiagnosticCode {
        match self {
            Self::NonFiniteViewport => VirtualLayoutDiagnosticCode::InputNonFiniteViewport,
            Self::InvertedViewport => VirtualLayoutDiagnosticCode::InputInvertedViewport,
            Self::NonFiniteOverscan => VirtualLayoutDiagnosticCode::InputNonFiniteOverscan,
            Self::NegativeOverscan => VirtualLayoutDiagnosticCode::InputNegativeOverscan,
        }
    }
}

/// Named construction fields for one immutable virtual-layout query.
#[derive(Clone, Debug)]
pub struct VirtualLayoutQueryInputParts {
    /// Stable mounted container identity.
    pub container_id: NodeId,
    /// Exact identity of the policy answering the query.
    pub policy_identity: VirtualLayoutPolicyIdentity,
    /// Generation of the mounted container instance.
    pub mount_generation: u64,
    /// Monotonic query sequence selected by the executor owner.
    pub query_sequence: u64,
    /// Finite, non-inverted logical viewport.
    pub viewport: Rect,
    /// Exact coordinate-space declaration for viewport and item bounds.
    pub coordinate_space: VirtualLayoutCoordinateSpace,
    /// Finite leading and trailing overscan.
    pub overscan: VirtualLayoutOverscan,
    /// Caller-provided bounded entry budget.
    pub budget: VirtualLayoutBudget,
    /// Exact viewport revision.
    pub viewport_revision: u64,
    /// Exact data snapshot revision.
    pub data_revision: u64,
    /// Exact policy configuration revision.
    pub policy_revision: u64,
    /// Exact accepted-measurement revision.
    pub measurement_revision: u64,
    /// Exact semantic-data revision.
    pub semantic_revision: u64,
}

/// Immutable, validated input supplied to a virtual-layout policy.
#[derive(Clone, Debug)]
pub struct VirtualLayoutQueryInput {
    parts: VirtualLayoutQueryInputParts,
}

impl VirtualLayoutQueryInput {
    /// Construct and validate an immutable query input from named parts.
    pub fn from_parts(
        parts: VirtualLayoutQueryInputParts,
    ) -> Result<Self, VirtualLayoutInputError> {
        if !parts.viewport.is_finite() {
            return Err(VirtualLayoutInputError::NonFiniteViewport);
        }
        if parts.viewport.min.x > parts.viewport.max.x
            || parts.viewport.min.y > parts.viewport.max.y
        {
            return Err(VirtualLayoutInputError::InvertedViewport);
        }
        if !parts.overscan.is_valid() {
            return Err(
                if parts.overscan.leading.is_finite() && parts.overscan.trailing.is_finite() {
                    VirtualLayoutInputError::NegativeOverscan
                } else {
                    VirtualLayoutInputError::NonFiniteOverscan
                },
            );
        }
        Ok(Self { parts })
    }

    /// Alias for [`Self::from_parts`].
    pub fn new(parts: VirtualLayoutQueryInputParts) -> Result<Self, VirtualLayoutInputError> {
        Self::from_parts(parts)
    }

    /// Return the stable mounted container identity.
    #[must_use]
    pub const fn container_id(&self) -> NodeId {
        self.parts.container_id
    }

    /// Return the exact policy identity.
    #[must_use]
    pub fn policy_identity(&self) -> &VirtualLayoutPolicyIdentity {
        &self.parts.policy_identity
    }

    /// Return the mount generation.
    #[must_use]
    pub const fn mount_generation(&self) -> u64 {
        self.parts.mount_generation
    }

    /// Return the query sequence.
    #[must_use]
    pub const fn query_sequence(&self) -> u64 {
        self.parts.query_sequence
    }

    /// Return the finite viewport.
    #[must_use]
    pub const fn viewport(&self) -> Rect {
        self.parts.viewport
    }

    /// Return the exact coordinate-space declaration.
    #[must_use]
    pub fn coordinate_space(&self) -> &VirtualLayoutCoordinateSpace {
        &self.parts.coordinate_space
    }

    /// Return the finite overscan evidence.
    #[must_use]
    pub const fn overscan(&self) -> VirtualLayoutOverscan {
        self.parts.overscan
    }

    /// Return the caller-provided entry budget.
    #[must_use]
    pub const fn budget(&self) -> VirtualLayoutBudget {
        self.parts.budget
    }

    /// Return the exact viewport revision.
    #[must_use]
    pub const fn viewport_revision(&self) -> u64 {
        self.parts.viewport_revision
    }

    /// Return the exact data revision.
    #[must_use]
    pub const fn data_revision(&self) -> u64 {
        self.parts.data_revision
    }

    /// Return the exact policy revision.
    #[must_use]
    pub const fn policy_revision(&self) -> u64 {
        self.parts.policy_revision
    }

    /// Return the exact measurement revision.
    #[must_use]
    pub const fn measurement_revision(&self) -> u64 {
        self.parts.measurement_revision
    }

    /// Return the exact semantic revision.
    #[must_use]
    pub const fn semantic_revision(&self) -> u64 {
        self.parts.semantic_revision
    }
}

/// Visible-window classification for one raw candidate and validated entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VirtualLayoutVisibility {
    /// The candidate belongs to the visible viewport window.
    Visible,
    /// The candidate belongs only to the finite overscan window.
    Overscan,
}

/// Confidence attached to one candidate's bounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VirtualLayoutBoundsConfidence {
    /// Bounds are exact for the supplied snapshot and revisions.
    Exact,
    /// Bounds are a finite estimate that remains fenced to this query.
    Estimated,
}

/// Raw key status submitted by a policy before executor validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VirtualLayoutItemKeyCandidate {
    /// No key could be resolved for this candidate.
    Missing,
    /// More than one key or record could satisfy this candidate.
    Ambiguous,
    /// One exact opaque key was resolved.
    Exact(VirtualLayoutItemKey),
}

/// Raw, unvalidated keyed geometry submitted through the executor-owned sink.
#[derive(Clone, Debug, PartialEq)]
pub struct VirtualLayoutItemCandidate {
    key: VirtualLayoutItemKeyCandidate,
    logical_index: usize,
    bounds: Rect,
    visibility: VirtualLayoutVisibility,
    confidence: VirtualLayoutBoundsConfidence,
}

impl VirtualLayoutItemCandidate {
    /// Construct a raw candidate with one exact key.
    #[must_use]
    pub fn new(
        key: VirtualLayoutItemKey,
        logical_index: usize,
        bounds: Rect,
        visibility: VirtualLayoutVisibility,
        confidence: VirtualLayoutBoundsConfidence,
    ) -> Self {
        Self::from_key_candidate(
            VirtualLayoutItemKeyCandidate::Exact(key),
            logical_index,
            bounds,
            visibility,
            confidence,
        )
    }

    /// Construct a raw candidate with an unresolved key.
    #[must_use]
    pub fn missing_key(
        logical_index: usize,
        bounds: Rect,
        visibility: VirtualLayoutVisibility,
        confidence: VirtualLayoutBoundsConfidence,
    ) -> Self {
        Self::from_key_candidate(
            VirtualLayoutItemKeyCandidate::Missing,
            logical_index,
            bounds,
            visibility,
            confidence,
        )
    }

    /// Construct a raw candidate whose key resolution was ambiguous.
    #[must_use]
    pub fn ambiguous_key(
        logical_index: usize,
        bounds: Rect,
        visibility: VirtualLayoutVisibility,
        confidence: VirtualLayoutBoundsConfidence,
    ) -> Self {
        Self::from_key_candidate(
            VirtualLayoutItemKeyCandidate::Ambiguous,
            logical_index,
            bounds,
            visibility,
            confidence,
        )
    }

    /// Construct a raw candidate from an explicit raw key status.
    #[must_use]
    pub fn from_key_candidate(
        key: VirtualLayoutItemKeyCandidate,
        logical_index: usize,
        bounds: Rect,
        visibility: VirtualLayoutVisibility,
        confidence: VirtualLayoutBoundsConfidence,
    ) -> Self {
        Self {
            key,
            logical_index,
            bounds,
            visibility,
            confidence,
        }
    }

    /// Return the raw key status.
    #[must_use]
    pub fn key_candidate(&self) -> &VirtualLayoutItemKeyCandidate {
        &self.key
    }

    /// Return the logical index hint.
    #[must_use]
    pub const fn logical_index(&self) -> usize {
        self.logical_index
    }

    /// Return the raw candidate bounds.
    #[must_use]
    pub const fn bounds(&self) -> Rect {
        self.bounds
    }

    /// Return the visible/overscan classification.
    #[must_use]
    pub const fn visibility(&self) -> VirtualLayoutVisibility {
        self.visibility
    }

    /// Return the raw bounds confidence.
    #[must_use]
    pub const fn confidence(&self) -> VirtualLayoutBoundsConfidence {
        self.confidence
    }
}

/// Raw extent status submitted through the executor-owned sink.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VirtualLayoutExtentCandidate {
    /// A finite exact logical content extent candidate.
    Exact(Vector2),
    /// A finite estimated logical content extent candidate.
    Estimated(Vector2),
    /// The policy cannot provide a usable extent for this query.
    Unavailable,
}

impl VirtualLayoutExtentCandidate {
    /// Construct an exact raw extent candidate.
    #[must_use]
    pub const fn exact(size: Vector2) -> Self {
        Self::Exact(size)
    }

    /// Construct an estimated raw extent candidate.
    #[must_use]
    pub const fn estimated(size: Vector2) -> Self {
        Self::Estimated(size)
    }

    /// Construct an unavailable raw extent candidate.
    #[must_use]
    pub const fn unavailable() -> Self {
        Self::Unavailable
    }
}

/// Exactness state of a validated logical content extent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VirtualLayoutExtentKind {
    /// The extent is exact for the fenced query.
    Exact,
    /// The extent is a finite estimate for the fenced query.
    Estimated,
    /// No usable extent is available.
    Unavailable,
}

/// Validated logical content extent returned by an accepted query.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VirtualLayoutExtent {
    kind: VirtualLayoutExtentKind,
    size: Option<Vector2>,
}

impl VirtualLayoutExtent {
    /// Return whether the extent is exact.
    #[must_use]
    pub const fn is_exact(self) -> bool {
        matches!(self.kind, VirtualLayoutExtentKind::Exact)
    }

    /// Return whether the extent is estimated.
    #[must_use]
    pub const fn is_estimated(self) -> bool {
        matches!(self.kind, VirtualLayoutExtentKind::Estimated)
    }

    /// Return whether no usable extent is available.
    #[must_use]
    pub const fn is_unavailable(self) -> bool {
        matches!(self.kind, VirtualLayoutExtentKind::Unavailable)
    }

    /// Return the validated extent kind.
    #[must_use]
    pub const fn kind(self) -> VirtualLayoutExtentKind {
        self.kind
    }

    /// Return the finite extent size when one was supplied.
    #[must_use]
    pub const fn size(self) -> Option<Vector2> {
        self.size
    }
}

/// Typed reason recorded for an unavailable policy query.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VirtualLayoutUnavailableReason {
    /// The bounded data snapshot cannot answer this query.
    DataUnavailable,
    /// The policy does not support this query shape.
    Unsupported,
}

/// Typed reason recorded for a deferred policy query.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VirtualLayoutDeferredReason {
    /// The data needed by the bounded query is pending.
    DataPending,
    /// Measurement evidence needed by the query is pending.
    MeasurementPending,
    /// The caller should issue a later query with a new sequence.
    Retry,
}

/// Typed structural validation code retained in bounded query diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VirtualLayoutDiagnosticCode {
    /// Input viewport had non-finite coordinates or extent.
    InputNonFiniteViewport,
    /// Input viewport was inverted.
    InputInvertedViewport,
    /// Input overscan had a non-finite value.
    InputNonFiniteOverscan,
    /// Input overscan had a negative value.
    InputNegativeOverscan,
    /// The policy submitted more entries than the admitted budget.
    OutputOverBudget,
    /// A candidate did not provide a key.
    OutputMissingKey,
    /// A candidate reported ambiguous key resolution.
    OutputAmbiguousKey,
    /// Two candidates exposed equal exact keys.
    OutputDuplicateKey,
    /// Key equality was not stable during validation.
    OutputUnstableKey,
    /// Two candidates exposed the same logical index.
    OutputDuplicateIndex,
    /// Candidate bounds or their derived extents were non-finite.
    OutputNonFiniteBounds,
    /// Candidate bounds were inverted.
    OutputInvertedBounds,
    /// The policy supplied an invalid extent or no extent.
    OutputInvalidExtent,
    /// The policy supplied no extent candidate.
    OutputMissingExtent,
    /// The policy supplied more than one extent candidate.
    OutputDuplicateExtent,
    /// A non-ready policy answer included output or diagnostics.
    OutputUnexpectedForDisposition,
    /// The policy explicitly rejected its own query answer.
    PolicyRejected,
    /// The result fence differed from the executor's current fence.
    FenceMismatch,
}

/// One exact field in a query fence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VirtualLayoutFenceField {
    /// Mounted container identity.
    ContainerIdentity,
    /// Registered policy identity.
    PolicyIdentity,
    /// Mount generation.
    MountGeneration,
    /// Query sequence.
    QuerySequence,
    /// Viewport revision.
    ViewportRevision,
    /// Data revision.
    DataRevision,
    /// Policy revision.
    PolicyRevision,
    /// Measurement revision.
    MeasurementRevision,
    /// Semantic revision.
    SemanticRevision,
    /// Exact viewport geometry.
    Viewport,
    /// Exact coordinate-space identity.
    CoordinateSpace,
    /// Exact overscan evidence.
    Overscan,
    /// Exact caller budget evidence.
    Budget,
}

/// Bounded set of fence fields that differ during exact acceptance.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VirtualLayoutFenceFields {
    bits: u16,
}

impl VirtualLayoutFenceFields {
    /// Return whether no fence fields differ.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.bits == 0
    }

    /// Return whether one exact field is present in this mismatch set.
    #[must_use]
    pub const fn contains(self, field: VirtualLayoutFenceField) -> bool {
        self.bits & field.bit() != 0
    }

    const fn with(mut self, field: VirtualLayoutFenceField) -> Self {
        self.bits |= field.bit();
        self
    }
}

impl VirtualLayoutFenceField {
    const fn bit(self) -> u16 {
        1_u16 << (self as u16)
    }
}

/// Bounded typed diagnostic retained for an invalid query.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VirtualLayoutDiagnostic {
    code: VirtualLayoutDiagnosticCode,
    entry_position: Option<usize>,
    logical_index: Option<usize>,
    fence_fields: VirtualLayoutFenceFields,
}

impl VirtualLayoutDiagnostic {
    /// Return the diagnostic code.
    #[must_use]
    pub const fn code(self) -> VirtualLayoutDiagnosticCode {
        self.code
    }

    /// Return the bounded raw-entry position associated with this diagnostic.
    #[must_use]
    pub const fn entry_position(self) -> Option<usize> {
        self.entry_position
    }

    /// Return the logical index associated with this diagnostic.
    #[must_use]
    pub const fn logical_index(self) -> Option<usize> {
        self.logical_index
    }

    /// Return the exact fence fields associated with a fence mismatch.
    #[must_use]
    pub const fn fence_fields(self) -> VirtualLayoutFenceFields {
        self.fence_fields
    }
}

/// Bounded diagnostic collection with no retained arbitrary strings.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VirtualLayoutDiagnostics {
    records: [Option<VirtualLayoutDiagnostic>; VIRTUAL_LAYOUT_MAX_DIAGNOSTICS],
    len: usize,
    omitted: usize,
}

impl VirtualLayoutDiagnostics {
    /// Return the number of retained diagnostics.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Return whether no diagnostics were retained.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Return the number of diagnostics omitted after the fixed cap.
    #[must_use]
    pub const fn omitted_count(&self) -> usize {
        self.omitted
    }

    /// Return one retained diagnostic by bounded position.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<VirtualLayoutDiagnostic> {
        self.records.get(index).and_then(|record| *record)
    }

    /// Iterate over the retained diagnostics without allocating.
    pub fn iter(&self) -> impl Iterator<Item = &VirtualLayoutDiagnostic> {
        self.records[..self.len].iter().filter_map(Option::as_ref)
    }

    fn push(&mut self, diagnostic: VirtualLayoutDiagnostic) {
        if self.len < VIRTUAL_LAYOUT_MAX_DIAGNOSTICS {
            self.records[self.len] = Some(diagnostic);
            self.len += 1;
        } else {
            self.omitted = self.omitted.saturating_add(1);
        }
    }

    fn code(&mut self, code: VirtualLayoutDiagnosticCode) {
        self.push(VirtualLayoutDiagnostic {
            code,
            entry_position: None,
            logical_index: None,
            fence_fields: VirtualLayoutFenceFields::default(),
        });
    }

    fn entry(
        &mut self,
        code: VirtualLayoutDiagnosticCode,
        entry_position: usize,
        logical_index: Option<usize>,
    ) {
        self.push(VirtualLayoutDiagnostic {
            code,
            entry_position: Some(entry_position),
            logical_index,
            fence_fields: VirtualLayoutFenceFields::default(),
        });
    }

    fn fence_mismatch(&mut self, fields: VirtualLayoutFenceFields) {
        self.push(VirtualLayoutDiagnostic {
            code: VirtualLayoutDiagnosticCode::FenceMismatch,
            entry_position: None,
            logical_index: None,
            fence_fields: fields,
        });
    }
}

/// Errors returned when a policy tries to exceed the executor-owned sink.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VirtualLayoutSinkError {
    /// The candidate would exceed the admitted entry budget.
    OverBudget,
    /// An extent was already supplied for this query.
    ExtentAlreadyProvided,
}

/// Executor-owned bounded raw output sink.
///
/// A policy receives this sink by mutable reference and may submit candidates
/// one at a time. The sink has no public constructor, and it exposes no fence,
/// runtime, widget, message, lifecycle, scheduler, or materializer handle.
pub struct VirtualLayoutQuerySink {
    entries: Vec<VirtualLayoutItemCandidate>,
    extent: Option<VirtualLayoutExtentCandidate>,
    diagnostics: VirtualLayoutDiagnostics,
    max_entries: usize,
    over_budget: bool,
}

impl VirtualLayoutQuerySink {
    fn new(max_entries: usize) -> Self {
        Self {
            entries: Vec::with_capacity(max_entries),
            extent: None,
            diagnostics: VirtualLayoutDiagnostics::default(),
            max_entries,
            over_budget: false,
        }
    }

    /// Submit one raw candidate. Over-budget submission invalidates the whole
    /// query; the candidate is not silently truncated into a valid result.
    pub fn visit_item(
        &mut self,
        candidate: VirtualLayoutItemCandidate,
    ) -> Result<(), VirtualLayoutSinkError> {
        if self.entries.len() >= self.max_entries {
            self.over_budget = true;
            self.diagnostics
                .code(VirtualLayoutDiagnosticCode::OutputOverBudget);
            return Err(VirtualLayoutSinkError::OverBudget);
        }
        self.entries.push(candidate);
        Ok(())
    }

    /// Alias for [`Self::visit_item`] for visitor-style policy code.
    pub fn visit(
        &mut self,
        candidate: VirtualLayoutItemCandidate,
    ) -> Result<(), VirtualLayoutSinkError> {
        self.visit_item(candidate)
    }

    /// Supply the one raw extent candidate for this query.
    pub fn set_extent(
        &mut self,
        extent: VirtualLayoutExtentCandidate,
    ) -> Result<(), VirtualLayoutSinkError> {
        if self.extent.is_some() {
            self.diagnostics
                .code(VirtualLayoutDiagnosticCode::OutputDuplicateExtent);
            return Err(VirtualLayoutSinkError::ExtentAlreadyProvided);
        }
        self.extent = Some(extent);
        Ok(())
    }

    /// Report one typed policy-side rejection without retaining a string.
    pub fn report(&mut self, code: VirtualLayoutDiagnosticCode) {
        self.diagnostics.code(code);
    }

    /// Return the number of entries still admitted by this sink.
    #[must_use]
    pub fn remaining_entries(&self) -> usize {
        self.max_entries.saturating_sub(self.entries.len())
    }
}

/// Object-safe, message-independent query policy for one virtual-layout
/// request.
pub trait VirtualLayoutPolicy {
    /// Answer one immutable query through the executor-owned bounded sink.
    fn query(
        &self,
        input: &VirtualLayoutQueryInput,
        sink: &mut VirtualLayoutQuerySink,
    ) -> VirtualLayoutPolicyDecision;
}

/// Policy-side disposition returned before executor validation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VirtualLayoutPolicyDecision {
    /// The policy submitted a complete candidate answer.
    Ready,
    /// The policy cannot answer from the supplied bounded snapshot.
    Unavailable(VirtualLayoutUnavailableReason),
    /// The policy needs a later fenced query.
    Deferred(VirtualLayoutDeferredReason),
    /// The policy explicitly rejected its answer with a typed code.
    Invalid(VirtualLayoutDiagnosticCode),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FloatEvidence(u32);

impl FloatEvidence {
    const fn new(value: f32) -> Self {
        Self(value.to_bits())
    }

    const fn value(self) -> f32 {
        f32::from_bits(self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RectEvidence {
    min_x: FloatEvidence,
    min_y: FloatEvidence,
    max_x: FloatEvidence,
    max_y: FloatEvidence,
}

impl RectEvidence {
    const fn new(rect: Rect) -> Self {
        Self {
            min_x: FloatEvidence::new(rect.min.x),
            min_y: FloatEvidence::new(rect.min.y),
            max_x: FloatEvidence::new(rect.max.x),
            max_y: FloatEvidence::new(rect.max.y),
        }
    }

    fn rect(self) -> Rect {
        Rect::from_min_max(
            Point::new(self.min_x.value(), self.min_y.value()),
            Point::new(self.max_x.value(), self.max_y.value()),
        )
    }
}

/// Exact acceptance fence captured by the query executor.
///
/// The type has no public constructor. Policies receive the immutable input,
/// never this fence, so policy code cannot forge acceptance evidence.
#[derive(Clone, Debug)]
pub struct VirtualLayoutQueryFence {
    container_id: NodeId,
    policy_identity: VirtualLayoutPolicyIdentity,
    mount_generation: u64,
    query_sequence: u64,
    viewport_revision: u64,
    data_revision: u64,
    policy_revision: u64,
    measurement_revision: u64,
    semantic_revision: u64,
    viewport: RectEvidence,
    coordinate_space: VirtualLayoutCoordinateSpace,
    overscan: VirtualLayoutOverscan,
    budget: VirtualLayoutBudget,
}

impl VirtualLayoutQueryFence {
    /// Return the mounted container identity in this fence.
    #[must_use]
    pub const fn container_id(&self) -> NodeId {
        self.container_id
    }

    /// Return the exact policy identity in this fence.
    #[must_use]
    pub fn policy_identity(&self) -> &VirtualLayoutPolicyIdentity {
        &self.policy_identity
    }

    /// Return the mount generation in this fence.
    #[must_use]
    pub const fn mount_generation(&self) -> u64 {
        self.mount_generation
    }

    /// Return the query sequence in this fence.
    #[must_use]
    pub const fn query_sequence(&self) -> u64 {
        self.query_sequence
    }

    /// Return the viewport revision in this fence.
    #[must_use]
    pub const fn viewport_revision(&self) -> u64 {
        self.viewport_revision
    }

    /// Return the data revision in this fence.
    #[must_use]
    pub const fn data_revision(&self) -> u64 {
        self.data_revision
    }

    /// Return the policy revision in this fence.
    #[must_use]
    pub const fn policy_revision(&self) -> u64 {
        self.policy_revision
    }

    /// Return the measurement revision in this fence.
    #[must_use]
    pub const fn measurement_revision(&self) -> u64 {
        self.measurement_revision
    }

    /// Return the semantic revision in this fence.
    #[must_use]
    pub const fn semantic_revision(&self) -> u64 {
        self.semantic_revision
    }

    /// Return the exact viewport evidence in this fence.
    #[must_use]
    pub fn viewport(&self) -> Rect {
        self.viewport.rect()
    }

    /// Return the exact coordinate-space evidence in this fence.
    #[must_use]
    pub fn coordinate_space(&self) -> &VirtualLayoutCoordinateSpace {
        &self.coordinate_space
    }

    /// Return the exact overscan evidence in this fence.
    #[must_use]
    pub const fn overscan(&self) -> VirtualLayoutOverscan {
        self.overscan
    }

    /// Return the exact caller budget in this fence.
    #[must_use]
    pub const fn budget(&self) -> VirtualLayoutBudget {
        self.budget
    }

    /// Return every field that differs from another fence.
    #[must_use]
    pub fn mismatched_fields(&self, other: &Self) -> VirtualLayoutFenceFields {
        let mut fields = VirtualLayoutFenceFields::default();
        if self.container_id != other.container_id {
            fields = fields.with(VirtualLayoutFenceField::ContainerIdentity);
        }
        if self.policy_identity != other.policy_identity {
            fields = fields.with(VirtualLayoutFenceField::PolicyIdentity);
        }
        if self.mount_generation != other.mount_generation {
            fields = fields.with(VirtualLayoutFenceField::MountGeneration);
        }
        if self.query_sequence != other.query_sequence {
            fields = fields.with(VirtualLayoutFenceField::QuerySequence);
        }
        if self.viewport_revision != other.viewport_revision {
            fields = fields.with(VirtualLayoutFenceField::ViewportRevision);
        }
        if self.data_revision != other.data_revision {
            fields = fields.with(VirtualLayoutFenceField::DataRevision);
        }
        if self.policy_revision != other.policy_revision {
            fields = fields.with(VirtualLayoutFenceField::PolicyRevision);
        }
        if self.measurement_revision != other.measurement_revision {
            fields = fields.with(VirtualLayoutFenceField::MeasurementRevision);
        }
        if self.semantic_revision != other.semantic_revision {
            fields = fields.with(VirtualLayoutFenceField::SemanticRevision);
        }
        if self.viewport != other.viewport {
            fields = fields.with(VirtualLayoutFenceField::Viewport);
        }
        if self.coordinate_space != other.coordinate_space {
            fields = fields.with(VirtualLayoutFenceField::CoordinateSpace);
        }
        if self.overscan != other.overscan {
            fields = fields.with(VirtualLayoutFenceField::Overscan);
        }
        if self.budget != other.budget {
            fields = fields.with(VirtualLayoutFenceField::Budget);
        }
        fields
    }
}

impl PartialEq for VirtualLayoutQueryFence {
    fn eq(&self, other: &Self) -> bool {
        self.mismatched_fields(other).is_empty()
    }
}

impl Eq for VirtualLayoutQueryFence {}

fn fence_from_input(input: &VirtualLayoutQueryInput) -> VirtualLayoutQueryFence {
    VirtualLayoutQueryFence {
        container_id: input.container_id(),
        policy_identity: input.policy_identity().clone(),
        mount_generation: input.mount_generation(),
        query_sequence: input.query_sequence(),
        viewport_revision: input.viewport_revision(),
        data_revision: input.data_revision(),
        policy_revision: input.policy_revision(),
        measurement_revision: input.measurement_revision(),
        semantic_revision: input.semantic_revision(),
        viewport: RectEvidence::new(input.viewport()),
        coordinate_space: input.coordinate_space().clone(),
        overscan: input.overscan(),
        budget: input.budget(),
    }
}

/// Validated keyed entry in an accepted virtual-layout result.
#[derive(Clone, Debug, PartialEq)]
pub struct VirtualLayoutItem {
    key: VirtualLayoutItemKey,
    logical_index: usize,
    bounds: Rect,
    visibility: VirtualLayoutVisibility,
    confidence: VirtualLayoutBoundsConfidence,
}

impl VirtualLayoutItem {
    /// Return the opaque exact item key.
    #[must_use]
    pub fn key(&self) -> &VirtualLayoutItemKey {
        &self.key
    }

    /// Return the logical index associated with the key in this snapshot.
    #[must_use]
    pub const fn logical_index(&self) -> usize {
        self.logical_index
    }

    /// Return the validated finite, non-inverted bounds.
    #[must_use]
    pub const fn bounds(&self) -> Rect {
        self.bounds
    }

    /// Return whether the item is visible or overscan.
    #[must_use]
    pub const fn visibility(&self) -> VirtualLayoutVisibility {
        self.visibility
    }

    /// Return the validated bounds confidence.
    #[must_use]
    pub const fn confidence(&self) -> VirtualLayoutBoundsConfidence {
        self.confidence
    }
}

/// Validated, atomically accepted query result.
#[derive(Clone, Debug, PartialEq)]
pub struct VirtualLayoutQueryResult {
    fence: VirtualLayoutQueryFence,
    entries: Vec<VirtualLayoutItem>,
    extent: VirtualLayoutExtent,
}

impl VirtualLayoutQueryResult {
    /// Return the executor-captured exact fence.
    #[must_use]
    pub const fn fence(&self) -> &VirtualLayoutQueryFence {
        &self.fence
    }

    /// Return the bounded accepted entry count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Return whether the accepted result contains no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Return one accepted entry by bounded position.
    #[must_use]
    pub fn entry(&self, position: usize) -> Option<&VirtualLayoutItem> {
        self.entries.get(position)
    }

    /// Borrow all accepted entries. The slice was bounded by the executor.
    #[must_use]
    pub fn entries(&self) -> &[VirtualLayoutItem] {
        &self.entries
    }

    /// Return the validated extent classification and size.
    #[must_use]
    pub const fn extent(&self) -> VirtualLayoutExtent {
        self.extent
    }
}

/// Distinct result of one executor-owned virtual-layout query.
#[derive(Clone, Debug, PartialEq)]
pub enum VirtualLayoutQueryOutcome {
    /// All raw output passed atomic validation under the captured fence.
    Ready(VirtualLayoutQueryResult),
    /// The policy cannot answer from the supplied bounded snapshot.
    Unavailable(VirtualLayoutUnavailableReason),
    /// The policy needs a later query.
    Deferred(VirtualLayoutDeferredReason),
    /// Input, policy output, or acceptance evidence was invalid.
    Invalid(VirtualLayoutDiagnostics),
}

/// Executor that captures query acceptance evidence and owns output bounds.
pub struct VirtualLayoutQueryExecutor {
    input: VirtualLayoutQueryInput,
    fence: VirtualLayoutQueryFence,
}

impl VirtualLayoutQueryExecutor {
    /// Create an executor from already validated immutable input.
    #[must_use]
    pub fn new(input: VirtualLayoutQueryInput) -> Self {
        let fence = fence_from_input(&input);
        Self { input, fence }
    }

    /// Validate named input and create an executor.
    pub fn from_parts(
        parts: VirtualLayoutQueryInputParts,
    ) -> Result<Self, VirtualLayoutInputError> {
        VirtualLayoutQueryInput::from_parts(parts).map(Self::new)
    }

    /// Validate named input before invoking the policy, then execute one query.
    pub fn execute_parts(
        parts: VirtualLayoutQueryInputParts,
        policy: &dyn VirtualLayoutPolicy,
    ) -> VirtualLayoutQueryOutcome {
        match Self::from_parts(parts) {
            Ok(executor) => executor.execute(policy),
            Err(error) => {
                let mut diagnostics = VirtualLayoutDiagnostics::default();
                diagnostics.code(error.diagnostic_code());
                VirtualLayoutQueryOutcome::Invalid(diagnostics)
            }
        }
    }

    /// Return the immutable input supplied to the policy.
    #[must_use]
    pub const fn input(&self) -> &VirtualLayoutQueryInput {
        &self.input
    }

    /// Return the private executor-captured fence for inspection.
    #[must_use]
    pub const fn fence(&self) -> &VirtualLayoutQueryFence {
        &self.fence
    }

    /// Return the hard-capped entry budget admitted to the sink.
    #[must_use]
    pub fn admitted_entry_budget(&self) -> usize {
        let caller_budget = self.input.budget().max_entries();
        if caller_budget < VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES {
            caller_budget
        } else {
            VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES
        }
    }

    /// Invoke the object-safe policy once through a bounded executor-owned sink.
    pub fn execute(&self, policy: &dyn VirtualLayoutPolicy) -> VirtualLayoutQueryOutcome {
        let mut sink = VirtualLayoutQuerySink::new(self.admitted_entry_budget());
        let decision = policy.query(&self.input, &mut sink);
        self.finish(decision, sink)
    }

    /// Accept a previously validated result only when every fence field is
    /// exactly equal to this executor's fence.
    pub fn accept(&self, result: VirtualLayoutQueryResult) -> VirtualLayoutQueryOutcome {
        let mismatch = self.fence.mismatched_fields(&result.fence);
        if mismatch.is_empty() {
            VirtualLayoutQueryOutcome::Ready(result)
        } else {
            let mut diagnostics = VirtualLayoutDiagnostics::default();
            diagnostics.fence_mismatch(mismatch);
            VirtualLayoutQueryOutcome::Invalid(diagnostics)
        }
    }

    fn finish(
        &self,
        decision: VirtualLayoutPolicyDecision,
        sink: VirtualLayoutQuerySink,
    ) -> VirtualLayoutQueryOutcome {
        match decision {
            VirtualLayoutPolicyDecision::Ready => self.validate_ready(sink),
            VirtualLayoutPolicyDecision::Unavailable(reason) => {
                self.finish_non_ready(sink, VirtualLayoutQueryOutcome::Unavailable(reason))
            }
            VirtualLayoutPolicyDecision::Deferred(reason) => {
                self.finish_non_ready(sink, VirtualLayoutQueryOutcome::Deferred(reason))
            }
            VirtualLayoutPolicyDecision::Invalid(code) => {
                let mut diagnostics = sink.diagnostics;
                diagnostics.code(code);
                VirtualLayoutQueryOutcome::Invalid(diagnostics)
            }
        }
    }

    fn finish_non_ready(
        &self,
        sink: VirtualLayoutQuerySink,
        outcome: VirtualLayoutQueryOutcome,
    ) -> VirtualLayoutQueryOutcome {
        if sink.entries.is_empty() && sink.extent.is_none() && sink.diagnostics.is_empty() {
            outcome
        } else {
            let mut diagnostics = sink.diagnostics;
            diagnostics.code(VirtualLayoutDiagnosticCode::OutputUnexpectedForDisposition);
            VirtualLayoutQueryOutcome::Invalid(diagnostics)
        }
    }

    fn validate_ready(&self, sink: VirtualLayoutQuerySink) -> VirtualLayoutQueryOutcome {
        let mut diagnostics = sink.diagnostics;
        if sink.over_budget {
            diagnostics.code(VirtualLayoutDiagnosticCode::OutputOverBudget);
        }
        let Some(extent_candidate) = sink.extent else {
            diagnostics.code(VirtualLayoutDiagnosticCode::OutputMissingExtent);
            return VirtualLayoutQueryOutcome::Invalid(diagnostics);
        };

        let mut validated_entries = Vec::with_capacity(sink.entries.len());
        for (position, candidate) in sink.entries.iter().enumerate() {
            match &candidate.key {
                VirtualLayoutItemKeyCandidate::Missing => diagnostics.entry(
                    VirtualLayoutDiagnosticCode::OutputMissingKey,
                    position,
                    Some(candidate.logical_index),
                ),
                VirtualLayoutItemKeyCandidate::Ambiguous => diagnostics.entry(
                    VirtualLayoutDiagnosticCode::OutputAmbiguousKey,
                    position,
                    Some(candidate.logical_index),
                ),
                VirtualLayoutItemKeyCandidate::Exact(key) => {
                    if key.stable_equals(key) != Some(true) {
                        diagnostics.entry(
                            VirtualLayoutDiagnosticCode::OutputUnstableKey,
                            position,
                            Some(candidate.logical_index),
                        );
                    }
                }
            }

            if !candidate.bounds.is_finite() {
                diagnostics.entry(
                    VirtualLayoutDiagnosticCode::OutputNonFiniteBounds,
                    position,
                    Some(candidate.logical_index),
                );
            } else if candidate.bounds.min.x > candidate.bounds.max.x
                || candidate.bounds.min.y > candidate.bounds.max.y
            {
                diagnostics.entry(
                    VirtualLayoutDiagnosticCode::OutputInvertedBounds,
                    position,
                    Some(candidate.logical_index),
                );
            }
        }

        for (left_position, left) in sink.entries.iter().enumerate() {
            for (right_position, right) in sink.entries.iter().enumerate().skip(left_position + 1) {
                if left.logical_index == right.logical_index {
                    diagnostics.entry(
                        VirtualLayoutDiagnosticCode::OutputDuplicateIndex,
                        right_position,
                        Some(right.logical_index),
                    );
                }
                if let (
                    VirtualLayoutItemKeyCandidate::Exact(left_key),
                    VirtualLayoutItemKeyCandidate::Exact(right_key),
                ) = (&left.key, &right.key)
                {
                    match left_key.stable_equals(right_key) {
                        Some(true) => diagnostics.entry(
                            VirtualLayoutDiagnosticCode::OutputDuplicateKey,
                            right_position,
                            Some(right.logical_index),
                        ),
                        Some(false) => {}
                        None => diagnostics.entry(
                            VirtualLayoutDiagnosticCode::OutputUnstableKey,
                            right_position,
                            Some(right.logical_index),
                        ),
                    }
                }
            }
        }

        let validated_extent = match extent_candidate {
            VirtualLayoutExtentCandidate::Exact(size) => {
                if !valid_extent_size(size) {
                    diagnostics.code(VirtualLayoutDiagnosticCode::OutputInvalidExtent);
                    None
                } else {
                    Some(VirtualLayoutExtent {
                        kind: VirtualLayoutExtentKind::Exact,
                        size: Some(size),
                    })
                }
            }
            VirtualLayoutExtentCandidate::Estimated(size) => {
                if !valid_extent_size(size) {
                    diagnostics.code(VirtualLayoutDiagnosticCode::OutputInvalidExtent);
                    None
                } else {
                    Some(VirtualLayoutExtent {
                        kind: VirtualLayoutExtentKind::Estimated,
                        size: Some(size),
                    })
                }
            }
            VirtualLayoutExtentCandidate::Unavailable => Some(VirtualLayoutExtent {
                kind: VirtualLayoutExtentKind::Unavailable,
                size: None,
            }),
        };

        if !diagnostics.is_empty() {
            return VirtualLayoutQueryOutcome::Invalid(diagnostics);
        }

        for candidate in sink.entries {
            let VirtualLayoutItemKeyCandidate::Exact(key) = candidate.key else {
                return invalid_single(VirtualLayoutDiagnosticCode::OutputMissingKey);
            };
            validated_entries.push(VirtualLayoutItem {
                key,
                logical_index: candidate.logical_index,
                bounds: candidate.bounds,
                visibility: candidate.visibility,
                confidence: candidate.confidence,
            });
        }

        match validated_extent {
            Some(extent) => VirtualLayoutQueryOutcome::Ready(VirtualLayoutQueryResult {
                fence: self.fence.clone(),
                entries: validated_entries,
                extent,
            }),
            None => invalid_single(VirtualLayoutDiagnosticCode::OutputInvalidExtent),
        }
    }
}

fn valid_extent_size(size: Vector2) -> bool {
    size.x.is_finite() && size.y.is_finite() && size.x >= 0.0 && size.y >= 0.0
}

fn invalid_single(code: VirtualLayoutDiagnosticCode) -> VirtualLayoutQueryOutcome {
    let mut diagnostics = VirtualLayoutDiagnostics::default();
    diagnostics.code(code);
    VirtualLayoutQueryOutcome::Invalid(diagnostics)
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    fn input_parts() -> VirtualLayoutQueryInputParts {
        VirtualLayoutQueryInputParts {
            container_id: 7,
            policy_identity: VirtualLayoutPolicyIdentity::new("policy"),
            mount_generation: 2,
            query_sequence: 3,
            viewport: Rect::from_xy_size(0.0, 10.0, 100.0, 80.0),
            coordinate_space: VirtualLayoutCoordinateSpace::logical(),
            overscan: VirtualLayoutOverscan::new(8.0, 12.0).expect("test overscan should be valid"),
            budget: VirtualLayoutBudget::new(4),
            viewport_revision: 11,
            data_revision: 12,
            policy_revision: 13,
            measurement_revision: 14,
            semantic_revision: 15,
        }
    }

    fn candidate(key: u32, index: usize) -> VirtualLayoutItemCandidate {
        VirtualLayoutItemCandidate::new(
            VirtualLayoutItemKey::new(key),
            index,
            Rect::from_xy_size(0.0, index as f32 * 20.0, 100.0, 20.0),
            VirtualLayoutVisibility::Visible,
            VirtualLayoutBoundsConfidence::Exact,
        )
    }

    struct ReadyPolicy;

    impl VirtualLayoutPolicy for ReadyPolicy {
        fn query(
            &self,
            _input: &VirtualLayoutQueryInput,
            sink: &mut VirtualLayoutQuerySink,
        ) -> VirtualLayoutPolicyDecision {
            assert!(sink.visit(candidate(1, 0)).is_ok());
            assert!(
                sink.set_extent(VirtualLayoutExtentCandidate::exact(Vector2::new(
                    100.0, 20.0
                )))
                .is_ok()
            );
            VirtualLayoutPolicyDecision::Ready
        }
    }

    #[test]
    fn exact_identity_uses_type_and_eq_without_hash_or_pointer_identity() {
        assert_eq!(
            VirtualLayoutPolicyIdentity::new("policy"),
            VirtualLayoutPolicyIdentity::new("policy")
        );
        assert_ne!(
            VirtualLayoutPolicyIdentity::new(1_u32),
            VirtualLayoutPolicyIdentity::new(1_u64)
        );
        assert_eq!(
            VirtualLayoutItemKey::new("item"),
            VirtualLayoutItemKey::new("item")
        );
    }

    #[test]
    fn ready_output_is_bounded_and_validated_atomically() {
        struct TooMany;

        impl VirtualLayoutPolicy for TooMany {
            fn query(
                &self,
                _input: &VirtualLayoutQueryInput,
                sink: &mut VirtualLayoutQuerySink,
            ) -> VirtualLayoutPolicyDecision {
                for index in 0..5 {
                    let _ = sink.visit(candidate(index as u32, index));
                }
                let _ = sink.set_extent(VirtualLayoutExtentCandidate::exact(Vector2::new(
                    100.0, 100.0,
                )));
                VirtualLayoutPolicyDecision::Ready
            }
        }

        let executor = VirtualLayoutQueryExecutor::new(
            VirtualLayoutQueryInput::from_parts(input_parts()).expect("input should be valid"),
        );
        assert_eq!(executor.admitted_entry_budget(), 4);
        let outcome = executor.execute(&TooMany);
        let VirtualLayoutQueryOutcome::Invalid(diagnostics) = outcome else {
            panic!("over-budget output must be invalid");
        };
        assert!(
            diagnostics.iter().any(
                |diagnostic| diagnostic.code() == VirtualLayoutDiagnosticCode::OutputOverBudget
            )
        );
    }

    #[test]
    fn invalid_keys_indices_bounds_and_extents_never_produce_ready() {
        struct Invalid;

        impl VirtualLayoutPolicy for Invalid {
            fn query(
                &self,
                _input: &VirtualLayoutQueryInput,
                sink: &mut VirtualLayoutQuerySink,
            ) -> VirtualLayoutPolicyDecision {
                let _ = sink.visit(VirtualLayoutItemCandidate::missing_key(
                    0,
                    Rect::from_min_max(Point::new(0.0, 0.0), Point::new(f32::NAN, 1.0)),
                    VirtualLayoutVisibility::Visible,
                    VirtualLayoutBoundsConfidence::Estimated,
                ));
                let _ = sink.visit(candidate(1, 0));
                let _ = sink.visit(candidate(2, 0));
                let _ = sink.set_extent(VirtualLayoutExtentCandidate::exact(Vector2::new(
                    -1.0,
                    f32::INFINITY,
                )));
                VirtualLayoutPolicyDecision::Ready
            }
        }

        let executor = VirtualLayoutQueryExecutor::new(
            VirtualLayoutQueryInput::from_parts(input_parts()).expect("input should be valid"),
        );
        let VirtualLayoutQueryOutcome::Invalid(diagnostics) = executor.execute(&Invalid) else {
            panic!("invalid output must not be ready");
        };
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code() == VirtualLayoutDiagnosticCode::OutputMissingKey
        }));
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code() == VirtualLayoutDiagnosticCode::OutputDuplicateIndex
        }));
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code() == VirtualLayoutDiagnosticCode::OutputInvalidExtent
        }));
    }

    #[test]
    fn stateful_key_equality_is_rejected_as_unstable() {
        #[derive(Eq)]
        struct UnstableKey {
            calls: Cell<u32>,
        }

        impl PartialEq for UnstableKey {
            fn eq(&self, _other: &Self) -> bool {
                let call = self.calls.get();
                self.calls.set(call.saturating_add(1));
                call.is_multiple_of(2)
            }
        }

        struct Policy {
            key: VirtualLayoutItemKey,
        }

        impl VirtualLayoutPolicy for Policy {
            fn query(
                &self,
                _input: &VirtualLayoutQueryInput,
                sink: &mut VirtualLayoutQuerySink,
            ) -> VirtualLayoutPolicyDecision {
                let _ = sink.visit(VirtualLayoutItemCandidate::new(
                    self.key.clone(),
                    0,
                    Rect::from_size(10.0, 10.0),
                    VirtualLayoutVisibility::Visible,
                    VirtualLayoutBoundsConfidence::Exact,
                ));
                let _ = sink.set_extent(VirtualLayoutExtentCandidate::exact(Vector2::new(
                    10.0, 10.0,
                )));
                VirtualLayoutPolicyDecision::Ready
            }
        }

        let executor = VirtualLayoutQueryExecutor::new(
            VirtualLayoutQueryInput::from_parts(input_parts()).expect("input should be valid"),
        );
        let outcome = executor.execute(&Policy {
            key: VirtualLayoutItemKey::new(UnstableKey {
                calls: Cell::new(0),
            }),
        });
        let VirtualLayoutQueryOutcome::Invalid(diagnostics) = outcome else {
            panic!("unstable key equality must be invalid");
        };
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code() == VirtualLayoutDiagnosticCode::OutputUnstableKey
        }));
    }

    #[test]
    fn exact_fence_rejects_each_changed_revision_without_ordering_comparison() {
        let first_input = input_parts();
        let first = VirtualLayoutQueryExecutor::new(
            VirtualLayoutQueryInput::from_parts(first_input.clone())
                .expect("input should be valid"),
        );
        let VirtualLayoutQueryOutcome::Ready(result) = first.execute(&ReadyPolicy) else {
            panic!("ready policy should produce a result");
        };

        let mut changed = first_input;
        changed.data_revision += 1;
        let second = VirtualLayoutQueryExecutor::new(
            VirtualLayoutQueryInput::from_parts(changed).expect("input should be valid"),
        );
        let VirtualLayoutQueryOutcome::Invalid(diagnostics) = second.accept(result) else {
            panic!("newer but unequal data revision must reject the result");
        };
        let diagnostic = diagnostics
            .get(0)
            .expect("fence diagnostic should be retained");
        assert!(
            diagnostic
                .fence_fields()
                .contains(VirtualLayoutFenceField::DataRevision)
        );
    }

    #[test]
    fn invalid_input_is_rejected_before_policy_invocation() {
        struct CountingPolicy {
            calls: Cell<u32>,
        }

        impl VirtualLayoutPolicy for CountingPolicy {
            fn query(
                &self,
                _input: &VirtualLayoutQueryInput,
                _sink: &mut VirtualLayoutQuerySink,
            ) -> VirtualLayoutPolicyDecision {
                self.calls.set(self.calls.get() + 1);
                VirtualLayoutPolicyDecision::Ready
            }
        }

        let policy = CountingPolicy {
            calls: Cell::new(0),
        };
        let mut parts = input_parts();
        parts.viewport = Rect::from_min_max(Point::new(10.0, 0.0), Point::new(0.0, 1.0));
        let outcome = VirtualLayoutQueryExecutor::execute_parts(parts, &policy);
        assert!(matches!(outcome, VirtualLayoutQueryOutcome::Invalid(_)));
        assert_eq!(policy.calls.get(), 0);
    }

    #[test]
    fn policy_dispositions_remain_distinct() {
        struct Disposition(VirtualLayoutPolicyDecision);

        impl VirtualLayoutPolicy for Disposition {
            fn query(
                &self,
                _input: &VirtualLayoutQueryInput,
                _sink: &mut VirtualLayoutQuerySink,
            ) -> VirtualLayoutPolicyDecision {
                self.0
            }
        }

        let executor = VirtualLayoutQueryExecutor::new(
            VirtualLayoutQueryInput::from_parts(input_parts()).expect("input should be valid"),
        );
        assert!(matches!(
            executor.execute(&Disposition(VirtualLayoutPolicyDecision::Unavailable(
                VirtualLayoutUnavailableReason::DataUnavailable
            ))),
            VirtualLayoutQueryOutcome::Unavailable(VirtualLayoutUnavailableReason::DataUnavailable)
        ));
        assert!(matches!(
            executor.execute(&Disposition(VirtualLayoutPolicyDecision::Deferred(
                VirtualLayoutDeferredReason::DataPending
            ))),
            VirtualLayoutQueryOutcome::Deferred(VirtualLayoutDeferredReason::DataPending)
        ));
        assert!(matches!(
            executor.execute(&Disposition(VirtualLayoutPolicyDecision::Invalid(
                VirtualLayoutDiagnosticCode::PolicyRejected
            ))),
            VirtualLayoutQueryOutcome::Invalid(_)
        ));
    }
}
