//! Private accepted-window materialization and recycling correctness kernel.
//!
//! This module deliberately stops at a host projector and a lifecycle adapter.
//! It does not register a runtime surface, construct widgets, schedule work, or
//! take ownership of focus, capture, IME, accessibility, culling, or paint.

#![expect(
    dead_code,
    reason = "The private materialization kernel is shipped before runtime registration"
)]

use super::coordinator::{
    CoordinatorIdentity, VirtualLayoutCommit, VirtualLayoutWindowCoordinator,
};
use super::{
    NodeId, OpaqueExactValue, VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES, VirtualLayoutBoundsConfidence,
    VirtualLayoutItem, VirtualLayoutItemKey, VirtualLayoutPolicyIdentity, VirtualLayoutQueryFence,
    VirtualLayoutVisibility,
};
use crate::gui::types::Rect;
use std::{cell::Cell, fmt, rc::Rc};

const MAX_MATERIALIZATION_DIAGNOSTICS: usize = 8;
const INITIAL_SLOT_GENERATION: u64 = 1;

/// Bounded diagnostic codes owned by one private materialization store.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VirtualLayoutMaterializationDiagnosticCode {
    ForeignContainer,
    ForeignPolicy,
    ForeignMount,
    ForeignOwner,
    UnstablePolicyIdentity,
    Unmounted,
    InvalidCommit,
    CapacityViolation,
    DuplicateKey,
    UnstableKey,
    DuplicateLogicalIndex,
    OlderRevision,
    DuplicateRevision,
    OlderFence,
    SlotArithmeticOverflow,
    GenerationOverflow,
    UnstableCompatibility,
    ProjectionFailed,
    ProjectionKindChanged,
    LifecycleFailed,
    ReentrantReconciliation,
    LifecycleIndeterminate,
    LifecyclePanicked,
}

/// Fixed-sample, saturating diagnostics with no retained event history.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct VirtualLayoutMaterializationDiagnostics {
    samples: [Option<VirtualLayoutMaterializationDiagnosticCode>; MAX_MATERIALIZATION_DIAGNOSTICS],
    sample_len: usize,
    total_count: u32,
}

impl VirtualLayoutMaterializationDiagnostics {
    /// Return the number of retained sample codes.
    #[must_use]
    pub(crate) const fn sample_len(&self) -> usize {
        self.sample_len
    }

    /// Return the saturating number of recorded failures.
    #[must_use]
    pub(crate) const fn total_count(&self) -> u32 {
        self.total_count
    }

    /// Return one retained sample code.
    #[must_use]
    pub(crate) fn sample(
        &self,
        index: usize,
    ) -> Option<VirtualLayoutMaterializationDiagnosticCode> {
        self.samples.get(index).and_then(|sample| *sample)
    }

    fn record(&mut self, code: VirtualLayoutMaterializationDiagnosticCode) {
        self.total_count = self.total_count.saturating_add(1);
        if self.sample_len < MAX_MATERIALIZATION_DIAGNOSTICS {
            self.samples[self.sample_len] = Some(code);
            self.sample_len += 1;
        }
    }
}

/// Error returned when an accepted commit cannot be materialized safely.
#[derive(Debug)]
pub(crate) enum VirtualLayoutMaterializationError<ProjectionError, LifecycleError> {
    ForeignContainer,
    ForeignPolicy,
    ForeignMount,
    ForeignOwner,
    UnstablePolicyIdentity,
    Unmounted,
    InvalidCommit,
    CapacityViolation,
    DuplicateKey,
    UnstableKey,
    DuplicateLogicalIndex,
    OlderRevision,
    DuplicateRevision,
    OlderFence,
    SlotArithmeticOverflow,
    GenerationOverflow,
    UnstableCompatibility,
    Projection(ProjectionError),
    ProjectionKindChanged,
    Lifecycle(LifecycleError),
    Reentrant,
    LifecycleIndeterminate,
}

/// Stable private identity for a concrete projected item kind.
///
/// This is intentionally distinct from [`VirtualLayoutPolicyIdentity`].  It
/// uses exact bidirectional equality and never hashes, formats, or compares
/// allocation addresses.
#[derive(Clone)]
pub(crate) struct VirtualLayoutProjectionKind {
    value: OpaqueExactValue,
}

impl VirtualLayoutProjectionKind {
    /// Construct a private projection-kind identity from an exact value.
    pub(crate) fn new<T>(value: T) -> Self
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

impl fmt::Debug for VirtualLayoutProjectionKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VirtualLayoutProjectionKind(..)")
    }
}

/// Identity of one materialized slot within one mounted container generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct VirtualLayoutSlotIdentity {
    container_id: NodeId,
    mount_generation: u64,
    slot_index: usize,
    checked_generation: u64,
}

impl VirtualLayoutSlotIdentity {
    /// Construct one exact slot identity supplied by a private adapter plan.
    pub(super) const fn from_parts(
        container_id: NodeId,
        mount_generation: u64,
        slot_index: usize,
        checked_generation: u64,
    ) -> Self {
        Self {
            container_id,
            mount_generation,
            slot_index,
            checked_generation,
        }
    }

    /// Construct one exact slot identity for cross-module unit-test fixtures.
    #[cfg(test)]
    pub(crate) const fn from_test_parts(
        container_id: NodeId,
        mount_generation: u64,
        slot_index: usize,
        checked_generation: u64,
    ) -> Self {
        Self::from_parts(
            container_id,
            mount_generation,
            slot_index,
            checked_generation,
        )
    }

    fn new(container_id: NodeId, mount_generation: u64, slot_index: usize) -> Self {
        Self {
            container_id,
            mount_generation,
            slot_index,
            checked_generation: INITIAL_SLOT_GENERATION,
        }
    }

    fn reset(self) -> Result<Self, VirtualLayoutMaterializationDiagnosticCode> {
        let checked_generation = self
            .checked_generation
            .checked_add(1)
            .ok_or(VirtualLayoutMaterializationDiagnosticCode::GenerationOverflow)?;
        Ok(Self {
            checked_generation,
            ..self
        })
    }

    /// Return the mounted container identity.
    #[must_use]
    pub(crate) const fn container_id(self) -> NodeId {
        self.container_id
    }

    /// Return the mounted container generation.
    #[must_use]
    pub(crate) const fn mount_generation(self) -> u64 {
        self.mount_generation
    }

    /// Return the stable slot index.
    #[must_use]
    pub(crate) const fn slot_index(self) -> usize {
        self.slot_index
    }

    /// Return the checked lifecycle generation.
    #[must_use]
    pub(crate) const fn checked_generation(self) -> u64 {
        self.checked_generation
    }
}

/// Pure evidence supplied to a host projector.
pub(crate) struct VirtualLayoutProjectionEvidence<'a> {
    fence: &'a VirtualLayoutQueryFence,
    item: &'a VirtualLayoutItem,
    key: &'a VirtualLayoutItemKey,
    logical_index: usize,
    bounds: Rect,
    visibility: VirtualLayoutVisibility,
    confidence: VirtualLayoutBoundsConfidence,
    proposed_slot: VirtualLayoutSlotIdentity,
}

impl<'a> VirtualLayoutProjectionEvidence<'a> {
    fn from_item(
        fence: &'a VirtualLayoutQueryFence,
        item: &'a VirtualLayoutItem,
        proposed_slot: VirtualLayoutSlotIdentity,
    ) -> Self {
        Self {
            fence,
            item,
            key: item.key(),
            logical_index: item.logical_index(),
            bounds: item.bounds(),
            visibility: item.visibility(),
            confidence: item.confidence(),
            proposed_slot,
        }
    }

    /// Return the exact accepted fence borrowed from the commit.
    #[must_use]
    pub(crate) const fn fence(&self) -> &VirtualLayoutQueryFence {
        self.fence
    }

    /// Return the exact accepted key.
    #[must_use]
    pub(crate) fn key(&self) -> &VirtualLayoutItemKey {
        self.key
    }

    /// Return the complete accepted item evidence backing this projection.
    #[must_use]
    pub(crate) fn item(&self) -> &VirtualLayoutItem {
        self.item
    }

    /// Return the accepted logical index.
    #[must_use]
    pub(crate) const fn logical_index(&self) -> usize {
        self.logical_index
    }

    /// Return the finite accepted bounds.
    #[must_use]
    pub(crate) const fn bounds(&self) -> Rect {
        self.bounds
    }

    /// Return the accepted visible/overscan classification.
    #[must_use]
    pub(crate) const fn visibility(&self) -> VirtualLayoutVisibility {
        self.visibility
    }

    /// Return the accepted bounds confidence.
    #[must_use]
    pub(crate) const fn confidence(&self) -> VirtualLayoutBoundsConfidence {
        self.confidence
    }

    /// Return the proposed stable slot identity.
    #[must_use]
    pub(crate) const fn proposed_slot(&self) -> VirtualLayoutSlotIdentity {
        self.proposed_slot
    }
}

/// Declarative projected payload paired with its private compatibility kind.
pub(crate) struct VirtualLayoutProjection<P> {
    kind: VirtualLayoutProjectionKind,
    payload: P,
}

impl<P> VirtualLayoutProjection<P> {
    /// Construct one declarative projected payload.
    pub(crate) fn new(kind: VirtualLayoutProjectionKind, payload: P) -> Self {
        Self { kind, payload }
    }
}

type VirtualLayoutBatchProjection<P, E> = Result<Option<Vec<VirtualLayoutProjection<P>>>, E>;

/// Explicit pure host projection boundary.
pub(crate) trait VirtualLayoutHostProjector {
    type Payload;
    type Error;

    /// Return the stable private kind used during compatibility preflight.
    fn projection_kind(
        &self,
        item: &VirtualLayoutItem,
    ) -> Result<VirtualLayoutProjectionKind, Self::Error>;

    /// Build a declarative payload from only accepted item evidence.
    fn project<'a>(
        &self,
        evidence: VirtualLayoutProjectionEvidence<'a>,
    ) -> Result<VirtualLayoutProjection<Self::Payload>, Self::Error>;

    /// Optionally lower one complete planned batch before lifecycle callbacks.
    ///
    /// The default keeps the original per-item projector contract. A runtime
    /// consumer that needs whole-shell identity admission may return all
    /// projections here after receiving the exact slot identities selected by
    /// the store. Returning `None` falls back to [`Self::project`].
    fn project_batch<'a>(
        &self,
        _commit: &VirtualLayoutCommit,
        _evidence: &[VirtualLayoutProjectionEvidence<'a>],
    ) -> VirtualLayoutBatchProjection<Self::Payload, Self::Error> {
        Ok(None)
    }
}

/// Marker returned by a lifecycle callback that attempts synchronous reentry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct VirtualLayoutReentryError;

/// Guard passed to lifecycle callbacks so reentrant reconciliation fails closed.
pub(crate) struct VirtualLayoutMaterializationReentry<'a> {
    committing: &'a Cell<bool>,
    attempted: &'a Cell<bool>,
}

impl VirtualLayoutMaterializationReentry<'_> {
    /// Reject a synchronous reconciliation attempt while publication is active.
    pub(crate) fn try_reconcile(&self) -> Result<(), VirtualLayoutReentryError> {
        if self.committing.get() {
            self.attempted.set(true);
            Err(VirtualLayoutReentryError)
        } else {
            Ok(())
        }
    }

    fn was_attempted(&self) -> bool {
        self.attempted.get()
    }
}

/// Minimal lifecycle owner for one projected-payload type.
pub(crate) trait VirtualLayoutLifecycleAdapter<P> {
    type Error;

    /// Compare only stable private projection kinds; `None` is unstable.
    fn compatible(
        &self,
        previous: &VirtualLayoutProjectionKind,
        next: &VirtualLayoutProjectionKind,
    ) -> Option<bool>;

    /// Release old item lifecycle state.
    fn unmount(
        &mut self,
        payload: &P,
        evidence: VirtualLayoutProjectionEvidence<'_>,
        reentry: &VirtualLayoutMaterializationReentry<'_>,
    ) -> Result<(), Self::Error>;

    /// Reset an old item before its slot becomes recyclable.
    fn reset(
        &mut self,
        payload: &P,
        evidence: VirtualLayoutProjectionEvidence<'_>,
        reentry: &VirtualLayoutMaterializationReentry<'_>,
    ) -> Result<(), Self::Error>;

