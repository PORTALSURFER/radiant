//! Bounded, query-only reconciliation for keyed virtual-layout windows.
//!
//! This module is intentionally private.  It owns accepted query evidence and
//! continuity by key, but it does not own a runtime registration, a materializer,
//! a widget, scrolling, clipping, painting, or scheduling.

#![expect(
    dead_code,
    reason = "Slice 2 is intentionally query-only until a later runtime materialization slice"
)]

use super::{
    NodeId, VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES, VirtualLayoutBoundsConfidence, VirtualLayoutBudget,
    VirtualLayoutCoordinateSpace, VirtualLayoutDeferredReason, VirtualLayoutDiagnosticCode,
    VirtualLayoutDiagnostics, VirtualLayoutExtent, VirtualLayoutExtentKind,
    VirtualLayoutInputError, VirtualLayoutItem, VirtualLayoutItemKey, VirtualLayoutPolicy,
    VirtualLayoutPolicyIdentity, VirtualLayoutQueryExecutor, VirtualLayoutQueryFence,
    VirtualLayoutQueryInput, VirtualLayoutQueryInputParts, VirtualLayoutQueryOutcome,
    VirtualLayoutQueryResult, VirtualLayoutUnavailableReason, VirtualLayoutVisibility,
};
use crate::gui::types::Rect;
use std::{cell::Cell, rc::Rc};

const MAX_COORDINATOR_DIAGNOSTICS: usize = 1;

/// The fixed upper bound used by every coordinator-owned record set.
pub(crate) const VIRTUAL_LAYOUT_MAX_COORDINATOR_RECORDS: usize = VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES;

/// The axis used by a query-only primary anchor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VirtualLayoutAnchorAxis {
    Horizontal,
    Vertical,
}

/// The edge of an anchored item whose screen position is preserved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VirtualLayoutAnchorEdge {
    Leading,
    Trailing,
}

/// A finite primary-anchor request.  It contains no scroll or runtime handle.
#[derive(Clone, Debug)]
pub(crate) struct VirtualLayoutAnchorRequest {
    pub(crate) key: VirtualLayoutItemKey,
    pub(crate) axis: VirtualLayoutAnchorAxis,
    pub(crate) edge: VirtualLayoutAnchorEdge,
    pub(crate) local_offset: f32,
    pub(crate) screen_offset: f32,
}

impl VirtualLayoutAnchorRequest {
    fn is_finite(&self) -> bool {
        self.local_offset.is_finite() && self.screen_offset.is_finite()
    }
}

/// Whether an anchor correction is exact or provisional.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VirtualLayoutCorrectionConfidence {
    Exact,
    Estimated,
}

/// Query-only evidence for a finite viewport-origin correction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct VirtualLayoutAnchorCorrection {
    pub(crate) delta: f32,
    pub(crate) confidence: VirtualLayoutCorrectionConfidence,
}

/// A retained anchor after one successful reconciliation.
#[derive(Clone, Debug)]
pub(crate) struct VirtualLayoutAnchor {
    pub(crate) key: VirtualLayoutItemKey,
    pub(crate) axis: VirtualLayoutAnchorAxis,
    pub(crate) edge: VirtualLayoutAnchorEdge,
    pub(crate) local_offset: f32,
    pub(crate) screen_offset: f32,
}

impl VirtualLayoutAnchor {
    fn from_request(request: VirtualLayoutAnchorRequest) -> Self {
        Self {
            key: request.key,
            axis: request.axis,
            edge: request.edge,
            local_offset: request.local_offset,
            screen_offset: request.screen_offset,
        }
    }
}

/// One coalesced invalidation category owned by the coordinator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VirtualLayoutInvalidation {
    Viewport,
    Data,
    Policy,
    Measurement,
    Semantic,
    OverscanBudget,
    RequiredKey,
    Anchor,
}

/// Bounded observable invalidation flags.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct VirtualLayoutInvalidationFlags {
    bits: u8,
}

impl VirtualLayoutInvalidationFlags {
    const VIEWPORT: u8 = 1 << 0;
    const DATA: u8 = 1 << 1;
    const POLICY: u8 = 1 << 2;
    const MEASUREMENT: u8 = 1 << 3;
    const SEMANTIC: u8 = 1 << 4;
    const OVERSCAN_BUDGET: u8 = 1 << 5;
    const ANCHOR: u8 = 1 << 6;
    const REQUIRED_KEY: u8 = 1 << 7;

    /// Return whether the category has been coalesced since the last commit.
    #[must_use]
    pub(crate) const fn contains(self, invalidation: VirtualLayoutInvalidation) -> bool {
        self.bits & invalidation.bit() != 0
    }

    fn insert(&mut self, invalidation: VirtualLayoutInvalidation) {
        self.bits |= invalidation.bit();
    }
}

impl VirtualLayoutInvalidation {
    const fn bit(self) -> u8 {
        match self {
            Self::Viewport => VirtualLayoutInvalidationFlags::VIEWPORT,
            Self::Data => VirtualLayoutInvalidationFlags::DATA,
            Self::Policy => VirtualLayoutInvalidationFlags::POLICY,
            Self::Measurement => VirtualLayoutInvalidationFlags::MEASUREMENT,
            Self::Semantic => VirtualLayoutInvalidationFlags::SEMANTIC,
            Self::OverscanBudget => VirtualLayoutInvalidationFlags::OVERSCAN_BUDGET,
            Self::RequiredKey => VirtualLayoutInvalidationFlags::REQUIRED_KEY,
            Self::Anchor => VirtualLayoutInvalidationFlags::ANCHOR,
        }
    }
}

/// Coordinator-owned revisions, including categories not present in a query fence.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct VirtualLayoutCoordinatorRevisions {
    pub(crate) viewport: u64,
    pub(crate) data: u64,
    pub(crate) policy: u64,
    pub(crate) measurement: u64,
    pub(crate) semantic: u64,
    pub(crate) overscan_budget: u64,
    pub(crate) required_key: u64,
    pub(crate) anchor: u64,
}

impl VirtualLayoutCoordinatorRevisions {
    fn from_input(input: &VirtualLayoutQueryInput) -> Self {
        Self {
            viewport: input.viewport_revision(),
            data: input.data_revision(),
            policy: input.policy_revision(),
            measurement: input.measurement_revision(),
            semantic: input.semantic_revision(),
            ..Self::default()
        }
    }

    fn fallback_revisions_match(self, fence: &VirtualLayoutQueryFence) -> bool {
        self.data == fence.data_revision()
            && self.policy == fence.policy_revision()
            && self.measurement == fence.measurement_revision()
            && self.semantic == fence.semantic_revision()
    }

    fn increment(
        &mut self,
        invalidation: VirtualLayoutInvalidation,
    ) -> Result<(), VirtualLayoutCoordinatorError> {
        let revision = match invalidation {
            VirtualLayoutInvalidation::Viewport => &mut self.viewport,
            VirtualLayoutInvalidation::Data => &mut self.data,
            VirtualLayoutInvalidation::Policy => &mut self.policy,
            VirtualLayoutInvalidation::Measurement => &mut self.measurement,
            VirtualLayoutInvalidation::Semantic => &mut self.semantic,
            VirtualLayoutInvalidation::OverscanBudget => &mut self.overscan_budget,
            VirtualLayoutInvalidation::RequiredKey => &mut self.required_key,
            VirtualLayoutInvalidation::Anchor => &mut self.anchor,
        };
        *revision = revision
            .checked_add(1)
            .ok_or(VirtualLayoutCoordinatorError::RevisionOverflow)?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScopeIdentity {
    container_id: NodeId,
    policy_identity: PolicyIdentityValue,
    mount_generation: u64,
}

#[derive(Clone, Debug)]
struct PolicyIdentityValue(VirtualLayoutPolicyIdentity);

impl PartialEq for PolicyIdentityValue {
    fn eq(&self, other: &Self) -> bool {
        self.0.stable_equals(&other.0) == Some(true)
    }
}

impl Eq for PolicyIdentityValue {}

impl ScopeIdentity {
    fn matches(&self, input: &VirtualLayoutQueryInputParts) -> bool {
        self.container_id == input.container_id
            && self.mount_generation == input.mount_generation
            && self.policy_identity == PolicyIdentityValue(input.policy_identity.clone())
    }

    fn matches_input(&self, input: &VirtualLayoutQueryInput) -> bool {
        self.container_id == input.container_id()
            && self.mount_generation == input.mount_generation()
            && self
                .policy_identity
                .0
                .stable_equals(input.policy_identity())
                == Some(true)
    }
}

/// Errors returned before a query token is created.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VirtualLayoutCoordinatorError {
    ScopeMismatch,
    InvalidInput(VirtualLayoutInputError),
    RevisionRegression,
    QuerySequenceOverflow,
    RevisionOverflow,
    ReentrantExecution,
    NonFiniteAnchor,
}

/// A bounded keyed continuity record.
#[derive(Clone, Debug)]
pub(crate) struct VirtualLayoutKeyChange {
    pub(crate) key: VirtualLayoutItemKey,
    pub(crate) kind: VirtualLayoutKeyChangeKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VirtualLayoutKeyChangeKind {
    Inserted { index: usize },
    Removed { index: usize },
    Moved { from: usize, to: usize },
}

#[derive(Clone, Debug, Default)]
pub(crate) struct VirtualLayoutKeyDelta {
    changes: Vec<VirtualLayoutKeyChange>,
    omitted: usize,
}

impl VirtualLayoutKeyDelta {
    /// Return the bounded number of retained key changes.
    #[must_use]
    pub(crate) fn len(&self) -> usize {
        self.changes.len()
    }

    /// Return one retained key change.
    #[must_use]
    pub(crate) fn get(&self, position: usize) -> Option<&VirtualLayoutKeyChange> {
        self.changes.get(position)
    }

    /// Return the number of changes omitted by the hard bound.
    #[must_use]
    pub(crate) const fn omitted(&self) -> usize {
        self.omitted
    }