    /// Synchronize a compatible same-key payload while retaining its slot.
    fn reconcile(
        &mut self,
        previous: &P,
        next: &P,
        evidence: VirtualLayoutProjectionEvidence<'_>,
        reentry: &VirtualLayoutMaterializationReentry<'_>,
    ) -> Result<(), Self::Error>;

    /// Mount a fresh or reset-shell-backed payload.
    fn mount(
        &mut self,
        recycled_shell: Option<&P>,
        next: &P,
        evidence: VirtualLayoutProjectionEvidence<'_>,
        reentry: &VirtualLayoutMaterializationReentry<'_>,
    ) -> Result<(), Self::Error>;
}

#[derive(Clone)]
struct MaterializationScope {
    container_id: NodeId,
    policy_identity: VirtualLayoutPolicyIdentity,
    mount_generation: u64,
}

impl MaterializationScope {
    fn from_coordinator(coordinator: &VirtualLayoutWindowCoordinator) -> Self {
        let (container_id, policy_identity, mount_generation) = coordinator.scope();
        Self {
            container_id,
            policy_identity: policy_identity.clone(),
            mount_generation,
        }
    }

    fn check_fence(
        &self,
        fence: &VirtualLayoutQueryFence,
    ) -> Result<(), VirtualLayoutMaterializationDiagnosticCode> {
        if self.container_id != fence.container_id() {
            return Err(VirtualLayoutMaterializationDiagnosticCode::ForeignContainer);
        }
        match self.policy_identity.stable_equals(fence.policy_identity()) {
            Some(true) => {}
            Some(false) => return Err(VirtualLayoutMaterializationDiagnosticCode::ForeignPolicy),
            None => return Err(VirtualLayoutMaterializationDiagnosticCode::UnstablePolicyIdentity),
        }
        if self.mount_generation != fence.mount_generation() {
            return Err(VirtualLayoutMaterializationDiagnosticCode::ForeignMount);
        }
        Ok(())
    }
}

struct ActiveSlot<P> {
    item: VirtualLayoutItem,
    kind: VirtualLayoutProjectionKind,
    identity: VirtualLayoutSlotIdentity,
    payload: P,
}

/// Read-only crate-private view of an active materialized slot.
pub(crate) struct VirtualLayoutMaterializedSlot<'a, P> {
    item: &'a VirtualLayoutItem,
    kind: &'a VirtualLayoutProjectionKind,
    identity: VirtualLayoutSlotIdentity,
    payload: &'a P,
}

impl<'a, P> VirtualLayoutMaterializedSlot<'a, P> {
    /// Return the accepted item evidence retained by this slot.
    #[must_use]
    pub(crate) fn item(&self) -> &VirtualLayoutItem {
        self.item
    }

    /// Return the private projection kind.
    #[must_use]
    pub(crate) fn kind(&self) -> &VirtualLayoutProjectionKind {
        self.kind
    }

    /// Return the stable slot identity.
    #[must_use]
    pub(crate) const fn identity(&self) -> VirtualLayoutSlotIdentity {
        self.identity
    }

    /// Return the declarative payload.
    #[must_use]
    pub(crate) fn payload(&self) -> &P {
        self.payload
    }
}

struct RecyclableSlot<P> {
    identity: VirtualLayoutSlotIdentity,
    last_logical_index: usize,
    payload: P,
}

#[derive(Clone, Copy)]
enum RecycleSource {
    Existing(usize),
    Removed(usize),
}

#[derive(Clone, Copy)]
enum PlannedAction {
    Compatible { old_index: usize },
    Replacement { old_index: usize },
    Inserted { recycled: Option<RecycleSource> },
}

struct PlannedItem {
    item: VirtualLayoutItem,
    kind: VirtualLayoutProjectionKind,
    identity: VirtualLayoutSlotIdentity,
    action: PlannedAction,
}

struct StagedItem<P> {
    plan: PlannedItem,
    projection: VirtualLayoutProjection<P>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LifecycleState {
    Mounted,
    CleanlyRetired,
    LifecycleIndeterminate,
}

/// Private bounded materialization/recycling owner for one immutable scope.
pub(crate) struct VirtualLayoutMaterializationStore<P, A> {
    scope: MaterializationScope,
    owner: Rc<CoordinatorIdentity>,
    lifecycle_state: LifecycleState,
    active: Vec<ActiveSlot<P>>,
    recyclable: Vec<RecyclableSlot<P>>,
    next_slot_index: usize,
    authoritative_fence: Option<VirtualLayoutQueryFence>,
    authoritative_revision: Option<u64>,
    diagnostics: VirtualLayoutMaterializationDiagnostics,
    commit_guard: Rc<Cell<bool>>,
    reentry_attempted: Cell<bool>,
    lifecycle: A,
}

impl<P, A> VirtualLayoutMaterializationStore<P, A> {
    /// Create a store bound immutably to one coordinator scope and owner.
    pub(crate) fn new(coordinator: &VirtualLayoutWindowCoordinator, lifecycle: A) -> Self {
        Self {
            scope: MaterializationScope::from_coordinator(coordinator),
            owner: coordinator.owner_evidence(),
            lifecycle_state: LifecycleState::Mounted,
            active: Vec::new(),
            recyclable: Vec::new(),
            next_slot_index: 0,
            authoritative_fence: None,
            authoritative_revision: None,
            diagnostics: VirtualLayoutMaterializationDiagnostics::default(),
            commit_guard: Rc::new(Cell::new(false)),
            reentry_attempted: Cell::new(false),
            lifecycle,
        }
    }

    /// Return the complete active slot set in new logical-index order.
    #[must_use]
    pub(crate) fn active_slots(&self) -> Vec<VirtualLayoutMaterializedSlot<'_, P>> {
        if self.lifecycle_state != LifecycleState::Mounted {
            return Vec::new();
        }
        self.active
            .iter()
            .map(|slot| VirtualLayoutMaterializedSlot {
                item: &slot.item,
                kind: &slot.kind,
                identity: slot.identity,
                payload: &slot.payload,
            })
            .collect()
    }

    /// Return the active slot count.
    #[must_use]
    pub(crate) const fn active_len(&self) -> usize {
        match self.lifecycle_state {
            LifecycleState::Mounted => self.active.len(),
            LifecycleState::CleanlyRetired | LifecycleState::LifecycleIndeterminate => 0,
        }
    }

    /// Return the recyclable shell count.
    #[must_use]
    pub(crate) const fn recyclable_len(&self) -> usize {
        match self.lifecycle_state {
            LifecycleState::Mounted => self.recyclable.len(),
            LifecycleState::CleanlyRetired | LifecycleState::LifecycleIndeterminate => 0,
        }
    }

    /// Return the current authoritative accepted revision.
    #[must_use]
    pub(crate) const fn authoritative_revision(&self) -> Option<u64> {
        match self.lifecycle_state {
            LifecycleState::Mounted => self.authoritative_revision,
            LifecycleState::CleanlyRetired | LifecycleState::LifecycleIndeterminate => None,
        }
    }

    /// Return the current authoritative accepted fence.
    #[must_use]
    pub(crate) fn authoritative_fence(&self) -> Option<&VirtualLayoutQueryFence> {
        if self.lifecycle_state == LifecycleState::Mounted {
            self.authoritative_fence.as_ref()
        } else {
            None
        }
    }

    /// Return bounded store diagnostics.
    #[must_use]
    pub(crate) const fn diagnostics(&self) -> &VirtualLayoutMaterializationDiagnostics {
        &self.diagnostics
    }

    /// Return whether the fixed scope is still mounted.
    #[must_use]
    pub(crate) const fn is_mounted(&self) -> bool {
        matches!(self.lifecycle_state, LifecycleState::Mounted)
    }

    /// Publish one complete committed accepted window.
    pub(crate) fn publish<J>(
        &mut self,
        commit: &VirtualLayoutCommit,
        projector: &J,
    ) -> Result<(), VirtualLayoutMaterializationError<J::Error, A::Error>>
    where
        J: VirtualLayoutHostProjector<Payload = P>,
        A: VirtualLayoutLifecycleAdapter<P>,
    {
        if self.commit_guard.replace(true) {
            self.reentry_attempted.set(true);
            self.poison_lifecycle(
                VirtualLayoutMaterializationDiagnosticCode::ReentrantReconciliation,
            );
            return Err(VirtualLayoutMaterializationError::Reentrant);
        }
        let guard = CommitGuard(Rc::clone(&self.commit_guard));
        self.reentry_attempted.set(false);

        match self.lifecycle_state {
            LifecycleState::Mounted => {}
            LifecycleState::CleanlyRetired => {
                return self.reject(
                    VirtualLayoutMaterializationDiagnosticCode::Unmounted,
                    VirtualLayoutMaterializationError::Unmounted,
                );
            }
            LifecycleState::LifecycleIndeterminate => {
                return self.reject(
                    VirtualLayoutMaterializationDiagnosticCode::LifecycleIndeterminate,
                    VirtualLayoutMaterializationError::LifecycleIndeterminate,
                );
            }
        }
        if !Rc::ptr_eq(&self.owner, commit.owner()) {
            return self.reject(
                VirtualLayoutMaterializationDiagnosticCode::ForeignOwner,
                VirtualLayoutMaterializationError::ForeignOwner,
            );
        }
        if let Err(code) = self.scope.check_fence(commit.fence()) {
            return self.reject(code, error_for_scope_code(code));
        }
        if commit.accepted_revision() == 0
            || commit.view().fallback
            || commit.view().clip.is_some()
            || commit.view().extent.is_none()
            || commit.view().accepted_revision != Some(commit.accepted_revision())
        {
            return self.reject(
                VirtualLayoutMaterializationDiagnosticCode::InvalidCommit,
                VirtualLayoutMaterializationError::InvalidCommit,
            );
        }

        if let Some(previous_revision) = self.authoritative_revision {
            if commit.accepted_revision() < previous_revision {
                return self.reject(
                    VirtualLayoutMaterializationDiagnosticCode::OlderRevision,
                    VirtualLayoutMaterializationError::OlderRevision,
                );
            }
            if commit.accepted_revision() == previous_revision {
                return self.reject(
                    VirtualLayoutMaterializationDiagnosticCode::DuplicateRevision,
                    VirtualLayoutMaterializationError::DuplicateRevision,
                );
            }
            if self
                .authoritative_fence
                .as_ref()
                .is_some_and(|fence| commit.fence().query_sequence() <= fence.query_sequence())
            {
                return self.reject(
                    VirtualLayoutMaterializationDiagnosticCode::OlderFence,
                    VirtualLayoutMaterializationError::OlderFence,
                );
            }
        }

        let bound = commit
            .fence()
            .budget()
            .max_entries()
            .min(VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES);
        if commit.view().entries.len() > bound
            || commit.view().entries.len() > VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES
            || self.active.len().saturating_add(self.recyclable.len())
                > VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES
        {
            return self.reject(
                VirtualLayoutMaterializationDiagnosticCode::CapacityViolation,
                VirtualLayoutMaterializationError::CapacityViolation,
            );
        }

        if let Err(code) = validate_entries(&commit.view().entries) {
            return self.reject(code, error_for_entry_code(code));
        }
        if let Err(code) = validate_active(&self.active) {
            return self.reject(code, error_for_entry_code(code));
        }

        let mut ordered_entries: Vec<&VirtualLayoutItem> = commit.view().entries.iter().collect();
        ordered_entries.sort_by_key(|item| item.logical_index());

        let mut removed_reset_identities: Vec<Option<VirtualLayoutSlotIdentity>> =
            vec![None; self.active.len()];
        let mut removed_indices = Vec::new();
        for (old_index, old_slot) in self.active.iter().enumerate() {
            let retained = match contains_key(&ordered_entries, old_slot.item.key()) {
                Ok(retained) => retained,
                Err(code) => return self.reject(code, error_for_entry_code(code)),
            };
            if !retained {
                let reset_identity = match old_slot.identity.reset() {
                    Ok(identity) => identity,
                    Err(code) => return self.reject(code, error_for_entry_code(code)),
                };
                removed_reset_identities[old_index] = Some(reset_identity);
                removed_indices.push(old_index);
            }
        }
        removed_indices.sort_by_key(|old_index| {
            (
                self.active[*old_index].item.logical_index(),
                self.active[*old_index].identity.slot_index(),
            )
        });

        let mut recycle_plans = Vec::new();
        for (index, shell) in self.recyclable.iter().enumerate() {
            recycle_plans.push(RecyclePlan {
                source: RecycleSource::Existing(index),
                identity: shell.identity,
                last_logical_index: shell.last_logical_index,
            });
        }
        for &old_index in &removed_indices {
            let Some(identity) = removed_reset_identities[old_index] else {
                return self.reject(
                    VirtualLayoutMaterializationDiagnosticCode::SlotArithmeticOverflow,
                    VirtualLayoutMaterializationError::SlotArithmeticOverflow,
                );
            };
            recycle_plans.push(RecyclePlan {
                source: RecycleSource::Removed(old_index),
                identity,
                last_logical_index: self.active[old_index].item.logical_index(),
            });
        }
        recycle_plans.sort_by_key(|plan| (plan.last_logical_index, plan.identity.slot_index()));
        let mut used_recycle = vec![false; recycle_plans.len()];
        let mut next_slot_index = self.next_slot_index;

        let mut kinded = Vec::with_capacity(ordered_entries.len());
        for item in ordered_entries {
            let kind = match projector.projection_kind(item) {
                Ok(kind) => kind,
                Err(error) => {
                    self.diagnostics
                        .record(VirtualLayoutMaterializationDiagnosticCode::ProjectionFailed);
                    return Err(VirtualLayoutMaterializationError::Projection(error));
                }
            };
            kinded.push((item, kind));
        }

        let mut plans = Vec::with_capacity(kinded.len());
        for (item, kind) in kinded {
            let old_index = match find_active_key(&self.active, item.key()) {
                Ok(old_index) => old_index,
                Err(code) => return self.reject(code, error_for_entry_code(code)),
            };
            if let Some(old_index) = old_index {
                let previous_kind = &self.active[old_index].kind;
                if previous_kind.stable_equals(previous_kind) != Some(true)
                    || kind.stable_equals(&kind) != Some(true)
                {
                    return self.reject(
                        VirtualLayoutMaterializationDiagnosticCode::UnstableCompatibility,
                        VirtualLayoutMaterializationError::UnstableCompatibility,
                    );
                }
                let same_kind = match previous_kind.stable_equals(&kind) {
                    Some(same_kind) => same_kind,
                    None => {
                        return self.reject(
                            VirtualLayoutMaterializationDiagnosticCode::UnstableCompatibility,
                            VirtualLayoutMaterializationError::UnstableCompatibility,
                        );
                    }
                };
                let compatible = if same_kind {
                    match self.lifecycle.compatible(previous_kind, &kind) {
                        Some(compatible) => compatible,
                        None => {
                            return self.reject(
                                VirtualLayoutMaterializationDiagnosticCode::UnstableCompatibility,
                                VirtualLayoutMaterializationError::UnstableCompatibility,
                            );
                        }
                    }
                } else {
                    false
                };
                if compatible {
                    plans.push(PlannedItem {
                        item: item.clone(),
                        kind,
                        identity: self.active[old_index].identity,
                        action: PlannedAction::Compatible { old_index },
                    });
                } else {
                    let identity = match self.active[old_index].identity.reset() {
                        Ok(identity) => identity,
                        Err(code) => return self.reject(code, error_for_entry_code(code)),
                    };
                    plans.push(PlannedItem {
                        item: item.clone(),
                        kind,
                        identity,
                        action: PlannedAction::Replacement { old_index },
                    });
                }
                continue;
            }

            let recycled = recycle_plans
                .iter()
                .enumerate()
                .find(|(index, _)| !used_recycle[*index]);
            let (identity, recycled_source) = if let Some((index, plan)) = recycled {
                used_recycle[index] = true;
                (plan.identity, Some(plan.source))
            } else {
                let slot_index = next_slot_index;
                next_slot_index = match next_slot_index.checked_add(1) {
                    Some(next) => next,
                    None => {
                        return self.reject(
                            VirtualLayoutMaterializationDiagnosticCode::SlotArithmeticOverflow,
                            VirtualLayoutMaterializationError::SlotArithmeticOverflow,
                        );
                    }
                };
                (
                    VirtualLayoutSlotIdentity::new(
                        self.scope.container_id,
                        self.scope.mount_generation,
                        slot_index,
                    ),
                    None,
                )
            };
            plans.push(PlannedItem {
                item: item.clone(),
                kind,
                identity,
                action: PlannedAction::Inserted {
                    recycled: recycled_source,
                },
            });
        }

        let batch_projections = {
            let batch_evidence = plans
                .iter()
                .map(|plan| {
                    VirtualLayoutProjectionEvidence::from_item(
                        commit.fence(),
                        &plan.item,
                        plan.identity,
                    )
                })
                .collect::<Vec<_>>();
            match projector.project_batch(commit, &batch_evidence) {
                Ok(Some(projections)) if projections.len() == plans.len() => Some(projections),
                Ok(Some(_)) => {
                    self.diagnostics
                        .record(VirtualLayoutMaterializationDiagnosticCode::ProjectionKindChanged);
                    return Err(VirtualLayoutMaterializationError::ProjectionKindChanged);
                }
                Ok(None) => None,
                Err(error) => {
                    self.diagnostics
                        .record(VirtualLayoutMaterializationDiagnosticCode::ProjectionFailed);
                    return Err(VirtualLayoutMaterializationError::Projection(error));
                }
            }
        };

        let mut batch_projections = batch_projections.map(Vec::into_iter);
        let mut staged = Vec::with_capacity(plans.len());
        for plan in plans {
            let projection = match batch_projections.as_mut() {
                Some(projections) => {
                    let Some(projection) = projections.next() else {
                        self.diagnostics.record(
                            VirtualLayoutMaterializationDiagnosticCode::ProjectionKindChanged,
                        );
                        return Err(VirtualLayoutMaterializationError::ProjectionKindChanged);
                    };
                    projection
                }
                None => match projector.project(VirtualLayoutProjectionEvidence::from_item(
                    commit.fence(),
                    &plan.item,
                    plan.identity,
                )) {
                    Ok(projection) => projection,
                    Err(error) => {
                        self.diagnostics
                            .record(VirtualLayoutMaterializationDiagnosticCode::ProjectionFailed);
                        return Err(VirtualLayoutMaterializationError::Projection(error));
                    }
                },
            };
            if projection.kind.stable_equals(&projection.kind) != Some(true) {
                return self.reject(
                    VirtualLayoutMaterializationDiagnosticCode::UnstableCompatibility,
                    VirtualLayoutMaterializationError::UnstableCompatibility,
                );
            }
            match projection.kind.stable_equals(&plan.kind) {
                Some(true) => {}
                Some(false) => {
                    return self.reject(
                        VirtualLayoutMaterializationDiagnosticCode::ProjectionKindChanged,
                        VirtualLayoutMaterializationError::ProjectionKindChanged,
                    );
                }
                None => {
                    return self.reject(
                        VirtualLayoutMaterializationDiagnosticCode::UnstableCompatibility,
                        VirtualLayoutMaterializationError::UnstableCompatibility,
                    );
                }
            }
            staged.push(StagedItem { plan, projection });
        }

        let mut reset_identities = removed_reset_identities.clone();
        for staged_item in &staged {
            if let PlannedAction::Replacement { old_index } = staged_item.plan.action {
                reset_identities[old_index] = Some(staged_item.plan.identity);
            }
        }
        let mut retired_indices = Vec::new();
        for (old_index, identity) in reset_identities.iter().enumerate() {
            if identity.is_some() {
                retired_indices.push(old_index);
            }
        }
        retired_indices.sort_by_key(|old_index| {
            (
                self.active[*old_index].item.logical_index(),
                self.active[*old_index].identity.slot_index(),
            )
        });

        if self.reentry_attempted.get() {
            self.poison_lifecycle(
                VirtualLayoutMaterializationDiagnosticCode::ReentrantReconciliation,
            );
            return Err(VirtualLayoutMaterializationError::Reentrant);
        }
        self.begin_lifecycle();
        for &old_index in &retired_indices {
            let result = {
                let old = &self.active[old_index];
                let evidence = VirtualLayoutProjectionEvidence::from_item(
                    commit.fence(),
                    &old.item,
                    old.identity,
                );
                let reentry = VirtualLayoutMaterializationReentry {
                    committing: self.commit_guard.as_ref(),
                    attempted: &self.reentry_attempted,
                };
                invoke_lifecycle_callback(|| {
                    self.lifecycle.unmount(&old.payload, evidence, &reentry)
                })
            };
            self.handle_lifecycle_result::<J::Error>(result)?;
        }
        for &old_index in &retired_indices {
            let result = {
                let old = &self.active[old_index];
                let reset_identity = match reset_identities[old_index] {
                    Some(identity) => identity,
                    None => old.identity,
                };
                let evidence = VirtualLayoutProjectionEvidence::from_item(
                    commit.fence(),
                    &old.item,
                    reset_identity,
                );
                let reentry = VirtualLayoutMaterializationReentry {
                    committing: self.commit_guard.as_ref(),
                    attempted: &self.reentry_attempted,
                };
                invoke_lifecycle_callback(|| self.lifecycle.reset(&old.payload, evidence, &reentry))
            };
            self.handle_lifecycle_result::<J::Error>(result)?;
        }
        for staged_item in &staged {
            if let PlannedAction::Compatible { old_index } = staged_item.plan.action {
                let result = {
                    let old = &self.active[old_index];
                    let evidence = VirtualLayoutProjectionEvidence::from_item(
                        commit.fence(),
                        &staged_item.plan.item,
                        staged_item.plan.identity,
                    );
                    let reentry = VirtualLayoutMaterializationReentry {
                        committing: self.commit_guard.as_ref(),
                        attempted: &self.reentry_attempted,
                    };
                    invoke_lifecycle_callback(|| {
                        self.lifecycle.reconcile(
                            &old.payload,
                            &staged_item.projection.payload,
                            evidence,
                            &reentry,
                        )
                    })
                };
                self.handle_lifecycle_result::<J::Error>(result)?;
            }
        }
        for staged_item in &staged {
            if !matches!(staged_item.plan.action, PlannedAction::Compatible { .. }) {
                let recycled_shell = match staged_item.plan.action {
                    PlannedAction::Replacement { old_index } => {
                        self.active.get(old_index).map(|slot| &slot.payload)
                    }
                    PlannedAction::Inserted {
                        recycled: Some(RecycleSource::Existing(index)),
                    } => self.recyclable.get(index).map(|shell| &shell.payload),
                    PlannedAction::Inserted {
                        recycled: Some(RecycleSource::Removed(index)),
                    } => self.active.get(index).map(|slot| &slot.payload),
                    _ => None,
                };
                let result = {
                    let evidence = VirtualLayoutProjectionEvidence::from_item(
                        commit.fence(),
                        &staged_item.plan.item,
                        staged_item.plan.identity,
                    );
                    let reentry = VirtualLayoutMaterializationReentry {
                        committing: self.commit_guard.as_ref(),
                        attempted: &self.reentry_attempted,
                    };
                    invoke_lifecycle_callback(|| {
                        self.lifecycle.mount(
                            recycled_shell,
                            &staged_item.projection.payload,
                            evidence,
                            &reentry,
                        )
                    })
                };
                self.handle_lifecycle_result::<J::Error>(result)?;
            }
        }

        let mut consumed_existing = vec![false; self.recyclable.len()];
        let mut consumed_removed = vec![false; self.active.len()];
        for staged_item in &staged {
            if let PlannedAction::Inserted {
                recycled: Some(source),
            } = staged_item.plan.action
            {
                match source {
                    RecycleSource::Existing(index) => consumed_existing[index] = true,
                    RecycleSource::Removed(index) => consumed_removed[index] = true,
                }
            }
        }

        let old_active = std::mem::take(&mut self.active);
        let old_recyclable = std::mem::take(&mut self.recyclable);
        let mut next_active = Vec::with_capacity(staged.len());
        for staged_item in staged {
            next_active.push(ActiveSlot {
                item: staged_item.plan.item,
                kind: staged_item.projection.kind,
                identity: staged_item.plan.identity,
                payload: staged_item.projection.payload,
            });
        }
        let mut next_recyclable = Vec::with_capacity(recycle_plans.len());
        for (index, shell) in old_recyclable.into_iter().enumerate() {
            if !consumed_existing[index] {
                next_recyclable.push(shell);
            }
        }
        for (index, old) in old_active.into_iter().enumerate() {
            if let Some(identity) = removed_reset_identities[index]
                && !consumed_removed[index]
            {
                next_recyclable.push(RecyclableSlot {
                    identity,
                    last_logical_index: old.item.logical_index(),
                    payload: old.payload,
                });
            }
        }
        next_recyclable
            .sort_by_key(|shell| (shell.last_logical_index, shell.identity.slot_index()));
        let recyclable_capacity = bound.saturating_sub(next_active.len());
        if next_recyclable.len() > recyclable_capacity {
            next_recyclable.truncate(recyclable_capacity);
        }

        self.active = next_active;
        self.recyclable = next_recyclable;
        self.next_slot_index = next_slot_index;
        self.authoritative_fence = Some(commit.fence().clone());
        self.authoritative_revision = Some(commit.accepted_revision());
        self.lifecycle_state = LifecycleState::Mounted;
        drop(guard);
        Ok(())
    }