    fn push(&mut self, change: VirtualLayoutKeyChange) {
        let order = key_change_sort_key(&change);
        if self.changes.len() == VIRTUAL_LAYOUT_MAX_COORDINATOR_RECORDS {
            self.omitted = self.omitted.saturating_add(1);
            let should_retain = self
                .changes
                .last()
                .is_some_and(|last| key_change_sort_key(last) > order);
            if !should_retain {
                return;
            }
            self.changes.pop();
        }

        let position = self
            .changes
            .iter()
            .position(|existing| key_change_sort_key(existing) > order)
            .unwrap_or(self.changes.len());
        self.changes.insert(position, change);
    }
}

/// A bounded accepted window owned by one coordinator.
#[derive(Clone, Debug)]
pub(crate) struct VirtualLayoutAcceptedWindow {
    pub(crate) fence: VirtualLayoutQueryFence,
    pub(crate) extent: VirtualLayoutExtent,
    pub(crate) entries: Vec<VirtualLayoutItem>,
    pub(crate) delta: VirtualLayoutKeyDelta,
    pub(crate) anchor: Option<VirtualLayoutAnchor>,
    pub(crate) correction: Option<VirtualLayoutAnchorCorrection>,
    pub(crate) accepted_revision: u64,
}

impl VirtualLayoutAcceptedWindow {
    fn bounded_entries(mut entries: Vec<VirtualLayoutItem>) -> Vec<VirtualLayoutItem> {
        if entries.len() > VIRTUAL_LAYOUT_MAX_COORDINATOR_RECORDS {
            entries.truncate(VIRTUAL_LAYOUT_MAX_COORDINATOR_RECORDS);
        }
        entries
    }
}

/// A view exposed while a query is pending or unavailable.
#[derive(Clone, Debug)]
pub(crate) struct VirtualLayoutWindowView {
    pub(crate) entries: Vec<VirtualLayoutItem>,
    pub(crate) extent: Option<VirtualLayoutExtent>,
    pub(crate) accepted_revision: Option<u64>,
    pub(crate) fallback: bool,
    pub(crate) clip: Option<Rect>,
}

impl VirtualLayoutWindowView {
    fn empty() -> Self {
        Self {
            entries: Vec::new(),
            extent: None,
            accepted_revision: None,
            fallback: false,
            clip: None,
        }
    }
}

/// A token that can be completed exactly once by the owning coordinator.
pub(crate) struct VirtualLayoutPendingQuery {
    executor: VirtualLayoutQueryExecutor,
    token: PendingToken,
    executed: Cell<bool>,
    execution_phase: Rc<Cell<bool>>,
}

struct ExecutionPhaseGuard<'a> {
    phase: &'a Cell<bool>,
}

impl Drop for ExecutionPhaseGuard<'_> {
    fn drop(&mut self) {
        self.phase.set(false);
    }
}

/// Non-zero-sized marker whose handle is compared only by allocation identity.
#[derive(Debug)]
pub(super) struct CoordinatorIdentity(u8);

impl VirtualLayoutPendingQuery {
    /// Execute the policy outside the coordinator's mutable commit path.
    pub(crate) fn execute(&self, policy: &dyn VirtualLayoutPolicy) -> VirtualLayoutQueryOutcome {
        if self.executed.replace(true) {
            return super::invalid_single(VirtualLayoutDiagnosticCode::PolicyRejected);
        }
        if self.execution_phase.replace(true) {
            return super::invalid_single(VirtualLayoutDiagnosticCode::PolicyRejected);
        }
        let _guard = ExecutionPhaseGuard {
            phase: &self.execution_phase,
        };
        self.executor.execute(policy)
    }

    /// Borrow the exact executor input used for this token.
    #[must_use]
    pub(crate) const fn input(&self) -> &VirtualLayoutQueryInput {
        self.executor.input()
    }

    /// Return the checked query sequence assigned by the coordinator.
    #[must_use]
    pub(crate) const fn query_sequence(&self) -> u64 {
        self.token.query_sequence
    }
}

#[derive(Clone, Debug)]
struct PendingToken {
    owner: Rc<CoordinatorIdentity>,
    scope: ScopeIdentity,
    query_sequence: u64,
    invalidation_epoch: u64,
    revisions: VirtualLayoutCoordinatorRevisions,
    input: InputEvidence,
    anchor: Option<VirtualLayoutAnchor>,
}

#[derive(Clone, Debug)]
struct InputEvidence {
    viewport: Rect,
    coordinate_space: VirtualLayoutCoordinateSpace,
    overscan: VirtualLayoutBudgetEvidence,
    budget: VirtualLayoutBudget,
    required_key: Option<VirtualLayoutItemKey>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct VirtualLayoutBudgetEvidence {
    leading: f32,
    trailing: f32,
}

impl InputEvidence {
    fn from_input(input: &VirtualLayoutQueryInput) -> Self {
        let overscan = input.overscan();
        Self {
            viewport: input.viewport(),
            coordinate_space: input.coordinate_space().clone(),
            overscan: VirtualLayoutBudgetEvidence {
                leading: overscan.leading(),
                trailing: overscan.trailing(),
            },
            budget: input.budget(),
            required_key: input.required_key().cloned(),
        }
    }

    fn same_geometry(&self, other: &Self) -> bool {
        self.viewport == other.viewport
            && self.coordinate_space == other.coordinate_space
            && self.overscan == other.overscan
            && self.budget == other.budget
    }
}

/// The disposition of a completed coordinator token.
#[derive(Clone, Debug)]
pub(crate) enum VirtualLayoutCompletion {
    Committed(Box<VirtualLayoutCommit>),
    Retained {
        reason: VirtualLayoutRetainReason,
        view: Box<VirtualLayoutWindowView>,
        diagnostics: Box<Option<VirtualLayoutDiagnostics>>,
    },
    Stale(VirtualLayoutCoordinatorDiagnostic),
    Rejected(VirtualLayoutCoordinatorDiagnostic),
}

#[derive(Clone, Debug)]
pub(crate) struct VirtualLayoutCommit {
    fence: VirtualLayoutQueryFence,
    owner: Rc<CoordinatorIdentity>,
    view: VirtualLayoutWindowView,
    delta: VirtualLayoutKeyDelta,
    anchor: Option<VirtualLayoutAnchor>,
    correction: Option<VirtualLayoutAnchorCorrection>,
    accepted_revision: u64,
}

impl VirtualLayoutCommit {
    /// Return the exact coordinator-accepted query fence.
    #[must_use]
    pub(crate) const fn fence(&self) -> &VirtualLayoutQueryFence {
        &self.fence
    }

    /// Return the coordinator-owned identity witness.
    #[must_use]
    pub(super) const fn owner(&self) -> &Rc<CoordinatorIdentity> {
        &self.owner
    }

    /// Return the immutable accepted window view.
    #[must_use]
    pub(crate) const fn view(&self) -> &VirtualLayoutWindowView {
        &self.view
    }