    /// Deterministically unmount every active slot and retire the store.
    pub(crate) fn unmount(&mut self) -> Result<(), VirtualLayoutMaterializationError<(), A::Error>>
    where
        A: VirtualLayoutLifecycleAdapter<P>,
    {
        if self.commit_guard.replace(true) {
            self.reentry_attempted.set(true);
            self.poison_lifecycle(
                VirtualLayoutMaterializationDiagnosticCode::ReentrantReconciliation,
            );
            return Err(VirtualLayoutMaterializationError::Reentrant);
        }
        let guard = CommitGuard(Rc::clone(&self.commit_guard));
        self.reentry_attempted.set(false);
        match self.lifecycle_state {
            LifecycleState::Mounted => {}
            LifecycleState::CleanlyRetired => {
                drop(guard);
                return Ok(());
            }
            LifecycleState::LifecycleIndeterminate => {
                return self.reject(
                    VirtualLayoutMaterializationDiagnosticCode::LifecycleIndeterminate,
                    VirtualLayoutMaterializationError::LifecycleIndeterminate,
                );
            }
        }

        let fence = match self.authoritative_fence.clone() {
            Some(fence) => fence,
            None if self.active.is_empty() => {
                self.active.clear();
                self.recyclable.clear();
                self.authoritative_revision = None;
                self.lifecycle_state = LifecycleState::CleanlyRetired;
                drop(guard);
                return Ok(());
            }
            None => {
                return self.reject(
                    VirtualLayoutMaterializationDiagnosticCode::InvalidCommit,
                    VirtualLayoutMaterializationError::InvalidCommit,
                );
            }
        };
        let mut active_indices: Vec<usize> = (0..self.active.len()).collect();
        active_indices.sort_by_key(|index| {
            (
                self.active[*index].item.logical_index(),
                self.active[*index].identity.slot_index(),
            )
        });
        self.begin_lifecycle();
        for index in active_indices {
            let result = {
                let slot = &self.active[index];
                let evidence =
                    VirtualLayoutProjectionEvidence::from_item(&fence, &slot.item, slot.identity);
                let reentry = VirtualLayoutMaterializationReentry {
                    committing: self.commit_guard.as_ref(),
                    attempted: &self.reentry_attempted,
                };
                invoke_lifecycle_callback(|| {
                    self.lifecycle.unmount(&slot.payload, evidence, &reentry)
                })
            };
            self.handle_lifecycle_result::<()>(result)?;
        }
        self.active.clear();
        self.recyclable.clear();
        self.authoritative_fence = None;
        self.authoritative_revision = None;
        self.lifecycle_state = LifecycleState::CleanlyRetired;
        drop(guard);
        Ok(())
    }

    fn begin_lifecycle(&mut self) {
        self.lifecycle_state = LifecycleState::LifecycleIndeterminate;
    }

    fn poison_lifecycle(&mut self, code: VirtualLayoutMaterializationDiagnosticCode) {
        self.lifecycle_state = LifecycleState::LifecycleIndeterminate;
        self.active.clear();
        self.recyclable.clear();
        self.authoritative_fence = None;
        self.authoritative_revision = None;
        self.diagnostics.record(code);
    }

    fn handle_lifecycle_result<ProjectionError>(
        &mut self,
        result: Result<Result<(), A::Error>, Box<dyn std::any::Any + Send>>,
    ) -> Result<(), VirtualLayoutMaterializationError<ProjectionError, A::Error>>
    where
        A: VirtualLayoutLifecycleAdapter<P>,
    {
        match result {
            Err(panic) => {
                self.poison_lifecycle(
                    VirtualLayoutMaterializationDiagnosticCode::LifecyclePanicked,
                );
                std::panic::resume_unwind(panic);
            }
            Ok(_) if self.reentry_attempted.get() => {
                self.poison_lifecycle(
                    VirtualLayoutMaterializationDiagnosticCode::ReentrantReconciliation,
                );
                Err(VirtualLayoutMaterializationError::Reentrant)
            }
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => {
                self.poison_lifecycle(VirtualLayoutMaterializationDiagnosticCode::LifecycleFailed);
                Err(VirtualLayoutMaterializationError::Lifecycle(error))
            }
        }
    }

    fn reject<E, F>(
        &mut self,
        code: VirtualLayoutMaterializationDiagnosticCode,
        error: VirtualLayoutMaterializationError<E, F>,
    ) -> Result<(), VirtualLayoutMaterializationError<E, F>> {
        self.diagnostics.record(code);
        Err(error)
    }
}

struct CommitGuard(Rc<Cell<bool>>);

impl Drop for CommitGuard {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

fn invoke_lifecycle_callback<Error>(
    callback: impl FnOnce() -> Result<(), Error>,
) -> Result<Result<(), Error>, Box<dyn std::any::Any + Send>> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(callback))
}

struct RecyclePlan {
    source: RecycleSource,
    identity: VirtualLayoutSlotIdentity,
    last_logical_index: usize,
}

fn error_for_scope_code<ProjectionError, LifecycleError>(
    code: VirtualLayoutMaterializationDiagnosticCode,
) -> VirtualLayoutMaterializationError<ProjectionError, LifecycleError> {
    match code {
        VirtualLayoutMaterializationDiagnosticCode::ForeignContainer => {
            VirtualLayoutMaterializationError::ForeignContainer
        }
        VirtualLayoutMaterializationDiagnosticCode::ForeignPolicy => {
            VirtualLayoutMaterializationError::ForeignPolicy
        }
        VirtualLayoutMaterializationDiagnosticCode::ForeignMount => {
            VirtualLayoutMaterializationError::ForeignMount
        }
        VirtualLayoutMaterializationDiagnosticCode::UnstablePolicyIdentity => {
            VirtualLayoutMaterializationError::UnstablePolicyIdentity
        }
        _ => VirtualLayoutMaterializationError::InvalidCommit,
    }
}

fn error_for_entry_code<ProjectionError, LifecycleError>(
    code: VirtualLayoutMaterializationDiagnosticCode,
) -> VirtualLayoutMaterializationError<ProjectionError, LifecycleError> {
    match code {
        VirtualLayoutMaterializationDiagnosticCode::DuplicateKey => {
            VirtualLayoutMaterializationError::DuplicateKey
        }
        VirtualLayoutMaterializationDiagnosticCode::UnstableKey => {
            VirtualLayoutMaterializationError::UnstableKey
        }
        VirtualLayoutMaterializationDiagnosticCode::DuplicateLogicalIndex => {
            VirtualLayoutMaterializationError::DuplicateLogicalIndex
        }
        VirtualLayoutMaterializationDiagnosticCode::GenerationOverflow => {
            VirtualLayoutMaterializationError::GenerationOverflow
        }
        VirtualLayoutMaterializationDiagnosticCode::UnstableCompatibility => {
            VirtualLayoutMaterializationError::UnstableCompatibility
        }
        VirtualLayoutMaterializationDiagnosticCode::SlotArithmeticOverflow => {
            VirtualLayoutMaterializationError::SlotArithmeticOverflow
        }
        _ => VirtualLayoutMaterializationError::InvalidCommit,
    }
}

fn validate_entries(
    entries: &[VirtualLayoutItem],
) -> Result<(), VirtualLayoutMaterializationDiagnosticCode> {
    for (position, entry) in entries.iter().enumerate() {
        if entry.key().stable_equals(entry.key()) != Some(true) {
            return Err(VirtualLayoutMaterializationDiagnosticCode::UnstableKey);
        }
        let bounds = entry.bounds();
        if !bounds.is_finite() || bounds.min.x > bounds.max.x || bounds.min.y > bounds.max.y {
            return Err(VirtualLayoutMaterializationDiagnosticCode::InvalidCommit);
        }
        for previous in entries.iter().take(position) {
            match previous.key().stable_equals(entry.key()) {
                Some(true) => return Err(VirtualLayoutMaterializationDiagnosticCode::DuplicateKey),
                Some(false) => {}
                None => return Err(VirtualLayoutMaterializationDiagnosticCode::UnstableKey),
            }
            if previous.logical_index() == entry.logical_index() {
                return Err(VirtualLayoutMaterializationDiagnosticCode::DuplicateLogicalIndex);
            }
        }
    }
    Ok(())
}

fn validate_active<P>(
    active: &[ActiveSlot<P>],
) -> Result<(), VirtualLayoutMaterializationDiagnosticCode> {
    for (position, slot) in active.iter().enumerate() {
        if slot.item.key().stable_equals(slot.item.key()) != Some(true) {
            return Err(VirtualLayoutMaterializationDiagnosticCode::UnstableKey);
        }
        if slot.kind.stable_equals(&slot.kind) != Some(true) {
            return Err(VirtualLayoutMaterializationDiagnosticCode::UnstableCompatibility);
        }
        for previous in active.iter().take(position) {
            match previous.item.key().stable_equals(slot.item.key()) {
                Some(true) => return Err(VirtualLayoutMaterializationDiagnosticCode::DuplicateKey),
                Some(false) => {}
                None => return Err(VirtualLayoutMaterializationDiagnosticCode::UnstableKey),
            }
        }
    }
    Ok(())
}

fn contains_key(
    entries: &[&VirtualLayoutItem],
    key: &VirtualLayoutItemKey,
) -> Result<bool, VirtualLayoutMaterializationDiagnosticCode> {
    for entry in entries {
        match entry.key().stable_equals(key) {
            Some(true) => return Ok(true),
            Some(false) => {}
            None => return Err(VirtualLayoutMaterializationDiagnosticCode::UnstableKey),
        }
    }
    Ok(false)
}