    /// Return the monotonic coordinator-accepted revision.
    #[must_use]
    pub(crate) const fn accepted_revision(&self) -> u64 {
        self.accepted_revision
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VirtualLayoutRetainReason {
    Pending,
    Deferred(VirtualLayoutDeferredReason),
    Unavailable(VirtualLayoutUnavailableReason),
    Invalid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VirtualLayoutCoordinatorDiagnosticCode {
    StaleQuery,
    ReentrantExecution,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct VirtualLayoutCoordinatorDiagnostic {
    pub(crate) code: VirtualLayoutCoordinatorDiagnosticCode,
}

/// One private query-only keyed-window coordinator.
pub(crate) struct VirtualLayoutWindowCoordinator {
    identity: Rc<CoordinatorIdentity>,
    scope: ScopeIdentity,
    query_sequence: u64,
    invalidation_epoch: u64,
    revisions: VirtualLayoutCoordinatorRevisions,
    initialized_revisions: bool,
    last_input: Option<InputEvidence>,
    invalidations: VirtualLayoutInvalidationFlags,
    pending: Option<PendingToken>,
    accepted: Option<VirtualLayoutAcceptedWindow>,
    explicit_anchor: Option<VirtualLayoutAnchor>,
    last_diagnostic: Option<VirtualLayoutCoordinatorDiagnostic>,
    execution_phase: Rc<Cell<bool>>,
}

impl VirtualLayoutWindowCoordinator {
    /// Create a coordinator for one immutable mounted container scope.
    #[must_use]
    pub(crate) fn new(
        container_id: NodeId,
        policy_identity: VirtualLayoutPolicyIdentity,
        mount_generation: u64,
    ) -> Self {
        Self {
            identity: Rc::new(CoordinatorIdentity(0)),
            scope: ScopeIdentity {
                container_id,
                policy_identity: PolicyIdentityValue(policy_identity),
                mount_generation,
            },
            query_sequence: 0,
            invalidation_epoch: 0,
            revisions: VirtualLayoutCoordinatorRevisions::default(),
            initialized_revisions: false,
            last_input: None,
            invalidations: VirtualLayoutInvalidationFlags::default(),
            pending: None,
            accepted: None,
            explicit_anchor: None,
            last_diagnostic: None,
            execution_phase: Rc::new(Cell::new(false)),
        }
    }

    /// Return the immutable scope identity as separate exact fields.
    #[must_use]
    pub(crate) fn scope(&self) -> (NodeId, &VirtualLayoutPolicyIdentity, u64) {
        (
            self.scope.container_id,
            &self.scope.policy_identity.0,
            self.scope.mount_generation,
        )
    }

    /// Return the current checked revision snapshot.
    #[must_use]
    pub(crate) const fn revisions(&self) -> VirtualLayoutCoordinatorRevisions {
        self.revisions
    }

    /// Return coalesced invalidation categories since the last accepted commit.
    #[must_use]
    pub(crate) const fn invalidations(&self) -> VirtualLayoutInvalidationFlags {
        self.invalidations
    }

    /// Set an explicit primary anchor for a later query.
    pub(crate) fn set_anchor(
        &mut self,
        request: VirtualLayoutAnchorRequest,
    ) -> Result<(), VirtualLayoutCoordinatorError> {
        self.ensure_not_executing()?;
        if !request.is_finite() {
            return Err(VirtualLayoutCoordinatorError::NonFiniteAnchor);
        }
        self.explicit_anchor = Some(VirtualLayoutAnchor::from_request(request));
        self.invalidate(VirtualLayoutInvalidation::Anchor)
    }

    /// Clear the explicit anchor; the next accepted visible item may become primary.
    pub(crate) fn clear_anchor(&mut self) -> Result<(), VirtualLayoutCoordinatorError> {
        self.ensure_not_executing()?;
        if self.explicit_anchor.take().is_some() {
            self.invalidate(VirtualLayoutInvalidation::Anchor)?;
        }
        Ok(())
    }

    /// Recapture the screen offset of the existing anchor without changing its key.
    pub(crate) fn recapture_anchor_screen_offset(
        &mut self,
        screen_offset: f32,
    ) -> Result<(), VirtualLayoutCoordinatorError> {
        self.ensure_not_executing()?;
        if !screen_offset.is_finite() {
            return Err(VirtualLayoutCoordinatorError::NonFiniteAnchor);
        }
        if let Some(anchor) = &mut self.explicit_anchor {
            anchor.screen_offset = screen_offset;
            self.invalidate(VirtualLayoutInvalidation::Anchor)?;
        }
        Ok(())
    }

    /// Coalesce one invalidation and make every older pending token stale.
    pub(crate) fn invalidate(
        &mut self,
        invalidation: VirtualLayoutInvalidation,
    ) -> Result<(), VirtualLayoutCoordinatorError> {
        self.ensure_not_executing()?;
        self.revisions.increment(invalidation)?;
        self.invalidation_epoch = self
            .invalidation_epoch
            .checked_add(1)
            .ok_or(VirtualLayoutCoordinatorError::RevisionOverflow)?;
        self.invalidations.insert(invalidation);
        Ok(())
    }

    /// Begin one exact-fenced query without a required item key.
    pub(crate) fn begin_query(
        &mut self,
        parts: VirtualLayoutQueryInputParts,
    ) -> Result<VirtualLayoutPendingQuery, VirtualLayoutCoordinatorError> {
        self.begin_query_with_required_key(parts, None)
    }

    /// Begin one exact-fenced query with the private optional required item
    /// key. The coordinator assigns the query sequence and owns supersession
    /// when the key changes.
    pub(crate) fn begin_query_with_required_key(
        &mut self,
        mut parts: VirtualLayoutQueryInputParts,
        required_key: Option<VirtualLayoutItemKey>,
    ) -> Result<VirtualLayoutPendingQuery, VirtualLayoutCoordinatorError> {
        self.ensure_not_executing()?;
        if !self.scope.matches(&parts) {
            return Err(VirtualLayoutCoordinatorError::ScopeMismatch);
        }

        let input_without_sequence = VirtualLayoutQueryInput::from_parts_with_required_key(
            parts.clone(),
            required_key.clone(),
        )
        .map_err(VirtualLayoutCoordinatorError::InvalidInput)?;
        self.sync_input(&input_without_sequence)?;

        self.query_sequence = self
            .query_sequence
            .checked_add(1)
            .ok_or(VirtualLayoutCoordinatorError::QuerySequenceOverflow)?;
        parts.query_sequence = self.query_sequence;
        let input = VirtualLayoutQueryInput::from_parts_with_required_key(parts, required_key)
            .map_err(VirtualLayoutCoordinatorError::InvalidInput)?;
        let executor = VirtualLayoutQueryExecutor::new(input.clone());
        let token = PendingToken {
            owner: Rc::clone(&self.identity),
            scope: self.scope.clone(),
            query_sequence: self.query_sequence,
            invalidation_epoch: self.invalidation_epoch,
            revisions: self.revisions,
            input: InputEvidence::from_input(&input),
            anchor: self.anchor_for_input(&input),
        };
        self.pending = Some(token.clone());
        Ok(VirtualLayoutPendingQuery {
            executor,
            token,
            executed: Cell::new(false),
            execution_phase: Rc::clone(&self.execution_phase),
        })
    }

    /// Complete one token.  Policy execution must have happened before this call.
    pub(crate) fn complete(
        &mut self,
        pending: VirtualLayoutPendingQuery,
        outcome: VirtualLayoutQueryOutcome,
    ) -> VirtualLayoutCompletion {
        if self.execution_phase.get() {
            let diagnostic = VirtualLayoutCoordinatorDiagnostic {
                code: VirtualLayoutCoordinatorDiagnosticCode::ReentrantExecution,
            };
            self.last_diagnostic = Some(diagnostic);
            return VirtualLayoutCompletion::Rejected(diagnostic);
        }
        if !self.token_is_current(&pending.token) {
            let diagnostic = VirtualLayoutCoordinatorDiagnostic {
                code: VirtualLayoutCoordinatorDiagnosticCode::StaleQuery,
            };
            self.last_diagnostic = Some(diagnostic);
            return VirtualLayoutCompletion::Stale(diagnostic);
        }

        self.pending = None;
        match outcome {
            VirtualLayoutQueryOutcome::Ready(result) => match pending.executor.accept(result) {
                VirtualLayoutQueryOutcome::Ready(result) => self.commit_ready(pending, result),
                VirtualLayoutQueryOutcome::Invalid(diagnostics) => {
                    self.retained_invalid(pending.token.input.clone(), diagnostics)
                }
                VirtualLayoutQueryOutcome::Unavailable(reason) => self.retained_non_ready(
                    pending.token.input.clone(),
                    VirtualLayoutRetainReason::Unavailable(reason),
                ),
                VirtualLayoutQueryOutcome::Deferred(reason) => self.retained_non_ready(
                    pending.token.input.clone(),
                    VirtualLayoutRetainReason::Deferred(reason),
                ),
            },
            VirtualLayoutQueryOutcome::Unavailable(reason) => self.retained_non_ready(
                pending.token.input.clone(),
                VirtualLayoutRetainReason::Unavailable(reason),
            ),
            VirtualLayoutQueryOutcome::Deferred(reason) => self.retained_non_ready(
                pending.token.input.clone(),
                VirtualLayoutRetainReason::Deferred(reason),
            ),
            VirtualLayoutQueryOutcome::Invalid(diagnostics) => {
                self.retained_invalid(pending.token.input.clone(), diagnostics)
            }
        }
    }

    fn ensure_not_executing(&self) -> Result<(), VirtualLayoutCoordinatorError> {
        if self.execution_phase.get() {
            Err(VirtualLayoutCoordinatorError::ReentrantExecution)
        } else {
            Ok(())
        }
    }

    /// Return the accepted authoritative window, if one exists.
    #[must_use]
    pub(crate) fn accepted(&self) -> Option<&VirtualLayoutAcceptedWindow> {
        self.accepted.as_ref()
    }

    /// Return the coordinator-owned evidence that authorizes private
    /// materialization of one committed window.
    pub(super) fn owner_evidence(&self) -> Rc<CoordinatorIdentity> {
        Rc::clone(&self.identity)
    }

    /// Return the eligible, clipped previous-valid fallback for a new input.
    #[must_use]
    pub(crate) fn fallback_for(&self, input: &VirtualLayoutQueryInput) -> VirtualLayoutWindowView {
        if !self.scope.matches_input(input) {
            return VirtualLayoutWindowView::empty();
        }
        self.fallback_for_evidence(
            &InputEvidence::from_input(input),
            VirtualLayoutCoordinatorRevisions::from_input(input),
        )
    }

    fn sync_input(
        &mut self,
        input: &VirtualLayoutQueryInput,
    ) -> Result<(), VirtualLayoutCoordinatorError> {
        let evidence = InputEvidence::from_input(input);
        if !self.initialized_revisions {
            self.revisions = VirtualLayoutCoordinatorRevisions::from_input(input);
            self.initialized_revisions = true;
            self.last_input = Some(evidence);
            return Ok(());
        }

        if input.viewport_revision() < self.revisions.viewport
            || input.data_revision() < self.revisions.data
            || input.policy_revision() < self.revisions.policy
            || input.measurement_revision() < self.revisions.measurement
            || input.semantic_revision() < self.revisions.semantic
        {
            return Err(VirtualLayoutCoordinatorError::RevisionRegression);
        }

        let mut changed = [false; 7];
        if self.revisions.viewport != input.viewport_revision() {
            self.revisions.viewport = input.viewport_revision();
            changed[0] = true;
        }
        if self.revisions.data != input.data_revision() {
            self.revisions.data = input.data_revision();
            changed[1] = true;
        }
        if self.revisions.policy != input.policy_revision() {
            self.revisions.policy = input.policy_revision();
            changed[2] = true;
        }
        if self.revisions.measurement != input.measurement_revision() {
            self.revisions.measurement = input.measurement_revision();
            changed[3] = true;
        }
        if self.revisions.semantic != input.semantic_revision() {
            self.revisions.semantic = input.semantic_revision();
            changed[4] = true;
        }

        if let Some(previous) = &self.last_input {
            if previous.viewport != evidence.viewport
                || previous.coordinate_space != evidence.coordinate_space
            {
                changed[0] = true;
            }
            if previous.overscan != evidence.overscan || previous.budget != evidence.budget {
                changed[5] = true;
            }
            if !same_optional_key(
                previous.required_key.as_ref(),
                evidence.required_key.as_ref(),
            ) {
                changed[6] = true;
            }
        }

        if changed[6] {
            self.revisions.required_key = self
                .revisions
                .required_key
                .checked_add(1)
                .ok_or(VirtualLayoutCoordinatorError::RevisionOverflow)?;
        }

        for (index, invalidation) in [
            VirtualLayoutInvalidation::Viewport,
            VirtualLayoutInvalidation::Data,
            VirtualLayoutInvalidation::Policy,
            VirtualLayoutInvalidation::Measurement,
            VirtualLayoutInvalidation::Semantic,
            VirtualLayoutInvalidation::OverscanBudget,
            VirtualLayoutInvalidation::RequiredKey,
        ]
        .into_iter()
        .enumerate()
        {
            if changed[index] {
                self.invalidation_epoch = self
                    .invalidation_epoch
                    .checked_add(1)
                    .ok_or(VirtualLayoutCoordinatorError::RevisionOverflow)?;
                self.invalidations.insert(invalidation);
            }
        }
        self.last_input = Some(evidence);
        Ok(())
    }

    fn token_is_current(&self, token: &PendingToken) -> bool {
        Rc::ptr_eq(&self.identity, &token.owner)
            && self.pending.as_ref().is_some_and(|current| {
                Rc::ptr_eq(&current.owner, &token.owner)
                    && current.query_sequence == token.query_sequence
                    && current.invalidation_epoch == token.invalidation_epoch
                    && current.scope == token.scope
            })
            && token.query_sequence == self.query_sequence
            && token.invalidation_epoch == self.invalidation_epoch
    }

    fn retained_invalid(
        &self,
        input: InputEvidence,
        diagnostics: VirtualLayoutDiagnostics,
    ) -> VirtualLayoutCompletion {
        VirtualLayoutCompletion::Retained {
            reason: VirtualLayoutRetainReason::Invalid,
            view: Box::new(self.fallback_for_evidence(&input, self.revisions)),
            diagnostics: Box::new(Some(limit_diagnostics(diagnostics))),
        }
    }

    fn retained_non_ready(
        &self,
        input: InputEvidence,
        reason: VirtualLayoutRetainReason,
    ) -> VirtualLayoutCompletion {
        VirtualLayoutCompletion::Retained {
            reason,
            view: Box::new(self.fallback_for_evidence(&input, self.revisions)),
            diagnostics: Box::new(None),
        }
    }

    fn commit_ready(
        &mut self,
        pending: VirtualLayoutPendingQuery,
        result: VirtualLayoutQueryResult,
    ) -> VirtualLayoutCompletion {
        let old = self.accepted.as_ref();
        let next_entries = VirtualLayoutAcceptedWindow::bounded_entries(result.entries.clone());
        let next_anchor = Self::resolve_anchor(pending.token.anchor.clone(), &next_entries);
        let correction = self.anchor_correction(old, &next_entries, &result, next_anchor.as_ref());
        let delta = old.map_or_else(VirtualLayoutKeyDelta::default, |old| {
            key_delta(&old.entries, &next_entries)
        });

        let accepted_revision =
            match self.accepted.as_ref() {
                Some(window) => match window.accepted_revision.checked_add(1) {
                    Some(revision) => revision,
                    None => {
                        return VirtualLayoutCompletion::Retained {
                            reason: VirtualLayoutRetainReason::Invalid,
                            view: Box::new(self.fallback_for_evidence(
                                &pending.token.input,
                                pending.token.revisions,
                            )),
                            diagnostics: Box::new(None),
                        };
                    }
                },
                None => 1,
            };

        let accepted = VirtualLayoutAcceptedWindow {
            fence: result.fence,
            extent: result.extent,
            entries: next_entries,
            delta: delta.clone(),
            anchor: next_anchor.clone(),
            correction,
            accepted_revision,
        };
        let commit_fence = accepted.fence.clone();
        let view = VirtualLayoutWindowView {
            entries: accepted.entries.clone(),
            extent: Some(accepted.extent),
            accepted_revision: Some(accepted.accepted_revision),
            fallback: false,
            clip: None,
        };
        self.accepted = Some(accepted);
        self.invalidations = VirtualLayoutInvalidationFlags::default();
        VirtualLayoutCompletion::Committed(Box::new(VirtualLayoutCommit {
            fence: commit_fence,
            owner: Rc::clone(&self.identity),
            view,
            delta,
            anchor: next_anchor,
            correction,
            accepted_revision,
        }))
    }

    fn anchor_for_input(&self, input: &VirtualLayoutQueryInput) -> Option<VirtualLayoutAnchor> {
        if let Some(anchor) = &self.explicit_anchor {
            return Some(anchor.clone());
        }
        let accepted = self.accepted.as_ref()?;
        accepted
            .entries
            .iter()
            .filter(|entry| entry.visibility() == VirtualLayoutVisibility::Visible)
            .find(|entry| entry.bounds().intersects(input.viewport()))
            .map(|entry| VirtualLayoutAnchor {
                key: entry.key().clone(),
                axis: VirtualLayoutAnchorAxis::Vertical,
                edge: VirtualLayoutAnchorEdge::Leading,
                local_offset: 0.0,
                screen_offset: entry.bounds().min.y - input.viewport().min.y,
            })
    }

    fn resolve_anchor(
        pending_anchor: Option<VirtualLayoutAnchor>,
        next_entries: &[VirtualLayoutItem],
    ) -> Option<VirtualLayoutAnchor> {
        let Some(anchor) = pending_anchor else {
            return next_entries
                .iter()
                .find(|entry| entry.visibility() == VirtualLayoutVisibility::Visible)
                .map(|entry| VirtualLayoutAnchor {
                    key: entry.key().clone(),
                    axis: VirtualLayoutAnchorAxis::Vertical,
                    edge: VirtualLayoutAnchorEdge::Leading,
                    local_offset: 0.0,
                    screen_offset: entry.bounds().min.y,
                });
        };
        // A bounded result does not prove that an explicit key was removed.
        // Keep the coordinator-owned key unresolved until it reappears.
        next_entries
            .iter()
            .any(|entry| key_matches(entry.key(), &anchor.key))
            .then_some(anchor)
    }

    fn anchor_correction(
        &self,
        old: Option<&VirtualLayoutAcceptedWindow>,
        next_entries: &[VirtualLayoutItem],
        result: &VirtualLayoutQueryResult,
        next_anchor: Option<&VirtualLayoutAnchor>,
    ) -> Option<VirtualLayoutAnchorCorrection> {
        let anchor = next_anchor?;
        let old_window = old?;
        let old_item = old_window
            .entries
            .iter()
            .find(|entry| key_matches(entry.key(), &anchor.key))?;
        let next_item = next_entries
            .iter()
            .find(|entry| key_matches(entry.key(), &anchor.key))?;
        let next_point = anchor_point(next_item, anchor);
        let new_origin = match anchor.axis {
            VirtualLayoutAnchorAxis::Horizontal => result.fence.viewport().min.x,
            VirtualLayoutAnchorAxis::Vertical => result.fence.viewport().min.y,
        };
        let raw_delta = next_point - anchor.screen_offset - new_origin;
        if !raw_delta.is_finite() {
            return None;
        }
        let confidence = if result.extent.kind() == VirtualLayoutExtentKind::Exact
            && old_item.confidence() == VirtualLayoutBoundsConfidence::Exact
            && next_item.confidence() == VirtualLayoutBoundsConfidence::Exact
        {
            VirtualLayoutCorrectionConfidence::Exact
        } else if result.extent.kind() == VirtualLayoutExtentKind::Unavailable {
            return None;
        } else {
            VirtualLayoutCorrectionConfidence::Estimated
        };
        let delta = clamp_correction(
            raw_delta,
            result.fence.viewport(),
            result.extent,
            anchor.axis,
        );
        Some(VirtualLayoutAnchorCorrection { delta, confidence })
    }

    fn fallback_for_evidence(
        &self,
        current: &InputEvidence,
        revisions: VirtualLayoutCoordinatorRevisions,
    ) -> VirtualLayoutWindowView {
        let Some(accepted) = &self.accepted else {
            return VirtualLayoutWindowView::empty();
        };
        let fence = &accepted.fence;
        if !revisions.fallback_revisions_match(fence)
            || !accepted
                .fence
                .policy_identity()
                .stable_equals(&self.scope.policy_identity.0)
                .is_some_and(|same| same)
            || accepted.fence.container_id() != self.scope.container_id
            || accepted.fence.mount_generation() != self.scope.mount_generation
            || !coordinate_space_matches(
                accepted.fence.coordinate_space(),
                &current.coordinate_space,
            )
            || !same_optional_key(accepted.fence.required_key(), current.required_key.as_ref())
        {
            return VirtualLayoutWindowView::empty();
        }
        let clip = fence.viewport().intersection(current.viewport);
        let Some(clip) = clip else {
            return VirtualLayoutWindowView {
                entries: Vec::new(),
                extent: Some(accepted.extent),
                accepted_revision: Some(accepted.accepted_revision),
                fallback: true,
                clip: None,
            };
        };
        let entries = accepted
            .entries
            .iter()
            .filter(|entry| entry.bounds().intersects(clip))
            .cloned()
            .collect();
        VirtualLayoutWindowView {
            entries,
            extent: Some(accepted.extent),
            accepted_revision: Some(accepted.accepted_revision),
            fallback: true,
            clip: Some(clip),
        }
    }
}

fn limit_diagnostics(mut diagnostics: VirtualLayoutDiagnostics) -> VirtualLayoutDiagnostics {
    if diagnostics.len() <= MAX_COORDINATOR_DIAGNOSTICS {
        return diagnostics;
    }
    let Some(first) = diagnostics.get(0) else {
        return diagnostics;
    };
    let mut limited = VirtualLayoutDiagnostics::default();
    limited.push(first);
    diagnostics = limited;
    diagnostics
}

fn key_matches(left: &VirtualLayoutItemKey, right: &VirtualLayoutItemKey) -> bool {
    left.stable_equals(right) == Some(true)
}

fn same_optional_key(
    left: Option<&VirtualLayoutItemKey>,
    right: Option<&VirtualLayoutItemKey>,
) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => left.stable_equals(right) == Some(true),
        _ => false,
    }
}

fn key_change_sort_key(change: &VirtualLayoutKeyChange) -> (usize, u8) {
    // Order by the reported logical position. At one position, remove before
    // insert makes same-index replacement deterministic; moves follow both.
    match change.kind {
        VirtualLayoutKeyChangeKind::Removed { index } => (index, 0),
        VirtualLayoutKeyChangeKind::Inserted { index } => (index, 1),
        VirtualLayoutKeyChangeKind::Moved { to, .. } => (to, 2),
    }
}

fn coordinate_space_matches(
    left: &VirtualLayoutCoordinateSpace,
    right: &VirtualLayoutCoordinateSpace,
) -> bool {
    match (left, right) {
        (VirtualLayoutCoordinateSpace::Logical, VirtualLayoutCoordinateSpace::Logical) => true,
        (
            VirtualLayoutCoordinateSpace::Custom(left),
            VirtualLayoutCoordinateSpace::Custom(right),
        ) => left.stable_equals(right) == Some(true),
        _ => false,
    }
}

fn key_delta(old: &[VirtualLayoutItem], next: &[VirtualLayoutItem]) -> VirtualLayoutKeyDelta {
    let mut delta = VirtualLayoutKeyDelta::default();
    for old_entry in old.iter().take(VIRTUAL_LAYOUT_MAX_COORDINATOR_RECORDS) {
        let next_entry = next
            .iter()
            .take(VIRTUAL_LAYOUT_MAX_COORDINATOR_RECORDS)
            .find(|next_entry| key_matches(old_entry.key(), next_entry.key()));
        if let Some(next_entry) = next_entry {
            if old_entry.logical_index() != next_entry.logical_index() {
                delta.push(VirtualLayoutKeyChange {
                    key: next_entry.key().clone(),
                    kind: VirtualLayoutKeyChangeKind::Moved {
                        from: old_entry.logical_index(),
                        to: next_entry.logical_index(),
                    },
                });
            }
        } else {
            delta.push(VirtualLayoutKeyChange {
                key: old_entry.key().clone(),
                kind: VirtualLayoutKeyChangeKind::Removed {
                    index: old_entry.logical_index(),
                },
            });
        }
    }
    for next_entry in next.iter().take(VIRTUAL_LAYOUT_MAX_COORDINATOR_RECORDS) {
        if !old
            .iter()
            .take(VIRTUAL_LAYOUT_MAX_COORDINATOR_RECORDS)
            .any(|old_entry| key_matches(old_entry.key(), next_entry.key()))
        {
            delta.push(VirtualLayoutKeyChange {
                key: next_entry.key().clone(),
                kind: VirtualLayoutKeyChangeKind::Inserted {
                    index: next_entry.logical_index(),
                },
            });
        }
    }
    delta
}

fn anchor_edge(
    item: &VirtualLayoutItem,
    edge: VirtualLayoutAnchorEdge,
    axis: VirtualLayoutAnchorAxis,
) -> f32 {
    match (axis, edge) {
        (VirtualLayoutAnchorAxis::Horizontal, VirtualLayoutAnchorEdge::Leading) => {
            item.bounds().min.x
        }
        (VirtualLayoutAnchorAxis::Horizontal, VirtualLayoutAnchorEdge::Trailing) => {
            item.bounds().max.x
        }
        (VirtualLayoutAnchorAxis::Vertical, VirtualLayoutAnchorEdge::Leading) => {
            item.bounds().min.y
        }
        (VirtualLayoutAnchorAxis::Vertical, VirtualLayoutAnchorEdge::Trailing) => {
            item.bounds().max.y
        }
    }
}

fn anchor_point(item: &VirtualLayoutItem, anchor: &VirtualLayoutAnchor) -> f32 {
    let edge = anchor_edge(item, anchor.edge, anchor.axis);
    match anchor.edge {
        VirtualLayoutAnchorEdge::Leading => edge + anchor.local_offset,
        VirtualLayoutAnchorEdge::Trailing => edge - anchor.local_offset,
    }
}

fn clamp_correction(
    raw_delta: f32,
    new_viewport: Rect,
    extent: VirtualLayoutExtent,
    axis: VirtualLayoutAnchorAxis,
) -> f32 {
    let Some(size) = extent.size() else {
        return raw_delta;
    };
    let (new_origin, viewport_length, extent_length) = match axis {
        VirtualLayoutAnchorAxis::Horizontal => (new_viewport.min.x, new_viewport.width(), size.x),
        VirtualLayoutAnchorAxis::Vertical => (new_viewport.min.y, new_viewport.height(), size.y),
    };
    if !new_origin.is_finite() || !viewport_length.is_finite() || !extent_length.is_finite() {
        return raw_delta;
    }
    let max_origin = (extent_length - viewport_length).max(0.0);
    let desired = (new_origin + raw_delta).clamp(0.0, max_origin);
    desired - new_origin
}

#[cfg(test)]
mod tests {
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    use super::super::VirtualLayoutItemCandidate;
    use super::*;
    use crate::gui::types::Vector2;

    use super::super::materialization::VirtualLayoutSlotIdentity;
    use super::super::materialization::{
        VirtualLayoutHostProjector, VirtualLayoutLifecycleAdapter,
        VirtualLayoutMaterializationError, VirtualLayoutMaterializationReentry,
        VirtualLayoutMaterializationStore, VirtualLayoutProjection,
        VirtualLayoutProjectionEvidence, VirtualLayoutProjectionKind,
    };
    use crate::application::{column, empty, text};

    fn parts(
        query_sequence: u64,
        viewport: Rect,
        data_revision: u64,
        budget: usize,
    ) -> VirtualLayoutQueryInputParts {
        VirtualLayoutQueryInputParts {
            container_id: 41,
            policy_identity: VirtualLayoutPolicyIdentity::new("policy"),
            mount_generation: 7,
            query_sequence,
            viewport,
            coordinate_space: VirtualLayoutCoordinateSpace::logical(),
            overscan: super::super::VirtualLayoutOverscan::new(0.0, 0.0).expect("valid overscan"),
            budget: VirtualLayoutBudget::new(budget),
            viewport_revision: 1,
            data_revision,
            policy_revision: 1,
            measurement_revision: 1,
            semantic_revision: 1,
        }
    }

    fn candidate(key: u32, index: usize, y: f32) -> VirtualLayoutItemCandidate {
        VirtualLayoutItemCandidate::new(
            VirtualLayoutItemKey::new(key),
            index,
            Rect::from_xy_size(0.0, y, 100.0, 10.0),
            VirtualLayoutVisibility::Visible,
            VirtualLayoutBoundsConfidence::Exact,
        )
    }

    fn ready_policy(keys: &[u32], height: f32) -> impl VirtualLayoutPolicy + '_ {
        struct Policy<'a> {
            keys: &'a [u32],
            height: f32,
        }
        impl VirtualLayoutPolicy for Policy<'_> {
            fn query(
                &self,
                _input: &VirtualLayoutQueryInput,
                sink: &mut super::super::VirtualLayoutQuerySink,
            ) -> super::super::VirtualLayoutPolicyDecision {
                for (index, key) in self.keys.iter().copied().enumerate() {
                    let _ = sink.visit(candidate(key, index, index as f32 * 10.0));
                }
                let _ = sink.set_extent(super::super::VirtualLayoutExtentCandidate::exact(
                    Vector2::new(100.0, self.height),
                ));
                super::super::VirtualLayoutPolicyDecision::Ready
            }
        }
        Policy { keys, height }
    }