fn find_active_key<P>(
    active: &[ActiveSlot<P>],
    key: &VirtualLayoutItemKey,
) -> Result<Option<usize>, VirtualLayoutMaterializationDiagnosticCode> {
    for (index, slot) in active.iter().enumerate() {
        match slot.item.key().stable_equals(key) {
            Some(true) => return Ok(Some(index)),
            Some(false) => {}
            None => return Err(VirtualLayoutMaterializationDiagnosticCode::UnstableKey),
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::types::Vector2;
    use std::{cell::RefCell, rc::Rc};

    use super::super::coordinator::{
        VirtualLayoutCompletion, VirtualLayoutCoordinatorDiagnostic,
        VirtualLayoutCoordinatorDiagnosticCode,
    };
    use super::super::{
        VirtualLayoutBoundsConfidence, VirtualLayoutBudget, VirtualLayoutCoordinateSpace,
        VirtualLayoutDeferredReason, VirtualLayoutDiagnosticCode, VirtualLayoutExtentCandidate,
        VirtualLayoutItemCandidate, VirtualLayoutPolicy, VirtualLayoutPolicyDecision,
        VirtualLayoutQueryInput, VirtualLayoutQueryInputParts, VirtualLayoutUnavailableReason,
        VirtualLayoutVisibility,
    };

    const CONTAINER_ID: NodeId = 41;
    const MOUNT_GENERATION: u64 = 7;
    const DEFAULT_BUDGET: usize = 8;

    #[derive(Clone, Copy)]
    struct Spec {
        key: u32,
        logical_index: usize,
        y: f32,
    }

    impl Spec {
        const fn new(key: u32, logical_index: usize, y: f32) -> Self {
            Self {
                key,
                logical_index,
                y,
            }
        }
    }

    struct TestPolicy {
        entries: Vec<Spec>,
        extent: VirtualLayoutExtentCandidate,
        decision: VirtualLayoutPolicyDecision,
        calls: Rc<Cell<usize>>,
    }

    impl TestPolicy {
        fn ready(entries: &[Spec], budget: usize, calls: Rc<Cell<usize>>) -> Self {
            let _ = budget;
            Self {
                entries: entries.to_vec(),
                extent: VirtualLayoutExtentCandidate::exact(Vector2::new(100.0, 1_000.0)),
                decision: VirtualLayoutPolicyDecision::Ready,
                calls,
            }
        }

        fn ready_with_extent(
            entries: &[Spec],
            extent: VirtualLayoutExtentCandidate,
            calls: Rc<Cell<usize>>,
        ) -> Self {
            Self {
                entries: entries.to_vec(),
                extent,
                decision: VirtualLayoutPolicyDecision::Ready,
                calls,
            }
        }

        fn disposition(decision: VirtualLayoutPolicyDecision, calls: Rc<Cell<usize>>) -> Self {
            Self {
                entries: Vec::new(),
                extent: VirtualLayoutExtentCandidate::exact(Vector2::new(100.0, 1_000.0)),
                decision,
                calls,
            }
        }
    }

    impl VirtualLayoutPolicy for TestPolicy {
        fn query(
            &self,
            _input: &VirtualLayoutQueryInput,
            sink: &mut super::super::VirtualLayoutQuerySink,
        ) -> VirtualLayoutPolicyDecision {
            self.calls.set(self.calls.get().saturating_add(1));
            if self.decision != VirtualLayoutPolicyDecision::Ready {
                return self.decision;
            }
            for entry in &self.entries {
                let _ = sink.visit(VirtualLayoutItemCandidate::new(
                    VirtualLayoutItemKey::new(entry.key),
                    entry.logical_index,
                    Rect::from_xy_size(0.0, entry.y, 100.0, 10.0),
                    VirtualLayoutVisibility::Visible,
                    VirtualLayoutBoundsConfidence::Exact,
                ));
            }
            let _ = sink.set_extent(self.extent);
            self.decision
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct ProjectionFailure;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct Payload {
        logical_index: usize,
        serial: usize,
    }

    struct TestProjector {
        calls: Rc<Cell<usize>>,
        kind_calls: Rc<Cell<usize>>,
        project_calls: Rc<Cell<usize>>,
        serial: Cell<usize>,
        kind: Cell<u8>,
        fail_kind: Cell<bool>,
        fail_project: Cell<bool>,
        mismatch_projected_kind: Cell<bool>,
    }

    impl TestProjector {
        fn new() -> Self {
            Self {
                calls: Rc::new(Cell::new(0)),
                kind_calls: Rc::new(Cell::new(0)),
                project_calls: Rc::new(Cell::new(0)),
                serial: Cell::new(0),
                kind: Cell::new(1),
                fail_kind: Cell::new(false),
                fail_project: Cell::new(false),
                mismatch_projected_kind: Cell::new(false),
            }
        }

        fn calls(&self) -> usize {
            self.calls.get()
        }

        fn next_serial(&self) -> usize {
            let next = self.serial.get().saturating_add(1);
            self.serial.set(next);
            next
        }
    }

    impl VirtualLayoutHostProjector for TestProjector {
        type Payload = Payload;
        type Error = ProjectionFailure;

        fn projection_kind(
            &self,
            _item: &VirtualLayoutItem,
        ) -> Result<VirtualLayoutProjectionKind, Self::Error> {
            self.calls.set(self.calls.get().saturating_add(1));
            self.kind_calls.set(self.kind_calls.get().saturating_add(1));
            if self.fail_kind.get() {
                return Err(ProjectionFailure);
            }
            Ok(VirtualLayoutProjectionKind::new(self.kind.get()))
        }

        fn project<'a>(
            &self,
            evidence: VirtualLayoutProjectionEvidence<'a>,
        ) -> Result<VirtualLayoutProjection<Self::Payload>, Self::Error> {
            self.calls.set(self.calls.get().saturating_add(1));
            self.project_calls
                .set(self.project_calls.get().saturating_add(1));
            if self.fail_project.get() {
                return Err(ProjectionFailure);
            }
            let kind = if self.mismatch_projected_kind.get() {
                self.kind.get().wrapping_add(1)
            } else {
                self.kind.get()
            };
            Ok(VirtualLayoutProjection::new(
                VirtualLayoutProjectionKind::new(kind),
                Payload {
                    logical_index: evidence.logical_index(),
                    serial: self.next_serial(),
                },
            ))
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Callback {
        Unmount,
        Reset,
        Reconcile,
        Mount,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum CompatibilityMode {
        Stable,
        ForceIncompatible,
        Unstable,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct LifecycleFailure;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct LifecyclePanic;

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct Event {
        callback: Callback,
        key: Option<u32>,
        logical_index: usize,
        generation: u64,
        shell_logical_index: Option<usize>,
    }

    type TestEvents = Rc<RefCell<Vec<Event>>>;
    type TestStore = VirtualLayoutMaterializationStore<Payload, TestLifecycle>;

    struct TestLifecycle {
        events: Rc<RefCell<Vec<Event>>>,
        known_keys: Rc<Vec<(u32, VirtualLayoutItemKey)>>,
        compatibility: Cell<CompatibilityMode>,
        fail_on: Cell<Option<Callback>>,
        reenter_on: Cell<Option<Callback>>,
        panic_on: Cell<Option<Callback>>,
    }

    impl TestLifecycle {
        fn new(keys: &[u32]) -> (Self, Rc<RefCell<Vec<Event>>>) {
            let events = Rc::new(RefCell::new(Vec::new()));
            let known_keys = keys
                .iter()
                .copied()
                .map(|key| (key, VirtualLayoutItemKey::new(key)))
                .collect();
            (
                Self {
                    events: Rc::clone(&events),
                    known_keys: Rc::new(known_keys),
                    compatibility: Cell::new(CompatibilityMode::Stable),
                    fail_on: Cell::new(None),
                    reenter_on: Cell::new(None),
                    panic_on: Cell::new(None),
                },
                events,
            )
        }

        fn key_label(&self, key: &VirtualLayoutItemKey) -> Option<u32> {
            self.known_keys
                .iter()
                .find(|(_, known)| known == key)
                .map(|(label, _)| *label)
        }

        fn record(
            &self,
            callback: Callback,
            evidence: &VirtualLayoutProjectionEvidence<'_>,
            shell_logical_index: Option<usize>,
        ) {
            self.events.borrow_mut().push(Event {
                callback,
                key: self.key_label(evidence.key()),
                logical_index: evidence.logical_index(),
                generation: evidence.proposed_slot().checked_generation(),
                shell_logical_index,
            });
        }

        fn callback_result(
            &self,
            callback: Callback,
            reentry: &VirtualLayoutMaterializationReentry<'_>,
        ) -> Result<(), LifecycleFailure> {
            if self.reenter_on.get() == Some(callback) {
                let _ = reentry.try_reconcile();
            }
            if self.panic_on.get() == Some(callback) {
                std::panic::panic_any(LifecyclePanic);
            }
            if self.fail_on.get() == Some(callback) {
                return Err(LifecycleFailure);
            }
            Ok(())
        }
    }

    impl VirtualLayoutLifecycleAdapter<Payload> for TestLifecycle {
        type Error = LifecycleFailure;

        fn compatible(
            &self,
            previous: &VirtualLayoutProjectionKind,
            next: &VirtualLayoutProjectionKind,
        ) -> Option<bool> {
            match self.compatibility.get() {
                CompatibilityMode::Stable => previous.stable_equals(next),
                CompatibilityMode::ForceIncompatible => Some(false),
                CompatibilityMode::Unstable => None,
            }
        }

        fn unmount(
            &mut self,
            payload: &Payload,
            evidence: VirtualLayoutProjectionEvidence<'_>,
            reentry: &VirtualLayoutMaterializationReentry<'_>,
        ) -> Result<(), Self::Error> {
            self.record(Callback::Unmount, &evidence, None);
            let _ = payload;
            self.callback_result(Callback::Unmount, reentry)
        }

        fn reset(
            &mut self,
            payload: &Payload,
            evidence: VirtualLayoutProjectionEvidence<'_>,
            reentry: &VirtualLayoutMaterializationReentry<'_>,
        ) -> Result<(), Self::Error> {
            self.record(Callback::Reset, &evidence, None);
            let _ = payload;
            self.callback_result(Callback::Reset, reentry)
        }

        fn reconcile(
            &mut self,
            previous: &Payload,
            next: &Payload,
            evidence: VirtualLayoutProjectionEvidence<'_>,
            reentry: &VirtualLayoutMaterializationReentry<'_>,
        ) -> Result<(), Self::Error> {
            self.record(Callback::Reconcile, &evidence, Some(previous.logical_index));
            let _ = next;
            self.callback_result(Callback::Reconcile, reentry)
        }

        fn mount(
            &mut self,
            recycled_shell: Option<&Payload>,
            next: &Payload,
            evidence: VirtualLayoutProjectionEvidence<'_>,
            reentry: &VirtualLayoutMaterializationReentry<'_>,
        ) -> Result<(), Self::Error> {
            self.record(
                Callback::Mount,
                &evidence,
                recycled_shell.map(|shell| shell.logical_index),
            );
            let _ = next;
            self.callback_result(Callback::Mount, reentry)
        }
    }

    fn coordinator_with(
        container_id: NodeId,
        policy_identity: VirtualLayoutPolicyIdentity,
        mount_generation: u64,
    ) -> VirtualLayoutWindowCoordinator {
        VirtualLayoutWindowCoordinator::new(container_id, policy_identity, mount_generation)
    }

    fn default_coordinator() -> VirtualLayoutWindowCoordinator {
        coordinator_with(
            CONTAINER_ID,
            VirtualLayoutPolicyIdentity::new("policy"),
            MOUNT_GENERATION,
        )
    }

    fn parts_for(
        coordinator: &VirtualLayoutWindowCoordinator,
        data_revision: u64,
        budget: usize,
    ) -> VirtualLayoutQueryInputParts {
        let (container_id, policy_identity, mount_generation) = coordinator.scope();
        VirtualLayoutQueryInputParts {
            container_id,
            policy_identity: policy_identity.clone(),
            mount_generation,
            query_sequence: 0,
            viewport: Rect::from_xy_size(0.0, 0.0, 100.0, 100.0),
            coordinate_space: VirtualLayoutCoordinateSpace::logical(),
            overscan: super::super::VirtualLayoutOverscan::new(0.0, 0.0)
                .expect("test overscan is finite"),
            budget: VirtualLayoutBudget::new(budget),
            viewport_revision: 1,
            data_revision,
            policy_revision: 1,
            measurement_revision: 1,
            semantic_revision: 1,
        }
    }

    fn committed(
        coordinator: &mut VirtualLayoutWindowCoordinator,
        entries: &[Spec],
        data_revision: u64,
        budget: usize,
        policy_calls: Rc<Cell<usize>>,
    ) -> Box<VirtualLayoutCommit> {
        let pending = coordinator
            .begin_query(parts_for(coordinator, data_revision, budget))
            .expect("test query should begin");
        let policy = TestPolicy::ready(entries, budget, policy_calls);
        let outcome = pending.execute(&policy);
        match coordinator.complete(pending, outcome) {
            VirtualLayoutCompletion::Committed(commit) => commit,
            _ => panic!("test policy should commit"),
        }
    }

    fn committed_with_extent(
        coordinator: &mut VirtualLayoutWindowCoordinator,
        entries: &[Spec],
        data_revision: u64,
        budget: usize,
        extent: VirtualLayoutExtentCandidate,
        policy_calls: Rc<Cell<usize>>,
    ) -> Box<VirtualLayoutCommit> {
        let pending = coordinator
            .begin_query(parts_for(coordinator, data_revision, budget))
            .expect("test query should begin");
        let policy = TestPolicy::ready_with_extent(entries, extent, policy_calls);
        let outcome = pending.execute(&policy);
        match coordinator.complete(pending, outcome) {
            VirtualLayoutCompletion::Committed(commit) => commit,
            _ => panic!("test policy should commit"),
        }
    }

    fn store_for(
        keys: &[u32],
    ) -> (
        VirtualLayoutWindowCoordinator,
        TestStore,
        TestProjector,
        TestEvents,
    ) {
        let coordinator = default_coordinator();
        let (lifecycle, events) = TestLifecycle::new(keys);
        let store = VirtualLayoutMaterializationStore::new(&coordinator, lifecycle);
        (coordinator, store, TestProjector::new(), events)
    }

    fn event_kinds(events: &[Event]) -> Vec<Callback> {
        events.iter().map(|event| event.callback).collect()
    }

    #[derive(Clone, Copy)]
    enum LifecycleFault {
        Error,
        Reentry,
    }

    fn lifecycle_fault_case(
        phase: Callback,
        fault: LifecycleFault,
    ) -> (
        VirtualLayoutWindowCoordinator,
        TestStore,
        TestProjector,
        TestEvents,
        Box<VirtualLayoutCommit>,
    ) {
        let (mut coordinator, mut store, projector, events) = store_for(&[1, 2]);
        let policy_calls = Rc::new(Cell::new(0));
        let initial = committed(
            &mut coordinator,
            &[Spec::new(1, 0, 0.0)],
            1,
            DEFAULT_BUDGET,
            Rc::clone(&policy_calls),
        );
        store.publish(&initial, &projector).expect("initial commit");
        events.borrow_mut().clear();

        let next_entries = match phase {
            Callback::Unmount | Callback::Reset => Vec::new(),
            Callback::Reconcile => vec![Spec::new(1, 0, 10.0)],
            Callback::Mount => {
                projector.kind.set(2);
                vec![Spec::new(1, 0, 20.0)]
            }
        };
        let next = committed(
            &mut coordinator,
            &next_entries,
            2,
            DEFAULT_BUDGET,
            Rc::clone(&policy_calls),
        );
        match fault {
            LifecycleFault::Error => store.lifecycle.fail_on.set(Some(phase)),
            LifecycleFault::Reentry => store.lifecycle.reenter_on.set(Some(phase)),
        }
        (coordinator, store, projector, events, next)
    }

    fn assert_lifecycle_indeterminate(store: &TestStore) {
        assert!(!store.is_mounted());
        assert!(store.active_slots().is_empty());
        assert_eq!(store.active_len(), 0);
        assert_eq!(store.recyclable_len(), 0);
        assert_eq!(store.authoritative_fence(), None);
        assert_eq!(store.authoritative_revision(), None);
    }

    fn empty_entries() -> [Spec; 0] {
        []
    }

    #[test]
    fn initial_accepted_commit_projects_and_mounts_exact_bounded_items() {
        let (mut coordinator, mut store, projector, events) = store_for(&[11, 22]);
        let policy_calls = Rc::new(Cell::new(0));
        let commit = committed(
            &mut coordinator,
            &[Spec::new(11, 4, 40.0), Spec::new(22, 9, 90.0)],
            1,
            DEFAULT_BUDGET,
            Rc::clone(&policy_calls),
        );

        store
            .publish(&commit, &projector)
            .expect("initial commit should materialize");

        assert_eq!(policy_calls.get(), 1);
        assert_eq!(projector.kind_calls.get(), 2);
        assert_eq!(projector.project_calls.get(), 2);
        let active = store.active_slots();
        assert_eq!(active.len(), 2);
        assert_eq!(active[0].item().logical_index(), 4);
        assert_eq!(active[1].item().logical_index(), 9);
        assert_eq!(active[0].identity().slot_index(), 0);
        assert_eq!(active[1].identity().slot_index(), 1);
        assert_eq!(active[0].identity().checked_generation(), 1);
        assert_eq!(active[1].identity().checked_generation(), 1);
        assert!(active[0].item().key() == &VirtualLayoutItemKey::new(11_u32));
        assert!(active[1].item().key() == &VirtualLayoutItemKey::new(22_u32));
        assert_eq!(
            event_kinds(&events.borrow()),
            vec![Callback::Mount, Callback::Mount]
        );
        assert_eq!(events.borrow()[0].key, Some(11));
        assert_eq!(events.borrow()[1].key, Some(22));
    }

    #[test]
    fn policy_query_has_zero_projector_calls_and_zero_entry_commits_project_nothing() {
        let (mut coordinator, mut store, projector, events) = store_for(&[]);
        let policy_calls = Rc::new(Cell::new(0));
        let pending = coordinator
            .begin_query(parts_for(&coordinator, 1, 0))
            .expect("zero-budget query should begin");
        let policy = TestPolicy::ready_with_extent(
            &empty_entries(),
            VirtualLayoutExtentCandidate::exact(Vector2::new(100.0, 1_000.0)),
            Rc::clone(&policy_calls),
        );
        let outcome = pending.execute(&policy);
        assert_eq!(policy_calls.get(), 1);
        assert_eq!(projector.calls(), 0);
        let VirtualLayoutCompletion::Committed(commit) = coordinator.complete(pending, outcome)
        else {
            panic!("extent-only query should commit");
        };
        store
            .publish(&commit, &projector)
            .expect("extent-only commit should be accepted");
        assert_eq!(projector.calls(), 0);
        assert!(events.borrow().is_empty());
        assert_eq!(store.active_len(), 0);
        assert_eq!(store.recyclable_len(), 0);

        let policy_calls = Rc::new(Cell::new(0));
        let unavailable_commit = committed_with_extent(
            &mut coordinator,
            &empty_entries(),
            2,
            DEFAULT_BUDGET,
            VirtualLayoutExtentCandidate::Unavailable,
            Rc::clone(&policy_calls),
        );
        store
            .publish(&unavailable_commit, &projector)
            .expect("unavailable extent-only accepted result should materialize zero slots");
        assert_eq!(policy_calls.get(), 1);
        assert_eq!(projector.calls(), 0);
        assert!(events.borrow().is_empty());
        assert_eq!(store.active_len(), 0);
    }

    #[test]
    fn deferred_unavailable_invalid_stale_and_rejected_outcomes_never_publish() {
        for decision in [
            VirtualLayoutPolicyDecision::Deferred(VirtualLayoutDeferredReason::Retry),
            VirtualLayoutPolicyDecision::Unavailable(VirtualLayoutUnavailableReason::Unsupported),
            VirtualLayoutPolicyDecision::Invalid(VirtualLayoutDiagnosticCode::PolicyRejected),
        ] {
            let (mut coordinator, store, projector, events) = store_for(&[1]);
            let policy_calls = Rc::new(Cell::new(0));
            let pending = coordinator
                .begin_query(parts_for(&coordinator, 1, DEFAULT_BUDGET))
                .expect("disposition query should begin");
            let policy = TestPolicy::disposition(decision, Rc::clone(&policy_calls));
            let outcome = pending.execute(&policy);
            let completion = coordinator.complete(pending, outcome);
            assert!(!matches!(completion, VirtualLayoutCompletion::Committed(_)));
            assert_eq!(projector.calls(), 0);
            assert!(events.borrow().is_empty());
            assert_eq!(store.active_len(), 0);
            assert_eq!(store.authoritative_revision(), None);
        }

        let (mut coordinator, store, projector, events) = store_for(&[1]);
        let policy_calls = Rc::new(Cell::new(0));
        let old_pending = coordinator
            .begin_query(parts_for(&coordinator, 1, DEFAULT_BUDGET))
            .expect("old query should begin");
        let new_pending = coordinator
            .begin_query(parts_for(&coordinator, 1, DEFAULT_BUDGET))
            .expect("new query should begin");
        let old_policy = TestPolicy::ready(
            &[Spec::new(1, 0, 0.0)],
            DEFAULT_BUDGET,
            Rc::clone(&policy_calls),
        );
        let stale_outcome = old_pending.execute(&old_policy);
        let stale = coordinator.complete(old_pending, stale_outcome);
        assert!(matches!(stale, VirtualLayoutCompletion::Stale(_)));
        drop(new_pending);
        assert_eq!(projector.calls(), 0);
        assert!(events.borrow().is_empty());

        let rejected = VirtualLayoutCompletion::Rejected(VirtualLayoutCoordinatorDiagnostic {
            code: VirtualLayoutCoordinatorDiagnosticCode::ReentrantExecution,
        });
        assert!(!matches!(rejected, VirtualLayoutCompletion::Committed(_)));
        assert_eq!(store.active_len(), 0);
        assert_eq!(store.authoritative_revision(), None);
    }

    #[test]
    fn same_key_reorder_preserves_identity_and_reconciles_only() {
        let (mut coordinator, mut store, projector, events) = store_for(&[1, 2, 3]);
        let policy_calls = Rc::new(Cell::new(0));
        let first = committed(
            &mut coordinator,
            &[
                Spec::new(1, 0, 0.0),
                Spec::new(2, 1, 10.0),
                Spec::new(3, 2, 20.0),
            ],
            1,
            DEFAULT_BUDGET,
            Rc::clone(&policy_calls),
        );
        store.publish(&first, &projector).expect("first commit");
        let first_ids: Vec<_> = store
            .active_slots()
            .into_iter()
            .map(|slot| slot.identity())
            .collect();
        events.borrow_mut().clear();

        let second = committed(
            &mut coordinator,
            &[
                Spec::new(3, 2, 20.0),
                Spec::new(1, 0, 0.0),
                Spec::new(2, 1, 10.0),
            ],
            1,
            DEFAULT_BUDGET,
            Rc::clone(&policy_calls),
        );
        store.publish(&second, &projector).expect("reorder commit");
        let second_ids: Vec<_> = store
            .active_slots()
            .into_iter()
            .map(|slot| slot.identity())
            .collect();
        assert_eq!(first_ids, second_ids);
        assert_eq!(
            event_kinds(&events.borrow()),
            vec![
                Callback::Reconcile,
                Callback::Reconcile,
                Callback::Reconcile
            ]
        );
        assert_eq!(store.active_len(), 3);
        assert_eq!(store.recyclable_len(), 0);
    }

    #[test]
    fn same_key_compatible_refresh_projects_fresh_payload_and_preserves_identity() {
        let (mut coordinator, mut store, projector, events) = store_for(&[1]);
        let policy_calls = Rc::new(Cell::new(0));
        let first = committed(
            &mut coordinator,
            &[Spec::new(1, 3, 30.0)],
            1,
            DEFAULT_BUDGET,
            Rc::clone(&policy_calls),
        );
        store.publish(&first, &projector).expect("first commit");
        let first_slot = store.active_slots().remove(0);
        let first_identity = first_slot.identity();
        let first_serial = first_slot.payload().serial;
        events.borrow_mut().clear();

        let second = committed(
            &mut coordinator,
            &[Spec::new(1, 3, 75.0)],
            2,
            DEFAULT_BUDGET,
            Rc::clone(&policy_calls),
        );
        store.publish(&second, &projector).expect("refresh commit");
        let second_slot = store.active_slots().remove(0);
        assert_eq!(second_slot.identity(), first_identity);
        assert_eq!(second_slot.item().bounds().min.y, 75.0);
        assert_ne!(second_slot.payload().serial, first_serial);
        assert_eq!(event_kinds(&events.borrow()), vec![Callback::Reconcile]);
        assert_eq!(projector.project_calls.get(), 2);
    }

    #[test]
    fn same_key_incompatible_kind_unmounts_resets_and_mounts_without_reconcile() {
        let (mut coordinator, mut store, projector, events) = store_for(&[1]);
        let policy_calls = Rc::new(Cell::new(0));
        let first = committed(
            &mut coordinator,
            &[Spec::new(1, 4, 40.0)],
            1,
            DEFAULT_BUDGET,
            Rc::clone(&policy_calls),
        );
        store.publish(&first, &projector).expect("first commit");
        events.borrow_mut().clear();
        projector.kind.set(2);

        let second = committed(
            &mut coordinator,
            &[Spec::new(1, 4, 45.0)],
            2,
            DEFAULT_BUDGET,
            Rc::clone(&policy_calls),
        );
        store
            .publish(&second, &projector)
            .expect("replacement commit");
        let recorded = events.borrow().clone();
        assert_eq!(
            event_kinds(&recorded),
            vec![Callback::Unmount, Callback::Reset, Callback::Mount]
        );
        assert_eq!(recorded[0].generation, 1);
        assert_eq!(recorded[1].generation, 2);
        assert_eq!(recorded[2].generation, 2);
        assert_eq!(recorded[2].shell_logical_index, Some(4));
        assert_eq!(store.active_slots()[0].identity().checked_generation(), 2);
    }

    #[test]
    fn removed_inserted_churn_unmounts_resets_before_reusing_only_reset_shell() {
        let (mut coordinator, mut store, projector, events) = store_for(&[10, 20, 30]);
        let policy_calls = Rc::new(Cell::new(0));
        let first = committed(
            &mut coordinator,
            &[Spec::new(10, 10, 100.0), Spec::new(20, 20, 200.0)],
            1,
            DEFAULT_BUDGET,
            Rc::clone(&policy_calls),
        );
        store.publish(&first, &projector).expect("first commit");
        let removed_identity = store.active_slots()[0].identity();
        events.borrow_mut().clear();

        let second = committed(
            &mut coordinator,
            &[Spec::new(30, 30, 300.0)],
            2,
            DEFAULT_BUDGET,
            Rc::clone(&policy_calls),
        );
        store.publish(&second, &projector).expect("churn commit");
        let recorded = events.borrow().clone();
        assert_eq!(
            event_kinds(&recorded),
            vec![
                Callback::Unmount,
                Callback::Unmount,
                Callback::Reset,
                Callback::Reset,
                Callback::Mount,
            ]
        );
        assert_eq!(recorded[0].key, Some(10));
        assert_eq!(recorded[1].key, Some(20));
        assert_eq!(recorded[2].key, Some(10));
        assert_eq!(recorded[3].key, Some(20));
        assert_eq!(recorded[4].key, Some(30));
        assert_eq!(recorded[0].generation, 1);
        assert_eq!(recorded[1].generation, 1);
        assert_eq!(recorded[2].generation, 2);
        assert_eq!(recorded[3].generation, 2);
        assert_eq!(recorded[4].generation, 2);
        assert_eq!(recorded[4].shell_logical_index, Some(10));
        let active = store.active_slots();
        assert_eq!(active.len(), 1);
        assert_eq!(
            active[0].identity().slot_index(),
            removed_identity.slot_index()
        );
        assert_eq!(active[0].identity().checked_generation(), 2);
        assert_eq!(store.recyclable_len(), 1);
    }

    #[test]
    fn lifecycle_callback_order_is_logical_not_policy_emission_order() {
        let (mut coordinator, mut store, projector, events) = store_for(&[10, 20, 30, 40]);
        let policy_calls = Rc::new(Cell::new(0));
        let first = committed(
            &mut coordinator,
            &[
                Spec::new(10, 30, 300.0),
                Spec::new(20, 10, 100.0),
                Spec::new(30, 20, 200.0),
            ],
            1,
            DEFAULT_BUDGET,
            Rc::clone(&policy_calls),
        );
        store.publish(&first, &projector).expect("first commit");
        events.borrow_mut().clear();
        store
            .lifecycle
            .compatibility
            .set(CompatibilityMode::ForceIncompatible);

        let second = committed(
            &mut coordinator,
            &[
                Spec::new(30, 20, 205.0),
                Spec::new(40, 5, 50.0),
                Spec::new(20, 10, 105.0),
            ],
            2,
            DEFAULT_BUDGET,
            Rc::clone(&policy_calls),
        );
        store
            .publish(&second, &projector)
            .expect("replacement set commit");
        let recorded = events.borrow().clone();
        let unmount_indices: Vec<_> = recorded
            .iter()
            .filter(|event| event.callback == Callback::Unmount)
            .map(|event| event.logical_index)
            .collect();
        let reset_indices: Vec<_> = recorded
            .iter()
            .filter(|event| event.callback == Callback::Reset)
            .map(|event| event.logical_index)
            .collect();
        let mount_indices: Vec<_> = recorded
            .iter()
            .filter(|event| event.callback == Callback::Mount)
            .map(|event| event.logical_index)
            .collect();
        assert_eq!(unmount_indices, vec![10, 20, 30]);
        assert_eq!(reset_indices, vec![10, 20, 30]);
        assert_eq!(mount_indices, vec![5, 10, 20]);
        let first_mount = recorded
            .iter()
            .position(|event| event.callback == Callback::Mount)
            .expect("mount event");
        assert!(
            recorded[..first_mount]
                .iter()
                .all(|event| matches!(event.callback, Callback::Unmount | Callback::Reset))
        );
    }

    #[test]
    fn foreign_container_policy_mount_and_owner_are_rejected_without_lifecycle_mutation() {
        let (mut local_coordinator, mut store, projector, events) = store_for(&[1]);
        let policy_calls = Rc::new(Cell::new(0));
        let initial = committed(
            &mut local_coordinator,
            &[Spec::new(1, 0, 0.0)],
            1,
            DEFAULT_BUDGET,
            Rc::clone(&policy_calls),
        );
        store.publish(&initial, &projector).expect("initial commit");
        let initial_identity = store.active_slots()[0].identity();
        events.borrow_mut().clear();

        let cases = [
            (
                "container",
                CONTAINER_ID + 1,
                VirtualLayoutPolicyIdentity::new("policy"),
                MOUNT_GENERATION,
                VirtualLayoutMaterializationDiagnosticCode::ForeignContainer,
            ),
            (
                "policy",
                CONTAINER_ID,
                VirtualLayoutPolicyIdentity::new("other-policy"),
                MOUNT_GENERATION,
                VirtualLayoutMaterializationDiagnosticCode::ForeignPolicy,
            ),
            (
                "mount",
                CONTAINER_ID,
                VirtualLayoutPolicyIdentity::new("policy"),
                MOUNT_GENERATION + 1,
                VirtualLayoutMaterializationDiagnosticCode::ForeignMount,
            ),
        ];
        for (label, container_id, policy_identity, mount_generation, expected_code) in cases {
            let original_scope = store.scope.clone();
            store.scope = MaterializationScope {
                container_id,
                policy_identity,
                mount_generation,
            };
            let result = store.publish(&initial, &projector);
            store.scope = original_scope;
            assert!(matches!(
                (label, expected_code, result),
                (
                    "container",
                    VirtualLayoutMaterializationDiagnosticCode::ForeignContainer,
                    Err(VirtualLayoutMaterializationError::ForeignContainer)
                ) | (
                    "policy",
                    VirtualLayoutMaterializationDiagnosticCode::ForeignPolicy,
                    Err(VirtualLayoutMaterializationError::ForeignPolicy)
                ) | (
                    "mount",
                    VirtualLayoutMaterializationDiagnosticCode::ForeignMount,
                    Err(VirtualLayoutMaterializationError::ForeignMount)
                )
            ));
        }

        let mut owner_coordinator = coordinator_with(
            CONTAINER_ID,
            VirtualLayoutPolicyIdentity::new("policy"),
            MOUNT_GENERATION,
        );
        let owner_commit = committed(
            &mut owner_coordinator,
            &[Spec::new(3, 1, 20.0)],
            1,
            DEFAULT_BUDGET,
            Rc::clone(&policy_calls),
        );
        let result = store.publish(&owner_commit, &projector);
        assert!(matches!(
            result,
            Err(VirtualLayoutMaterializationError::ForeignOwner)
        ));
        assert!(events.borrow().is_empty());
        assert_eq!(store.active_slots()[0].identity(), initial_identity);
    }

    #[test]
    fn older_and_duplicate_revisions_are_rejected_but_a_later_complete_revision_may_skip() {
        let (mut coordinator, mut store, projector, events) = store_for(&[1]);
        let policy_calls = Rc::new(Cell::new(0));
        let first = committed(
            &mut coordinator,
            &[Spec::new(1, 0, 0.0)],
            1,
            DEFAULT_BUDGET,
            Rc::clone(&policy_calls),
        );
        store.publish(&first, &projector).expect("first commit");
        events.borrow_mut().clear();
        let duplicate = (*first).clone();
        assert!(matches!(
            store.publish(&duplicate, &projector),
            Err(VirtualLayoutMaterializationError::DuplicateRevision)
        ));
        assert!(events.borrow().is_empty());

        let second = committed(
            &mut coordinator,
            &[Spec::new(1, 0, 10.0)],
            2,
            DEFAULT_BUDGET,
            Rc::clone(&policy_calls),
        );
        store.publish(&second, &projector).expect("second commit");
        events.borrow_mut().clear();
        let _third = committed(
            &mut coordinator,
            &[Spec::new(1, 0, 20.0)],
            3,
            DEFAULT_BUDGET,
            Rc::clone(&policy_calls),
        );
        let _fourth = committed(
            &mut coordinator,
            &[Spec::new(1, 0, 30.0)],
            4,
            DEFAULT_BUDGET,
            Rc::clone(&policy_calls),
        );
        let skipped = committed(
            &mut coordinator,
            &[Spec::new(1, 0, 40.0)],
            5,
            DEFAULT_BUDGET,
            Rc::clone(&policy_calls),
        );
        store
            .publish(&skipped, &projector)
            .expect("later complete revision may skip");
        assert_eq!(store.authoritative_revision(), Some(5));
        events.borrow_mut().clear();
        assert!(matches!(
            store.publish(&second, &projector),
            Err(VirtualLayoutMaterializationError::OlderRevision)
        ));
        assert!(events.borrow().is_empty());
    }

    #[test]
    fn projector_and_kind_failures_remain_recoverable_before_lifecycle_callbacks() {
        let (mut coordinator, mut store, projector, events) = store_for(&[1]);
        let policy_calls = Rc::new(Cell::new(0));
        let initial = committed(
            &mut coordinator,
            &[Spec::new(1, 0, 0.0)],
            1,
            DEFAULT_BUDGET,
            Rc::clone(&policy_calls),
        );
        store.publish(&initial, &projector).expect("initial commit");
        let initial_slot = store.active_slots()[0].identity();

        events.borrow_mut().clear();
        projector.fail_kind.set(true);
        let kind_failure = committed(
            &mut coordinator,
            &[Spec::new(1, 0, 10.0)],
            2,
            DEFAULT_BUDGET,
            Rc::clone(&policy_calls),
        );
        assert!(matches!(
            store.publish(&kind_failure, &projector),
            Err(VirtualLayoutMaterializationError::Projection(_))
        ));
        projector.fail_kind.set(false);
        assert_eq!(store.active_slots()[0].identity(), initial_slot);
        assert!(events.borrow().is_empty());

        projector.fail_project.set(true);
        let project_failure = committed(
            &mut coordinator,
            &[Spec::new(1, 0, 20.0)],
            3,
            DEFAULT_BUDGET,
            Rc::clone(&policy_calls),
        );
        assert!(matches!(
            store.publish(&project_failure, &projector),
            Err(VirtualLayoutMaterializationError::Projection(_))
        ));
        projector.fail_project.set(false);
        assert_eq!(store.active_slots()[0].identity(), initial_slot);
        assert!(events.borrow().is_empty());

        let recovery = committed(
            &mut coordinator,
            &[Spec::new(1, 0, 30.0)],
            4,
            DEFAULT_BUDGET,
            Rc::clone(&policy_calls),
        );
        store
            .publish(&recovery, &projector)
            .expect("pre-callback failures must remain recoverable");
        assert!(store.is_mounted());
        assert_eq!(store.active_slots()[0].identity(), initial_slot);
        assert_eq!(store.active_slots()[0].item().bounds().min.y, 30.0);
        events.borrow_mut().clear();

        projector.mismatch_projected_kind.set(true);
        let kind_mismatch = committed(
            &mut coordinator,
            &[Spec::new(1, 0, 50.0)],
            5,
            DEFAULT_BUDGET,
            Rc::clone(&policy_calls),
        );
        assert!(matches!(
            store.publish(&kind_mismatch, &projector),
            Err(VirtualLayoutMaterializationError::ProjectionKindChanged)
        ));
        projector.mismatch_projected_kind.set(false);
        assert!(store.is_mounted());
        assert_eq!(store.active_slots()[0].identity(), initial_slot);
        assert_eq!(store.active_slots()[0].item().bounds().min.y, 30.0);
        events.borrow_mut().clear();

        let final_recovery = committed(
            &mut coordinator,
            &[Spec::new(1, 0, 60.0)],
            6,
            DEFAULT_BUDGET,
            Rc::clone(&policy_calls),
        );
        store
            .publish(&final_recovery, &projector)
            .expect("projection-kind failure must remain recoverable");
        assert!(store.is_mounted());
        assert_eq!(store.active_slots()[0].identity(), initial_slot);
        assert_eq!(store.active_slots()[0].item().bounds().min.y, 60.0);
        assert_eq!(event_kinds(&events.borrow()), vec![Callback::Reconcile]);
        events.borrow_mut().clear();

        let admission_recovery = committed(
            &mut coordinator,
            &[Spec::new(1, 0, 70.0)],
            7,
            DEFAULT_BUDGET,
            Rc::clone(&policy_calls),
        );
        let original_scope = store.scope.clone();
        store.scope.container_id = CONTAINER_ID + 1;
        assert!(matches!(
            store.publish(&admission_recovery, &projector),
            Err(VirtualLayoutMaterializationError::ForeignContainer)
        ));
        store.scope = original_scope;
        assert!(store.is_mounted());
        assert_eq!(store.active_slots()[0].item().bounds().min.y, 60.0);
        assert!(events.borrow().is_empty());
        store
            .publish(&admission_recovery, &projector)
            .expect("admission failure must remain recoverable");
        assert_eq!(store.active_slots()[0].item().bounds().min.y, 70.0);
        assert_eq!(event_kinds(&events.borrow()), vec![Callback::Reconcile]);
    }

    #[test]
    fn lifecycle_errors_after_observable_mutation_poison_without_replay() {
        for phase in [
            Callback::Unmount,
            Callback::Reset,
            Callback::Reconcile,
            Callback::Mount,
        ] {
            let (_coordinator, mut store, projector, events, commit) =
                lifecycle_fault_case(phase, LifecycleFault::Error);
            let result = store.publish(&commit, &projector);
            assert!(matches!(
                result,
                Err(VirtualLayoutMaterializationError::Lifecycle(_))
            ));
            let recorded = events.borrow().clone();
            assert!(!recorded.is_empty());
            assert_eq!(recorded.last().map(|event| event.callback), Some(phase));
            assert_lifecycle_indeterminate(&store);

            let projector_calls = projector.calls();
            store.lifecycle.fail_on.set(None);
            assert!(matches!(
                store.publish(&commit, &projector),
                Err(VirtualLayoutMaterializationError::LifecycleIndeterminate)
            ));
            assert!(matches!(
                store.unmount(),
                Err(VirtualLayoutMaterializationError::LifecycleIndeterminate)
            ));
            assert_eq!(*events.borrow(), recorded);
            assert_eq!(projector.calls(), projector_calls);
            assert!((0..store.diagnostics().sample_len()).any(|index| {
                store.diagnostics().sample(index)
                    == Some(VirtualLayoutMaterializationDiagnosticCode::LifecycleFailed)
            }));
        }
    }

    #[test]
    fn lifecycle_reentry_poison_applies_to_every_callback_phase() {
        for phase in [
            Callback::Unmount,
            Callback::Reset,
            Callback::Reconcile,
            Callback::Mount,
        ] {
            let (_coordinator, mut store, projector, events, commit) =
                lifecycle_fault_case(phase, LifecycleFault::Reentry);
            let result = store.publish(&commit, &projector);
            assert!(matches!(
                result,
                Err(VirtualLayoutMaterializationError::Reentrant)
            ));
            let recorded = events.borrow().clone();
            assert!(!recorded.is_empty());
            assert_eq!(recorded.last().map(|event| event.callback), Some(phase));
            assert_lifecycle_indeterminate(&store);

            store.lifecycle.reenter_on.set(None);
            assert!(matches!(
                store.publish(&commit, &projector),
                Err(VirtualLayoutMaterializationError::LifecycleIndeterminate)
            ));
            assert!(matches!(
                store.unmount(),
                Err(VirtualLayoutMaterializationError::LifecycleIndeterminate)
            ));
            assert_eq!(*events.borrow(), recorded);
        }
    }

    #[test]
    fn lifecycle_unwind_poison_survives_caught_panic() {
        let (mut coordinator, mut store, projector, events) = store_for(&[1, 2]);
        let policy_calls = Rc::new(Cell::new(0));
        let initial = committed(
            &mut coordinator,
            &[Spec::new(1, 0, 0.0), Spec::new(2, 1, 10.0)],
            1,
            DEFAULT_BUDGET,
            Rc::clone(&policy_calls),
        );
        store.publish(&initial, &projector).expect("initial commit");
        events.borrow_mut().clear();
        store.lifecycle.panic_on.set(Some(Callback::Unmount));

        let unwind = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = store.unmount();
        }));
        assert!(unwind.is_err());
        let recorded = events.borrow().clone();
        assert_eq!(event_kinds(&recorded), vec![Callback::Unmount]);
        assert_lifecycle_indeterminate(&store);

        store.lifecycle.panic_on.set(None);
        assert!(matches!(
            store.unmount(),
            Err(VirtualLayoutMaterializationError::LifecycleIndeterminate)
        ));
        assert_eq!(*events.borrow(), recorded);
        assert!((0..store.diagnostics().sample_len()).any(|index| {
            store.diagnostics().sample(index)
                == Some(VirtualLayoutMaterializationDiagnosticCode::LifecyclePanicked)
        }));
    }

    #[test]
    fn partial_container_unmount_failure_never_replays_later_callbacks() {
        let (mut coordinator, mut store, projector, events) = store_for(&[1, 2]);
        let policy_calls = Rc::new(Cell::new(0));
        let initial = committed(
            &mut coordinator,
            &[Spec::new(1, 0, 0.0), Spec::new(2, 1, 10.0)],
            1,
            DEFAULT_BUDGET,
            Rc::clone(&policy_calls),
        );
        store.publish(&initial, &projector).expect("initial commit");
        events.borrow_mut().clear();
        store.lifecycle.fail_on.set(Some(Callback::Unmount));

        assert!(matches!(
            store.unmount(),
            Err(VirtualLayoutMaterializationError::Lifecycle(_))
        ));
        let recorded = events.borrow().clone();
        assert_eq!(event_kinds(&recorded), vec![Callback::Unmount]);
        assert_eq!(recorded[0].key, Some(1));
        assert_lifecycle_indeterminate(&store);

        store.lifecycle.fail_on.set(None);
        assert!(matches!(
            store.unmount(),
            Err(VirtualLayoutMaterializationError::LifecycleIndeterminate)
        ));
        assert_eq!(*events.borrow(), recorded);
    }

    #[test]
    fn unstable_compatibility_capacity_and_overflow_fail_closed() {
        let (mut coordinator, mut store, projector, events) = store_for(&[1]);
        let policy_calls = Rc::new(Cell::new(0));
        let initial = committed(
            &mut coordinator,
            &[Spec::new(1, 0, 0.0)],
            1,
            DEFAULT_BUDGET,
            Rc::clone(&policy_calls),
        );
        store.publish(&initial, &projector).expect("initial commit");
        let stable_identity = store.active_slots()[0].identity();
        events.borrow_mut().clear();

        store
            .lifecycle
            .compatibility
            .set(CompatibilityMode::Unstable);
        let unstable = committed(
            &mut coordinator,
            &[Spec::new(1, 0, 10.0)],
            2,
            DEFAULT_BUDGET,
            Rc::clone(&policy_calls),
        );
        assert!(matches!(
            store.publish(&unstable, &projector),
            Err(VirtualLayoutMaterializationError::UnstableCompatibility)
        ));
        store.lifecycle.compatibility.set(CompatibilityMode::Stable);
        assert_eq!(store.active_slots()[0].identity(), stable_identity);
        assert!(events.borrow().is_empty());

        assert_eq!(store.active_slots()[0].identity(), stable_identity);
        assert!(events.borrow().is_empty());

        store.active[0].identity.checked_generation = u64::MAX;
        let overflow = committed(
            &mut coordinator,
            &empty_entries(),
            3,
            DEFAULT_BUDGET,
            Rc::clone(&policy_calls),
        );
        assert!(matches!(
            store.publish(&overflow, &projector),
            Err(VirtualLayoutMaterializationError::GenerationOverflow)
        ));
        assert_eq!(store.active_len(), 1);
        assert!(events.borrow().is_empty());

        let (mut slot_coordinator, mut slot_store, slot_projector, slot_events) = store_for(&[]);
        slot_store.next_slot_index = usize::MAX;
        let slot_overflow = committed(
            &mut slot_coordinator,
            &[Spec::new(5, 0, 0.0)],
            1,
            DEFAULT_BUDGET,
            Rc::new(Cell::new(0)),
        );
        assert!(matches!(
            slot_store.publish(&slot_overflow, &slot_projector),
            Err(VirtualLayoutMaterializationError::SlotArithmeticOverflow)
        ));
        assert_eq!(slot_store.active_len(), 0);
        assert!(slot_events.borrow().is_empty());

        let mut capacity_store = store;
        capacity_store.recyclable = (0..VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES)
            .map(|slot_index| RecyclableSlot {
                identity: VirtualLayoutSlotIdentity::new(
                    CONTAINER_ID,
                    MOUNT_GENERATION,
                    slot_index,
                ),
                last_logical_index: slot_index,
                payload: Payload {
                    logical_index: slot_index,
                    serial: slot_index,
                },
            })
            .collect();
        let capacity_commit = committed(
            &mut coordinator,
            &[Spec::new(2, 2, 20.0)],
            4,
            DEFAULT_BUDGET,
            Rc::clone(&policy_calls),
        );
        assert!(matches!(
            capacity_store.publish(&capacity_commit, &projector),
            Err(VirtualLayoutMaterializationError::CapacityViolation)
        ));
        assert_eq!(capacity_store.active_len(), 1);
    }

    #[test]
    fn unmount_releases_every_active_item_once_retires_store_and_rejects_old_commit() {
        let (mut coordinator, mut store, projector, events) = store_for(&[11, 22]);
        let policy_calls = Rc::new(Cell::new(0));
        let commit = committed(
            &mut coordinator,
            &[Spec::new(11, 0, 0.0), Spec::new(22, 1, 10.0)],
            1,
            DEFAULT_BUDGET,
            Rc::clone(&policy_calls),
        );
        store.publish(&commit, &projector).expect("initial commit");
        events.borrow_mut().clear();

        store.unmount().expect("unmount should retire the store");
        let first_unmount = events.borrow().clone();
        assert_eq!(
            event_kinds(&first_unmount),
            vec![Callback::Unmount, Callback::Unmount]
        );
        assert_eq!(first_unmount[0].key, Some(11));
        assert_eq!(first_unmount[1].key, Some(22));
        assert!(first_unmount.iter().all(|event| event.generation == 1));
        assert_eq!(store.active_len(), 0);
        assert_eq!(store.recyclable_len(), 0);
        assert!(!store.is_mounted());
        assert_eq!(store.authoritative_fence(), None);
        assert_eq!(store.authoritative_revision(), None);

        store.unmount().expect("retired unmount is idempotent");
        assert_eq!(*events.borrow(), first_unmount);
        assert!(matches!(
            store.publish(&commit, &projector),
            Err(VirtualLayoutMaterializationError::Unmounted)
        ));
        assert_eq!(*events.borrow(), first_unmount);
    }

    #[test]
    fn active_and_recyclable_slots_stay_within_hard_cap_and_active_keys_are_unique() {
        let keys: Vec<u32> = (0..VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES as u32).collect();
        let specs: Vec<Spec> = keys
            .iter()
            .copied()
            .enumerate()
            .map(|(index, key)| Spec::new(key, index, index as f32 * 10.0))
            .collect();
        let (mut coordinator, mut store, projector, events) = store_for(&keys);
        let policy_calls = Rc::new(Cell::new(0));
        let initial = committed(
            &mut coordinator,
            &specs,
            1,
            VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES,
            Rc::clone(&policy_calls),
        );
        store
            .publish(&initial, &projector)
            .expect("maximum bounded commit");
        assert_eq!(store.active_len(), VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES);
        assert_eq!(
            store.active_len() + store.recyclable_len(),
            VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES
        );
        events.borrow_mut().clear();

        let empty = committed(
            &mut coordinator,
            &empty_entries(),
            2,
            VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES,
            Rc::clone(&policy_calls),
        );
        store
            .publish(&empty, &projector)
            .expect("empty bounded commit");
        assert_eq!(store.active_len(), 0);
        assert_eq!(store.recyclable_len(), VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES);

        let replacement = committed(
            &mut coordinator,
            &[Spec::new(50_000, 0, 0.0)],
            3,
            VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES,
            Rc::clone(&policy_calls),
        );
        store
            .publish(&replacement, &projector)
            .expect("bounded recycle commit");
        assert_eq!(
            store.active_len() + store.recyclable_len(),
            VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES
        );
        let active = store.active_slots();
        assert_eq!(active.len(), 1);
        assert!(active[0].item().key() == &VirtualLayoutItemKey::new(50_000_u32));
    }
}