    fn ready_entries_policy(entries: &[(u32, usize, f32)]) -> impl VirtualLayoutPolicy + '_ {
        struct Policy<'a> {
            entries: &'a [(u32, usize, f32)],
        }
        impl VirtualLayoutPolicy for Policy<'_> {
            fn query(
                &self,
                _input: &VirtualLayoutQueryInput,
                sink: &mut super::super::VirtualLayoutQuerySink,
            ) -> super::super::VirtualLayoutPolicyDecision {
                for &(key, index, y) in self.entries {
                    let _ = sink.visit(candidate(key, index, y));
                }
                let _ = sink.set_extent(super::super::VirtualLayoutExtentCandidate::exact(
                    Vector2::new(100.0, 100.0),
                ));
                super::super::VirtualLayoutPolicyDecision::Ready
            }
        }
        Policy { entries }
    }

    fn commit_policy(
        coordinator: &mut VirtualLayoutWindowCoordinator,
        policy: &dyn VirtualLayoutPolicy,
    ) -> Box<VirtualLayoutCommit> {
        let pending = coordinator
            .begin_query(parts(0, Rect::from_xy_size(0.0, 0.0, 100.0, 20.0), 1, 8))
            .expect("query should begin");
        let outcome = pending.execute(policy);
        let VirtualLayoutCompletion::Committed(commit) = coordinator.complete(pending, outcome)
        else {
            panic!("query should commit")
        };
        commit
    }

    // Malformed commit copies are constructed only in this coordinator-owned
    // test boundary. Production siblings receive no constructor or mutator.
    fn clone_commit_with_entries(
        commit: &VirtualLayoutCommit,
        entries: Vec<VirtualLayoutItem>,
    ) -> Box<VirtualLayoutCommit> {
        let mut clone = Box::new(commit.clone());
        clone.view.entries = entries;
        clone
    }

    struct DispositionPolicy {
        decision: super::super::VirtualLayoutPolicyDecision,
    }

    impl VirtualLayoutPolicy for DispositionPolicy {
        fn query(
            &self,
            _input: &VirtualLayoutQueryInput,
            _sink: &mut super::super::VirtualLayoutQuerySink,
        ) -> super::super::VirtualLayoutPolicyDecision {
            self.decision
        }
    }

    fn assert_explicit_anchor_key(coordinator: &VirtualLayoutWindowCoordinator, expected_key: u32) {
        assert_eq!(
            coordinator
                .explicit_anchor
                .as_ref()
                .expect("explicit anchor should remain set")
                .key,
            VirtualLayoutItemKey::new(expected_key)
        );
    }

    fn ready(coordinator: &mut VirtualLayoutWindowCoordinator, keys: &[u32]) {
        let policy = ready_policy(keys, 100.0);
        let _ = commit_policy(coordinator, &policy);
    }

    #[test]
    fn retained_adapter_admits_complete_batch_and_preserves_slot_tuples() {
        let mut coordinator =
            VirtualLayoutWindowCoordinator::new(41, VirtualLayoutPolicyIdentity::new("policy"), 7);
        let policy = ready_policy(&[1, 2], 100.0);
        let commit = commit_policy(&mut coordinator, &policy);
        let result = super::super::adapter::admit_virtual_layout_batch(
            &commit,
            column([text::<()>("shell")]),
            vec![
                (
                    VirtualLayoutItemKey::new(1_u32),
                    text::<()>("one"),
                    VirtualLayoutSlotIdentity::from_parts(41, 7, 4, 1),
                ),
                (
                    VirtualLayoutItemKey::new(2_u32),
                    text::<()>("two"),
                    VirtualLayoutSlotIdentity::from_parts(41, 7, 5, 2),
                ),
            ],
        )
        .expect("complete retained batch should be admitted");
        assert_eq!(result.shell.id(), 41);
        assert_eq!(result.items.len(), 2);
        assert_eq!(
            result.items[0].item.key(),
            &VirtualLayoutItemKey::new(1_u32)
        );
        assert_eq!(result.items[0].slot.slot_index(), 4);
        assert_eq!(result.items[0].slot.checked_generation(), 1);
        assert_eq!(result.items[1].slot.slot_index(), 5);
        assert_eq!(result.items[1].slot.checked_generation(), 2);
    }

    #[test]
    fn retained_adapter_rejects_incomplete_or_colliding_batches_before_lowering() {
        let mut coordinator =
            VirtualLayoutWindowCoordinator::new(41, VirtualLayoutPolicyIdentity::new("policy"), 7);
        let policy = ready_policy(&[1, 2], 100.0);
        let commit = commit_policy(&mut coordinator, &policy);
        let slot = || VirtualLayoutSlotIdentity::from_parts(41, 7, 4, 1);

        assert!(matches!(
            super::super::adapter::admit_virtual_layout_batch(
                &commit,
                empty::<()>(),
                vec![(VirtualLayoutItemKey::new(1_u32), text::<()>("one"), slot())],
            ),
            Err(super::super::adapter::VirtualLayoutRetainedBatchError::MissingItem)
        ));
        assert!(matches!(
            super::super::adapter::admit_virtual_layout_batch(
                &commit,
                empty::<()>(),
                vec![
                    (VirtualLayoutItemKey::new(1_u32), text::<()>("one"), slot()),
                    (
                        VirtualLayoutItemKey::new(1_u32),
                        text::<()>("duplicate"),
                        VirtualLayoutSlotIdentity::from_parts(41, 7, 5, 1)
                    ),
                ],
            ),
            Err(super::super::adapter::VirtualLayoutRetainedBatchError::DuplicateItem)
        ));
        assert!(matches!(
            super::super::adapter::admit_virtual_layout_batch(
                &commit,
                empty::<()>(),
                vec![
                    (VirtualLayoutItemKey::new(1_u32), text::<()>("one"), slot()),
                    (VirtualLayoutItemKey::new(2_u32), text::<()>("two"), slot()),
                ],
            ),
            Err(super::super::adapter::VirtualLayoutRetainedBatchError::SlotCollision)
        ));
    }

    struct TestMaterializationProjector;

    impl VirtualLayoutHostProjector for TestMaterializationProjector {
        type Payload = ();
        type Error = ();

        fn projection_kind(
            &self,
            _item: &VirtualLayoutItem,
        ) -> Result<VirtualLayoutProjectionKind, Self::Error> {
            Ok(VirtualLayoutProjectionKind::new("test-kind"))
        }

        fn project<'a>(
            &self,
            _evidence: VirtualLayoutProjectionEvidence<'a>,
        ) -> Result<VirtualLayoutProjection<Self::Payload>, Self::Error> {
            Ok(VirtualLayoutProjection::new(
                VirtualLayoutProjectionKind::new("test-kind"),
                (),
            ))
        }
    }

    struct TestMaterializationLifecycle;

    impl VirtualLayoutLifecycleAdapter<()> for TestMaterializationLifecycle {
        type Error = ();

        fn compatible(
            &self,
            _previous: &VirtualLayoutProjectionKind,
            _next: &VirtualLayoutProjectionKind,
        ) -> Option<bool> {
            Some(true)
        }

        fn unmount(
            &mut self,
            _payload: &(),
            _evidence: VirtualLayoutProjectionEvidence<'_>,
            _reentry: &VirtualLayoutMaterializationReentry<'_>,
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        fn reset(
            &mut self,
            _payload: &(),
            _evidence: VirtualLayoutProjectionEvidence<'_>,
            _reentry: &VirtualLayoutMaterializationReentry<'_>,
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        fn reconcile(
            &mut self,
            _previous: &(),
            _next: &(),
            _evidence: VirtualLayoutProjectionEvidence<'_>,
            _reentry: &VirtualLayoutMaterializationReentry<'_>,
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        fn mount(
            &mut self,
            _recycled_shell: Option<&()>,
            _next: &(),
            _evidence: VirtualLayoutProjectionEvidence<'_>,
            _reentry: &VirtualLayoutMaterializationReentry<'_>,
        ) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    #[derive(Debug)]
    struct FlakyKey {
        state: Rc<Cell<bool>>,
    }

    impl PartialEq for FlakyKey {
        fn eq(&self, _other: &Self) -> bool {
            let value = self.state.get();
            self.state.set(!value);
            value
        }
    }

    impl Eq for FlakyKey {}

    #[test]
    fn materialization_rejects_forged_entries_within_coordinator_test_boundary() {
        let mut coordinator =
            VirtualLayoutWindowCoordinator::new(41, VirtualLayoutPolicyIdentity::new("policy"), 7);
        let valid = commit_policy(&mut coordinator, &ready_policy(&[1], 100.0));
        let mut store =
            VirtualLayoutMaterializationStore::new(&coordinator, TestMaterializationLifecycle);
        let projector = TestMaterializationProjector;
        let first = valid.view().entries[0].clone();

        let mut duplicate_key_entry = first.clone();
        duplicate_key_entry.logical_index = 1;
        let duplicate_key =
            clone_commit_with_entries(&valid, vec![first.clone(), duplicate_key_entry]);
        assert!(matches!(
            store.publish(&duplicate_key, &projector),
            Err(VirtualLayoutMaterializationError::DuplicateKey)
        ));

        let mut duplicate_index_entry = first.clone();
        duplicate_index_entry.key = VirtualLayoutItemKey::new(99_u32);
        let duplicate_index =
            clone_commit_with_entries(&valid, vec![first.clone(), duplicate_index_entry]);
        assert!(matches!(
            store.publish(&duplicate_index, &projector),
            Err(VirtualLayoutMaterializationError::DuplicateLogicalIndex)
        ));

        let state = Rc::new(Cell::new(true));
        let unstable_key = clone_commit_with_entries(
            &valid,
            vec![VirtualLayoutItem {
                key: VirtualLayoutItemKey::new(FlakyKey { state }),
                logical_index: 0,
                bounds: Rect::from_xy_size(0.0, 0.0, 100.0, 10.0),
                visibility: VirtualLayoutVisibility::Visible,
                confidence: VirtualLayoutBoundsConfidence::Exact,
            }],
        );
        assert!(matches!(
            store.publish(&unstable_key, &projector),
            Err(VirtualLayoutMaterializationError::UnstableKey)
        ));
        assert_eq!(store.active_len(), 0);
    }

    #[test]
    fn initial_ready_commit_is_fenced_and_bounded() {
        let mut coordinator =
            VirtualLayoutWindowCoordinator::new(41, VirtualLayoutPolicyIdentity::new("policy"), 7);
        ready(&mut coordinator, &[1, 2]);
        let accepted = coordinator.accepted().expect("accepted window");
        assert_eq!(accepted.entries.len(), 2);
        assert_eq!(accepted.accepted_revision, 1);
        assert_eq!(accepted.fence.query_sequence(), 1);
        assert_eq!(
            coordinator.invalidations(),
            VirtualLayoutInvalidationFlags::default()
        );
    }

    #[test]
    fn materialization_boundary_can_only_borrow_coordinator_admitted_commit_evidence() {
        let mut coordinator =
            VirtualLayoutWindowCoordinator::new(41, VirtualLayoutPolicyIdentity::new("policy"), 7);
        let commit = commit_policy(&mut coordinator, &ready_policy(&[1, 2], 100.0));
        let clone = commit.clone();

        // This regression stays in coordinator::tests because only the
        // coordinator boundary may name or mutate commit fields. The sibling
        // materialization module receives only these immutable accessors.
        assert_eq!(clone.fence(), commit.fence());
        assert!(Rc::ptr_eq(clone.owner(), commit.owner()));
        assert_eq!(clone.view().entries.len(), 2);
        assert_eq!(clone.accepted_revision(), 1);
        assert_eq!(
            clone.view().accepted_revision,
            Some(clone.accepted_revision())
        );
    }

    #[test]
    fn policy_executes_before_commit_and_invalid_output_is_atomic() {
        struct CountingPolicy {
            calls: Cell<u32>,
        }
        impl VirtualLayoutPolicy for CountingPolicy {
            fn query(
                &self,
                _input: &VirtualLayoutQueryInput,
                sink: &mut super::super::VirtualLayoutQuerySink,
            ) -> super::super::VirtualLayoutPolicyDecision {
                self.calls.set(self.calls.get() + 1);
                let _ = sink.visit(candidate(1, 0, 0.0));
                super::super::VirtualLayoutPolicyDecision::Ready
            }
        }

        let mut coordinator =
            VirtualLayoutWindowCoordinator::new(41, VirtualLayoutPolicyIdentity::new("policy"), 7);
        ready(&mut coordinator, &[1]);
        let before = coordinator.accepted().expect("accepted").accepted_revision;
        let pending = coordinator
            .begin_query(parts(0, Rect::from_xy_size(0.0, 0.0, 100.0, 20.0), 1, 8))
            .expect("query should begin");
        let policy = CountingPolicy {
            calls: Cell::new(0),
        };
        let outcome = pending.execute(&policy);
        assert_eq!(policy.calls.get(), 1);
        assert!(matches!(
            coordinator.complete(pending, outcome),
            VirtualLayoutCompletion::Retained {
                reason: VirtualLayoutRetainReason::Invalid,
                ..
            }
        ));
        assert_eq!(
            coordinator.accepted().expect("accepted").accepted_revision,
            before
        );
    }

    #[test]
    fn policy_reentry_is_rejected_while_execution_phase_is_active() {
        struct ReentrantPolicy {
            coordinator: Rc<RefCell<VirtualLayoutWindowCoordinator>>,
            rejected: Cell<bool>,
        }

        impl VirtualLayoutPolicy for ReentrantPolicy {
            fn query(
                &self,
                input: &VirtualLayoutQueryInput,
                sink: &mut super::super::VirtualLayoutQuerySink,
            ) -> super::super::VirtualLayoutPolicyDecision {
                let attempted = self.coordinator.borrow_mut().begin_query(parts(
                    0,
                    input.viewport(),
                    input.data_revision(),
                    input.budget().max_entries(),
                ));
                self.rejected.set(matches!(
                    attempted,
                    Err(VirtualLayoutCoordinatorError::ReentrantExecution)
                ));
                let _ = sink.visit(candidate(1, 0, 0.0));
                let _ = sink.set_extent(super::super::VirtualLayoutExtentCandidate::exact(
                    Vector2::new(100.0, 100.0),
                ));
                super::super::VirtualLayoutPolicyDecision::Ready
            }
        }

        let coordinator = Rc::new(RefCell::new(VirtualLayoutWindowCoordinator::new(
            41,
            VirtualLayoutPolicyIdentity::new("policy"),
            7,
        )));
        let pending = coordinator
            .borrow_mut()
            .begin_query(parts(0, Rect::from_xy_size(0.0, 0.0, 100.0, 20.0), 1, 8))
            .expect("query should begin");
        let policy = ReentrantPolicy {
            coordinator: Rc::clone(&coordinator),
            rejected: Cell::new(false),
        };
        let outcome = pending.execute(&policy);
        assert!(policy.rejected.get());
        assert!(matches!(
            coordinator.borrow_mut().complete(pending, outcome),
            VirtualLayoutCompletion::Committed(_)
        ));
    }

    #[test]
    fn same_scope_coordinators_reject_foreign_tokens_without_consuming_local_pending() {
        let mut coordinator_a =
            VirtualLayoutWindowCoordinator::new(41, VirtualLayoutPolicyIdentity::new("policy"), 7);
        let mut coordinator_b =
            VirtualLayoutWindowCoordinator::new(41, VirtualLayoutPolicyIdentity::new("policy"), 7);
        let pending_a = coordinator_a
            .begin_query(parts(0, Rect::from_xy_size(0.0, 0.0, 100.0, 20.0), 1, 8))
            .expect("A query should begin");
        let pending_b = coordinator_b
            .begin_query(parts(0, Rect::from_xy_size(0.0, 0.0, 100.0, 20.0), 1, 8))
            .expect("B query should begin");
        assert_eq!(pending_a.query_sequence(), 1);
        assert_eq!(pending_b.query_sequence(), 1);
        assert_eq!(
            coordinator_a.invalidation_epoch,
            coordinator_b.invalidation_epoch
        );

        let revisions_before = coordinator_a.revisions();
        let epoch_before = coordinator_a.invalidation_epoch;
        let invalidations_before = coordinator_a.invalidations();
        assert!(coordinator_a.accepted().is_none());
        assert!(coordinator_a.explicit_anchor.is_none());

        let outcome_b = pending_b.execute(&ready_policy(&[2], 100.0));
        assert!(matches!(
            coordinator_a.complete(pending_b, outcome_b),
            VirtualLayoutCompletion::Stale(diagnostic)
                if diagnostic.code == VirtualLayoutCoordinatorDiagnosticCode::StaleQuery
        ));
        assert!(coordinator_a.accepted().is_none());
        assert_eq!(coordinator_a.revisions(), revisions_before);
        assert_eq!(coordinator_a.invalidation_epoch, epoch_before);
        assert_eq!(coordinator_a.invalidations(), invalidations_before);
        assert!(coordinator_a.explicit_anchor.is_none());
        assert_eq!(
            coordinator_a
                .pending
                .as_ref()
                .expect("A pending slot should remain")
                .query_sequence,
            1
        );

        let outcome_a = pending_a.execute(&ready_policy(&[1], 100.0));
        assert!(matches!(
            coordinator_a.complete(pending_a, outcome_a),
            VirtualLayoutCompletion::Committed(_)
        ));
        assert_eq!(
            coordinator_a.accepted().expect("A accepted window").entries[0].key(),
            &VirtualLayoutItemKey::new(1_u32)
        );

        let newer_pending_b = coordinator_b
            .begin_query(parts(0, Rect::from_xy_size(0.0, 0.0, 100.0, 20.0), 1, 8))
            .expect("B should supersede its orphaned pending record");
        assert_eq!(newer_pending_b.query_sequence(), 2);
        let newer_outcome_b = newer_pending_b.execute(&ready_policy(&[3], 100.0));
        assert!(matches!(
            coordinator_b.complete(newer_pending_b, newer_outcome_b),
            VirtualLayoutCompletion::Committed(_)
        ));
        assert_eq!(
            coordinator_b.accepted().expect("B accepted window").entries[0].key(),
            &VirtualLayoutItemKey::new(3_u32)
        );
    }

    #[test]
    fn stale_tokens_and_revision_invalidations_cannot_overwrite() {
        let mut coordinator =
            VirtualLayoutWindowCoordinator::new(41, VirtualLayoutPolicyIdentity::new("policy"), 7);
        ready(&mut coordinator, &[1]);
        let first = coordinator
            .begin_query(parts(0, Rect::from_xy_size(0.0, 0.0, 100.0, 20.0), 1, 8))
            .expect("first query");
        let second = coordinator
            .begin_query(parts(0, Rect::from_xy_size(0.0, 0.0, 100.0, 20.0), 1, 8))
            .expect("second query");
        let stale_outcome = first.execute(&ready_policy(&[2], 100.0));
        assert!(matches!(
            coordinator.complete(first, stale_outcome),
            VirtualLayoutCompletion::Stale(_)
        ));
        let second_outcome = second.execute(&ready_policy(&[2], 100.0));
        assert!(matches!(
            coordinator.complete(second, second_outcome),
            VirtualLayoutCompletion::Committed(_)
        ));

        let third = coordinator
            .begin_query(parts(0, Rect::from_xy_size(0.0, 0.0, 100.0, 20.0), 1, 8))
            .expect("third query");
        coordinator
            .invalidate(VirtualLayoutInvalidation::Measurement)
            .expect("revision should increment");
        let third_outcome = third.execute(&ready_policy(&[3], 100.0));
        assert!(matches!(
            coordinator.complete(third, third_outcome),
            VirtualLayoutCompletion::Stale(_)
        ));
        assert_eq!(
            coordinator.accepted().expect("accepted").entries[0].logical_index(),
            0
        );
        assert!(
            coordinator
                .invalidations()
                .contains(VirtualLayoutInvalidation::Measurement)
        );
    }

    #[test]
    fn deferred_unavailable_and_invalid_completions_preserve_explicit_anchor() {
        let mut coordinator =
            VirtualLayoutWindowCoordinator::new(41, VirtualLayoutPolicyIdentity::new("policy"), 7);
        coordinator
            .set_anchor(VirtualLayoutAnchorRequest {
                key: VirtualLayoutItemKey::new(99_u32),
                axis: VirtualLayoutAnchorAxis::Vertical,
                edge: VirtualLayoutAnchorEdge::Leading,
                local_offset: 0.0,
                screen_offset: 3.0,
            })
            .expect("anchor should be valid");

        let decisions = [
            super::super::VirtualLayoutPolicyDecision::Deferred(
                VirtualLayoutDeferredReason::DataPending,
            ),
            super::super::VirtualLayoutPolicyDecision::Unavailable(
                VirtualLayoutUnavailableReason::DataUnavailable,
            ),
            super::super::VirtualLayoutPolicyDecision::Invalid(
                VirtualLayoutDiagnosticCode::PolicyRejected,
            ),
        ];
        for decision in decisions {
            let pending = coordinator
                .begin_query(parts(0, Rect::from_xy_size(0.0, 0.0, 100.0, 20.0), 1, 8))
                .expect("query should begin");
            let outcome = pending.execute(&DispositionPolicy { decision });
            assert!(matches!(
                coordinator.complete(pending, outcome),
                VirtualLayoutCompletion::Retained { .. }
            ));
            assert_explicit_anchor_key(&coordinator, 99);
        }
    }

    #[test]
    fn stale_completion_preserves_explicit_anchor() {
        let mut coordinator =
            VirtualLayoutWindowCoordinator::new(41, VirtualLayoutPolicyIdentity::new("policy"), 7);
        coordinator
            .set_anchor(VirtualLayoutAnchorRequest {
                key: VirtualLayoutItemKey::new(99_u32),
                axis: VirtualLayoutAnchorAxis::Vertical,
                edge: VirtualLayoutAnchorEdge::Leading,
                local_offset: 0.0,
                screen_offset: 3.0,
            })
            .expect("anchor should be valid");
        let first = coordinator
            .begin_query(parts(0, Rect::from_xy_size(0.0, 0.0, 100.0, 20.0), 1, 8))
            .expect("first query");
        let second = coordinator
            .begin_query(parts(0, Rect::from_xy_size(0.0, 0.0, 100.0, 20.0), 1, 8))
            .expect("second query");
        let stale_outcome = first.execute(&DispositionPolicy {
            decision: super::super::VirtualLayoutPolicyDecision::Deferred(
                VirtualLayoutDeferredReason::DataPending,
            ),
        });
        assert!(matches!(
            coordinator.complete(first, stale_outcome),
            VirtualLayoutCompletion::Stale(_)
        ));
        assert_explicit_anchor_key(&coordinator, 99);

        let second_outcome = second.execute(&DispositionPolicy {
            decision: super::super::VirtualLayoutPolicyDecision::Deferred(
                VirtualLayoutDeferredReason::DataPending,
            ),
        });
        let _ = coordinator.complete(second, second_outcome);
        assert_explicit_anchor_key(&coordinator, 99);
    }

    #[test]
    fn rejected_completion_preserves_explicit_anchor() {
        struct RejectingPolicy {
            coordinator: Rc<RefCell<VirtualLayoutWindowCoordinator>>,
            foreign_pending: RefCell<Option<VirtualLayoutPendingQuery>>,
            rejected: Cell<bool>,
        }

        impl VirtualLayoutPolicy for RejectingPolicy {
            fn query(
                &self,
                _input: &VirtualLayoutQueryInput,
                _sink: &mut super::super::VirtualLayoutQuerySink,
            ) -> super::super::VirtualLayoutPolicyDecision {
                let foreign_pending = self
                    .foreign_pending
                    .borrow_mut()
                    .take()
                    .expect("foreign pending query");
                let completion = self.coordinator.borrow_mut().complete(
                    foreign_pending,
                    super::super::VirtualLayoutQueryOutcome::Deferred(
                        VirtualLayoutDeferredReason::DataPending,
                    ),
                );
                self.rejected
                    .set(matches!(completion, VirtualLayoutCompletion::Rejected(_)));
                super::super::VirtualLayoutPolicyDecision::Deferred(
                    VirtualLayoutDeferredReason::DataPending,
                )
            }
        }

        let coordinator = Rc::new(RefCell::new(VirtualLayoutWindowCoordinator::new(
            41,
            VirtualLayoutPolicyIdentity::new("policy"),
            7,
        )));
        coordinator
            .borrow_mut()
            .set_anchor(VirtualLayoutAnchorRequest {
                key: VirtualLayoutItemKey::new(99_u32),
                axis: VirtualLayoutAnchorAxis::Vertical,
                edge: VirtualLayoutAnchorEdge::Leading,
                local_offset: 0.0,
                screen_offset: 3.0,
            })
            .expect("anchor should be valid");
        let mut foreign_coordinator =
            VirtualLayoutWindowCoordinator::new(41, VirtualLayoutPolicyIdentity::new("policy"), 7);
        let foreign_pending = foreign_coordinator
            .begin_query(parts(0, Rect::from_xy_size(0.0, 0.0, 100.0, 20.0), 1, 8))
            .expect("foreign query should begin");
        let pending = coordinator
            .borrow_mut()
            .begin_query(parts(0, Rect::from_xy_size(0.0, 0.0, 100.0, 20.0), 1, 8))
            .expect("query should begin");
        let policy = RejectingPolicy {
            coordinator: Rc::clone(&coordinator),
            foreign_pending: RefCell::new(Some(foreign_pending)),
            rejected: Cell::new(false),
        };
        let outcome = pending.execute(&policy);
        assert!(policy.rejected.get());
        assert!(matches!(
            coordinator.borrow_mut().complete(pending, outcome),
            VirtualLayoutCompletion::Retained {
                reason: VirtualLayoutRetainReason::Deferred(_),
                ..
            }
        ));
        assert_explicit_anchor_key(&coordinator.borrow(), 99);
    }

    #[test]
    fn revision_regressions_cannot_replace_newer_accepted_state() {
        let mut coordinator =
            VirtualLayoutWindowCoordinator::new(41, VirtualLayoutPolicyIdentity::new("policy"), 7);
        ready(&mut coordinator, &[1]);

        let newer = coordinator
            .begin_query(parts(0, Rect::from_xy_size(0.0, 0.0, 100.0, 20.0), 2, 8))
            .expect("newer revision should begin");
        let newer_outcome = newer.execute(&ready_policy(&[2], 100.0));
        assert!(matches!(
            coordinator.complete(newer, newer_outcome),
            VirtualLayoutCompletion::Committed(_)
        ));

        let lower =
            coordinator.begin_query(parts(0, Rect::from_xy_size(0.0, 0.0, 100.0, 20.0), 1, 8));
        assert!(matches!(
            lower,
            Err(VirtualLayoutCoordinatorError::RevisionRegression)
        ));
        assert_eq!(
            coordinator.accepted().expect("accepted window").entries[0].key(),
            &VirtualLayoutItemKey::new(2_u32)
        );
    }

    #[test]
    fn key_delta_ignores_policy_emission_order_for_same_key_index_mappings() {
        let mut coordinator =
            VirtualLayoutWindowCoordinator::new(41, VirtualLayoutPolicyIdentity::new("policy"), 7);
        let first = [(1, 8, 80.0), (2, 2, 20.0), (3, 11, 110.0)];
        let second = [(3, 11, 110.0), (1, 8, 80.0), (2, 2, 20.0)];
        let first_policy = ready_entries_policy(&first);
        let _ = commit_policy(&mut coordinator, &first_policy);
        let second_policy = ready_entries_policy(&second);
        let commit = commit_policy(&mut coordinator, &second_policy);
        assert_eq!(commit.delta.len(), 0);
    }

    #[test]
    fn key_delta_reports_sparse_logical_indices_in_deterministic_order() {
        let mut coordinator =
            VirtualLayoutWindowCoordinator::new(41, VirtualLayoutPolicyIdentity::new("policy"), 7);
        let first = [(1, 2, 20.0), (4, 10, 100.0)];
        let second = [(7, 50, 500.0), (1, 2, 20.0)];
        let first_policy = ready_entries_policy(&first);
        let _ = commit_policy(&mut coordinator, &first_policy);
        let second_policy = ready_entries_policy(&second);
        let commit = commit_policy(&mut coordinator, &second_policy);
        assert!(commit.delta.get(0).is_some_and(|change| {
            change.key == VirtualLayoutItemKey::new(4_u32)
                && matches!(
                    change.kind,
                    VirtualLayoutKeyChangeKind::Removed { index: 10 }
                )
        }));
        assert!(commit.delta.get(1).is_some_and(|change| {
            change.key == VirtualLayoutItemKey::new(7_u32)
                && matches!(
                    change.kind,
                    VirtualLayoutKeyChangeKind::Inserted { index: 50 }
                )
        }));
    }

    #[test]
    fn key_delta_reports_logical_index_move_for_a_retained_key() {
        let mut coordinator =
            VirtualLayoutWindowCoordinator::new(41, VirtualLayoutPolicyIdentity::new("policy"), 7);
        let first = [(1, 9, 90.0)];
        let second = [(1, 42, 420.0)];
        let first_policy = ready_entries_policy(&first);
        let _ = commit_policy(&mut coordinator, &first_policy);
        let second_policy = ready_entries_policy(&second);
        let commit = commit_policy(&mut coordinator, &second_policy);
        assert!(commit.delta.get(0).is_some_and(|change| {
            change.key == VirtualLayoutItemKey::new(1_u32)
                && matches!(
                    change.kind,
                    VirtualLayoutKeyChangeKind::Moved { from: 9, to: 42 }
                )
        }));
        assert_eq!(commit.delta.len(), 1);
    }

    #[test]
    fn key_delta_replaces_a_key_at_one_logical_index_remove_then_insert() {
        let mut coordinator =
            VirtualLayoutWindowCoordinator::new(41, VirtualLayoutPolicyIdentity::new("policy"), 7);
        let first = [(1, 7, 70.0)];
        let second = [(2, 7, 70.0)];
        let first_policy = ready_entries_policy(&first);
        let _ = commit_policy(&mut coordinator, &first_policy);
        let second_policy = ready_entries_policy(&second);
        let commit = commit_policy(&mut coordinator, &second_policy);
        assert!(commit.delta.get(0).is_some_and(|change| {
            change.key == VirtualLayoutItemKey::new(1_u32)
                && matches!(
                    change.kind,
                    VirtualLayoutKeyChangeKind::Removed { index: 7 }
                )
        }));
        assert!(commit.delta.get(1).is_some_and(|change| {
            change.key == VirtualLayoutItemKey::new(2_u32)
                && matches!(
                    change.kind,
                    VirtualLayoutKeyChangeKind::Inserted { index: 7 }
                )
        }));
        assert_eq!(commit.delta.len(), 2);
    }

    #[test]
    fn fallback_is_clipped_and_disabled_by_content_revision() {
        let mut coordinator =
            VirtualLayoutWindowCoordinator::new(41, VirtualLayoutPolicyIdentity::new("policy"), 7);
        ready(&mut coordinator, &[1, 2, 3]);
        let same_content = VirtualLayoutQueryInput::from_parts(parts(
            0,
            Rect::from_xy_size(0.0, 5.0, 100.0, 10.0),
            1,
            8,
        ))
        .expect("input");
        let fallback = coordinator.fallback_for(&same_content);
        assert!(fallback.fallback);
        assert_eq!(fallback.entries.len(), 2);
        let changed_content = VirtualLayoutQueryInput::from_parts(parts(
            0,
            Rect::from_xy_size(0.0, 5.0, 100.0, 10.0),
            2,
            8,
        ))
        .expect("input");
        assert!(
            coordinator
                .fallback_for(&changed_content)
                .entries
                .is_empty()
        );
    }

    #[test]
    fn fallback_requires_exact_container_policy_mount_and_coordinate_scope() {
        let mut coordinator =
            VirtualLayoutWindowCoordinator::new(41, VirtualLayoutPolicyIdentity::new("policy"), 7);
        ready(&mut coordinator, &[1]);

        let mut mismatched_container = parts(0, Rect::from_xy_size(0.0, 0.0, 100.0, 20.0), 1, 8);
        mismatched_container.container_id = 42;
        assert!(
            coordinator
                .fallback_for(&VirtualLayoutQueryInput::from_parts(mismatched_container).unwrap())
                .entries
                .is_empty()
        );

        let mut mismatched_policy = parts(0, Rect::from_xy_size(0.0, 0.0, 100.0, 20.0), 1, 8);
        mismatched_policy.policy_identity = VirtualLayoutPolicyIdentity::new("other-policy");
        assert!(
            coordinator
                .fallback_for(&VirtualLayoutQueryInput::from_parts(mismatched_policy).unwrap())
                .entries
                .is_empty()
        );

        let mut mismatched_mount = parts(0, Rect::from_xy_size(0.0, 0.0, 100.0, 20.0), 1, 8);
        mismatched_mount.mount_generation = 8;
        assert!(
            coordinator
                .fallback_for(&VirtualLayoutQueryInput::from_parts(mismatched_mount).unwrap())
                .entries
                .is_empty()
        );

        let mut mismatched_coordinate = parts(0, Rect::from_xy_size(0.0, 0.0, 100.0, 20.0), 1, 8);
        mismatched_coordinate.coordinate_space = VirtualLayoutCoordinateSpace::custom(
            VirtualLayoutPolicyIdentity::new("other-coordinate-space"),
        );
        assert!(
            coordinator
                .fallback_for(&VirtualLayoutQueryInput::from_parts(mismatched_coordinate).unwrap())
                .entries
                .is_empty()
        );
    }

    #[test]
    fn required_key_change_invalidates_fallback_and_supersedes_pending_query() {
        let mut coordinator =
            VirtualLayoutWindowCoordinator::new(41, VirtualLayoutPolicyIdentity::new("policy"), 7);
        let first_pending = coordinator
            .begin_query_with_required_key(
                parts(0, Rect::from_xy_size(0.0, 0.0, 100.0, 20.0), 1, 8),
                Some(VirtualLayoutItemKey::new(1_u32)),
            )
            .expect("first required-key query should begin");
        let first_outcome = first_pending.execute(&ready_policy(&[1], 100.0));
        assert!(matches!(
            coordinator.complete(first_pending, first_outcome),
            VirtualLayoutCompletion::Committed(_)
        ));

        let changed_parts = parts(0, Rect::from_xy_size(0.0, 0.0, 100.0, 20.0), 1, 8);
        let changed_input = VirtualLayoutQueryInput::from_parts_with_required_key(
            changed_parts.clone(),
            Some(VirtualLayoutItemKey::new(2_u32)),
        )
        .expect("changed required-key input should be valid");
        assert!(coordinator.fallback_for(&changed_input).entries.is_empty());

        let stale_pending = coordinator
            .begin_query_with_required_key(
                changed_parts.clone(),
                Some(VirtualLayoutItemKey::new(1_u32)),
            )
            .expect("stale query should begin");
        let current_pending = coordinator
            .begin_query_with_required_key(changed_parts, Some(VirtualLayoutItemKey::new(2_u32)))
            .expect("current query should begin");
        assert!(
            coordinator
                .invalidations()
                .contains(VirtualLayoutInvalidation::RequiredKey)
        );
        let current_outcome = current_pending.execute(&ready_policy(&[2], 100.0));
        assert!(matches!(
            coordinator.complete(current_pending, current_outcome),
            VirtualLayoutCompletion::Committed(_)
        ));
        let stale_outcome = stale_pending.execute(&ready_policy(&[1], 100.0));
        assert!(matches!(
            coordinator.complete(stale_pending, stale_outcome),
            VirtualLayoutCompletion::Stale(_)
        ));
    }

    #[test]
    fn off_window_explicit_anchor_stays_unresolved_until_the_key_reappears() {
        let mut coordinator =
            VirtualLayoutWindowCoordinator::new(41, VirtualLayoutPolicyIdentity::new("policy"), 7);
        ready(&mut coordinator, &[1]);
        coordinator
            .set_anchor(VirtualLayoutAnchorRequest {
                key: VirtualLayoutItemKey::new(99_u32),
                axis: VirtualLayoutAnchorAxis::Vertical,
                edge: VirtualLayoutAnchorEdge::Leading,
                local_offset: 0.0,
                screen_offset: 0.0,
            })
            .expect("anchor should be valid");

        let pending = coordinator
            .begin_query(parts(0, Rect::from_xy_size(0.0, 0.0, 100.0, 20.0), 1, 8))
            .expect("query should begin");
        let outcome = pending.execute(&ready_policy(&[2], 100.0));
        let VirtualLayoutCompletion::Committed(commit) = coordinator.complete(pending, outcome)
        else {
            panic!("query should commit")
        };
        assert!(commit.anchor.is_none());
        assert!(commit.correction.is_none());
        assert_eq!(
            coordinator
                .explicit_anchor
                .as_ref()
                .expect("explicit anchor should remain authoritative")
                .key,
            VirtualLayoutItemKey::new(99_u32)
        );

        let pending = coordinator
            .begin_query(parts(0, Rect::from_xy_size(0.0, 0.0, 100.0, 20.0), 1, 8))
            .expect("query should begin");
        let outcome = pending.execute(&ready_entries_policy(&[(99, 20, 200.0)]));
        let VirtualLayoutCompletion::Committed(commit) = coordinator.complete(pending, outcome)
        else {
            panic!("query should commit")
        };
        assert_eq!(
            commit
                .anchor
                .as_ref()
                .expect("reappearing anchor should resolve")
                .key,
            VirtualLayoutItemKey::new(99_u32)
        );
        assert!(commit.correction.is_none());
    }

    #[test]
    fn estimated_anchor_correction_is_provisional_and_unavailable_is_none() {
        let mut coordinator =
            VirtualLayoutWindowCoordinator::new(41, VirtualLayoutPolicyIdentity::new("policy"), 7);
        ready(&mut coordinator, &[1]);
        coordinator
            .set_anchor(VirtualLayoutAnchorRequest {
                key: VirtualLayoutItemKey::new(1_u32),
                axis: VirtualLayoutAnchorAxis::Vertical,
                edge: VirtualLayoutAnchorEdge::Leading,
                local_offset: 0.0,
                screen_offset: 0.0,
            })
            .expect("anchor");
        let pending = coordinator
            .begin_query(parts(0, Rect::from_xy_size(0.0, 0.0, 100.0, 20.0), 1, 8))
            .expect("query");
        let policy = |extent: super::super::VirtualLayoutExtentCandidate| {
            struct P {
                extent: super::super::VirtualLayoutExtentCandidate,
            }
            impl VirtualLayoutPolicy for P {
                fn query(
                    &self,
                    _input: &VirtualLayoutQueryInput,
                    sink: &mut super::super::VirtualLayoutQuerySink,
                ) -> super::super::VirtualLayoutPolicyDecision {
                    let _ = sink.visit(candidate(1, 0, 5.0));
                    let _ = sink.set_extent(self.extent);
                    super::super::VirtualLayoutPolicyDecision::Ready
                }
            }
            P { extent }
        };
        let outcome = pending.execute(&policy(
            super::super::VirtualLayoutExtentCandidate::estimated(Vector2::new(100.0, 100.0)),
        ));
        let VirtualLayoutCompletion::Committed(commit) = coordinator.complete(pending, outcome)
        else {
            panic!("query should commit")
        };
        assert_eq!(
            commit.correction.expect("correction").confidence,
            VirtualLayoutCorrectionConfidence::Estimated
        );
    }
}
