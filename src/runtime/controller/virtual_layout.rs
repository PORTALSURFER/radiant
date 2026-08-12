//! Private synchronous virtual-layout registration and materialization bridge.
//!
//! This module is deliberately owned by `SurfaceRuntime`. It does not expose a
//! registration API, schedule policy work, or let a bridge/application object
//! retain materialized slots.

use super::{
    SurfaceRuntime, VirtualLayoutAutomationComposition,
    semantic_demand::{
        SemanticDemand, SemanticDemandAdmissionError, SemanticDemandCompletion,
        SemanticDemandExecutionError, SemanticDemandOwner, SemanticProviderCompletion,
        SemanticPublicationAuthorities, SemanticPublicationClassification,
        SemanticPublicationFallbackReason, SemanticPublicationOutcome,
    },
};
#[cfg(target_os = "macos")]
use crate::{
    application::virtual_layout::VirtualLayoutSemanticCardinality,
    gui::automation::{AutomationNodeId, AutomationNodeSnapshot},
    runtime::automation::NativeSemanticContainerSnapshot,
};
use crate::{
    gui::layout_core::{
        VirtualLayoutCompletion, VirtualLayoutLifecycleAdapter, VirtualLayoutMaterializationError,
        VirtualLayoutMaterializationReentry, VirtualLayoutMaterializationStore, VirtualLayoutPin,
        VirtualLayoutPinReason, VirtualLayoutProjectionEvidence, VirtualLayoutProjectionKind,
        VirtualLayoutRetainReason, VirtualLayoutSemanticProjection,
        VirtualLayoutSemanticProjectionAuthority, VirtualLayoutSemanticProjectionBatch,
        VirtualLayoutSemanticQueryOutcome, VirtualLayoutSemanticRange,
        VirtualLayoutSemanticRangeProviderOutcome, VirtualLayoutSemanticRangeQueryOutcome,
        VirtualLayoutSemanticRangeRequest, VirtualLayoutSemanticRejectedReason,
        VirtualLayoutSemanticRequest, VirtualLayoutSemanticUnavailableReason,
        VirtualLayoutSlotIdentity, VirtualLayoutWindowCoordinator,
    },
    gui::types::Rect,
    layout::{
        NodeId, VirtualLayoutBudget, VirtualLayoutCoordinateSpace, VirtualLayoutItemKey,
        VirtualLayoutPolicyIdentity, VirtualLayoutQueryFence, VirtualLayoutQueryInputParts,
    },
    runtime::{
        SurfaceNode, SurfaceTraversalIndex, UiSurface,
        automation::{
            SemanticAutomationContainerHandle, SemanticAutomationDemand,
            SemanticAutomationDemandError, SemanticAutomationFallbackReason,
            SemanticAutomationRefreshStatus, SemanticAutomationSessionError,
            SemanticAutomationSessionHandle,
        },
        surface::{
            MAX_VIRTUAL_LAYOUT_REGISTRATIONS, SourceTraversalIndex, VirtualLayoutRegistration,
        },
    },
};
use std::convert::Infallible;

#[cfg(target_os = "macos")]
fn count_automation_nodes_with_id(
    node: &AutomationNodeSnapshot,
    wanted: &AutomationNodeId,
) -> usize {
    usize::from(node.id == *wanted)
        + node
            .children
            .iter()
            .map(|child| count_automation_nodes_with_id(child, wanted))
            .sum::<usize>()
}

#[cfg(target_os = "macos")]
fn native_semantic_registration_is_admitted(
    coordinate_space: &VirtualLayoutCoordinateSpace,
    cardinality: Option<VirtualLayoutSemanticCardinality>,
    has_range_provider: bool,
) -> bool {
    coordinate_space == &VirtualLayoutCoordinateSpace::Logical
        && cardinality
            .is_some_and(|cardinality| cardinality.logical_item_count == 0 || has_range_provider)
}

#[derive(Default)]
struct RuntimeVirtualLayoutLifecycle;

impl<Message> VirtualLayoutLifecycleAdapter<SurfaceNode<Message>>
    for RuntimeVirtualLayoutLifecycle
{
    type Error = Infallible;

    fn compatible(
        &self,
        _previous: &VirtualLayoutProjectionKind,
        _next: &VirtualLayoutProjectionKind,
    ) -> Option<bool> {
        Some(true)
    }

    fn unmount(
        &mut self,
        _payload: &SurfaceNode<Message>,
        _evidence: VirtualLayoutProjectionEvidence<'_>,
        _reentry: &VirtualLayoutMaterializationReentry<'_>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn reset(
        &mut self,
        _payload: &SurfaceNode<Message>,
        _evidence: VirtualLayoutProjectionEvidence<'_>,
        _reentry: &VirtualLayoutMaterializationReentry<'_>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn reconcile(
        &mut self,
        _previous: &SurfaceNode<Message>,
        _next: &SurfaceNode<Message>,
        _evidence: VirtualLayoutProjectionEvidence<'_>,
        _reentry: &VirtualLayoutMaterializationReentry<'_>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn mount(
        &mut self,
        _recycled_shell: Option<&SurfaceNode<Message>>,
        _next: &SurfaceNode<Message>,
        _evidence: VirtualLayoutProjectionEvidence<'_>,
        _reentry: &VirtualLayoutMaterializationReentry<'_>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

type RuntimeMaterialization<Message> =
    VirtualLayoutMaterializationStore<SurfaceNode<Message>, RuntimeVirtualLayoutLifecycle>;

struct RuntimeVirtualLayoutSubtree<Message> {
    shell: SurfaceNode<Message>,
    items: Vec<SurfaceNode<Message>>,
    registration: VirtualLayoutRegistration<Message>,
}

impl<Message> Clone for RuntimeVirtualLayoutSubtree<Message> {
    fn clone(&self) -> Self {
        Self {
            shell: self.shell.clone(),
            items: self.items.clone(),
            registration: self.registration.clone(),
        }
    }
}

struct RuntimeVirtualLayoutCommittedBatch<Message> {
    query: VirtualLayoutQueryInputParts,
    subtree: RuntimeVirtualLayoutSubtree<Message>,
}

enum RuntimeVirtualLayoutMaterialization<Message> {
    Reused,
    Retained,
    Suppressed,
    Committed(Box<RuntimeVirtualLayoutCommittedBatch<Message>>),
    Retired,
}

/// Private origin evidence for one semantic projection classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VirtualLayoutSemanticClassificationOrigin {
    Materialized {
        slot_identity: VirtualLayoutSlotIdentity,
        payload_root: NodeId,
    },
    Unmaterialized,
}

/// One semantic projection paired with exact retained-materialization origin.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct VirtualLayoutSemanticClassification {
    projection: VirtualLayoutSemanticProjection,
    origin: VirtualLayoutSemanticClassificationOrigin,
}

#[allow(dead_code)]
impl VirtualLayoutSemanticClassification {
    pub(super) fn new(
        projection: VirtualLayoutSemanticProjection,
        origin: VirtualLayoutSemanticClassificationOrigin,
    ) -> Self {
        Self { projection, origin }
    }

    pub(crate) fn projection(&self) -> &VirtualLayoutSemanticProjection {
        &self.projection
    }

    pub(crate) const fn origin(&self) -> VirtualLayoutSemanticClassificationOrigin {
        self.origin
    }
}

/// Atomic ordered output of one exact semantic/materialization classification.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct VirtualLayoutSemanticClassificationBatch {
    request: VirtualLayoutSemanticRangeRequest,
    classifications: Vec<VirtualLayoutSemanticClassification>,
}

#[allow(dead_code)]
impl VirtualLayoutSemanticClassificationBatch {
    pub(super) fn new(
        request: VirtualLayoutSemanticRangeRequest,
        classifications: Vec<VirtualLayoutSemanticClassification>,
    ) -> Self {
        Self {
            request,
            classifications,
        }
    }

    pub(crate) fn request(&self) -> &VirtualLayoutSemanticRangeRequest {
        &self.request
    }

    pub(crate) fn classifications(&self) -> &[VirtualLayoutSemanticClassification] {
        &self.classifications
    }
}

/// First-class classification output for one semantic pin.  It intentionally
/// retains the one-item request rather than manufacturing a one-entry range.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct VirtualLayoutSemanticPinClassification {
    request: VirtualLayoutSemanticRequest,
    classification: VirtualLayoutSemanticClassification,
}

#[allow(dead_code)]
impl VirtualLayoutSemanticPinClassification {
    pub(super) fn new(
        request: VirtualLayoutSemanticRequest,
        classification: VirtualLayoutSemanticClassification,
    ) -> Self {
        Self {
            request,
            classification,
        }
    }

    pub(crate) fn request(&self) -> &VirtualLayoutSemanticRequest {
        &self.request
    }

    pub(crate) fn classification(&self) -> &VirtualLayoutSemanticClassification {
        &self.classification
    }
}

/// One normalized source of semantic classification evidence for the
/// compositor.  Range and pin inputs remain source-distinct until the
/// compositor's exact overlap coalescing pass.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum VirtualLayoutSemanticClassificationInput {
    Range(VirtualLayoutSemanticClassificationBatch),
    Pin(Box<VirtualLayoutSemanticPinClassification>),
}

/// Exact fence fields admitted by the private semantic/materialization bridge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VirtualLayoutSemanticClassificationFenceField {
    ContainerIdentity,
    PolicyIdentity,
    MountGeneration,
    DataRevision,
    PolicyRevision,
    MeasurementRevision,
    SemanticRevision,
    CoordinateSpace,
    Budget,
}

/// Conservative failure for one all-or-nothing classification attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VirtualLayoutSemanticClassificationError {
    UnknownContainer,
    Retired,
    MaterializationAuthorityUnavailable,
    MalformedBatch,
    UnstablePolicyIdentity,
    UnstableCoordinateSpace,
    FenceMismatch(VirtualLayoutSemanticClassificationFenceField),
    UnstableKey,
    AmbiguousMaterialization,
    KeyIndexMismatch,
    UnmatchedMaterializedSlot,
}

struct RuntimeVirtualLayoutRecord<Message> {
    registration: VirtualLayoutRegistration<Message>,
    mount_generation: u64,
    coordinator: VirtualLayoutWindowCoordinator,
    materialization: RuntimeMaterialization<Message>,
    last_query: Option<VirtualLayoutQueryInputParts>,
    last_required_key: Option<VirtualLayoutItemKey>,
    pin: Option<VirtualLayoutPin>,
    cached_subtree: Option<RuntimeVirtualLayoutSubtree<Message>>,
    retired: bool,
}

pub(super) struct RuntimeVirtualLayoutProjectionProbe<Message> {
    pub(super) traversal: SurfaceTraversalIndex<Message>,
    pub(super) source: SourceTraversalIndex,
}

impl<Message> RuntimeVirtualLayoutRecord<Message> {
    fn new(registration: VirtualLayoutRegistration<Message>, mount_generation: u64) -> Self {
        let coordinator = VirtualLayoutWindowCoordinator::new(
            registration.container_id,
            registration.policy_identity.clone(),
            mount_generation,
        );
        let materialization = RuntimeMaterialization::new(&coordinator, Default::default());
        Self {
            registration,
            mount_generation,
            coordinator,
            materialization,
            last_query: None,
            last_required_key: None,
            pin: None,
            cached_subtree: None,
            retired: false,
        }
    }

    fn update_registration(&mut self, registration: VirtualLayoutRegistration<Message>) {
        if self.registration.data_revision() != registration.data_revision()
            || self.registration.policy_revision() != registration.policy_revision()
            || self.registration.measurement_revision() != registration.measurement_revision()
            || self.registration.semantic_revision() != registration.semantic_revision()
            || self.registration.semantic_cardinality != registration.semantic_cardinality
            || self.registration.coordinate_space != registration.coordinate_space
            || !self.registration.semantic_provider_is_same(&registration)
            || !same_optional_key(
                self.registration.required_key(),
                registration.required_key(),
            )
        {
            self.pin = None;
        }
        self.registration = registration;
        if let Some(cached) = &mut self.cached_subtree {
            cached.registration = self.registration.clone();
        }
    }

    fn needs_query(
        &self,
        parts: &VirtualLayoutQueryInputParts,
        required_key: Option<&VirtualLayoutItemKey>,
    ) -> bool {
        let Some(previous) = &self.last_query else {
            return true;
        };
        previous.container_id != parts.container_id
            || previous.policy_identity != parts.policy_identity
            || previous.mount_generation != parts.mount_generation
            || previous.viewport != parts.viewport
            || previous.coordinate_space != parts.coordinate_space
            || previous.overscan != parts.overscan
            || previous.budget != parts.budget
            || previous.viewport_revision != parts.viewport_revision
            || previous.data_revision != parts.data_revision
            || previous.policy_revision != parts.policy_revision
            || previous.measurement_revision != parts.measurement_revision
            || previous.semantic_revision != parts.semantic_revision
            || !same_optional_key(self.last_required_key.as_ref(), required_key)
    }

    fn needs_query_for_viewport(&self, viewport: Rect) -> bool {
        self.needs_query(
            &self
                .registration
                .query_parts(viewport, self.mount_generation),
            self.registration.required_key(),
        )
    }

    fn materialize(&mut self, viewport: Rect) -> RuntimeVirtualLayoutMaterialization<Message> {
        if self.retired {
            return RuntimeVirtualLayoutMaterialization::Retired;
        }
        let parts = self
            .registration
            .query_parts(viewport, self.mount_generation);
        if !self.needs_query(&parts, self.registration.required_key()) {
            return RuntimeVirtualLayoutMaterialization::Reused;
        }
        let pending = match self
            .coordinator
            .begin_query_with_required_key(parts.clone(), self.registration.required_key().cloned())
        {
            Ok(pending) => pending,
            Err(_) => {
                self.retire();
                return RuntimeVirtualLayoutMaterialization::Retired;
            }
        };
        let outcome = pending.execute(&*self.registration.policy);
        let completion = self.coordinator.complete(pending, outcome);
        let fallback_authorized = self.fallback_authorizes_cached_window(&completion);
        match completion {
            VirtualLayoutCompletion::Committed(commit) => {
                let projector = self.registration.projector();
                match self.materialization.publish(&commit, &projector) {
                    Ok(()) => {
                        let Some(shell) = projector.take_shell() else {
                            self.retire();
                            return RuntimeVirtualLayoutMaterialization::Retired;
                        };
                        let items = self.active_payloads();
                        RuntimeVirtualLayoutMaterialization::Committed(Box::new(
                            RuntimeVirtualLayoutCommittedBatch {
                                query: parts,
                                subtree: RuntimeVirtualLayoutSubtree {
                                    shell,
                                    items,
                                    registration: self.registration.clone(),
                                },
                            },
                        ))
                    }
                    Err(
                        VirtualLayoutMaterializationError::Lifecycle(_)
                        | VirtualLayoutMaterializationError::Reentrant
                        | VirtualLayoutMaterializationError::LifecycleIndeterminate
                        | VirtualLayoutMaterializationError::ForeignContainer
                        | VirtualLayoutMaterializationError::ForeignPolicy
                        | VirtualLayoutMaterializationError::ForeignMount
                        | VirtualLayoutMaterializationError::ForeignOwner
                        | VirtualLayoutMaterializationError::UnstablePolicyIdentity
                        | VirtualLayoutMaterializationError::Unmounted
                        | VirtualLayoutMaterializationError::InvalidCommit
                        | VirtualLayoutMaterializationError::CapacityViolation
                        | VirtualLayoutMaterializationError::DuplicateKey
                        | VirtualLayoutMaterializationError::UnstableKey
                        | VirtualLayoutMaterializationError::DuplicateLogicalIndex
                        | VirtualLayoutMaterializationError::OlderRevision
                        | VirtualLayoutMaterializationError::DuplicateRevision
                        | VirtualLayoutMaterializationError::OlderFence
                        | VirtualLayoutMaterializationError::SlotArithmeticOverflow
                        | VirtualLayoutMaterializationError::GenerationOverflow
                        | VirtualLayoutMaterializationError::UnstableCompatibility
                        | VirtualLayoutMaterializationError::Projection(_)
                        | VirtualLayoutMaterializationError::ProjectionKindChanged,
                    ) => {
                        self.retire();
                        RuntimeVirtualLayoutMaterialization::Retired
                    }
                }
            }
            VirtualLayoutCompletion::Retained { reason, .. } => match reason {
                VirtualLayoutRetainReason::Pending
                | VirtualLayoutRetainReason::Deferred(_)
                | VirtualLayoutRetainReason::Unavailable(_) => {
                    if fallback_authorized {
                        RuntimeVirtualLayoutMaterialization::Retained
                    } else {
                        RuntimeVirtualLayoutMaterialization::Suppressed
                    }
                }
                VirtualLayoutRetainReason::Invalid => {
                    self.retire();
                    RuntimeVirtualLayoutMaterialization::Retired
                }
            },
            VirtualLayoutCompletion::Stale(_) | VirtualLayoutCompletion::Rejected(_) => {
                self.retire();
                RuntimeVirtualLayoutMaterialization::Retired
            }
        }
    }

    fn fallback_authorizes_cached_window(&self, completion: &VirtualLayoutCompletion) -> bool {
        let VirtualLayoutCompletion::Retained { view, .. } = completion else {
            return false;
        };
        if !view.fallback || view.extent.is_none() {
            return false;
        }
        let Some(accepted_revision) = view.accepted_revision else {
            return false;
        };
        if self.materialization.authoritative_revision() != Some(accepted_revision) {
            return false;
        }
        let Some(cached) = &self.cached_subtree else {
            return false;
        };
        let active_slots = self.materialization.active_slots();
        if active_slots.len() != cached.items.len() {
            return false;
        }

        let mut active_items: Vec<_> = active_slots
            .into_iter()
            .map(|slot| slot.item().clone())
            .collect();
        let mut fallback_items = view.entries.clone();
        active_items.sort_by_key(|item| item.logical_index());
        fallback_items.sort_by_key(|item| item.logical_index());
        active_items == fallback_items
    }

    fn active_payloads(&self) -> Vec<SurfaceNode<Message>> {
        self.materialization
            .active_slots()
            .into_iter()
            .map(|slot| slot.payload().clone())
            .collect()
    }

    fn commit_batch(&mut self, batch: RuntimeVirtualLayoutCommittedBatch<Message>) {
        self.last_query = Some(batch.query);
        self.last_required_key = batch.subtree.registration.required_key().cloned();
        self.cached_subtree = Some(batch.subtree);
    }

    fn retire(&mut self) {
        self.pin = None;
        if !self.retired {
            self.retired = true;
            let _ = self.materialization.unmount();
        }
        self.cached_subtree = None;
        self.last_required_key = None;
    }

    fn cached_subtree_matches_required_key(&self) -> bool {
        self.cached_subtree.is_some()
            && same_optional_key(
                self.last_required_key.as_ref(),
                self.registration.required_key(),
            )
    }

    fn project_current_semantics(&self) -> Option<VirtualLayoutSemanticProjection> {
        if self.retired {
            return None;
        }
        let pin = self.pin.as_ref()?;
        if pin.reason() != VirtualLayoutPinReason::Semantic
            || pin
                .request()
                .validate_scope(
                    self.registration.container_id,
                    &self.registration.policy_identity,
                    self.mount_generation,
                    self.registration.data_revision(),
                    self.registration.policy_revision(),
                    self.registration.measurement_revision(),
                    self.registration.semantic_revision(),
                )
                .is_err()
        {
            return None;
        }

        VirtualLayoutSemanticProjection::from_validated_semantic_pin(
            pin,
            self.registration.coordinate_space.clone(),
        )
    }

    fn classify_semantic_range(
        &self,
        batch: &VirtualLayoutSemanticProjectionBatch,
    ) -> Result<VirtualLayoutSemanticClassificationBatch, VirtualLayoutSemanticClassificationError>
    {
        if self.retired {
            return Err(VirtualLayoutSemanticClassificationError::Retired);
        }
        validate_semantic_classification_batch(batch)?;
        validate_semantic_request_scope(
            batch.request(),
            self.registration.container_id,
            &self.registration.policy_identity,
            self.mount_generation,
            self.registration.data_revision(),
            self.registration.policy_revision(),
            self.registration.measurement_revision(),
            self.registration.semantic_revision(),
            &self.registration.coordinate_space,
            self.registration.budget,
        )?;
        let Some(authoritative_fence) = self.materialization.authoritative_fence() else {
            return Err(
                VirtualLayoutSemanticClassificationError::MaterializationAuthorityUnavailable,
            );
        };
        validate_semantic_materialization_fence(batch.request(), authoritative_fence)?;

        let active_len = self.materialization.active_len();
        if active_len > batch.request().budget().max_entries()
            || active_len > crate::layout::VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES
        {
            return Err(VirtualLayoutSemanticClassificationError::MalformedBatch);
        }
        let active_slots = self.materialization.active_slots();
        let mut matched_projection = vec![None; batch.projections().len()];
        let mut matched_active = vec![None; active_slots.len()];

        for (projection_index, projection) in batch.projections().iter().enumerate() {
            for (active_index, slot) in active_slots.iter().enumerate() {
                match slot.item().key().stable_equals(projection.identity().key()) {
                    Some(true) => {
                        if slot.item().logical_index() != projection.logical_index() {
                            return Err(VirtualLayoutSemanticClassificationError::KeyIndexMismatch);
                        }
                        if matched_projection[projection_index]
                            .replace(active_index)
                            .is_some()
                            || matched_active[active_index]
                                .replace(projection_index)
                                .is_some()
                        {
                            return Err(
                                VirtualLayoutSemanticClassificationError::AmbiguousMaterialization,
                            );
                        }
                    }
                    Some(false) => {}
                    None => {
                        return Err(VirtualLayoutSemanticClassificationError::UnstableKey);
                    }
                }
            }
        }

        for (active_index, slot) in active_slots.iter().enumerate() {
            if batch
                .request()
                .range()
                .contains(slot.item().logical_index())
                && matched_active[active_index].is_none()
            {
                return Err(VirtualLayoutSemanticClassificationError::UnmatchedMaterializedSlot);
            }
        }

        let classifications = batch
            .projections()
            .iter()
            .enumerate()
            .map(|(projection_index, projection)| {
                let origin = matched_projection[projection_index]
                    .map(|active_index| {
                        let slot = &active_slots[active_index];
                        VirtualLayoutSemanticClassificationOrigin::Materialized {
                            slot_identity: slot.identity(),
                            payload_root: slot.payload().id(),
                        }
                    })
                    .unwrap_or(VirtualLayoutSemanticClassificationOrigin::Unmaterialized);
                VirtualLayoutSemanticClassification {
                    projection: projection.clone(),
                    origin,
                }
            })
            .collect();

        Ok(VirtualLayoutSemanticClassificationBatch {
            request: batch.request().clone(),
            classifications,
        })
    }

    #[allow(dead_code)]
    fn classify_semantic_pin(
        &self,
        request: &VirtualLayoutSemanticRequest,
        projection: &VirtualLayoutSemanticProjection,
    ) -> Result<VirtualLayoutSemanticPinClassification, VirtualLayoutSemanticClassificationError>
    {
        if self.retired {
            return Err(VirtualLayoutSemanticClassificationError::Retired);
        }
        validate_semantic_pin_classification(
            request,
            projection,
            self.registration.container_id,
            &self.registration.policy_identity,
            self.mount_generation,
            self.registration.data_revision(),
            self.registration.policy_revision(),
            self.registration.measurement_revision(),
            self.registration.semantic_revision(),
            &self.registration.coordinate_space,
        )?;
        let Some(authoritative_fence) = self.materialization.authoritative_fence() else {
            return Err(
                VirtualLayoutSemanticClassificationError::MaterializationAuthorityUnavailable,
            );
        };
        if authoritative_fence.container_id() != self.registration.container_id
            || authoritative_fence.mount_generation() != self.mount_generation
            || authoritative_fence.data_revision() != self.registration.data_revision()
            || authoritative_fence.policy_revision() != self.registration.policy_revision()
            || authoritative_fence.measurement_revision()
                != self.registration.measurement_revision()
            || authoritative_fence.semantic_revision() != self.registration.semantic_revision()
            || authoritative_fence.budget() != self.registration.budget
            || authoritative_fence
                .policy_identity()
                .stable_equals(&self.registration.policy_identity)
                != Some(true)
            || stable_coordinate_space_equals(
                authoritative_fence.coordinate_space(),
                &self.registration.coordinate_space,
            ) != Some(true)
        {
            return Err(VirtualLayoutSemanticClassificationError::FenceMismatch(
                VirtualLayoutSemanticClassificationFenceField::SemanticRevision,
            ));
        }

        let mut matched_active = None;
        for (active_index, slot) in self.materialization.active_slots().iter().enumerate() {
            match slot.item().key().stable_equals(projection.identity().key()) {
                Some(true) => {
                    if slot.item().logical_index() != projection.logical_index() {
                        return Err(VirtualLayoutSemanticClassificationError::KeyIndexMismatch);
                    }
                    if matched_active.replace(active_index).is_some() {
                        return Err(
                            VirtualLayoutSemanticClassificationError::AmbiguousMaterialization,
                        );
                    }
                }
                Some(false) => {}
                None => return Err(VirtualLayoutSemanticClassificationError::UnstableKey),
            }
        }

        let origin = matched_active
            .map(|active_index| {
                let slot = &self.materialization.active_slots()[active_index];
                VirtualLayoutSemanticClassificationOrigin::Materialized {
                    slot_identity: slot.identity(),
                    payload_root: slot.payload().id(),
                }
            })
            .unwrap_or(VirtualLayoutSemanticClassificationOrigin::Unmaterialized);
        Ok(VirtualLayoutSemanticPinClassification::new(
            request.clone(),
            VirtualLayoutSemanticClassification::new(projection.clone(), origin),
        ))
    }
}

impl<Message> Drop for RuntimeVirtualLayoutRecord<Message> {
    fn drop(&mut self) {
        self.retire();
    }
}

/// Runtime-owned bounded registry of mounted virtual-layout records.
pub(in crate::runtime) struct RuntimeVirtualLayoutState<Message> {
    records: Vec<RuntimeVirtualLayoutRecord<Message>>,
    next_mount_generation: u64,
    projection_probe: Option<RuntimeVirtualLayoutProjectionProbe<Message>>,
    semantic_demand: SemanticDemandOwner<Message>,
    semantic_session: Option<RuntimeSemanticAutomationSession>,
    next_semantic_session_generation: u64,
    #[cfg(test)]
    materialization_passes: u32,
}

struct RuntimeSemanticAutomationSession {
    handle: SemanticAutomationSessionHandle,
    selection: Option<RuntimeSemanticAutomationSelection>,
}

struct RuntimeSemanticAutomationSelection {
    composition: VirtualLayoutAutomationComposition,
    ordinary: crate::gui::automation::GuiAutomationSnapshot,
    runtime_projection_generation: u64,
    status: SemanticAutomationRefreshStatus,
}

pub(crate) struct RuntimeSemanticAutomationPublication {
    pub(crate) composition: VirtualLayoutAutomationComposition,
    pub(crate) status: SemanticAutomationRefreshStatus,
}

impl<Message> Default for RuntimeVirtualLayoutState<Message> {
    fn default() -> Self {
        Self {
            records: Vec::new(),
            next_mount_generation: 0,
            projection_probe: None,
            semantic_demand: SemanticDemandOwner::default(),
            semantic_session: None,
            next_semantic_session_generation: 0,
            #[cfg(test)]
            materialization_passes: 0,
        }
    }
}

impl<Message> RuntimeVirtualLayoutState<Message> {
    #[cfg(target_os = "macos")]
    /// Return only current logical registrations with one unambiguous ordinary
    /// automation anchor and an exact cardinality.  This is a passive view:
    /// cloning a provider capability is not a provider call and does not
    /// create semantic demand.
    pub(crate) fn native_semantic_containers(
        &self,
        ordinary: &crate::gui::automation::GuiAutomationSnapshot,
    ) -> Vec<NativeSemanticContainerSnapshot> {
        let mut accepted = Vec::new();
        for record in self.records.iter().filter(|record| !record.retired) {
            let Some(cardinality) = record.registration.semantic_cardinality else {
                continue;
            };
            let has_range_provider = record
                .registration
                .semantic_range_provider_handle()
                .is_some();
            if !native_semantic_registration_is_admitted(
                &record.registration.coordinate_space,
                Some(cardinality),
                has_range_provider,
            ) {
                continue;
            }
            let anchor = AutomationNodeId::new(record.registration.container_id.to_string());
            if count_automation_nodes_with_id(&ordinary.root, &anchor) != 1 {
                continue;
            }
            let Some((registration_generation, provider_generation)) = self
                .semantic_demand
                .native_range_authority(record.registration.container_id)
            else {
                continue;
            };
            accepted.push(NativeSemanticContainerSnapshot {
                container_id: record.registration.container_id,
                mount_generation: record.mount_generation,
                registration_generation,
                provider_generation,
                cardinality,
                has_range_provider,
                max_entries: record.registration.budget.max_entries(),
            });
            if accepted.len() == MAX_VIRTUAL_LAYOUT_REGISTRATIONS {
                break;
            }
        }
        accepted
    }

    fn synchronize_semantic_demand(&mut self) {
        let semantic_registrations = self
            .records
            .iter()
            .filter(|record| !record.retired)
            .map(|record| (record.registration.clone(), record.mount_generation))
            .collect::<Vec<_>>();
        let (authority_changed, _) = self
            .semantic_demand
            .synchronize_with_change(&semantic_registrations);
        if authority_changed && let Some(session) = &mut self.semantic_session {
            // A real lifecycle/authority synchronization is an invalidation
            // boundary for a previously selected publication.  Rejected
            // demand updates leave the owner unchanged and retain it.
            session.selection = None;
        }
    }

    pub(super) fn prepare_surface(
        &mut self,
        surface: &mut UiSurface<Message>,
        registrations: &[VirtualLayoutRegistration<Message>],
    ) {
        self.clear_projection_probe_if_empty();
        if registrations.len() > MAX_VIRTUAL_LAYOUT_REGISTRATIONS {
            self.retire_all();
            return;
        }
        let mut duplicate_containers = Vec::new();
        for (index, registration) in registrations.iter().enumerate() {
            if registrations[..index]
                .iter()
                .any(|previous| previous.container_id == registration.container_id)
                && !duplicate_containers.contains(&registration.container_id)
            {
                duplicate_containers.push(registration.container_id);
            }
        }
        let accepted: Vec<_> = registrations
            .iter()
            .filter(|registration| !duplicate_containers.contains(&registration.container_id))
            .cloned()
            .collect();

        let mut index = 0;
        while index < self.records.len() {
            if !accepted.iter().any(|registration| {
                registration.container_id == self.records[index].registration.container_id
            }) {
                self.records.remove(index).retire();
            } else {
                index += 1;
            }
        }

        for registration in accepted {
            let Some(existing_index) = self
                .records
                .iter()
                .position(|record| record.registration.container_id == registration.container_id)
            else {
                let Some(generation) = self.allocate_generation() else {
                    continue;
                };
                self.records
                    .push(RuntimeVirtualLayoutRecord::new(registration, generation));
                continue;
            };
            if self.records[existing_index]
                .registration
                .same_scope(&registration)
            {
                self.records[existing_index].update_registration(registration);
            } else {
                self.records[existing_index].retire();
                let Some(generation) = self.allocate_generation() else {
                    continue;
                };
                self.records[existing_index] =
                    RuntimeVirtualLayoutRecord::new(registration, generation);
            }
        }

        for index in 0..self.records.len() {
            if self.records[index].retired {
                continue;
            }
            let container_id = self.records[index].registration.container_id;
            if self.records[index].cached_subtree_matches_required_key()
                && let Some(cached) = self.records[index].cached_subtree.as_ref()
            {
                let installed = surface.install_virtual_layout_subtree(
                    container_id,
                    &cached.shell,
                    &cached.registration,
                    &cached.items,
                );
                if !installed {
                    // The pulled surface no longer admits this mounted record.
                    // Retire it without attempting to lower or retry the old
                    // retained payloads.
                    self.records[index].retire();
                }
                continue;
            }
            let Some(shell) = self.records[index].registration.lowered_shell() else {
                self.records[index].retire();
                continue;
            };
            if !surface.replace_virtual_layout_shell(
                container_id,
                shell,
                self.records[index].registration.clone(),
            ) {
                self.records[index].retire();
            }
        }

        // Synchronize only after the existing registration, duplicate, mount,
        // and shell-admission decisions have produced the accepted live set.
        // This hook creates no demand for a new capability registration and
        // does not execute or publish semantic evidence.
        self.synchronize_semantic_demand();
        self.clear_projection_probe_if_empty();
    }

    pub(super) fn requires_materialization(
        &self,
        layout: &crate::layout::LayoutOutput,
        force_pass: bool,
    ) -> bool {
        self.records.iter().any(|record| {
            if record.retired {
                return false;
            }
            if force_pass || record.cached_subtree.is_none() {
                return true;
            }
            let Some(viewport) = layout
                .viewport_bounds
                .get(&record.registration.container_id)
                .copied()
            else {
                return true;
            };
            record.needs_query_for_viewport(viewport)
        })
    }

    pub(super) fn materialize_surface(
        &mut self,
        surface: &mut UiSurface<Message>,
        layout: &crate::layout::LayoutOutput,
    ) {
        #[cfg(test)]
        {
            self.materialization_passes = self.materialization_passes.saturating_add(1);
        }
        for index in 0..self.records.len() {
            if self.records[index].retired {
                continue;
            }
            let previous_subtree = self.records[index].cached_subtree.clone();
            let Some(viewport) = layout
                .viewport_bounds
                .get(&self.records[index].registration.container_id)
                .copied()
            else {
                self.records[index].retire();
                suppress_cached_virtual_layout_subtree(
                    surface,
                    self.records[index].registration.container_id,
                    previous_subtree,
                );
                continue;
            };
            match self.records[index].materialize(viewport) {
                RuntimeVirtualLayoutMaterialization::Reused
                | RuntimeVirtualLayoutMaterialization::Retained => {}
                RuntimeVirtualLayoutMaterialization::Suppressed => {
                    suppress_cached_virtual_layout_subtree(
                        surface,
                        self.records[index].registration.container_id,
                        previous_subtree,
                    );
                }
                RuntimeVirtualLayoutMaterialization::Retired => {
                    suppress_cached_virtual_layout_subtree(
                        surface,
                        self.records[index].registration.container_id,
                        previous_subtree,
                    );
                }
                RuntimeVirtualLayoutMaterialization::Committed(batch) => {
                    let container_id = self.records[index].registration.container_id;
                    let installed = surface.install_virtual_layout_subtree(
                        container_id,
                        &batch.subtree.shell,
                        &batch.subtree.registration,
                        &batch.subtree.items,
                    );
                    if installed {
                        self.records[index].commit_batch(*batch);
                    } else {
                        self.records[index].retire();
                        suppress_cached_virtual_layout_subtree(
                            surface,
                            container_id,
                            previous_subtree,
                        );
                    }
                }
            }
        }
        self.synchronize_semantic_demand();
        self.clear_projection_probe_if_empty();
    }

    pub(super) fn retire_all(&mut self) {
        self.semantic_demand.retire_all();
        for record in &mut self.records {
            record.retire();
        }
        self.records.clear();
        self.projection_probe = None;
        self.semantic_session = None;
    }

    /// Query one bounded pin without entering any materialization or
    /// presentation path. The request fence is checked before the provider is
    /// allowed to observe it, and a mounted record retains at most one entry.
    #[allow(dead_code)]
    pub(crate) fn query_pin(
        &mut self,
        request: &VirtualLayoutSemanticRequest,
        reason: VirtualLayoutPinReason,
    ) -> VirtualLayoutSemanticQueryOutcome {
        let Some(index) = self
            .records
            .iter()
            .position(|record| record.registration.container_id == request.container_id())
        else {
            return VirtualLayoutSemanticQueryOutcome::Rejected(
                VirtualLayoutSemanticRejectedReason::UnknownContainer,
            );
        };
        let record = &mut self.records[index];
        if record.retired {
            record.pin = None;
            return VirtualLayoutSemanticQueryOutcome::Rejected(
                VirtualLayoutSemanticRejectedReason::Retired,
            );
        }
        if let Err(reason) = request.validate_scope(
            record.registration.container_id,
            &record.registration.policy_identity,
            record.mount_generation,
            record.registration.data_revision(),
            record.registration.policy_revision(),
            record.registration.measurement_revision(),
            record.registration.semantic_revision(),
        ) {
            record.pin = None;
            return VirtualLayoutSemanticQueryOutcome::Rejected(reason);
        }
        if request.key().stable_equals(request.key()) != Some(true) {
            record.pin = None;
            return VirtualLayoutSemanticQueryOutcome::Rejected(
                VirtualLayoutSemanticRejectedReason::UnstableKey,
            );
        }
        let Some(provider) = record.registration.semantic_provider() else {
            record.pin = None;
            return VirtualLayoutSemanticQueryOutcome::Unavailable(
                VirtualLayoutSemanticUnavailableReason::NoProvider,
            );
        };
        let outcome = provider.lookup(request);
        match outcome {
            VirtualLayoutSemanticQueryOutcome::Found(entry) => {
                if let Err(reason) = entry.validate_for(request) {
                    record.pin = None;
                    return VirtualLayoutSemanticQueryOutcome::Rejected(reason);
                }
                if let Err(reason) =
                    validate_semantic_entry_against_pin(record.pin.as_ref(), request, &entry)
                {
                    return VirtualLayoutSemanticQueryOutcome::Rejected(reason);
                }
                record.pin = Some(VirtualLayoutPin::new(
                    reason,
                    request.clone(),
                    entry.as_ref().clone(),
                ));
                VirtualLayoutSemanticQueryOutcome::Found(entry)
            }
            VirtualLayoutSemanticQueryOutcome::NotFound => {
                record.pin = None;
                VirtualLayoutSemanticQueryOutcome::NotFound
            }
            VirtualLayoutSemanticQueryOutcome::Deferred(reason) => {
                record.pin = None;
                VirtualLayoutSemanticQueryOutcome::Deferred(reason)
            }
            VirtualLayoutSemanticQueryOutcome::Unavailable(reason) => {
                record.pin = None;
                VirtualLayoutSemanticQueryOutcome::Unavailable(reason)
            }
            VirtualLayoutSemanticQueryOutcome::Rejected(reason) => {
                record.pin = None;
                VirtualLayoutSemanticQueryOutcome::Rejected(reason)
            }
        }
    }

    /// Query one semantic entry through the bounded semantic pin owner.
    #[allow(dead_code)]
    pub(crate) fn query_semantics(
        &mut self,
        request: &VirtualLayoutSemanticRequest,
    ) -> VirtualLayoutSemanticQueryOutcome {
        self.query_pin(request, VirtualLayoutPinReason::Semantic)
    }

    /// Query one exact semantic range without changing the single-item pin.
    /// Provider output is validated atomically against the live registration
    /// fence before any private projection is constructed.
    #[allow(dead_code)]
    pub(crate) fn query_semantic_range(
        &mut self,
        request: &VirtualLayoutSemanticRangeRequest,
    ) -> VirtualLayoutSemanticRangeQueryOutcome {
        let Some(index) = self
            .records
            .iter()
            .position(|record| record.registration.container_id == request.container_id())
        else {
            return VirtualLayoutSemanticRangeQueryOutcome::Rejected(
                VirtualLayoutSemanticRejectedReason::UnknownContainer,
            );
        };
        let record = &mut self.records[index];
        if record.retired {
            return VirtualLayoutSemanticRangeQueryOutcome::Rejected(
                VirtualLayoutSemanticRejectedReason::Retired,
            );
        }
        if let Err(reason) = request.validate_scope(
            record.registration.container_id,
            &record.registration.policy_identity,
            record.mount_generation,
            record.registration.data_revision(),
            record.registration.policy_revision(),
            record.registration.measurement_revision(),
            record.registration.semantic_revision(),
            &record.registration.coordinate_space,
            record.registration.budget,
        ) {
            return VirtualLayoutSemanticRangeQueryOutcome::Rejected(reason);
        }
        if let Err(reason) = request.range().validate_budget(record.registration.budget) {
            return VirtualLayoutSemanticRangeQueryOutcome::Rejected(reason);
        }
        let Some(provider) = record.registration.semantic_range_provider() else {
            return VirtualLayoutSemanticRangeQueryOutcome::Unavailable(
                VirtualLayoutSemanticUnavailableReason::NoProvider,
            );
        };
        let outcome = provider.lookup_range(request);

        // Recheck every fence field after the provider returns. The range is
        // private evidence only, so a stale result is rejected as a whole.
        if let Err(reason) = request.validate_scope(
            record.registration.container_id,
            &record.registration.policy_identity,
            record.mount_generation,
            record.registration.data_revision(),
            record.registration.policy_revision(),
            record.registration.measurement_revision(),
            record.registration.semantic_revision(),
            &record.registration.coordinate_space,
            record.registration.budget,
        ) {
            return VirtualLayoutSemanticRangeQueryOutcome::Rejected(reason);
        }

        match outcome {
            VirtualLayoutSemanticRangeProviderOutcome::Found(entries) => {
                if let Err(reason) =
                    validate_semantic_range_entries(request, &entries, record.pin.as_ref())
                {
                    return VirtualLayoutSemanticRangeQueryOutcome::Rejected(reason);
                }
                let projections = entries
                    .iter()
                    .map(|entry| {
                        VirtualLayoutSemanticProjection::from_validated_semantic_range_entry(
                            request,
                            entry,
                            record.registration.coordinate_space.clone(),
                        )
                    })
                    .collect::<Option<Vec<_>>>();
                let Some(projections) = projections else {
                    return VirtualLayoutSemanticRangeQueryOutcome::Rejected(
                        VirtualLayoutSemanticRejectedReason::WrongLogicalIndex,
                    );
                };
                VirtualLayoutSemanticRangeQueryOutcome::Found(
                    VirtualLayoutSemanticProjectionBatch::new(request.clone(), projections),
                )
            }
            VirtualLayoutSemanticRangeProviderOutcome::NotFound => {
                VirtualLayoutSemanticRangeQueryOutcome::NotFound
            }
            VirtualLayoutSemanticRangeProviderOutcome::Unavailable(reason) => {
                VirtualLayoutSemanticRangeQueryOutcome::Unavailable(reason)
            }
            VirtualLayoutSemanticRangeProviderOutcome::Deferred(reason) => {
                VirtualLayoutSemanticRangeQueryOutcome::Deferred(reason)
            }
            VirtualLayoutSemanticRangeProviderOutcome::Rejected(reason) => {
                VirtualLayoutSemanticRangeQueryOutcome::Rejected(reason)
            }
        }
    }

    /// Project one already-valid semantic pin without materializing or
    /// refreshing the runtime tree.
    #[allow(dead_code)]
    pub(crate) fn project_current_semantics(
        &self,
        container_id: crate::layout::NodeId,
    ) -> Option<VirtualLayoutSemanticProjection> {
        self.records
            .iter()
            .find(|record| record.registration.container_id == container_id)
            .and_then(RuntimeVirtualLayoutRecord::project_current_semantics)
    }

    /// Admit one semantic item from the selected record's current authority.
    ///
    /// The caller supplies only the mounted container identity and opaque key;
    /// all request fence evidence comes from the live registration record.
    #[allow(dead_code)]
    pub(crate) fn admit_current_semantics(
        &mut self,
        container_id: crate::layout::NodeId,
        key: VirtualLayoutItemKey,
    ) -> VirtualLayoutSemanticQueryOutcome {
        let Some(index) = self
            .records
            .iter()
            .position(|record| record.registration.container_id == container_id)
        else {
            return VirtualLayoutSemanticQueryOutcome::Rejected(
                VirtualLayoutSemanticRejectedReason::UnknownContainer,
            );
        };
        let record = &mut self.records[index];
        if record.retired {
            record.pin = None;
            return VirtualLayoutSemanticQueryOutcome::Rejected(
                VirtualLayoutSemanticRejectedReason::Retired,
            );
        }
        if key.stable_equals(&key) != Some(true) {
            record.pin = None;
            return VirtualLayoutSemanticQueryOutcome::Rejected(
                VirtualLayoutSemanticRejectedReason::UnstableKey,
            );
        }

        let request = VirtualLayoutSemanticRequest::new(
            record.registration.container_id,
            record.registration.policy_identity.clone(),
            record.mount_generation,
            record.registration.data_revision(),
            record.registration.policy_revision(),
            record.registration.measurement_revision(),
            record.registration.semantic_revision(),
            key,
        );
        self.query_semantics(&request)
    }

    /// Build an exact current-authority range request from one mounted record.
    /// Invalid ranges are rejected before any provider can be observed.
    #[allow(dead_code)]
    pub(crate) fn admit_current_semantic_range(
        &mut self,
        container_id: crate::layout::NodeId,
        start_index: usize,
        length: usize,
    ) -> VirtualLayoutSemanticRangeQueryOutcome {
        let range = match VirtualLayoutSemanticRange::new(start_index, length) {
            Ok(range) => range,
            Err(reason) => return VirtualLayoutSemanticRangeQueryOutcome::Rejected(reason),
        };
        let Some(record) = self
            .records
            .iter()
            .find(|record| record.registration.container_id == container_id)
        else {
            return VirtualLayoutSemanticRangeQueryOutcome::Rejected(
                VirtualLayoutSemanticRejectedReason::UnknownContainer,
            );
        };
        let request = VirtualLayoutSemanticRangeRequest::new(
            record.registration.container_id,
            record.registration.policy_identity.clone(),
            record.mount_generation,
            record.registration.data_revision(),
            record.registration.policy_revision(),
            record.registration.measurement_revision(),
            record.registration.semantic_revision(),
            record.registration.coordinate_space.clone(),
            record.registration.budget,
            range,
        );
        self.query_semantic_range(&request)
    }

    /// Classify one already-validated semantic range against the matching live
    /// materialization authority without invoking providers or mutating state.
    pub(crate) fn classify_virtual_layout_semantic_range(
        &self,
        batch: &VirtualLayoutSemanticProjectionBatch,
    ) -> Result<VirtualLayoutSemanticClassificationBatch, VirtualLayoutSemanticClassificationError>
    {
        let mut matching_records = self
            .records
            .iter()
            .filter(|record| record.registration.container_id == batch.request().container_id());
        let Some(record) = matching_records.next() else {
            return Err(VirtualLayoutSemanticClassificationError::UnknownContainer);
        };
        if matching_records.next().is_some() {
            return Err(VirtualLayoutSemanticClassificationError::AmbiguousMaterialization);
        }
        record.classify_semantic_range(batch)
    }

    /// Classify one exact one-item semantic projection against the current
    /// materialization authority.  This is intentionally not represented as a
    /// synthetic one-entry range.
    #[allow(dead_code)]
    pub(crate) fn classify_virtual_layout_semantic_pin(
        &self,
        request: &VirtualLayoutSemanticRequest,
        projection: &VirtualLayoutSemanticProjection,
    ) -> Result<VirtualLayoutSemanticPinClassification, VirtualLayoutSemanticClassificationError>
    {
        let mut matching_records = self
            .records
            .iter()
            .filter(|record| record.registration.container_id == request.container_id());
        let Some(record) = matching_records.next() else {
            return Err(VirtualLayoutSemanticClassificationError::UnknownContainer);
        };
        if matching_records.next().is_some() {
            return Err(VirtualLayoutSemanticClassificationError::AmbiguousMaterialization);
        }
        record.classify_semantic_pin(request, projection)
    }

    /// Compose already-classified semantic evidence against one ordinary
    /// automation snapshot after revalidating the exact current record and
    /// materialization authority. This path performs no provider call or
    /// runtime mutation.
    #[allow(dead_code)]
    pub(crate) fn compose_virtual_layout_automation_snapshot(
        &self,
        ordinary: &crate::gui::automation::GuiAutomationSnapshot,
        batches: &[VirtualLayoutSemanticClassificationBatch],
    ) -> Result<
        crate::runtime::controller::VirtualLayoutAutomationComposition,
        crate::runtime::controller::VirtualLayoutAutomationCompositionError,
    > {
        for batch in batches {
            let mut matching_records = self.records.iter().filter(|record| {
                record.registration.container_id == batch.request().container_id()
            });
            let Some(record) = matching_records.next() else {
                return Err(
                    crate::runtime::controller::VirtualLayoutAutomationCompositionError::LiveRecordUnavailable,
                );
            };
            if matching_records.next().is_some() {
                return Err(
                    crate::runtime::controller::VirtualLayoutAutomationCompositionError::LiveRecordUnavailable,
                );
            }

            let projections = batch
                .classifications()
                .iter()
                .map(|classification| classification.projection().clone())
                .collect();
            let projection_batch =
                VirtualLayoutSemanticProjectionBatch::new(batch.request().clone(), projections);
            let live = record
                .classify_semantic_range(&projection_batch)
                .map_err(|_| {
                    crate::runtime::controller::VirtualLayoutAutomationCompositionError::LiveClassificationMismatch
                })?;
            if live != *batch {
                return Err(
                    crate::runtime::controller::VirtualLayoutAutomationCompositionError::LiveClassificationMismatch,
                );
            }
        }

        super::automation_compositor::compose_virtual_layout_automation_snapshot(ordinary, batches)
    }

    /// Compose one owner-prepared publication after current materialization
    /// classification.  This path remains crate-private and is not wired to
    /// ordinary automation snapshot reads.
    #[allow(dead_code)]
    pub(super) fn compose_semantic_publication(
        &mut self,
        ordinary: &crate::gui::automation::GuiAutomationSnapshot,
        authorities: SemanticPublicationAuthorities,
    ) -> SemanticPublicationOutcome {
        self.synchronize_semantic_demand();
        let plan = self.semantic_demand.publication_plan(authorities);
        if !plan.complete() {
            return self.semantic_demand.finish_publication(ordinary, plan, &[]);
        }

        let mut classifications = Vec::new();
        for member in plan.members() {
            let Some(evidence) = member.evidence() else {
                continue;
            };
            let result = if let Some((request, projections)) = evidence.range_parts() {
                let batch = VirtualLayoutSemanticProjectionBatch::new(
                    request.clone(),
                    projections.to_vec(),
                );
                self.classify_virtual_layout_semantic_range(&batch)
                    .map(VirtualLayoutSemanticClassificationInput::Range)
            } else if let Some((request, projection)) = evidence.pin_parts() {
                self.classify_virtual_layout_semantic_pin(request, projection)
                    .map(|classification| {
                        VirtualLayoutSemanticClassificationInput::Pin(Box::new(classification))
                    })
            } else {
                return self.semantic_demand.finish_publication(ordinary, plan, &[]);
            };
            let Ok(classification) = result else {
                return self.semantic_demand.finish_publication(ordinary, plan, &[]);
            };
            classifications.push(SemanticPublicationClassification::new(
                member.fence().clone(),
                classification,
            ));
        }
        self.semantic_demand
            .finish_publication(ordinary, plan, &classifications)
    }

    pub(crate) fn open_semantic_automation_session(
        &mut self,
        runtime_id: u64,
    ) -> Result<SemanticAutomationSessionHandle, SemanticAutomationSessionError> {
        if self.semantic_session.is_some() {
            return Err(SemanticAutomationSessionError::SessionAlreadyActive);
        }
        let generation = self.next_semantic_session_generation.checked_add(1).ok_or(
            SemanticAutomationSessionError::InvalidDemand(
                SemanticAutomationDemandError::CounterOverflow,
            ),
        )?;
        self.next_semantic_session_generation = generation;
        self.semantic_demand
            .clear_demands()
            .map_err(map_semantic_demand_error)?;
        let handle = SemanticAutomationSessionHandle {
            runtime_id,
            generation,
        };
        self.semantic_session = Some(RuntimeSemanticAutomationSession {
            handle,
            selection: None,
        });
        Ok(handle)
    }

    pub(crate) fn semantic_automation_containers(
        &self,
        runtime_id: u64,
        session: SemanticAutomationSessionHandle,
    ) -> Result<Vec<SemanticAutomationContainerHandle>, SemanticAutomationSessionError> {
        self.validate_semantic_automation_session(runtime_id, session)?;
        Ok(self
            .records
            .iter()
            .filter(|record| !record.retired)
            .map(|record| SemanticAutomationContainerHandle {
                runtime_id,
                session_generation: session.generation,
                container_id: record.registration.container_id,
                mount_generation: record.mount_generation,
            })
            .collect())
    }

    pub(crate) fn refresh_semantic_automation(
        &mut self,
        runtime_id: u64,
        session: SemanticAutomationSessionHandle,
        demands: &[SemanticAutomationDemand],
        ordinary: &crate::gui::automation::GuiAutomationSnapshot,
        authorities: (u64, u64, u64),
    ) -> Result<RuntimeSemanticAutomationPublication, SemanticAutomationSessionError> {
        self.validate_semantic_automation_session(runtime_id, session)?;
        self.synchronize_semantic_demand();
        let (materialization_authority, classification_authority, ordinary_projection_generation) =
            authorities;

        let mut requested = Vec::with_capacity(demands.len());
        for demand in demands {
            requested.push(self.lower_semantic_automation_demand(runtime_id, session, demand)?);
        }

        let tickets = self
            .semantic_demand
            .replace_demand_set(&requested)
            .map_err(map_semantic_demand_error)?;
        let mut failure_reason = None;
        for ticket in tickets {
            let completion = match self.semantic_demand.execute(ticket) {
                Ok(completion) => completion,
                Err(SemanticDemandExecutionError::Reentrant) => {
                    return Err(SemanticAutomationSessionError::Reentrant);
                }
                Err(SemanticDemandExecutionError::AlreadyExecuted) => {
                    failure_reason.get_or_insert(SemanticAutomationFallbackReason::Stale);
                    continue;
                }
            };
            if let SemanticProviderCompletion::Stale = completion {
                failure_reason.get_or_insert(SemanticAutomationFallbackReason::Stale);
            }
            let completion = self.semantic_demand.complete(completion);
            if let Some(reason) = semantic_completion_fallback_reason(&completion) {
                failure_reason.get_or_insert(reason);
            }
        }

        let publication = self.compose_semantic_publication(
            ordinary,
            SemanticPublicationAuthorities {
                session_generation: session.generation,
                materialization_authority,
                classification_authority,
                ordinary_projection_generation,
            },
        );
        let (composition, status) = match publication {
            SemanticPublicationOutcome::Published(composition) => {
                if requested.is_empty() {
                    (
                        composition,
                        SemanticAutomationRefreshStatus::Baseline {
                            reason: SemanticAutomationFallbackReason::NoDemand,
                        },
                    )
                } else if let Some(reason) = failure_reason {
                    (
                        composition,
                        SemanticAutomationRefreshStatus::Retained { reason },
                    )
                } else {
                    (composition, SemanticAutomationRefreshStatus::Published)
                }
            }
            SemanticPublicationOutcome::OrdinaryBaseline {
                composition,
                reason,
            } => {
                let reason = failure_reason.unwrap_or_else(|| map_publication_reason(reason));
                (
                    composition,
                    SemanticAutomationRefreshStatus::Baseline { reason },
                )
            }
        };

        let publication = RuntimeSemanticAutomationPublication {
            composition: composition.clone(),
            status,
        };
        if let Some(session_state) = &mut self.semantic_session {
            session_state.selection = Some(RuntimeSemanticAutomationSelection {
                composition,
                ordinary: ordinary.clone(),
                runtime_projection_generation: ordinary_projection_generation,
                status,
            });
        }
        Ok(publication)
    }

    pub(crate) fn retry_semantic_automation(
        &mut self,
        runtime_id: u64,
        session: SemanticAutomationSessionHandle,
        ordinary: &crate::gui::automation::GuiAutomationSnapshot,
        authorities: (u64, u64, u64),
    ) -> Result<RuntimeSemanticAutomationPublication, SemanticAutomationSessionError> {
        self.validate_semantic_automation_session(runtime_id, session)?;
        self.synchronize_semantic_demand();
        let (materialization_authority, classification_authority, ordinary_projection_generation) =
            authorities;
        let tickets = self
            .semantic_demand
            .retry_all()
            .map_err(map_semantic_demand_error)?;
        let mut failure_reason = None;
        for ticket in tickets {
            let completion = match self.semantic_demand.execute(ticket) {
                Ok(completion) => completion,
                Err(SemanticDemandExecutionError::Reentrant) => {
                    return Err(SemanticAutomationSessionError::Reentrant);
                }
                Err(SemanticDemandExecutionError::AlreadyExecuted) => {
                    failure_reason.get_or_insert(SemanticAutomationFallbackReason::Stale);
                    continue;
                }
            };
            if let SemanticProviderCompletion::Stale = completion {
                failure_reason.get_or_insert(SemanticAutomationFallbackReason::Stale);
            }
            let completion = self.semantic_demand.complete(completion);
            if let Some(reason) = semantic_completion_fallback_reason(&completion) {
                failure_reason.get_or_insert(reason);
            }
        }

        let publication = self.compose_semantic_publication(
            ordinary,
            SemanticPublicationAuthorities {
                session_generation: session.generation,
                materialization_authority,
                classification_authority,
                ordinary_projection_generation,
            },
        );
        let (composition, status) = match publication {
            SemanticPublicationOutcome::Published(composition) => {
                if let Some(reason) = failure_reason {
                    (
                        composition,
                        SemanticAutomationRefreshStatus::Retained { reason },
                    )
                } else {
                    (composition, SemanticAutomationRefreshStatus::Published)
                }
            }
            SemanticPublicationOutcome::OrdinaryBaseline {
                composition,
                reason,
            } => (
                composition,
                SemanticAutomationRefreshStatus::Baseline {
                    reason: failure_reason.unwrap_or_else(|| map_publication_reason(reason)),
                },
            ),
        };
        let publication = RuntimeSemanticAutomationPublication {
            composition: composition.clone(),
            status,
        };
        if let Some(session_state) = &mut self.semantic_session {
            session_state.selection = Some(RuntimeSemanticAutomationSelection {
                composition,
                ordinary: ordinary.clone(),
                runtime_projection_generation: ordinary_projection_generation,
                status,
            });
        }
        Ok(publication)
    }

    #[cfg(target_os = "macos")]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn retry_semantic_automation_range(
        &mut self,
        runtime_id: u64,
        session: SemanticAutomationSessionHandle,
        container: SemanticAutomationContainerHandle,
        start_index: usize,
        length: usize,
        ordinary: &crate::gui::automation::GuiAutomationSnapshot,
        authorities: (u64, u64, u64),
    ) -> Result<RuntimeSemanticAutomationPublication, SemanticAutomationSessionError> {
        self.validate_semantic_automation_session(runtime_id, session)?;
        self.synchronize_semantic_demand();
        let demand = SemanticAutomationDemand::range(container, start_index, length);
        let (container_id, _) =
            self.lower_semantic_automation_demand(runtime_id, session, &demand)?;
        if !self
            .semantic_demand
            .active_range_matches(container_id, start_index, length)
        {
            return Err(SemanticAutomationSessionError::StaleContainerHandle);
        }

        let (materialization_authority, classification_authority, ordinary_projection_generation) =
            authorities;
        let ticket = self
            .semantic_demand
            .retry_range(container_id)
            .map_err(map_semantic_demand_error)?;
        let mut failure_reason = None;
        let completion = match self.semantic_demand.execute(ticket) {
            Ok(completion) => completion,
            Err(SemanticDemandExecutionError::Reentrant) => {
                return Err(SemanticAutomationSessionError::Reentrant);
            }
            Err(SemanticDemandExecutionError::AlreadyExecuted) => {
                failure_reason = Some(SemanticAutomationFallbackReason::Stale);
                SemanticProviderCompletion::Stale
            }
        };
        if let SemanticProviderCompletion::Stale = completion {
            failure_reason.get_or_insert(SemanticAutomationFallbackReason::Stale);
        }
        let completion = self.semantic_demand.complete(completion);
        if let Some(reason) = semantic_completion_fallback_reason(&completion) {
            failure_reason.get_or_insert(reason);
        }

        let publication = self.compose_semantic_publication(
            ordinary,
            SemanticPublicationAuthorities {
                session_generation: session.generation,
                materialization_authority,
                classification_authority,
                ordinary_projection_generation,
            },
        );
        let (composition, status) = match publication {
            SemanticPublicationOutcome::Published(composition) => {
                if let Some(reason) = failure_reason {
                    (
                        composition,
                        SemanticAutomationRefreshStatus::Retained { reason },
                    )
                } else {
                    (composition, SemanticAutomationRefreshStatus::Published)
                }
            }
            SemanticPublicationOutcome::OrdinaryBaseline {
                composition,
                reason,
            } => (
                composition,
                SemanticAutomationRefreshStatus::Baseline {
                    reason: failure_reason.unwrap_or_else(|| map_publication_reason(reason)),
                },
            ),
        };
        let publication = RuntimeSemanticAutomationPublication {
            composition: composition.clone(),
            status,
        };
        if let Some(session_state) = &mut self.semantic_session {
            session_state.selection = Some(RuntimeSemanticAutomationSelection {
                composition,
                ordinary: ordinary.clone(),
                runtime_projection_generation: ordinary_projection_generation,
                status,
            });
        }
        Ok(publication)
    }

    pub(crate) fn selected_semantic_automation(
        &self,
        runtime_id: u64,
        session: SemanticAutomationSessionHandle,
        ordinary: &crate::gui::automation::GuiAutomationSnapshot,
        ordinary_projection_generation: u64,
    ) -> Result<Option<RuntimeSemanticAutomationPublication>, SemanticAutomationSessionError> {
        self.validate_semantic_automation_session(runtime_id, session)?;
        let Some(selection) = self
            .semantic_session
            .as_ref()
            .and_then(|session| session.selection.as_ref())
        else {
            return Ok(None);
        };
        if selection.ordinary != *ordinary
            || selection.runtime_projection_generation != ordinary_projection_generation
        {
            return Ok(None);
        }
        Ok(Some(RuntimeSemanticAutomationPublication {
            composition: selection.composition.clone(),
            status: selection.status,
        }))
    }

    pub(crate) fn close_semantic_automation_session(
        &mut self,
        runtime_id: u64,
        session: SemanticAutomationSessionHandle,
    ) -> Result<(), SemanticAutomationSessionError> {
        self.validate_semantic_automation_session(runtime_id, session)?;
        self.semantic_demand
            .clear_demands()
            .map_err(map_semantic_demand_error)?;
        self.semantic_session = None;
        Ok(())
    }

    fn validate_semantic_automation_session(
        &self,
        runtime_id: u64,
        session: SemanticAutomationSessionHandle,
    ) -> Result<(), SemanticAutomationSessionError> {
        if session.runtime_id != runtime_id
            || self
                .semantic_session
                .as_ref()
                .is_none_or(|current| current.handle != session)
        {
            return Err(SemanticAutomationSessionError::UnknownSession);
        }
        Ok(())
    }

    fn lower_semantic_automation_demand(
        &self,
        runtime_id: u64,
        session: SemanticAutomationSessionHandle,
        demand: &SemanticAutomationDemand,
    ) -> Result<(NodeId, SemanticDemand), SemanticAutomationSessionError> {
        let (container, lowered) = match demand {
            SemanticAutomationDemand::Range {
                container,
                start_index,
                length,
            } => (
                container,
                SemanticDemand::Range(
                    VirtualLayoutSemanticRange::new(*start_index, *length).map_err(|reason| {
                        SemanticAutomationSessionError::InvalidDemand(map_range_demand_error(
                            reason,
                        ))
                    })?,
                ),
            ),
            SemanticAutomationDemand::RequiredItem { container, key } => {
                if key.stable_equals(key) != Some(true) {
                    return Err(SemanticAutomationSessionError::InvalidDemand(
                        SemanticAutomationDemandError::InvalidKey,
                    ));
                }
                (container, SemanticDemand::RequiredItemPin(key.clone()))
            }
        };
        let Some(record) = self.records.iter().find(|record| {
            !record.retired
                && record.registration.container_id == container.container_id
                && record.mount_generation == container.mount_generation
        }) else {
            return Err(SemanticAutomationSessionError::StaleContainerHandle);
        };
        if container.runtime_id != runtime_id || container.session_generation != session.generation
        {
            return Err(SemanticAutomationSessionError::StaleContainerHandle);
        }
        Ok((record.registration.container_id, lowered))
    }

    pub(super) fn take_projection_probe(
        &mut self,
    ) -> Option<RuntimeVirtualLayoutProjectionProbe<Message>> {
        self.projection_probe.take()
    }

    pub(super) fn store_projection_probe(
        &mut self,
        probe: RuntimeVirtualLayoutProjectionProbe<Message>,
    ) {
        self.projection_probe = Some(probe);
    }

    fn clear_projection_probe_if_empty(&mut self) {
        if self.is_empty() {
            self.projection_probe = None;
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.records.iter().all(|record| record.retired)
    }

    fn allocate_generation(&mut self) -> Option<u64> {
        let next = self.next_mount_generation.checked_add(1)?;
        self.next_mount_generation = next;
        Some(next)
    }
}

fn map_semantic_demand_error(
    error: SemanticDemandAdmissionError,
) -> SemanticAutomationSessionError {
    match error {
        SemanticDemandAdmissionError::DuplicateSource => {
            SemanticAutomationSessionError::InvalidDemand(
                SemanticAutomationDemandError::DuplicateSource,
            )
        }
        SemanticDemandAdmissionError::UnknownContainer
        | SemanticDemandAdmissionError::Retired
        | SemanticDemandAdmissionError::ScopeMismatch => {
            SemanticAutomationSessionError::StaleContainerHandle
        }
        SemanticDemandAdmissionError::InvalidKey => {
            SemanticAutomationSessionError::InvalidDemand(SemanticAutomationDemandError::InvalidKey)
        }
        SemanticDemandAdmissionError::InvalidRange(reason) => {
            SemanticAutomationSessionError::InvalidDemand(map_range_demand_error(reason))
        }
        SemanticDemandAdmissionError::CustomCoordinate => {
            SemanticAutomationSessionError::InvalidDemand(
                SemanticAutomationDemandError::CustomCoordinateSpace,
            )
        }
        SemanticDemandAdmissionError::AggregateBudgetExceeded => {
            SemanticAutomationSessionError::InvalidDemand(
                SemanticAutomationDemandError::AggregateRangeBudgetExceeded,
            )
        }
        SemanticDemandAdmissionError::CounterOverflow => {
            SemanticAutomationSessionError::InvalidDemand(
                SemanticAutomationDemandError::CounterOverflow,
            )
        }
        SemanticDemandAdmissionError::NoActiveDemand => {
            SemanticAutomationSessionError::NoActiveDemand
        }
        SemanticDemandAdmissionError::Reentrant => SemanticAutomationSessionError::Reentrant,
    }
}

fn map_range_demand_error(
    reason: VirtualLayoutSemanticRejectedReason,
) -> SemanticAutomationDemandError {
    match reason {
        VirtualLayoutSemanticRejectedReason::RangeLengthZero => {
            SemanticAutomationDemandError::RangeLengthZero
        }
        VirtualLayoutSemanticRejectedReason::RangeIndexOverflow => {
            SemanticAutomationDemandError::RangeIndexOverflow
        }
        VirtualLayoutSemanticRejectedReason::RangeOverBudget => {
            SemanticAutomationDemandError::RangeOverBudget
        }
        _ => SemanticAutomationDemandError::RangeOverBudget,
    }
}

fn map_publication_reason(
    reason: SemanticPublicationFallbackReason,
) -> SemanticAutomationFallbackReason {
    match reason {
        SemanticPublicationFallbackReason::IncompleteDemandSet => {
            SemanticAutomationFallbackReason::IncompleteDemandSet
        }
        SemanticPublicationFallbackReason::StalePlan => SemanticAutomationFallbackReason::Stale,
        SemanticPublicationFallbackReason::ClassificationRejected
        | SemanticPublicationFallbackReason::CompositionRejected => {
            SemanticAutomationFallbackReason::Malformed
        }
        SemanticPublicationFallbackReason::CounterOverflow => {
            SemanticAutomationFallbackReason::CounterOverflow
        }
    }
}

fn semantic_completion_fallback_reason(
    completion: &SemanticDemandCompletion,
) -> Option<SemanticAutomationFallbackReason> {
    match completion {
        SemanticDemandCompletion::Stale => Some(SemanticAutomationFallbackReason::Stale),
        SemanticDemandCompletion::RequiredItemPin(outcome) => match outcome {
            VirtualLayoutSemanticQueryOutcome::Found(_)
            | VirtualLayoutSemanticQueryOutcome::NotFound => None,
            VirtualLayoutSemanticQueryOutcome::Deferred(_) => {
                Some(SemanticAutomationFallbackReason::Deferred)
            }
            VirtualLayoutSemanticQueryOutcome::Unavailable(reason) => Some(match reason {
                VirtualLayoutSemanticUnavailableReason::NoProvider => {
                    SemanticAutomationFallbackReason::NoProvider
                }
                VirtualLayoutSemanticUnavailableReason::DataUnavailable => {
                    SemanticAutomationFallbackReason::DataUnavailable
                }
                VirtualLayoutSemanticUnavailableReason::Unsupported => {
                    SemanticAutomationFallbackReason::Unsupported
                }
            }),
            VirtualLayoutSemanticQueryOutcome::Rejected(reason) => {
                Some(map_rejected_reason(*reason))
            }
        },
        SemanticDemandCompletion::Range(outcome) => match outcome {
            VirtualLayoutSemanticRangeQueryOutcome::Found(_)
            | VirtualLayoutSemanticRangeQueryOutcome::NotFound => None,
            VirtualLayoutSemanticRangeQueryOutcome::Deferred(_) => {
                Some(SemanticAutomationFallbackReason::Deferred)
            }
            VirtualLayoutSemanticRangeQueryOutcome::Unavailable(reason) => Some(match reason {
                VirtualLayoutSemanticUnavailableReason::NoProvider => {
                    SemanticAutomationFallbackReason::NoProvider
                }
                VirtualLayoutSemanticUnavailableReason::DataUnavailable => {
                    SemanticAutomationFallbackReason::DataUnavailable
                }
                VirtualLayoutSemanticUnavailableReason::Unsupported => {
                    SemanticAutomationFallbackReason::Unsupported
                }
            }),
            VirtualLayoutSemanticRangeQueryOutcome::Rejected(reason) => {
                Some(map_rejected_reason(*reason))
            }
        },
    }
}

fn map_rejected_reason(
    reason: VirtualLayoutSemanticRejectedReason,
) -> SemanticAutomationFallbackReason {
    match reason {
        VirtualLayoutSemanticRejectedReason::UnknownContainer
        | VirtualLayoutSemanticRejectedReason::Retired
        | VirtualLayoutSemanticRejectedReason::ScopeMismatch
        | VirtualLayoutSemanticRejectedReason::Stale => SemanticAutomationFallbackReason::Stale,
        VirtualLayoutSemanticRejectedReason::RangeCountMismatch
        | VirtualLayoutSemanticRejectedReason::WrongLogicalIndex
        | VirtualLayoutSemanticRejectedReason::RangeOutOfOrder
        | VirtualLayoutSemanticRejectedReason::DuplicateKey
        | VirtualLayoutSemanticRejectedReason::DuplicateSemanticNodeId
        | VirtualLayoutSemanticRejectedReason::SemanticNodeIdDrift
        | VirtualLayoutSemanticRejectedReason::NonFiniteBounds
        | VirtualLayoutSemanticRejectedReason::InvertedBounds => {
            SemanticAutomationFallbackReason::Malformed
        }
        _ => SemanticAutomationFallbackReason::Rejected,
    }
}

fn suppress_cached_virtual_layout_subtree<Message>(
    surface: &mut UiSurface<Message>,
    container_id: crate::layout::NodeId,
    subtree: Option<RuntimeVirtualLayoutSubtree<Message>>,
) {
    let Some(subtree) = subtree else {
        return;
    };
    let _ = surface.replace_virtual_layout_shell(container_id, subtree.shell, subtree.registration);
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

fn validate_semantic_classification_batch(
    batch: &VirtualLayoutSemanticProjectionBatch,
) -> Result<(), VirtualLayoutSemanticClassificationError> {
    let request = batch.request();
    let range = request.range();
    if batch.projections().len() != range.length()
        || range.length() > request.budget().max_entries()
        || range.length() > crate::layout::VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES
    {
        return Err(VirtualLayoutSemanticClassificationError::MalformedBatch);
    }
    for (offset, projection) in batch.projections().iter().enumerate() {
        if projection.authority() != VirtualLayoutSemanticProjectionAuthority::Unmaterialized
            || range.expected_index(offset) != Some(projection.logical_index())
            || projection.identity().container_id() != request.container_id()
            || !projection.bounds().is_finite()
            || projection.bounds().min.x > projection.bounds().max.x
            || projection.bounds().min.y > projection.bounds().max.y
        {
            return Err(VirtualLayoutSemanticClassificationError::MalformedBatch);
        }
        let Some(projection_range_request) = projection.range_request() else {
            return Err(VirtualLayoutSemanticClassificationError::MalformedBatch);
        };
        validate_matching_semantic_range_request(request, projection_range_request)?;
        validate_matching_semantic_item_request(request, projection.request())?;
        let projection_key = projection.identity().key();
        let item_request_key = projection.request().key();
        if projection_key.stable_equals(projection_key) != Some(true)
            || item_request_key.stable_equals(item_request_key) != Some(true)
        {
            return Err(VirtualLayoutSemanticClassificationError::UnstableKey);
        }
        match projection_key.stable_equals(item_request_key) {
            Some(true) => {}
            Some(false) => {
                return Err(VirtualLayoutSemanticClassificationError::MalformedBatch);
            }
            None => return Err(VirtualLayoutSemanticClassificationError::UnstableKey),
        }
        match stable_coordinate_space_equals(
            projection.coordinate_space(),
            request.coordinate_space(),
        ) {
            Some(true) => {}
            Some(false) => {
                return Err(VirtualLayoutSemanticClassificationError::MalformedBatch);
            }
            None => {
                return Err(VirtualLayoutSemanticClassificationError::UnstableCoordinateSpace);
            }
        }
    }
    for (left_index, left_projection) in batch.projections().iter().enumerate() {
        for right_projection in batch.projections().iter().skip(left_index + 1) {
            match left_projection
                .identity()
                .key()
                .stable_equals(right_projection.identity().key())
            {
                Some(true) => {
                    return Err(VirtualLayoutSemanticClassificationError::AmbiguousMaterialization);
                }
                Some(false) => {}
                None => return Err(VirtualLayoutSemanticClassificationError::UnstableKey),
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_semantic_pin_classification(
    request: &VirtualLayoutSemanticRequest,
    projection: &VirtualLayoutSemanticProjection,
    container_id: NodeId,
    policy_identity: &VirtualLayoutPolicyIdentity,
    mount_generation: u64,
    data_revision: u64,
    policy_revision: u64,
    measurement_revision: u64,
    semantic_revision: u64,
    coordinate_space: &VirtualLayoutCoordinateSpace,
) -> Result<(), VirtualLayoutSemanticClassificationError> {
    if projection.authority() != VirtualLayoutSemanticProjectionAuthority::Unmaterialized
        || projection.range_request().is_some()
        || projection.identity().container_id() != container_id
        || !projection.bounds().is_finite()
        || projection.bounds().min.x > projection.bounds().max.x
        || projection.bounds().min.y > projection.bounds().max.y
    {
        return Err(VirtualLayoutSemanticClassificationError::MalformedBatch);
    }
    if request.container_id() != container_id
        || request.mount_generation() != mount_generation
        || request.data_revision() != data_revision
        || request.policy_revision() != policy_revision
        || request.measurement_revision() != measurement_revision
        || request.semantic_revision() != semantic_revision
    {
        return Err(VirtualLayoutSemanticClassificationError::FenceMismatch(
            VirtualLayoutSemanticClassificationFenceField::SemanticRevision,
        ));
    }
    match request.policy_identity().stable_equals(policy_identity) {
        Some(true) => {}
        Some(false) => {
            return Err(VirtualLayoutSemanticClassificationError::FenceMismatch(
                VirtualLayoutSemanticClassificationFenceField::PolicyIdentity,
            ));
        }
        None => return Err(VirtualLayoutSemanticClassificationError::UnstablePolicyIdentity),
    }
    match stable_coordinate_space_equals(projection.coordinate_space(), coordinate_space) {
        Some(true) => {}
        Some(false) => {
            return Err(VirtualLayoutSemanticClassificationError::FenceMismatch(
                VirtualLayoutSemanticClassificationFenceField::CoordinateSpace,
            ));
        }
        None => return Err(VirtualLayoutSemanticClassificationError::UnstableCoordinateSpace),
    }
    if projection.request().container_id() != request.container_id()
        || projection.request().mount_generation() != request.mount_generation()
        || projection.request().data_revision() != request.data_revision()
        || projection.request().policy_revision() != request.policy_revision()
        || projection.request().measurement_revision() != request.measurement_revision()
        || projection.request().semantic_revision() != request.semantic_revision()
    {
        return Err(VirtualLayoutSemanticClassificationError::MalformedBatch);
    }
    match projection
        .request()
        .policy_identity()
        .stable_equals(request.policy_identity())
    {
        Some(true) => {}
        Some(false) => return Err(VirtualLayoutSemanticClassificationError::MalformedBatch),
        None => return Err(VirtualLayoutSemanticClassificationError::UnstablePolicyIdentity),
    }
    match projection.identity().key().stable_equals(request.key()) {
        Some(true) => {}
        Some(false) => return Err(VirtualLayoutSemanticClassificationError::MalformedBatch),
        None => return Err(VirtualLayoutSemanticClassificationError::UnstableKey),
    }
    match projection.request().key().stable_equals(request.key()) {
        Some(true) => Ok(()),
        Some(false) => Err(VirtualLayoutSemanticClassificationError::MalformedBatch),
        None => Err(VirtualLayoutSemanticClassificationError::UnstableKey),
    }
}

fn validate_matching_semantic_range_request(
    expected: &VirtualLayoutSemanticRangeRequest,
    actual: &VirtualLayoutSemanticRangeRequest,
) -> Result<(), VirtualLayoutSemanticClassificationError> {
    if expected.container_id() != actual.container_id()
        || expected.mount_generation() != actual.mount_generation()
        || expected.data_revision() != actual.data_revision()
        || expected.policy_revision() != actual.policy_revision()
        || expected.measurement_revision() != actual.measurement_revision()
        || expected.semantic_revision() != actual.semantic_revision()
        || expected.budget() != actual.budget()
        || expected.range() != actual.range()
    {
        return Err(VirtualLayoutSemanticClassificationError::MalformedBatch);
    }
    match expected
        .policy_identity()
        .stable_equals(actual.policy_identity())
    {
        Some(true) => {}
        Some(false) => return Err(VirtualLayoutSemanticClassificationError::MalformedBatch),
        None => return Err(VirtualLayoutSemanticClassificationError::UnstablePolicyIdentity),
    }
    match stable_coordinate_space_equals(expected.coordinate_space(), actual.coordinate_space()) {
        Some(true) => Ok(()),
        Some(false) => Err(VirtualLayoutSemanticClassificationError::MalformedBatch),
        None => Err(VirtualLayoutSemanticClassificationError::UnstableCoordinateSpace),
    }
}

fn validate_matching_semantic_item_request(
    range_request: &VirtualLayoutSemanticRangeRequest,
    item_request: &VirtualLayoutSemanticRequest,
) -> Result<(), VirtualLayoutSemanticClassificationError> {
    if range_request.container_id() != item_request.container_id()
        || range_request.mount_generation() != item_request.mount_generation()
        || range_request.data_revision() != item_request.data_revision()
        || range_request.policy_revision() != item_request.policy_revision()
        || range_request.measurement_revision() != item_request.measurement_revision()
        || range_request.semantic_revision() != item_request.semantic_revision()
    {
        return Err(VirtualLayoutSemanticClassificationError::MalformedBatch);
    }
    match range_request
        .policy_identity()
        .stable_equals(item_request.policy_identity())
    {
        Some(true) => Ok(()),
        Some(false) => Err(VirtualLayoutSemanticClassificationError::MalformedBatch),
        None => Err(VirtualLayoutSemanticClassificationError::UnstablePolicyIdentity),
    }
}

#[allow(clippy::too_many_arguments)]
fn validate_semantic_request_scope(
    request: &VirtualLayoutSemanticRangeRequest,
    container_id: NodeId,
    policy_identity: &VirtualLayoutPolicyIdentity,
    mount_generation: u64,
    data_revision: u64,
    policy_revision: u64,
    measurement_revision: u64,
    semantic_revision: u64,
    coordinate_space: &VirtualLayoutCoordinateSpace,
    budget: VirtualLayoutBudget,
) -> Result<(), VirtualLayoutSemanticClassificationError> {
    validate_semantic_scope_fields(
        request,
        container_id,
        policy_identity,
        mount_generation,
        data_revision,
        policy_revision,
        measurement_revision,
        semantic_revision,
        coordinate_space,
        budget,
    )
}

fn validate_semantic_materialization_fence(
    request: &VirtualLayoutSemanticRangeRequest,
    fence: &VirtualLayoutQueryFence,
) -> Result<(), VirtualLayoutSemanticClassificationError> {
    validate_semantic_scope_fields(
        request,
        fence.container_id(),
        fence.policy_identity(),
        fence.mount_generation(),
        fence.data_revision(),
        fence.policy_revision(),
        fence.measurement_revision(),
        fence.semantic_revision(),
        fence.coordinate_space(),
        fence.budget(),
    )
}

#[allow(clippy::too_many_arguments)]
fn validate_semantic_scope_fields(
    request: &VirtualLayoutSemanticRangeRequest,
    container_id: NodeId,
    policy_identity: &VirtualLayoutPolicyIdentity,
    mount_generation: u64,
    data_revision: u64,
    policy_revision: u64,
    measurement_revision: u64,
    semantic_revision: u64,
    coordinate_space: &VirtualLayoutCoordinateSpace,
    budget: VirtualLayoutBudget,
) -> Result<(), VirtualLayoutSemanticClassificationError> {
    if request.container_id() != container_id {
        return Err(VirtualLayoutSemanticClassificationError::FenceMismatch(
            VirtualLayoutSemanticClassificationFenceField::ContainerIdentity,
        ));
    }
    match request.policy_identity().stable_equals(policy_identity) {
        Some(true) => {}
        Some(false) => {
            return Err(VirtualLayoutSemanticClassificationError::FenceMismatch(
                VirtualLayoutSemanticClassificationFenceField::PolicyIdentity,
            ));
        }
        None => return Err(VirtualLayoutSemanticClassificationError::UnstablePolicyIdentity),
    }
    let revision_fields = [
        (
            request.mount_generation(),
            mount_generation,
            VirtualLayoutSemanticClassificationFenceField::MountGeneration,
        ),
        (
            request.data_revision(),
            data_revision,
            VirtualLayoutSemanticClassificationFenceField::DataRevision,
        ),
        (
            request.policy_revision(),
            policy_revision,
            VirtualLayoutSemanticClassificationFenceField::PolicyRevision,
        ),
        (
            request.measurement_revision(),
            measurement_revision,
            VirtualLayoutSemanticClassificationFenceField::MeasurementRevision,
        ),
        (
            request.semantic_revision(),
            semantic_revision,
            VirtualLayoutSemanticClassificationFenceField::SemanticRevision,
        ),
    ];
    for (actual, expected, field) in revision_fields {
        if actual != expected {
            return Err(VirtualLayoutSemanticClassificationError::FenceMismatch(
                field,
            ));
        }
    }
    match stable_coordinate_space_equals(request.coordinate_space(), coordinate_space) {
        Some(true) => {}
        Some(false) => {
            return Err(VirtualLayoutSemanticClassificationError::FenceMismatch(
                VirtualLayoutSemanticClassificationFenceField::CoordinateSpace,
            ));
        }
        None => return Err(VirtualLayoutSemanticClassificationError::UnstableCoordinateSpace),
    }
    if request.budget() != budget {
        return Err(VirtualLayoutSemanticClassificationError::FenceMismatch(
            VirtualLayoutSemanticClassificationFenceField::Budget,
        ));
    }
    Ok(())
}

fn stable_coordinate_space_equals(
    left: &VirtualLayoutCoordinateSpace,
    right: &VirtualLayoutCoordinateSpace,
) -> Option<bool> {
    match (left, right) {
        (VirtualLayoutCoordinateSpace::Logical, VirtualLayoutCoordinateSpace::Logical) => {
            Some(true)
        }
        (
            VirtualLayoutCoordinateSpace::Custom(left),
            VirtualLayoutCoordinateSpace::Custom(right),
        ) => left.stable_equals(right),
        (VirtualLayoutCoordinateSpace::Logical, VirtualLayoutCoordinateSpace::Custom(_))
        | (VirtualLayoutCoordinateSpace::Custom(_), VirtualLayoutCoordinateSpace::Logical) => {
            Some(false)
        }
    }
}

fn validate_semantic_range_entries(
    request: &VirtualLayoutSemanticRangeRequest,
    entries: &[crate::gui::layout_core::VirtualLayoutSemanticEntry],
    existing_pin: Option<&VirtualLayoutPin>,
) -> Result<(), VirtualLayoutSemanticRejectedReason> {
    let range = request.range();
    if entries.len() != range.length() {
        return Err(VirtualLayoutSemanticRejectedReason::RangeCountMismatch);
    }

    for pair in entries.windows(2) {
        if pair[1].logical_index() <= pair[0].logical_index() {
            return Err(VirtualLayoutSemanticRejectedReason::RangeOutOfOrder);
        }
    }

    for (offset, entry) in entries.iter().enumerate() {
        if range.expected_index(offset) != Some(entry.logical_index()) {
            return Err(VirtualLayoutSemanticRejectedReason::WrongLogicalIndex);
        }
        entry.validate_for_range()?;
        for previous in &entries[..offset] {
            match entry
                .requested_key()
                .stable_equals(previous.requested_key())
            {
                Some(true) => return Err(VirtualLayoutSemanticRejectedReason::DuplicateKey),
                Some(false) => {
                    if entry.automation_node_id() == previous.automation_node_id() {
                        return Err(VirtualLayoutSemanticRejectedReason::DuplicateSemanticNodeId);
                    }
                }
                None => return Err(VirtualLayoutSemanticRejectedReason::UnstableKey),
            }
        }
        let item_request = request.item_request(entry.requested_key().clone());
        validate_semantic_entry_against_pin(existing_pin, &item_request, entry)?;
    }
    Ok(())
}

fn validate_semantic_entry_against_pin(
    existing_pin: Option<&VirtualLayoutPin>,
    request: &VirtualLayoutSemanticRequest,
    entry: &crate::gui::layout_core::VirtualLayoutSemanticEntry,
) -> Result<(), VirtualLayoutSemanticRejectedReason> {
    let Some(existing_pin) = existing_pin else {
        return Ok(());
    };
    match existing_pin
        .request()
        .key()
        .stable_equals(entry.requested_key())
    {
        Some(true) => {
            if same_semantic_request_fence(existing_pin.request(), request)
                && existing_pin.automation_node_id() != entry.automation_node_id()
            {
                return Err(VirtualLayoutSemanticRejectedReason::SemanticNodeIdDrift);
            }
        }
        Some(false) => {}
        None => return Err(VirtualLayoutSemanticRejectedReason::UnstableKey),
    }
    Ok(())
}

fn same_semantic_request_fence(
    left: &VirtualLayoutSemanticRequest,
    right: &VirtualLayoutSemanticRequest,
) -> bool {
    left.container_id() == right.container_id()
        && left
            .policy_identity()
            .stable_equals(right.policy_identity())
            == Some(true)
        && left.mount_generation() == right.mount_generation()
        && left.data_revision() == right.data_revision()
        && left.policy_revision() == right.policy_revision()
        && left.measurement_revision() == right.measurement_revision()
        && left.semantic_revision() == right.semantic_revision()
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: crate::runtime::RuntimeBridge<Message>,
{
    pub(super) fn prepare_virtual_layout_surface(
        &mut self,
        registrations: &[VirtualLayoutRegistration<Message>],
    ) {
        self.virtual_layout
            .prepare_surface(&mut self.surface, registrations);
    }

    pub(super) fn rebuild_virtual_layout_shell_layout(&mut self) {
        self.layout_engine.layout_with_state_into(
            &self.layout_root,
            self.viewport,
            &self.layout_state,
            self.layout_debug_options,
            &mut self.layout,
        );
    }

    pub(super) fn materialize_virtual_layout_surface(&mut self) {
        self.virtual_layout
            .materialize_surface(&mut self.surface, &self.layout);
    }

    pub(super) fn requires_virtual_layout_materialization(&self, force_pass: bool) -> bool {
        self.virtual_layout
            .requires_materialization(&self.layout, force_pass)
    }

    pub(super) fn relayout_virtual_layout_for_geometry(&mut self) -> bool {
        if self.virtual_layout.is_empty() {
            return false;
        }
        let registrations = self
            .traversal
            .containers
            .virtual_layout_registrations
            .clone();
        self.prepare_virtual_layout_surface(&registrations);
        let mut traversal = self.take_reusable_traversal_index(true);
        self.layout_root = self.surface.runtime_projection_reusing_with_scratch(
            &mut traversal,
            &mut self.scratch.projection_scroll_stack,
            &mut self.scratch.projection_child_path,
            &mut self.scratch.projection_source,
        );
        self.rebuild_virtual_layout_shell_layout();
        self.materialize_virtual_layout_surface();
        self.layout_root = self.surface.runtime_projection_reusing_with_scratch(
            &mut traversal,
            &mut self.scratch.projection_scroll_stack,
            &mut self.scratch.projection_child_path,
            &mut self.scratch.projection_source,
        );
        self.relayout_with_traversal(traversal);
        self.install_declarative_owner_projection();
        true
    }

    pub(super) fn retire_virtual_layout(&mut self) {
        self.virtual_layout.retire_all();
    }

    /// Admit one semantic item using only the current mounted registration
    /// authority. This path performs no materialization or runtime refresh.
    #[allow(dead_code)]
    pub(crate) fn admit_virtual_layout_semantics(
        &mut self,
        container_id: crate::layout::NodeId,
        key: VirtualLayoutItemKey,
    ) -> VirtualLayoutSemanticQueryOutcome {
        self.virtual_layout
            .admit_current_semantics(container_id, key)
    }

    /// Project the current semantic pin as private evidence only.
    #[allow(dead_code)]
    pub(crate) fn project_virtual_layout_semantics(
        &self,
        container_id: crate::layout::NodeId,
    ) -> Option<VirtualLayoutSemanticProjection> {
        self.virtual_layout.project_current_semantics(container_id)
    }

    /// Query one exact current-authority semantic range as private evidence.
    #[allow(dead_code)]
    pub(crate) fn admit_virtual_layout_semantic_range(
        &mut self,
        container_id: crate::layout::NodeId,
        start_index: usize,
        length: usize,
    ) -> VirtualLayoutSemanticRangeQueryOutcome {
        self.virtual_layout
            .admit_current_semantic_range(container_id, start_index, length)
    }

    /// Query a preconstructed private semantic range request without changing
    /// runtime, materialization, interaction, or presentation state.
    #[allow(dead_code)]
    pub(crate) fn query_virtual_layout_semantic_range(
        &mut self,
        request: &VirtualLayoutSemanticRangeRequest,
    ) -> VirtualLayoutSemanticRangeQueryOutcome {
        self.virtual_layout.query_semantic_range(request)
    }

    /// Classify one validated semantic range against exact retained slot
    /// evidence without invoking providers or changing runtime state.
    #[allow(dead_code)]
    pub(crate) fn classify_virtual_layout_semantic_range(
        &self,
        batch: &VirtualLayoutSemanticProjectionBatch,
    ) -> Result<VirtualLayoutSemanticClassificationBatch, VirtualLayoutSemanticClassificationError>
    {
        self.virtual_layout
            .classify_virtual_layout_semantic_range(batch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::controller::semantic_demand::{
        SemanticDemandAdmission, SemanticDemandAdmissionError, SemanticDemandCompletion,
        SemanticProviderCompletion,
    };
    use crate::{
        application::{View, empty, scroll, spacer, text},
        gui::{
            automation::{
                AutomationBounds, AutomationNodeId, AutomationNodeSemantics,
                AutomationNodeSnapshot, AutomationRole, GuiAutomationSnapshot,
            },
            layout_core::{
                VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES, VirtualLayoutPinReason,
                VirtualLayoutSemanticDeferredReason, VirtualLayoutSemanticEntry,
                VirtualLayoutSemanticProjectionAuthority, VirtualLayoutSemanticProvider,
                VirtualLayoutSemanticQueryOutcome, VirtualLayoutSemanticRange,
                VirtualLayoutSemanticRangeProvider, VirtualLayoutSemanticRangeProviderOutcome,
                VirtualLayoutSemanticRangeQueryOutcome, VirtualLayoutSemanticRangeRequest,
                VirtualLayoutSemanticRejectedReason, VirtualLayoutSemanticRequest,
                VirtualLayoutSemanticUnavailableReason,
            },
            types::{Point, Rect, Vector2},
        },
        layout::{
            ContainerKind, ContainerPolicy, OverflowPolicy, VirtualLayoutBoundsConfidence,
            VirtualLayoutBudget, VirtualLayoutCoordinateSpace, VirtualLayoutDeferredReason,
            VirtualLayoutExtentCandidate, VirtualLayoutItemCandidate, VirtualLayoutItemKey,
            VirtualLayoutOverscan, VirtualLayoutPolicy, VirtualLayoutPolicyDecision,
            VirtualLayoutPolicyIdentity, VirtualLayoutQueryInput, VirtualLayoutQueryInputParts,
            VirtualLayoutQuerySink, VirtualLayoutUnavailableReason, VirtualLayoutVisibility,
        },
        runtime::{
            RuntimeBridge, SemanticAutomationDemand, SemanticAutomationDemandError,
            SemanticAutomationFallbackReason, SemanticAutomationRefreshStatus,
            SemanticAutomationSessionError, SemanticAutomationSessionHandle, SurfaceChild,
            SurfaceNode, UiSurface, surface::VirtualLayoutRegistrationRevisions,
        },
        widgets::WidgetSizing,
    };
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
        sync::Arc,
    };

    const CONTAINER_ID: u64 = 710;
    const ROOT_ID: u64 = 711;
    const ORDINARY_CHILD_ID: u64 = 712;
    const SEMANTIC_MOUNT_GENERATION: u64 = 41;

    struct ReadyPolicy {
        calls: Rc<Cell<u32>>,
        key: u32,
    }

    struct RequiredKeyPolicy {
        calls: Rc<Cell<u32>>,
        required_key: u32,
    }

    impl VirtualLayoutPolicy for ReadyPolicy {
        fn query(
            &self,
            _input: &VirtualLayoutQueryInput,
            sink: &mut VirtualLayoutQuerySink,
        ) -> VirtualLayoutPolicyDecision {
            self.calls.set(self.calls.get().saturating_add(1));
            assert!(
                sink.visit(VirtualLayoutItemCandidate::new(
                    VirtualLayoutItemKey::new(self.key),
                    0,
                    Rect::from_xy_size(0.0, 0.0, 100.0, 20.0),
                    VirtualLayoutVisibility::Visible,
                    VirtualLayoutBoundsConfidence::Exact,
                ))
                .is_ok()
            );
            assert!(
                sink.set_extent(VirtualLayoutExtentCandidate::exact(Vector2::new(
                    100.0, 20.0,
                )))
                .is_ok()
            );
            VirtualLayoutPolicyDecision::Ready
        }
    }

    struct MaterializationPolicy {
        calls: Rc<Cell<u32>>,
        entries: Vec<(VirtualLayoutItemKey, usize)>,
    }

    impl VirtualLayoutPolicy for MaterializationPolicy {
        fn query(
            &self,
            _input: &VirtualLayoutQueryInput,
            sink: &mut VirtualLayoutQuerySink,
        ) -> VirtualLayoutPolicyDecision {
            self.calls.set(self.calls.get().saturating_add(1));
            for (key, logical_index) in &self.entries {
                assert!(
                    sink.visit(VirtualLayoutItemCandidate::new(
                        key.clone(),
                        *logical_index,
                        Rect::from_xy_size(0.0, *logical_index as f32 * 12.0, 24.0, 10.0),
                        VirtualLayoutVisibility::Visible,
                        VirtualLayoutBoundsConfidence::Exact,
                    ))
                    .is_ok()
                );
            }
            assert!(
                sink.set_extent(VirtualLayoutExtentCandidate::exact(Vector2::new(
                    240.0, 120.0,
                )))
                .is_ok()
            );
            VirtualLayoutPolicyDecision::Ready
        }
    }

    impl VirtualLayoutPolicy for RequiredKeyPolicy {
        fn query(
            &self,
            input: &VirtualLayoutQueryInput,
            sink: &mut VirtualLayoutQuerySink,
        ) -> VirtualLayoutPolicyDecision {
            self.calls.set(self.calls.get().saturating_add(1));
            let expected = VirtualLayoutItemKey::new(self.required_key);
            assert!(
                input
                    .required_key()
                    .is_some_and(|key| { key.stable_equals(&expected) == Some(true) })
            );
            assert!(
                sink.visit(VirtualLayoutItemCandidate::new(
                    VirtualLayoutItemKey::new(self.required_key),
                    0,
                    Rect::from_xy_size(0.0, 0.0, 100.0, 20.0),
                    VirtualLayoutVisibility::Visible,
                    VirtualLayoutBoundsConfidence::Exact,
                ))
                .is_ok()
            );
            assert!(
                sink.set_extent(VirtualLayoutExtentCandidate::exact(Vector2::new(
                    100.0, 20.0,
                )))
                .is_ok()
            );
            VirtualLayoutPolicyDecision::Ready
        }
    }

    struct ControlledPolicy {
        calls: Rc<Cell<u32>>,
        decision: Cell<VirtualLayoutPolicyDecision>,
        key: u32,
    }

    struct SemanticProvider {
        calls: Rc<Cell<u32>>,
        outcome: Rc<RefCell<VirtualLayoutSemanticQueryOutcome>>,
        requests: Rc<RefCell<Vec<VirtualLayoutSemanticRequest>>>,
    }

    struct SemanticRangeProvider {
        calls: Rc<Cell<u32>>,
        outcome: Rc<RefCell<VirtualLayoutSemanticRangeProviderOutcome>>,
        requests: Rc<RefCell<Vec<VirtualLayoutSemanticRangeRequest>>>,
    }

    impl VirtualLayoutSemanticProvider for SemanticProvider {
        fn lookup(
            &self,
            request: &VirtualLayoutSemanticRequest,
        ) -> VirtualLayoutSemanticQueryOutcome {
            self.calls.set(self.calls.get().saturating_add(1));
            self.requests.borrow_mut().push(request.clone());
            self.outcome.borrow().clone()
        }
    }

    impl VirtualLayoutSemanticRangeProvider for SemanticRangeProvider {
        fn lookup_range(
            &self,
            request: &VirtualLayoutSemanticRangeRequest,
        ) -> VirtualLayoutSemanticRangeProviderOutcome {
            self.calls.set(self.calls.get().saturating_add(1));
            self.requests.borrow_mut().push(request.clone());
            self.outcome.borrow().clone()
        }
    }

    struct UnstableKey;

    impl PartialEq for UnstableKey {
        fn eq(&self, _other: &Self) -> bool {
            false
        }
    }

    impl Eq for UnstableKey {}

    impl VirtualLayoutPolicy for ControlledPolicy {
        fn query(
            &self,
            _input: &VirtualLayoutQueryInput,
            sink: &mut VirtualLayoutQuerySink,
        ) -> VirtualLayoutPolicyDecision {
            self.calls.set(self.calls.get().saturating_add(1));
            let decision = self.decision.get();
            if decision == VirtualLayoutPolicyDecision::Ready {
                assert!(
                    sink.visit(VirtualLayoutItemCandidate::new(
                        VirtualLayoutItemKey::new(self.key),
                        0,
                        Rect::from_xy_size(0.0, 0.0, 100.0, 20.0),
                        VirtualLayoutVisibility::Visible,
                        VirtualLayoutBoundsConfidence::Exact,
                    ))
                    .is_ok()
                );
                assert!(
                    sink.set_extent(VirtualLayoutExtentCandidate::exact(Vector2::new(
                        100.0, 20.0,
                    )))
                    .is_ok()
                );
            }
            decision
        }
    }

    type VirtualLayoutItemFactory = Rc<dyn Fn(&crate::layout::VirtualLayoutItem) -> View<()>>;

    struct RegistrationParts {
        policy: Rc<dyn VirtualLayoutPolicy>,
        policy_identity: VirtualLayoutPolicyIdentity,
        revisions: VirtualLayoutRegistrationRevisions,
        shell: Rc<dyn Fn() -> View<()>>,
        item: VirtualLayoutItemFactory,
        kind: Rc<dyn Fn(&crate::layout::VirtualLayoutItem) -> VirtualLayoutPolicyIdentity>,
    }

    fn registration_with_parts(parts: RegistrationParts) -> VirtualLayoutRegistration<()> {
        VirtualLayoutRegistration::new(
            CONTAINER_ID,
            parts.policy_identity,
            parts.policy,
            VirtualLayoutCoordinateSpace::logical(),
            VirtualLayoutOverscan::new(0.0, 0.0).expect("finite overscan"),
            VirtualLayoutBudget::new(4),
            parts.revisions,
            parts.shell,
            parts.item,
            parts.kind,
        )
    }

    fn registration(
        policy: Rc<dyn VirtualLayoutPolicy>,
        policy_identity: VirtualLayoutPolicyIdentity,
    ) -> VirtualLayoutRegistration<()> {
        registration_with_parts(RegistrationParts {
            policy,
            policy_identity,
            revisions: Default::default(),
            shell: Rc::new(|| scroll(spacer::<()>())),
            item: Rc::new(|_| text::<()>("virtual item")),
            kind: Rc::new(|_| VirtualLayoutPolicyIdentity::new("item-kind")),
        })
    }

    fn registration_with_required_key(
        policy: Rc<dyn VirtualLayoutPolicy>,
        policy_identity: VirtualLayoutPolicyIdentity,
        required_key: u32,
    ) -> VirtualLayoutRegistration<()> {
        registration(policy, policy_identity)
            .with_required_key(VirtualLayoutItemKey::new(required_key))
    }

    fn semantic_entry(key: u32, bounds: Rect) -> VirtualLayoutSemanticEntry {
        semantic_entry_with_id(key, AutomationNodeId::new("semantic-item"), bounds)
    }

    fn semantic_entry_with_id(
        key: u32,
        automation_node_id: AutomationNodeId,
        bounds: Rect,
    ) -> VirtualLayoutSemanticEntry {
        VirtualLayoutSemanticEntry::new(
            VirtualLayoutItemKey::new(key),
            key as usize,
            bounds,
            AutomationNodeSemantics::new(AutomationRole::Button).with_label("semantic item"),
            automation_node_id,
        )
    }

    fn semantic_request(
        policy_identity: &str,
        mount_generation: u64,
        semantic_revision: u64,
        key: u32,
    ) -> VirtualLayoutSemanticRequest {
        semantic_request_with_revisions(
            policy_identity,
            mount_generation,
            VirtualLayoutRegistrationRevisions {
                semantic: semantic_revision,
                ..Default::default()
            },
            key,
        )
    }

    fn semantic_request_with_revisions(
        policy_identity: &str,
        mount_generation: u64,
        revisions: VirtualLayoutRegistrationRevisions,
        key: u32,
    ) -> VirtualLayoutSemanticRequest {
        VirtualLayoutSemanticRequest::new(
            CONTAINER_ID,
            VirtualLayoutPolicyIdentity::new(policy_identity.to_owned()),
            mount_generation,
            revisions.data,
            revisions.policy,
            revisions.measurement,
            revisions.semantic,
            VirtualLayoutItemKey::new(key),
        )
    }

    fn semantic_provider(
        outcome: VirtualLayoutSemanticQueryOutcome,
    ) -> (
        Rc<SemanticProvider>,
        Rc<Cell<u32>>,
        Rc<RefCell<VirtualLayoutSemanticQueryOutcome>>,
    ) {
        let calls = Rc::new(Cell::new(0));
        let outcome = Rc::new(RefCell::new(outcome));
        let provider = Rc::new(SemanticProvider {
            calls: Rc::clone(&calls),
            outcome: Rc::clone(&outcome),
            requests: Rc::new(RefCell::new(Vec::new())),
        });
        (provider, calls, outcome)
    }

    fn semantic_range_provider(
        outcome: VirtualLayoutSemanticRangeProviderOutcome,
    ) -> (
        Rc<SemanticRangeProvider>,
        Rc<Cell<u32>>,
        Rc<RefCell<VirtualLayoutSemanticRangeProviderOutcome>>,
    ) {
        let calls = Rc::new(Cell::new(0));
        let outcome = Rc::new(RefCell::new(outcome));
        let provider = Rc::new(SemanticRangeProvider {
            calls: Rc::clone(&calls),
            outcome: Rc::clone(&outcome),
            requests: Rc::new(RefCell::new(Vec::new())),
        });
        (provider, calls, outcome)
    }

    fn semantic_range_entries_with_prefix(
        prefix: &str,
        start_index: usize,
        length: usize,
    ) -> Vec<VirtualLayoutSemanticEntry> {
        (0..length)
            .map(|offset| {
                let logical_index = start_index + offset;
                semantic_range_entry_with_id(
                    100 + logical_index as u32,
                    logical_index,
                    Rect::from_xy_size(4.0, logical_index as f32 * 12.0, 24.0, 10.0),
                    AutomationNodeId::new(format!("{prefix}-{logical_index}")),
                )
            })
            .collect()
    }

    fn semantic_range_entry(
        key: u32,
        logical_index: usize,
        bounds: Rect,
    ) -> VirtualLayoutSemanticEntry {
        semantic_range_entry_with_id(
            key,
            logical_index,
            bounds,
            AutomationNodeId::new("range-item"),
        )
    }

    fn semantic_range_entry_with_id(
        key: u32,
        logical_index: usize,
        bounds: Rect,
        automation_node_id: AutomationNodeId,
    ) -> VirtualLayoutSemanticEntry {
        VirtualLayoutSemanticEntry::new(
            VirtualLayoutItemKey::new(key),
            logical_index,
            bounds,
            AutomationNodeSemantics::new(AutomationRole::Button)
                .with_label(format!("semantic item {logical_index}")),
            automation_node_id,
        )
    }

    fn semantic_range_entry_with_key(
        key: VirtualLayoutItemKey,
        logical_index: usize,
        automation_node_id: AutomationNodeId,
    ) -> VirtualLayoutSemanticEntry {
        VirtualLayoutSemanticEntry::new(
            key,
            logical_index,
            Rect::from_xy_size(4.0, logical_index as f32 * 12.0, 24.0, 10.0),
            AutomationNodeSemantics::new(AutomationRole::Button)
                .with_label(format!("semantic item {logical_index}")),
            automation_node_id,
        )
    }

    fn semantic_range_request(
        state: &RuntimeVirtualLayoutState<()>,
        start_index: usize,
        length: usize,
    ) -> VirtualLayoutSemanticRangeRequest {
        let record = &state.records[0];
        VirtualLayoutSemanticRangeRequest::new(
            record.registration.container_id,
            record.registration.policy_identity.clone(),
            record.mount_generation,
            record.registration.data_revision(),
            record.registration.policy_revision(),
            record.registration.measurement_revision(),
            record.registration.semantic_revision(),
            record.registration.coordinate_space.clone(),
            record.registration.budget,
            VirtualLayoutSemanticRange::new(start_index, length)
                .expect("test semantic range should be valid"),
        )
    }

    fn semantic_range_state(
        provider: Rc<dyn VirtualLayoutSemanticRangeProvider>,
        coordinate_space: VirtualLayoutCoordinateSpace,
        budget: VirtualLayoutBudget,
    ) -> RuntimeVirtualLayoutState<()> {
        let mut registration = registration(
            Rc::new(ReadyPolicy {
                calls: Rc::new(Cell::new(0)),
                key: 1,
            }),
            VirtualLayoutPolicyIdentity::new("semantic-range-policy".to_owned()),
        );
        registration.coordinate_space = coordinate_space;
        registration.budget = budget;
        registration = registration.with_semantic_range_provider(provider);
        let mut state = RuntimeVirtualLayoutState::default();
        state.records.push(RuntimeVirtualLayoutRecord::new(
            registration,
            SEMANTIC_MOUNT_GENERATION,
        ));
        state
    }

    fn semantic_state(
        provider: Rc<dyn VirtualLayoutSemanticProvider>,
        semantic_revision: u64,
    ) -> RuntimeVirtualLayoutState<()> {
        let mut registration = registration(
            Rc::new(ReadyPolicy {
                calls: Rc::new(Cell::new(0)),
                key: 1,
            }),
            VirtualLayoutPolicyIdentity::new("semantic-policy".to_owned()),
        )
        .with_semantic_provider(provider);
        registration.revisions.semantic = semantic_revision;
        let mut state = RuntimeVirtualLayoutState::default();
        state.records.push(RuntimeVirtualLayoutRecord::new(
            registration,
            SEMANTIC_MOUNT_GENERATION,
        ));
        state
    }

    fn valid_semantic_range_entries(
        start_index: usize,
        length: usize,
    ) -> Vec<VirtualLayoutSemanticEntry> {
        let automation_node_ids = [
            "range-zero",
            "range-one",
            "range-two",
            "range-three",
            "range-four",
            "range-five",
        ];
        (0..length)
            .map(|offset| {
                let logical_index = start_index + offset;
                semantic_range_entry_with_id(
                    100 + logical_index as u32,
                    logical_index,
                    Rect::from_xy_size(4.0, logical_index as f32 * 12.0, 24.0, 10.0),
                    AutomationNodeId::new(automation_node_ids[offset]),
                )
            })
            .collect()
    }

    fn projection_batch_from_keys(
        request: &VirtualLayoutSemanticRangeRequest,
        keys: &[VirtualLayoutItemKey],
    ) -> VirtualLayoutSemanticProjectionBatch {
        let projections = keys
            .iter()
            .enumerate()
            .map(|(logical_index, key)| {
                let entry = semantic_range_entry_with_key(
                    key.clone(),
                    logical_index,
                    AutomationNodeId::new(format!("test-key-{logical_index}")),
                );
                VirtualLayoutSemanticProjection::from_validated_semantic_range_entry(
                    request,
                    &entry,
                    request.coordinate_space().clone(),
                )
                .expect("the test projection entry should be valid")
            })
            .collect();
        VirtualLayoutSemanticProjectionBatch::new(request.clone(), projections)
    }

    type MaterializedStateAndBatch = (
        RuntimeVirtualLayoutState<()>,
        VirtualLayoutSemanticProjectionBatch,
        Rc<Cell<u32>>,
        Rc<Cell<u32>>,
    );

    fn materialized_state_and_batch(
        materialized: Vec<(VirtualLayoutItemKey, usize)>,
        semantic_entries: Vec<VirtualLayoutSemanticEntry>,
    ) -> MaterializedStateAndBatch {
        let policy_calls = Rc::new(Cell::new(0));
        let (provider, semantic_calls, _) = semantic_range_provider(
            VirtualLayoutSemanticRangeProviderOutcome::Found(semantic_entries.clone()),
        );
        let registration = registration_with_parts(RegistrationParts {
            policy: Rc::new(MaterializationPolicy {
                calls: Rc::clone(&policy_calls),
                entries: materialized,
            }),
            policy_identity: VirtualLayoutPolicyIdentity::new("classification-policy"),
            revisions: Default::default(),
            shell: Rc::new(|| scroll(spacer::<()>())),
            item: Rc::new(|_| text::<()>("classification item")),
            kind: Rc::new(|_| VirtualLayoutPolicyIdentity::new("classification-item-kind")),
        })
        .with_semantic_range_provider(provider);
        let mut record = RuntimeVirtualLayoutRecord::new(registration, SEMANTIC_MOUNT_GENERATION);
        let committed = record.materialize(Rect::from_xy_size(0.0, 0.0, 160.0, 80.0));
        let RuntimeVirtualLayoutMaterialization::Committed(batch) = committed else {
            panic!("the classification fixture should materialize a committed window");
        };
        record.commit_batch(*batch);
        let mut state = RuntimeVirtualLayoutState::default();
        state.records.push(record);
        let VirtualLayoutSemanticRangeQueryOutcome::Found(batch) =
            state.admit_current_semantic_range(CONTAINER_ID, 0, semantic_entries.len())
        else {
            panic!("the classification fixture should admit its semantic range");
        };
        (state, batch, semantic_calls, policy_calls)
    }

    fn committed_range_record(
        registration: VirtualLayoutRegistration<()>,
    ) -> RuntimeVirtualLayoutRecord<()> {
        let mut record = RuntimeVirtualLayoutRecord::new(registration, SEMANTIC_MOUNT_GENERATION);
        let RuntimeVirtualLayoutMaterialization::Committed(batch) =
            record.materialize(Rect::from_xy_size(0.0, 0.0, 160.0, 80.0))
        else {
            panic!("the range fixture should materialize a committed window");
        };
        record.commit_batch(*batch);
        record
    }

    type TwoRangeState = (
        RuntimeVirtualLayoutState<()>,
        Rc<Cell<u32>>,
        Rc<Cell<u32>>,
        Rc<RefCell<VirtualLayoutSemanticRangeProviderOutcome>>,
        Rc<RefCell<VirtualLayoutSemanticRangeProviderOutcome>>,
    );

    fn two_range_state() -> TwoRangeState {
        let initial_a = semantic_range_entries_with_prefix("a-initial", 0, 2);
        let initial_b = semantic_range_entries_with_prefix("b-initial", 2, 2);
        let (provider_a, calls_a, outcome_a) =
            semantic_range_provider(VirtualLayoutSemanticRangeProviderOutcome::Found(initial_a));
        let (provider_b, calls_b, outcome_b) =
            semantic_range_provider(VirtualLayoutSemanticRangeProviderOutcome::Found(initial_b));

        let mut registration_a = registration_with_parts(RegistrationParts {
            policy: Rc::new(MaterializationPolicy {
                calls: Rc::new(Cell::new(0)),
                entries: Vec::new(),
            }),
            policy_identity: VirtualLayoutPolicyIdentity::new("range-a"),
            revisions: Default::default(),
            shell: Rc::new(|| scroll(spacer::<()>())),
            item: Rc::new(|_| text::<()>("range-a item")),
            kind: Rc::new(|_| VirtualLayoutPolicyIdentity::new("range-a-kind")),
        })
        .with_semantic_range_provider(provider_a);
        registration_a.container_id = CONTAINER_ID;

        let mut registration_b = registration_with_parts(RegistrationParts {
            policy: Rc::new(MaterializationPolicy {
                calls: Rc::new(Cell::new(0)),
                entries: Vec::new(),
            }),
            policy_identity: VirtualLayoutPolicyIdentity::new("range-b"),
            revisions: Default::default(),
            shell: Rc::new(|| scroll(spacer::<()>())),
            item: Rc::new(|_| text::<()>("range-b item")),
            kind: Rc::new(|_| VirtualLayoutPolicyIdentity::new("range-b-kind")),
        })
        .with_semantic_range_provider(provider_b);
        registration_b.container_id = CONTAINER_ID + 1;

        let mut state = RuntimeVirtualLayoutState::default();
        state.records.push(committed_range_record(registration_a));
        state.records.push(committed_range_record(registration_b));
        (state, calls_a, calls_b, outcome_a, outcome_b)
    }

    fn semantic_pin_publication_state(
        provider: Rc<dyn VirtualLayoutSemanticProvider>,
    ) -> RuntimeVirtualLayoutState<()> {
        let registration = registration_with_parts(RegistrationParts {
            policy: Rc::new(MaterializationPolicy {
                calls: Rc::new(Cell::new(0)),
                entries: Vec::new(),
            }),
            policy_identity: VirtualLayoutPolicyIdentity::new("publication-policy"),
            revisions: Default::default(),
            shell: Rc::new(|| scroll(spacer::<()>())),
            item: Rc::new(|_| text::<()>("publication item")),
            kind: Rc::new(|_| VirtualLayoutPolicyIdentity::new("publication-item-kind")),
        })
        .with_semantic_provider(provider);
        let mut record = RuntimeVirtualLayoutRecord::new(registration, SEMANTIC_MOUNT_GENERATION);
        let RuntimeVirtualLayoutMaterialization::Committed(batch) =
            record.materialize(Rect::from_xy_size(0.0, 0.0, 160.0, 80.0))
        else {
            panic!("the publication fixture should materialize an empty committed window");
        };
        record.commit_batch(*batch);
        let mut state = RuntimeVirtualLayoutState::default();
        state.records.push(record);
        state
    }

    fn semantic_publication_snapshot() -> GuiAutomationSnapshot {
        semantic_publication_snapshot_for(&[CONTAINER_ID])
    }

    fn semantic_publication_snapshot_for(container_ids: &[u64]) -> GuiAutomationSnapshot {
        let mut root = AutomationNodeSnapshot::from_semantics(
            AutomationNodeId::new("publication-root"),
            AutomationBounds {
                x: 0.0,
                y: 0.0,
                width: 160.0,
                height: 80.0,
            },
            AutomationNodeSemantics::new(AutomationRole::Group),
        );
        root.children
            .extend(container_ids.iter().map(|container_id| {
                AutomationNodeSnapshot::from_semantics(
                    AutomationNodeId::new(container_id.to_string()),
                    AutomationBounds {
                        x: 0.0,
                        y: 0.0,
                        width: 160.0,
                        height: 80.0,
                    },
                    AutomationNodeSemantics::new(AutomationRole::Group),
                )
            }));
        GuiAutomationSnapshot {
            schema_version: 2,
            viewport_width: 160,
            viewport_height: 80,
            root,
        }
    }

    #[derive(Debug, PartialEq)]
    struct QueryPartsSnapshot {
        container_id: crate::layout::NodeId,
        policy_identity: VirtualLayoutPolicyIdentity,
        mount_generation: u64,
        query_sequence: u64,
        viewport: Rect,
        coordinate_space: VirtualLayoutCoordinateSpace,
        overscan: VirtualLayoutOverscan,
        budget: VirtualLayoutBudget,
        viewport_revision: u64,
        data_revision: u64,
        policy_revision: u64,
        measurement_revision: u64,
        semantic_revision: u64,
    }

    fn query_parts_snapshot(
        parts: Option<&VirtualLayoutQueryInputParts>,
    ) -> Option<QueryPartsSnapshot> {
        parts.map(|parts| QueryPartsSnapshot {
            container_id: parts.container_id,
            policy_identity: parts.policy_identity.clone(),
            mount_generation: parts.mount_generation,
            query_sequence: parts.query_sequence,
            viewport: parts.viewport,
            coordinate_space: parts.coordinate_space.clone(),
            overscan: parts.overscan,
            budget: parts.budget,
            viewport_revision: parts.viewport_revision,
            data_revision: parts.data_revision,
            policy_revision: parts.policy_revision,
            measurement_revision: parts.measurement_revision,
            semantic_revision: parts.semantic_revision,
        })
    }

    #[test]
    fn semantic_automation_session_refreshes_explicitly_and_preserves_unmaterialized_authority() {
        let entries = valid_semantic_range_entries(0, 2);
        let (mut state, _batch, calls, _policy_calls) =
            materialized_state_and_batch(Vec::new(), entries);
        let session = state
            .open_semantic_automation_session(900)
            .expect("one session should open");
        let containers = state
            .semantic_automation_containers(900, session)
            .expect("the live mounted container should enumerate");
        assert_eq!(containers.len(), 1);
        let ordinary = semantic_publication_snapshot();
        assert_eq!(calls.get(), 1);

        let demand = SemanticAutomationDemand::range(containers[0], 0, 2);
        let publication = state
            .refresh_semantic_automation(
                session.runtime_id,
                session,
                &[demand],
                &ordinary,
                (1, 2, 3),
            )
            .expect("the explicit logical refresh should publish");
        assert_eq!(calls.get(), 2);
        assert_eq!(
            publication.status,
            SemanticAutomationRefreshStatus::Published
        );
        assert_eq!(
            publication.composition.snapshot().root.children[0]
                .children
                .iter()
                .map(|child| child.id.clone())
                .collect::<Vec<_>>(),
            vec![
                AutomationNodeId::new("range-zero"),
                AutomationNodeId::new("range-one"),
            ]
        );
        let targets = publication.composition.target_snapshot(3);
        for target_id in ["range-zero", "range-one"] {
            let target = targets
                .targets
                .iter()
                .find(|target| target.id == AutomationNodeId::new(target_id))
                .expect("the semantic target should be present");
            assert_eq!(
                target.authority,
                Some(crate::runtime::AutomationTargetAuthority {
                    runtime_generation: 3,
                    materialized: false,
                })
            );
        }

        let selected = state
            .selected_semantic_automation(900, session, &ordinary, 3)
            .expect("selected read should remain pure")
            .expect("the explicit publication should be selected");
        assert_eq!(selected.status, SemanticAutomationRefreshStatus::Published);
        assert_eq!(selected.composition.normalized_sidecar().entries().len(), 2);
        assert!(
            state
                .selected_semantic_automation(900, session, &ordinary, 4)
                .expect("a changed projection generation should remain a pure read")
                .is_none()
        );
        assert_eq!(calls.get(), 2);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn native_semantic_passive_admission_reads_cardinality_without_demand() {
        let (provider, calls, _) = semantic_range_provider(
            VirtualLayoutSemanticRangeProviderOutcome::Found(valid_semantic_range_entries(0, 1)),
        );
        let mut state = semantic_range_state(
            provider,
            VirtualLayoutCoordinateSpace::Logical,
            VirtualLayoutBudget::new(4),
        );
        state.records[0].registration.semantic_cardinality =
            Some(VirtualLayoutSemanticCardinality::new(0, 1));
        state.synchronize_semantic_demand();
        let containers = state.native_semantic_containers(&semantic_publication_snapshot());
        assert_eq!(containers.len(), 1);
        assert_eq!(containers[0].cardinality.logical_item_count, 0);
        assert!(state.semantic_session.is_none());
        assert_eq!(calls.get(), 0);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn native_semantic_passive_admission_vetoes_positive_without_range_provider() {
        let mut registration = registration(
            Rc::new(ReadyPolicy {
                calls: Rc::new(Cell::new(0)),
                key: 1,
            }),
            VirtualLayoutPolicyIdentity::new("native-veto"),
        );
        registration.semantic_cardinality = Some(VirtualLayoutSemanticCardinality::new(2, 1));
        let mut state = RuntimeVirtualLayoutState::default();
        state.records.push(RuntimeVirtualLayoutRecord::new(
            registration,
            SEMANTIC_MOUNT_GENERATION,
        ));
        state.synchronize_semantic_demand();
        assert!(
            state
                .native_semantic_containers(&semantic_publication_snapshot())
                .is_empty()
        );
        assert!(state.semantic_session.is_none());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn native_semantic_admission_rejects_custom_zero_before_provider_access() {
        let zero = Some(VirtualLayoutSemanticCardinality::new(0, 1));
        assert!(native_semantic_registration_is_admitted(
            &VirtualLayoutCoordinateSpace::Logical,
            zero,
            false,
        ));
        assert!(!native_semantic_registration_is_admitted(
            &VirtualLayoutCoordinateSpace::Custom(VirtualLayoutPolicyIdentity::new("custom")),
            zero,
            false,
        ));
    }

    #[test]
    fn semantic_session_contention_is_typed_without_eviction() {
        let (provider, _calls, _) = semantic_range_provider(
            VirtualLayoutSemanticRangeProviderOutcome::Found(valid_semantic_range_entries(0, 1)),
        );
        let mut state = semantic_range_state(
            provider,
            VirtualLayoutCoordinateSpace::Logical,
            VirtualLayoutBudget::new(4),
        );
        let first = state
            .open_semantic_automation_session(904)
            .expect("the first session should own the lease");
        assert_eq!(
            state.open_semantic_automation_session(904),
            Err(SemanticAutomationSessionError::SessionAlreadyActive)
        );
        assert_eq!(
            state
                .semantic_session
                .as_ref()
                .map(|session| session.handle),
            Some(first)
        );
    }

    #[test]
    fn semantic_automation_rejected_replacement_preserves_selected_publication() {
        let entries = valid_semantic_range_entries(0, 2);
        let (mut state, _batch, calls, _policy_calls) =
            materialized_state_and_batch(Vec::new(), entries);
        let session = state
            .open_semantic_automation_session(903)
            .expect("one session should open");
        let container = state
            .semantic_automation_containers(903, session)
            .expect("the live mounted container should enumerate")[0];
        let ordinary = semantic_publication_snapshot();
        let published = state
            .refresh_semantic_automation(
                903,
                session,
                &[SemanticAutomationDemand::range(container, 0, 2)],
                &ordinary,
                (1, 2, 3),
            )
            .expect("the valid demand should publish");
        let published_snapshot = published.composition.snapshot().clone();
        assert_eq!(published.status, SemanticAutomationRefreshStatus::Published);
        assert_eq!(calls.get(), 2);

        let rejected = state.refresh_semantic_automation(
            903,
            session,
            &[
                SemanticAutomationDemand::range(container, 0, 2),
                SemanticAutomationDemand::range(container, 0, 2),
            ],
            &ordinary,
            (1, 2, 3),
        );
        assert!(matches!(
            rejected,
            Err(SemanticAutomationSessionError::InvalidDemand(
                SemanticAutomationDemandError::DuplicateSource,
            ))
        ));
        assert_eq!(calls.get(), 2);

        let selected = state
            .selected_semantic_automation(903, session, &ordinary, 3)
            .expect("selected read should remain pure")
            .expect("the prior publication should remain selected");
        assert_eq!(selected.status, SemanticAutomationRefreshStatus::Published);
        assert_eq!(selected.composition.snapshot(), &published_snapshot);
        assert_eq!(
            selected.composition.normalized_sidecar(),
            published.composition.normalized_sidecar()
        );
        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn semantic_automation_demand_validation_is_atomic_and_does_not_call_custom_providers() {
        let (provider, calls, _) = semantic_range_provider(
            VirtualLayoutSemanticRangeProviderOutcome::Found(valid_semantic_range_entries(0, 1)),
        );
        let mut state = semantic_range_state(
            provider,
            VirtualLayoutCoordinateSpace::Custom(VirtualLayoutPolicyIdentity::new(
                "unsupported-coordinate",
            )),
            VirtualLayoutBudget::new(4),
        );
        let session = state
            .open_semantic_automation_session(901)
            .expect("one session should open");
        let container = state
            .semantic_automation_containers(901, session)
            .expect("the mounted container should enumerate")[0];
        let ordinary = semantic_publication_snapshot();
        let result = state.refresh_semantic_automation(
            901,
            session,
            &[SemanticAutomationDemand::range(container, 0, 1)],
            &ordinary,
            (1, 2, 3),
        );
        assert!(matches!(
            result,
            Err(SemanticAutomationSessionError::InvalidDemand(
                SemanticAutomationDemandError::CustomCoordinateSpace,
            ))
        ));
        assert_eq!(calls.get(), 0);
    }

    #[test]
    fn semantic_automation_deferred_result_is_conservative_until_explicit_retry() {
        let (provider, calls, outcome) =
            semantic_provider(VirtualLayoutSemanticQueryOutcome::Deferred(
                VirtualLayoutSemanticDeferredReason::SemanticPending,
            ));
        let mut state = semantic_pin_publication_state(provider);
        let session = state
            .open_semantic_automation_session(902)
            .expect("one session should open");
        let container = state
            .semantic_automation_containers(902, session)
            .expect("the mounted container should enumerate")[0];
        let ordinary = semantic_publication_snapshot();
        let first = state
            .refresh_semantic_automation(
                902,
                session,
                &[SemanticAutomationDemand::required_item(
                    container,
                    VirtualLayoutItemKey::new(7_u32),
                )],
                &ordinary,
                (1, 2, 3),
            )
            .expect("deferred provider output should produce a typed baseline");
        assert_eq!(
            first.status,
            SemanticAutomationRefreshStatus::Baseline {
                reason: SemanticAutomationFallbackReason::Deferred,
            }
        );
        assert!(first.composition.normalized_sidecar().entries().is_empty());
        assert_eq!(calls.get(), 1);

        *outcome.borrow_mut() = VirtualLayoutSemanticQueryOutcome::Found(Box::new(semantic_entry(
            7,
            Rect::from_xy_size(4.0, 4.0, 24.0, 10.0),
        )));
        let retry = state
            .retry_semantic_automation(902, session, &ordinary, (1, 2, 3))
            .expect("explicit retry should execute the provider again");
        assert_eq!(retry.status, SemanticAutomationRefreshStatus::Published);
        assert_eq!(retry.composition.normalized_sidecar().entries().len(), 1);
        assert_eq!(calls.get(), 2);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn exact_range_retry_reexecutes_only_one_member_and_preserves_atomic_selection() {
        let (mut state, calls_a, calls_b, outcome_a, outcome_b) = two_range_state();
        let session = state
            .open_semantic_automation_session(905)
            .expect("one session should open");
        let containers = state
            .semantic_automation_containers(905, session)
            .expect("both mounted containers should enumerate");
        let container_a = containers
            .iter()
            .find(|container| container.container_id == CONTAINER_ID)
            .copied()
            .expect("container A should be present");
        let container_b = containers
            .iter()
            .find(|container| container.container_id == CONTAINER_ID + 1)
            .copied()
            .expect("container B should be present");
        let ordinary = semantic_publication_snapshot_for(&[CONTAINER_ID, CONTAINER_ID + 1]);
        let initial = state
            .refresh_semantic_automation(
                session.runtime_id,
                session,
                &[
                    SemanticAutomationDemand::range(container_a, 0, 2),
                    SemanticAutomationDemand::range(container_b, 2, 2),
                ],
                &ordinary,
                (1, 2, 3),
            )
            .expect("the complete A+B refresh should publish");
        assert_eq!(initial.status, SemanticAutomationRefreshStatus::Published);
        assert_eq!(calls_a.get(), 1);
        assert_eq!(calls_b.get(), 1);

        let b_fence_before = state
            .semantic_demand
            .active_range_fence(container_b.container_id)
            .expect("B range slot should be active");
        let b_attempt_before = b_fence_before.attempt;

        *outcome_a.borrow_mut() = VirtualLayoutSemanticRangeProviderOutcome::Found(
            semantic_range_entries_with_prefix("a-retry", 0, 2),
        );
        let exact_retry = state
            .retry_semantic_automation_range(
                session.runtime_id,
                session,
                container_a,
                0,
                2,
                &ordinary,
                (1, 2, 3),
            )
            .expect("the exact A retry should publish the complete surface");
        assert_eq!(
            exact_retry.status,
            SemanticAutomationRefreshStatus::Published
        );
        assert_eq!(calls_a.get(), 2);
        assert_eq!(calls_b.get(), 1);

        let b_fence_after = state
            .semantic_demand
            .active_range_fence(container_b.container_id)
            .expect("B range slot should remain active");
        assert_eq!(b_fence_after, b_fence_before);
        assert_eq!(b_fence_after.attempt, b_attempt_before);
        let a_anchor = exact_retry
            .composition
            .snapshot()
            .root
            .children
            .iter()
            .find(|node| node.id == AutomationNodeId::new(CONTAINER_ID.to_string()))
            .expect("the A anchor should remain in the composed surface");
        let b_anchor = exact_retry
            .composition
            .snapshot()
            .root
            .children
            .iter()
            .find(|node| node.id == AutomationNodeId::new((CONTAINER_ID + 1).to_string()))
            .expect("the B anchor should remain in the composed surface");
        assert!(
            a_anchor
                .children
                .iter()
                .any(|child| child.id == AutomationNodeId::new("a-retry-0"))
        );
        assert!(
            b_anchor
                .children
                .iter()
                .any(|child| child.id == AutomationNodeId::new("b-initial-2"))
        );

        let selected_before_rejection = state
            .selected_semantic_automation(session.runtime_id, session, &ordinary, 3)
            .expect("selected read should succeed")
            .expect("the exact retry should remain selected");
        assert!(matches!(
            state.retry_semantic_automation_range(
                session.runtime_id,
                session,
                container_a,
                1,
                2,
                &ordinary,
                (1, 2, 3),
            ),
            Err(SemanticAutomationSessionError::StaleContainerHandle)
        ));
        assert!(matches!(
            state.retry_semantic_automation_range(
                session.runtime_id,
                session,
                container_b,
                0,
                2,
                &ordinary,
                (1, 2, 3),
            ),
            Err(SemanticAutomationSessionError::StaleContainerHandle)
        ));
        let stale_session = SemanticAutomationSessionHandle {
            runtime_id: session.runtime_id,
            generation: session.generation + 1,
        };
        assert!(matches!(
            state.retry_semantic_automation_range(
                session.runtime_id,
                stale_session,
                container_a,
                0,
                2,
                &ordinary,
                (1, 2, 3),
            ),
            Err(SemanticAutomationSessionError::UnknownSession)
        ));
        assert_eq!(calls_a.get(), 2);
        assert_eq!(calls_b.get(), 1);
        let selected_after_rejection = state
            .selected_semantic_automation(session.runtime_id, session, &ordinary, 3)
            .expect("selected read after rejection should succeed")
            .expect("the prior selection should remain selected");
        assert_eq!(
            selected_after_rejection.composition,
            selected_before_rejection.composition
        );

        *outcome_b.borrow_mut() = VirtualLayoutSemanticRangeProviderOutcome::Found(
            semantic_range_entries_with_prefix("b-whole", 2, 2),
        );
        let whole_retry = state
            .retry_semantic_automation(session.runtime_id, session, &ordinary, (1, 2, 3))
            .expect("the public whole-session retry should retain its behavior");
        assert_eq!(
            whole_retry.status,
            SemanticAutomationRefreshStatus::Published
        );
        assert_eq!(calls_a.get(), 3);
        assert_eq!(calls_b.get(), 2);
    }

    #[test]
    fn semantic_range_classification_preserves_order_and_exact_origin_without_side_effects() {
        let cases = [
            (
                vec![
                    (VirtualLayoutItemKey::new(100_u32), 0),
                    (VirtualLayoutItemKey::new(102_u32), 2),
                ],
                [true, false, true],
            ),
            (
                vec![
                    (VirtualLayoutItemKey::new(100_u32), 0),
                    (VirtualLayoutItemKey::new(101_u32), 1),
                    (VirtualLayoutItemKey::new(102_u32), 2),
                ],
                [true, true, true],
            ),
            (Vec::new(), [false, false, false]),
        ];

        for (materialized, expected_materialized) in cases {
            let entries = valid_semantic_range_entries(0, 3);
            let (state, batch, semantic_calls, policy_calls) =
                materialized_state_and_batch(materialized, entries.clone());
            let fence_before = state.records[0]
                .materialization
                .authoritative_fence()
                .cloned();
            let active_before = state.records[0]
                .materialization
                .active_slots()
                .into_iter()
                .map(|slot| (slot.item().logical_index(), slot.payload().id()))
                .collect::<Vec<_>>();
            let last_query_before = query_parts_snapshot(state.records[0].last_query.as_ref());
            let retired_before = state.records[0].retired;

            let classified = state
                .classify_virtual_layout_semantic_range(&batch)
                .expect("the matching materialization fence should classify");

            assert_eq!(semantic_calls.get(), 1);
            assert_eq!(policy_calls.get(), 1);
            assert_eq!(classified.request(), batch.request());
            assert_eq!(classified.classifications().len(), entries.len());
            for (index, (classification, expected_materialized)) in classified
                .classifications()
                .iter()
                .zip(expected_materialized)
                .enumerate()
            {
                let projection = classification.projection();
                assert_eq!(projection.logical_index(), index);
                assert_eq!(projection.identity().key(), entries[index].requested_key());
                assert_eq!(projection.bounds(), entries[index].bounds());
                assert_eq!(projection.semantics(), entries[index].semantics());
                assert_eq!(
                    projection.automation_node_id(),
                    entries[index].automation_node_id()
                );
                assert_eq!(
                    projection.coordinate_space(),
                    batch.request().coordinate_space()
                );
                assert_eq!(projection.range_request(), Some(batch.request()));
                assert_eq!(
                    projection.authority(),
                    VirtualLayoutSemanticProjectionAuthority::Unmaterialized
                );
                match (classification.origin(), expected_materialized) {
                    (
                        VirtualLayoutSemanticClassificationOrigin::Materialized {
                            slot_identity,
                            payload_root,
                        },
                        true,
                    ) => {
                        let slot = state.records[0]
                            .materialization
                            .active_slots()
                            .into_iter()
                            .find(|slot| slot.item().logical_index() == index)
                            .expect("the exact materialized index should be retained");
                        assert_eq!(slot_identity, slot.identity());
                        assert_eq!(payload_root, slot.payload().id());
                    }
                    (VirtualLayoutSemanticClassificationOrigin::Unmaterialized, false) => {}
                    (origin, expected) => {
                        panic!("unexpected classification origin {origin:?}, expected {expected}");
                    }
                }
            }

            assert_eq!(
                state.records[0]
                    .materialization
                    .authoritative_fence()
                    .cloned(),
                fence_before
            );
            assert_eq!(
                state.records[0]
                    .materialization
                    .active_slots()
                    .into_iter()
                    .map(|slot| (slot.item().logical_index(), slot.payload().id()))
                    .collect::<Vec<_>>(),
                active_before
            );
            assert_eq!(
                query_parts_snapshot(state.records[0].last_query.as_ref()),
                last_query_before
            );
            assert_eq!(state.records[0].retired, retired_before);
        }
    }

    #[test]
    fn semantic_publication_reclassifies_retained_pin_without_provider_reentry() {
        let (provider, calls, outcome) =
            semantic_provider(VirtualLayoutSemanticQueryOutcome::Found(Box::new(
                semantic_entry(1, Rect::from_xy_size(4.0, 0.0, 24.0, 10.0)),
            )));
        let mut state = semantic_pin_publication_state(provider);
        state.synchronize_semantic_demand();
        let ticket = match state
            .semantic_demand
            .semantic_pin(CONTAINER_ID, VirtualLayoutItemKey::new(1_u32))
            .expect("pin demand")
        {
            SemanticDemandAdmission::Started(ticket) => ticket,
            SemanticDemandAdmission::Unchanged => panic!("pin demand should start"),
        };
        let completion = state
            .semantic_demand
            .execute(ticket)
            .expect("pin attempt executes once");
        assert!(matches!(
            state.semantic_demand.complete(completion),
            SemanticDemandCompletion::RequiredItemPin(VirtualLayoutSemanticQueryOutcome::Found(_))
        ));

        let ordinary = semantic_publication_snapshot();
        let first = state.compose_semantic_publication(
            &ordinary,
            SemanticPublicationAuthorities {
                session_generation: 1,
                materialization_authority: 1,
                classification_authority: 2,
                ordinary_projection_generation: 3,
            },
        );
        assert!(matches!(first, SemanticPublicationOutcome::Published(_)));
        assert_eq!(calls.get(), 1);

        *outcome.borrow_mut() = VirtualLayoutSemanticQueryOutcome::Deferred(
            VirtualLayoutSemanticDeferredReason::SemanticPending,
        );
        let retry = state
            .semantic_demand
            .retry_semantic_pin(CONTAINER_ID)
            .expect("retry retained pin");
        let completion = state
            .semantic_demand
            .execute(retry)
            .expect("retry executes once");
        assert!(matches!(
            state.semantic_demand.complete(completion),
            SemanticDemandCompletion::RequiredItemPin(VirtualLayoutSemanticQueryOutcome::Deferred(
                _
            ))
        ));
        assert_eq!(calls.get(), 2);

        let second = state.compose_semantic_publication(
            &ordinary,
            SemanticPublicationAuthorities {
                session_generation: 1,
                materialization_authority: 4,
                classification_authority: 5,
                ordinary_projection_generation: 6,
            },
        );
        let SemanticPublicationOutcome::Published(composition) = second else {
            panic!("eligible retained pin evidence should reclassify and publish");
        };
        assert_eq!(calls.get(), 2);
        assert_eq!(
            composition.snapshot().root.children[0]
                .children
                .iter()
                .map(|child| child.id.clone())
                .collect::<Vec<_>>(),
            vec![AutomationNodeId::new("semantic-item")]
        );
    }

    #[test]
    fn runtime_composition_rejects_forged_materialized_origin_before_staging() {
        let (state, projection_batch, semantic_calls, policy_calls) = materialized_state_and_batch(
            vec![(VirtualLayoutItemKey::new(100_u32), 0)],
            valid_semantic_range_entries(0, 1),
        );
        let classified = state
            .classify_virtual_layout_semantic_range(&projection_batch)
            .expect("the live materialization should classify the range");
        let source = GuiAutomationSnapshot {
            schema_version: 2,
            viewport_width: 160,
            viewport_height: 80,
            root: AutomationNodeSnapshot::from_semantics(
                AutomationNodeId::new("root"),
                AutomationBounds {
                    x: 0.0,
                    y: 0.0,
                    width: 160.0,
                    height: 80.0,
                },
                AutomationNodeSemantics::new(AutomationRole::Group),
            ),
        };
        let source_before = source.clone();
        let fence_before = state.records[0]
            .materialization
            .authoritative_fence()
            .cloned();
        let active_before = state.records[0]
            .materialization
            .active_slots()
            .into_iter()
            .map(|slot| (slot.item().logical_index(), slot.payload().id()))
            .collect::<Vec<_>>();
        let last_query_before = query_parts_snapshot(state.records[0].last_query.as_ref());
        let retired_before = state.records[0].retired;

        let mut forged = classified;
        let VirtualLayoutSemanticClassificationOrigin::Materialized {
            slot_identity,
            payload_root,
        } = forged.classifications[0].origin
        else {
            panic!("the fixture should produce materialized origin evidence");
        };
        forged.classifications[0].origin =
            VirtualLayoutSemanticClassificationOrigin::Materialized {
                slot_identity,
                payload_root: payload_root + 1,
            };

        assert_eq!(
            state.compose_virtual_layout_automation_snapshot(&source, &[forged]),
            Err(
                crate::runtime::controller::VirtualLayoutAutomationCompositionError::
                    LiveClassificationMismatch
            )
        );
        assert_eq!(source, source_before);
        assert_eq!(
            state.records[0]
                .materialization
                .authoritative_fence()
                .cloned(),
            fence_before
        );
        assert_eq!(
            state.records[0]
                .materialization
                .active_slots()
                .into_iter()
                .map(|slot| (slot.item().logical_index(), slot.payload().id()))
                .collect::<Vec<_>>(),
            active_before
        );
        assert_eq!(
            query_parts_snapshot(state.records[0].last_query.as_ref()),
            last_query_before
        );
        assert_eq!(state.records[0].retired, retired_before);
        assert_eq!(semantic_calls.get(), 1);
        assert_eq!(policy_calls.get(), 1);
    }

    #[test]
    fn semantic_range_classification_rejects_each_materialization_fence_mismatch() {
        fn mismatch_container(record: &mut RuntimeVirtualLayoutRecord<()>) {
            record.registration.container_id += 1;
        }
        fn mismatch_policy(record: &mut RuntimeVirtualLayoutRecord<()>) {
            record.registration.policy_identity = VirtualLayoutPolicyIdentity::new("other-policy");
        }
        fn mismatch_mount(record: &mut RuntimeVirtualLayoutRecord<()>) {
            record.mount_generation += 1;
        }
        fn mismatch_data(record: &mut RuntimeVirtualLayoutRecord<()>) {
            record.registration.revisions.data += 1;
        }
        fn mismatch_policy_revision(record: &mut RuntimeVirtualLayoutRecord<()>) {
            record.registration.revisions.policy += 1;
        }
        fn mismatch_measurement(record: &mut RuntimeVirtualLayoutRecord<()>) {
            record.registration.revisions.measurement += 1;
        }
        fn mismatch_semantic(record: &mut RuntimeVirtualLayoutRecord<()>) {
            record.registration.revisions.semantic += 1;
        }
        fn mismatch_coordinate(record: &mut RuntimeVirtualLayoutRecord<()>) {
            record.registration.coordinate_space = VirtualLayoutCoordinateSpace::custom(
                VirtualLayoutPolicyIdentity::new("other-coordinate-space"),
            );
        }
        fn mismatch_budget(record: &mut RuntimeVirtualLayoutRecord<()>) {
            record.registration.budget = VirtualLayoutBudget::new(3);
        }

        type SemanticFenceMismatchCase = (
            VirtualLayoutSemanticClassificationFenceField,
            fn(&mut RuntimeVirtualLayoutRecord<()>),
        );
        let cases: &[SemanticFenceMismatchCase] = &[
            (
                VirtualLayoutSemanticClassificationFenceField::ContainerIdentity,
                mismatch_container,
            ),
            (
                VirtualLayoutSemanticClassificationFenceField::PolicyIdentity,
                mismatch_policy,
            ),
            (
                VirtualLayoutSemanticClassificationFenceField::MountGeneration,
                mismatch_mount,
            ),
            (
                VirtualLayoutSemanticClassificationFenceField::DataRevision,
                mismatch_data,
            ),
            (
                VirtualLayoutSemanticClassificationFenceField::PolicyRevision,
                mismatch_policy_revision,
            ),
            (
                VirtualLayoutSemanticClassificationFenceField::MeasurementRevision,
                mismatch_measurement,
            ),
            (
                VirtualLayoutSemanticClassificationFenceField::SemanticRevision,
                mismatch_semantic,
            ),
            (
                VirtualLayoutSemanticClassificationFenceField::CoordinateSpace,
                mismatch_coordinate,
            ),
            (
                VirtualLayoutSemanticClassificationFenceField::Budget,
                mismatch_budget,
            ),
        ];

        for &(field, mutate) in cases {
            let (mut state, batch, semantic_calls, policy_calls) = materialized_state_and_batch(
                vec![(VirtualLayoutItemKey::new(100_u32), 0)],
                valid_semantic_range_entries(0, 1),
            );
            mutate(&mut state.records[0]);
            assert_eq!(
                state.records[0].classify_semantic_range(&batch),
                Err(VirtualLayoutSemanticClassificationError::FenceMismatch(
                    field
                ))
            );
            assert_eq!(semantic_calls.get(), 1);
            assert_eq!(policy_calls.get(), 1);
        }
    }

    #[test]
    fn semantic_range_classification_rejects_key_index_incoherence_atomically() {
        let cases = [
            (
                vec![(VirtualLayoutItemKey::new(100_u32), 1)],
                VirtualLayoutSemanticClassificationError::KeyIndexMismatch,
            ),
            (
                vec![(VirtualLayoutItemKey::new(101_u32), 0)],
                VirtualLayoutSemanticClassificationError::UnmatchedMaterializedSlot,
            ),
        ];
        for (materialized, expected_error) in cases {
            let (state, batch, semantic_calls, policy_calls) = materialized_state_and_batch(
                materialized,
                vec![semantic_range_entry_with_id(
                    100,
                    0,
                    Rect::from_xy_size(4.0, 0.0, 24.0, 10.0),
                    AutomationNodeId::new("classification-zero"),
                )],
            );
            let fence_before = state.records[0]
                .materialization
                .authoritative_fence()
                .cloned();
            let active_before = state.records[0].materialization.active_len();
            assert_eq!(
                state.classify_virtual_layout_semantic_range(&batch),
                Err(expected_error)
            );
            assert_eq!(semantic_calls.get(), 1);
            assert_eq!(policy_calls.get(), 1);
            assert_eq!(state.records[0].materialization.active_len(), active_before);
            assert_eq!(
                state.records[0]
                    .materialization
                    .authoritative_fence()
                    .cloned(),
                fence_before
            );
        }
    }

    #[test]
    fn semantic_range_classification_rejects_malformed_batch_without_partial_output() {
        let (state, batch, semantic_calls, policy_calls) = materialized_state_and_batch(
            vec![(VirtualLayoutItemKey::new(100_u32), 0)],
            valid_semantic_range_entries(0, 1),
        );
        let malformed =
            VirtualLayoutSemanticProjectionBatch::new(batch.request().clone(), Vec::new());
        let active_before = state.records[0].materialization.active_len();
        assert_eq!(
            state.classify_virtual_layout_semantic_range(&malformed),
            Err(VirtualLayoutSemanticClassificationError::MalformedBatch)
        );
        assert_eq!(state.records[0].materialization.active_len(), active_before);
        assert_eq!(semantic_calls.get(), 1);
        assert_eq!(policy_calls.get(), 1);
    }

    #[test]
    fn semantic_range_classification_rejects_unstable_key_equality_without_provider_reentry() {
        let unstable = Rc::new(Cell::new(false));
        let comparisons = Rc::new(Cell::new(0));
        let key = VirtualLayoutItemKey::new(FlakyKey {
            unstable: Rc::clone(&unstable),
            comparisons: Rc::clone(&comparisons),
        });
        let (state, batch, semantic_calls, policy_calls) = materialized_state_and_batch(
            vec![(key.clone(), 0)],
            vec![semantic_range_entry_with_key(
                key,
                0,
                AutomationNodeId::new("flaky-key"),
            )],
        );
        unstable.set(true);
        let active_before = state.records[0].materialization.active_len();
        assert_eq!(
            state.classify_virtual_layout_semantic_range(&batch),
            Err(VirtualLayoutSemanticClassificationError::UnstableKey)
        );
        assert!(comparisons.get() > 0);
        assert_eq!(semantic_calls.get(), 1);
        assert_eq!(policy_calls.get(), 1);
        assert_eq!(state.records[0].materialization.active_len(), active_before);
    }

    #[test]
    fn semantic_range_classification_rejects_unstable_key_with_empty_authority() {
        let unstable = Rc::new(Cell::new(false));
        let comparisons = Rc::new(Cell::new(0));
        let key = VirtualLayoutItemKey::new(FlakyKey {
            unstable: Rc::clone(&unstable),
            comparisons: Rc::clone(&comparisons),
        });
        let (state, batch, semantic_calls, policy_calls) = materialized_state_and_batch(
            Vec::new(),
            vec![semantic_range_entry_with_key(
                key,
                0,
                AutomationNodeId::new("flaky-empty-authority"),
            )],
        );
        assert_eq!(semantic_calls.get(), 1);
        assert_eq!(policy_calls.get(), 1);
        unstable.set(true);

        let fence_before = state.records[0]
            .materialization
            .authoritative_fence()
            .cloned();
        let active_before = state.records[0]
            .materialization
            .active_slots()
            .into_iter()
            .map(|slot| (slot.item().logical_index(), slot.payload().id()))
            .collect::<Vec<_>>();
        let last_query_before = query_parts_snapshot(state.records[0].last_query.as_ref());
        let retired_before = state.records[0].retired;
        assert!(fence_before.is_some());
        assert!(active_before.is_empty());

        assert_eq!(
            state.classify_virtual_layout_semantic_range(&batch),
            Err(VirtualLayoutSemanticClassificationError::UnstableKey)
        );
        assert!(comparisons.get() > 0);
        assert_eq!(semantic_calls.get(), 1);
        assert_eq!(policy_calls.get(), 1);
        assert_eq!(
            state.records[0]
                .materialization
                .authoritative_fence()
                .cloned(),
            fence_before
        );
        assert_eq!(
            state.records[0]
                .materialization
                .active_slots()
                .into_iter()
                .map(|slot| (slot.item().logical_index(), slot.payload().id()))
                .collect::<Vec<_>>(),
            active_before
        );
        assert_eq!(
            query_parts_snapshot(state.records[0].last_query.as_ref()),
            last_query_before
        );
        assert_eq!(state.records[0].retired, retired_before);
    }

    #[test]
    fn semantic_range_classification_rejects_pairwise_duplicate_or_unstable_keys_atomically() {
        let (state, admitted_batch, semantic_calls, policy_calls) =
            materialized_state_and_batch(Vec::new(), valid_semantic_range_entries(0, 2));
        let duplicate_key = VirtualLayoutItemKey::new(100_u32);
        let duplicate_batch = projection_batch_from_keys(
            admitted_batch.request(),
            &[duplicate_key.clone(), duplicate_key],
        );
        let comparisons = Rc::new(Cell::new(0));
        let unstable_batch = projection_batch_from_keys(
            admitted_batch.request(),
            &[
                VirtualLayoutItemKey::new(PairwiseUnstableKey {
                    id: 0,
                    comparisons: Rc::clone(&comparisons),
                }),
                VirtualLayoutItemKey::new(PairwiseUnstableKey {
                    id: 1,
                    comparisons: Rc::clone(&comparisons),
                }),
            ],
        );
        let cases = [
            (
                duplicate_batch,
                VirtualLayoutSemanticClassificationError::AmbiguousMaterialization,
            ),
            (
                unstable_batch,
                VirtualLayoutSemanticClassificationError::UnstableKey,
            ),
        ];
        let fence_before = state.records[0]
            .materialization
            .authoritative_fence()
            .cloned();
        let active_before = state.records[0].materialization.active_len();

        for (batch, expected_error) in cases {
            assert_eq!(
                state.classify_virtual_layout_semantic_range(&batch),
                Err(expected_error)
            );
            assert_eq!(semantic_calls.get(), 1);
            assert_eq!(policy_calls.get(), 1);
            assert_eq!(
                state.records[0]
                    .materialization
                    .authoritative_fence()
                    .cloned(),
                fence_before
            );
            assert_eq!(state.records[0].materialization.active_len(), active_before);
        }
        assert!(comparisons.get() > 0);
    }

    #[test]
    fn semantic_range_classification_rejects_retired_and_authorityless_materialization() {
        let (mut retired_state, retired_batch, semantic_calls, policy_calls) =
            materialized_state_and_batch(
                vec![(VirtualLayoutItemKey::new(100_u32), 0)],
                valid_semantic_range_entries(0, 1),
            );
        retired_state.records[0].retire();
        assert_eq!(
            retired_state.classify_virtual_layout_semantic_range(&retired_batch),
            Err(VirtualLayoutSemanticClassificationError::Retired)
        );
        assert_eq!(semantic_calls.get(), 1);
        assert_eq!(policy_calls.get(), 1);

        let (provider, authorityless_calls, _) = semantic_range_provider(
            VirtualLayoutSemanticRangeProviderOutcome::Found(valid_semantic_range_entries(0, 1)),
        );
        let mut authorityless_state = semantic_range_state(
            provider,
            VirtualLayoutCoordinateSpace::logical(),
            VirtualLayoutBudget::new(4),
        );
        let VirtualLayoutSemanticRangeQueryOutcome::Found(authorityless_batch) =
            authorityless_state.admit_current_semantic_range(CONTAINER_ID, 0, 1)
        else {
            panic!("the authority-less fixture should admit semantic evidence");
        };
        assert_eq!(
            authorityless_state.classify_virtual_layout_semantic_range(&authorityless_batch),
            Err(VirtualLayoutSemanticClassificationError::MaterializationAuthorityUnavailable)
        );
        assert_eq!(authorityless_calls.get(), 1);
    }

    struct FlakyKey {
        unstable: Rc<Cell<bool>>,
        comparisons: Rc<Cell<u32>>,
    }

    impl PartialEq for FlakyKey {
        fn eq(&self, _other: &Self) -> bool {
            let comparison = self.comparisons.get();
            self.comparisons.set(comparison.saturating_add(1));
            !self.unstable.get() || comparison.is_multiple_of(2)
        }
    }

    impl Eq for FlakyKey {}

    struct PairwiseUnstableKey {
        id: u8,
        comparisons: Rc<Cell<u32>>,
    }

    impl PartialEq for PairwiseUnstableKey {
        fn eq(&self, other: &Self) -> bool {
            if self.id == other.id {
                return true;
            }
            let comparison = self.comparisons.get();
            self.comparisons.set(comparison.saturating_add(1));
            comparison.is_multiple_of(2)
        }
    }

    impl Eq for PairwiseUnstableKey {}

    #[test]
    fn semantic_range_success_is_ordered_fenced_and_coordinate_declared() {
        for coordinate_space in [
            VirtualLayoutCoordinateSpace::logical(),
            VirtualLayoutCoordinateSpace::custom(VirtualLayoutPolicyIdentity::new(
                "timeline-canvas",
            )),
        ] {
            let entries = valid_semantic_range_entries(2, 4);
            let (provider, calls, _) = semantic_range_provider(
                VirtualLayoutSemanticRangeProviderOutcome::Found(entries.clone()),
            );
            let mut state = semantic_range_state(
                provider.clone(),
                coordinate_space.clone(),
                VirtualLayoutBudget::new(4),
            );

            let outcome = state.admit_current_semantic_range(CONTAINER_ID, 2, 4);
            let VirtualLayoutSemanticRangeQueryOutcome::Found(batch) = outcome else {
                panic!("the bounded range should be accepted");
            };
            assert_eq!(calls.get(), 1);
            assert_eq!(
                provider.requests.borrow().as_slice(),
                &[batch.request().clone()]
            );
            assert_eq!(batch.request().range().start_index(), 2);
            assert_eq!(batch.request().range().length(), 4);
            assert_eq!(batch.request().range().end_index(), 6);
            assert_eq!(batch.request().coordinate_space(), &coordinate_space);
            assert_eq!(batch.projections().len(), 4);

            for (offset, projection) in batch.projections().iter().enumerate() {
                let entry = &entries[offset];
                assert_eq!(projection.identity().container_id(), CONTAINER_ID);
                assert_eq!(projection.identity().key(), entry.requested_key());
                assert_eq!(projection.coordinate_space(), &coordinate_space);
                assert_eq!(projection.logical_index(), 2 + offset);
                assert_eq!(projection.bounds(), entry.bounds());
                assert_eq!(projection.semantics(), entry.semantics());
                assert_eq!(projection.automation_node_id(), entry.automation_node_id());
                assert_eq!(projection.range_request(), Some(batch.request()));
                assert_eq!(
                    projection.authority(),
                    VirtualLayoutSemanticProjectionAuthority::Unmaterialized
                );
            }
            assert!(state.records[0].pin.is_none());
        }
    }

    #[test]
    fn semantic_range_rejects_invalid_request_sizes_before_provider_invocation() {
        let (provider, calls, _) = semantic_range_provider(
            VirtualLayoutSemanticRangeProviderOutcome::Found(valid_semantic_range_entries(0, 4)),
        );
        let mut state = semantic_range_state(
            provider,
            VirtualLayoutCoordinateSpace::logical(),
            VirtualLayoutBudget::new(4),
        );

        for (start_index, length, reason) in [
            (0, 0, VirtualLayoutSemanticRejectedReason::RangeLengthZero),
            (
                usize::MAX,
                1,
                VirtualLayoutSemanticRejectedReason::RangeIndexOverflow,
            ),
            (0, 5, VirtualLayoutSemanticRejectedReason::RangeOverBudget),
        ] {
            assert_eq!(
                state.admit_current_semantic_range(CONTAINER_ID, start_index, length),
                VirtualLayoutSemanticRangeQueryOutcome::Rejected(reason)
            );
        }
        assert_eq!(calls.get(), 0);

        let (hard_cap_provider, hard_cap_calls, _) =
            semantic_range_provider(VirtualLayoutSemanticRangeProviderOutcome::Found(Vec::new()));
        let mut hard_cap_state = semantic_range_state(
            hard_cap_provider,
            VirtualLayoutCoordinateSpace::logical(),
            VirtualLayoutBudget::new(VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES + 1),
        );
        assert_eq!(
            hard_cap_state.admit_current_semantic_range(
                CONTAINER_ID,
                0,
                VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES + 1,
            ),
            VirtualLayoutSemanticRangeQueryOutcome::Rejected(
                VirtualLayoutSemanticRejectedReason::RangeOverBudget
            )
        );
        assert_eq!(hard_cap_calls.get(), 0);
    }

    #[test]
    fn semantic_range_rejects_missing_retired_unstable_stale_and_missing_provider() {
        let (provider, calls, _) = semantic_range_provider(
            VirtualLayoutSemanticRangeProviderOutcome::Found(valid_semantic_range_entries(0, 1)),
        );
        let mut state = semantic_range_state(
            provider,
            VirtualLayoutCoordinateSpace::logical(),
            VirtualLayoutBudget::new(4),
        );
        assert_eq!(
            state.admit_current_semantic_range(CONTAINER_ID + 1, 0, 1),
            VirtualLayoutSemanticRangeQueryOutcome::Rejected(
                VirtualLayoutSemanticRejectedReason::UnknownContainer
            )
        );
        state.records[0].retire();
        assert_eq!(
            state.admit_current_semantic_range(CONTAINER_ID, 0, 1),
            VirtualLayoutSemanticRangeQueryOutcome::Rejected(
                VirtualLayoutSemanticRejectedReason::Retired
            )
        );
        assert_eq!(calls.get(), 0);

        let (unstable_provider, unstable_calls, _) =
            semantic_range_provider(VirtualLayoutSemanticRangeProviderOutcome::Found(vec![
                VirtualLayoutSemanticEntry::new(
                    VirtualLayoutItemKey::new(UnstableKey),
                    0,
                    Rect::from_xy_size(0.0, 0.0, 10.0, 10.0),
                    AutomationNodeSemantics::new(AutomationRole::Button),
                    AutomationNodeId::new("unstable-range-key"),
                ),
            ]));
        let mut unstable_state = semantic_range_state(
            unstable_provider,
            VirtualLayoutCoordinateSpace::logical(),
            VirtualLayoutBudget::new(4),
        );
        assert_eq!(
            unstable_state.admit_current_semantic_range(CONTAINER_ID, 0, 1),
            VirtualLayoutSemanticRangeQueryOutcome::Rejected(
                VirtualLayoutSemanticRejectedReason::UnstableKey
            )
        );
        assert_eq!(unstable_calls.get(), 1);

        let (stale_provider, stale_calls, _) =
            semantic_range_provider(VirtualLayoutSemanticRangeProviderOutcome::NotFound);
        let mut stale_state = semantic_range_state(
            stale_provider,
            VirtualLayoutCoordinateSpace::logical(),
            VirtualLayoutBudget::new(4),
        );
        let mut stale_request = semantic_range_request(&stale_state, 0, 1);
        stale_request = VirtualLayoutSemanticRangeRequest::new(
            stale_request.container_id(),
            VirtualLayoutPolicyIdentity::new("other-policy"),
            stale_request.mount_generation(),
            stale_request.data_revision(),
            stale_request.policy_revision(),
            stale_request.measurement_revision(),
            stale_request.semantic_revision(),
            stale_request.coordinate_space().clone(),
            stale_request.budget(),
            stale_request.range(),
        );
        assert_eq!(
            stale_state.query_semantic_range(&stale_request),
            VirtualLayoutSemanticRangeQueryOutcome::Rejected(
                VirtualLayoutSemanticRejectedReason::ScopeMismatch
            )
        );
        assert_eq!(stale_calls.get(), 0);

        let mut no_provider = RuntimeVirtualLayoutState::default();
        no_provider.records.push(RuntimeVirtualLayoutRecord::new(
            registration(
                Rc::new(ReadyPolicy {
                    calls: Rc::new(Cell::new(0)),
                    key: 1,
                }),
                VirtualLayoutPolicyIdentity::new("semantic-range-policy"),
            ),
            SEMANTIC_MOUNT_GENERATION,
        ));
        assert_eq!(
            no_provider.admit_current_semantic_range(CONTAINER_ID, 0, 1),
            VirtualLayoutSemanticRangeQueryOutcome::Unavailable(
                VirtualLayoutSemanticUnavailableReason::NoProvider
            )
        );
    }

    #[test]
    fn semantic_range_terminal_provider_outcomes_are_typed_and_atomic() {
        let outcomes = [
            (
                VirtualLayoutSemanticRangeProviderOutcome::NotFound,
                VirtualLayoutSemanticRangeQueryOutcome::NotFound,
            ),
            (
                VirtualLayoutSemanticRangeProviderOutcome::Unavailable(
                    VirtualLayoutSemanticUnavailableReason::DataUnavailable,
                ),
                VirtualLayoutSemanticRangeQueryOutcome::Unavailable(
                    VirtualLayoutSemanticUnavailableReason::DataUnavailable,
                ),
            ),
            (
                VirtualLayoutSemanticRangeProviderOutcome::Deferred(
                    VirtualLayoutSemanticDeferredReason::SemanticPending,
                ),
                VirtualLayoutSemanticRangeQueryOutcome::Deferred(
                    VirtualLayoutSemanticDeferredReason::SemanticPending,
                ),
            ),
            (
                VirtualLayoutSemanticRangeProviderOutcome::Rejected(
                    VirtualLayoutSemanticRejectedReason::ProviderRejected,
                ),
                VirtualLayoutSemanticRangeQueryOutcome::Rejected(
                    VirtualLayoutSemanticRejectedReason::ProviderRejected,
                ),
            ),
        ];
        for (provider_outcome, expected) in outcomes {
            let (provider, calls, _) = semantic_range_provider(provider_outcome);
            let mut state = semantic_range_state(
                provider,
                VirtualLayoutCoordinateSpace::logical(),
                VirtualLayoutBudget::new(4),
            );
            assert_eq!(
                state.admit_current_semantic_range(CONTAINER_ID, 0, 2),
                expected
            );
            assert_eq!(calls.get(), 1);
            assert!(state.records[0].pin.is_none());
        }
    }

    #[test]
    fn semantic_range_rejects_short_long_order_index_duplicate_unstable_and_malformed_output() {
        let valid = || valid_semantic_range_entries(0, 4);
        let mut cases = Vec::new();
        cases.push((
            valid()[..3].to_vec(),
            VirtualLayoutSemanticRejectedReason::RangeCountMismatch,
        ));
        let mut long = valid();
        long.push(semantic_range_entry(
            104,
            4,
            Rect::from_xy_size(0.0, 48.0, 10.0, 10.0),
        ));
        cases.push((
            long,
            VirtualLayoutSemanticRejectedReason::RangeCountMismatch,
        ));
        cases.push((
            vec![
                semantic_range_entry(100, 0, Rect::from_xy_size(0.0, 0.0, 10.0, 10.0)),
                semantic_range_entry(102, 2, Rect::from_xy_size(0.0, 20.0, 10.0, 10.0)),
                semantic_range_entry(101, 1, Rect::from_xy_size(0.0, 10.0, 10.0, 10.0)),
                semantic_range_entry(103, 3, Rect::from_xy_size(0.0, 30.0, 10.0, 10.0)),
            ],
            VirtualLayoutSemanticRejectedReason::RangeOutOfOrder,
        ));
        cases.push((
            vec![
                semantic_range_entry_with_id(
                    100,
                    0,
                    Rect::from_xy_size(0.0, 0.0, 10.0, 10.0),
                    AutomationNodeId::new("wrong-index-zero"),
                ),
                semantic_range_entry_with_id(
                    101,
                    1,
                    Rect::from_xy_size(0.0, 10.0, 10.0, 10.0),
                    AutomationNodeId::new("wrong-index-one"),
                ),
                semantic_range_entry_with_id(
                    102,
                    2,
                    Rect::from_xy_size(0.0, 20.0, 10.0, 10.0),
                    AutomationNodeId::new("wrong-index-two"),
                ),
                semantic_range_entry_with_id(
                    104,
                    4,
                    Rect::from_xy_size(0.0, 40.0, 10.0, 10.0),
                    AutomationNodeId::new("wrong-index-four"),
                ),
            ],
            VirtualLayoutSemanticRejectedReason::WrongLogicalIndex,
        ));
        cases.push((
            vec![
                semantic_range_entry(100, 0, Rect::from_xy_size(0.0, 0.0, 10.0, 10.0)),
                semantic_range_entry(100, 1, Rect::from_xy_size(0.0, 10.0, 10.0, 10.0)),
                semantic_range_entry(102, 2, Rect::from_xy_size(0.0, 20.0, 10.0, 10.0)),
                semantic_range_entry(103, 3, Rect::from_xy_size(0.0, 30.0, 10.0, 10.0)),
            ],
            VirtualLayoutSemanticRejectedReason::DuplicateKey,
        ));
        cases.push((
            vec![
                semantic_range_entry_with_id(
                    100,
                    0,
                    Rect::from_xy_size(0.0, 0.0, 10.0, 10.0),
                    AutomationNodeId::new("shared-range-node"),
                ),
                semantic_range_entry_with_id(
                    101,
                    1,
                    Rect::from_xy_size(0.0, 10.0, 10.0, 10.0),
                    AutomationNodeId::new("shared-range-node"),
                ),
                semantic_range_entry(102, 2, Rect::from_xy_size(0.0, 20.0, 10.0, 10.0)),
                semantic_range_entry(103, 3, Rect::from_xy_size(0.0, 30.0, 10.0, 10.0)),
            ],
            VirtualLayoutSemanticRejectedReason::DuplicateSemanticNodeId,
        ));
        cases.push((
            vec![
                VirtualLayoutSemanticEntry::new(
                    VirtualLayoutItemKey::new(UnstableKey),
                    0,
                    Rect::from_xy_size(0.0, 0.0, 10.0, 10.0),
                    AutomationNodeSemantics::new(AutomationRole::Button),
                    AutomationNodeId::new("unstable-range-key"),
                ),
                semantic_range_entry(101, 1, Rect::from_xy_size(0.0, 10.0, 10.0, 10.0)),
                semantic_range_entry(102, 2, Rect::from_xy_size(0.0, 20.0, 10.0, 10.0)),
                semantic_range_entry(103, 3, Rect::from_xy_size(0.0, 30.0, 10.0, 10.0)),
            ],
            VirtualLayoutSemanticRejectedReason::UnstableKey,
        ));
        cases.push((
            vec![
                semantic_range_entry(100, 0, Rect::from_xy_size(0.0, 0.0, f32::NAN, 10.0)),
                semantic_range_entry(101, 1, Rect::from_xy_size(0.0, 10.0, 10.0, 10.0)),
                semantic_range_entry(102, 2, Rect::from_xy_size(0.0, 20.0, 10.0, 10.0)),
                semantic_range_entry(103, 3, Rect::from_xy_size(0.0, 30.0, 10.0, 10.0)),
            ],
            VirtualLayoutSemanticRejectedReason::NonFiniteBounds,
        ));
        cases.push((
            vec![
                semantic_range_entry(
                    100,
                    0,
                    Rect::from_min_max(Point::new(10.0, 0.0), Point::new(0.0, 10.0)),
                ),
                semantic_range_entry(101, 1, Rect::from_xy_size(0.0, 10.0, 10.0, 10.0)),
                semantic_range_entry(102, 2, Rect::from_xy_size(0.0, 20.0, 10.0, 10.0)),
                semantic_range_entry(103, 3, Rect::from_xy_size(0.0, 30.0, 10.0, 10.0)),
            ],
            VirtualLayoutSemanticRejectedReason::InvertedBounds,
        ));

        for (entries, reason) in cases {
            let (provider, calls, _) =
                semantic_range_provider(VirtualLayoutSemanticRangeProviderOutcome::Found(entries));
            let mut state = semantic_range_state(
                provider,
                VirtualLayoutCoordinateSpace::logical(),
                VirtualLayoutBudget::new(4),
            );
            assert_eq!(
                state.admit_current_semantic_range(CONTAINER_ID, 0, 4),
                VirtualLayoutSemanticRangeQueryOutcome::Rejected(reason)
            );
            assert_eq!(calls.get(), 1);
            assert!(state.records[0].pin.is_none());
        }
    }

    #[test]
    fn semantic_range_leaves_the_existing_one_item_pin_unchanged_and_never_retains_a_second_pin() {
        let (item_provider, _, _) = semantic_provider(VirtualLayoutSemanticQueryOutcome::Found(
            Box::new(semantic_entry(1, Rect::from_xy_size(0.0, 0.0, 10.0, 10.0))),
        ));
        let (range_provider, range_calls, range_outcome) = semantic_range_provider(
            VirtualLayoutSemanticRangeProviderOutcome::Found(valid_semantic_range_entries(0, 2)),
        );
        let mut registration = registration(
            Rc::new(ReadyPolicy {
                calls: Rc::new(Cell::new(0)),
                key: 1,
            }),
            VirtualLayoutPolicyIdentity::new("semantic-policy"),
        )
        .with_semantic_provider(item_provider)
        .with_semantic_range_provider(range_provider);
        registration.budget = VirtualLayoutBudget::new(4);
        let mut state = RuntimeVirtualLayoutState::default();
        state.records.push(RuntimeVirtualLayoutRecord::new(
            registration,
            SEMANTIC_MOUNT_GENERATION,
        ));
        assert!(matches!(
            state.admit_current_semantics(CONTAINER_ID, VirtualLayoutItemKey::new(1_u32)),
            VirtualLayoutSemanticQueryOutcome::Found(_)
        ));
        let pin_before = state.records[0].pin.clone();
        assert!(matches!(
            state.admit_current_semantic_range(CONTAINER_ID, 0, 2),
            VirtualLayoutSemanticRangeQueryOutcome::Found(_)
        ));
        assert_eq!(range_calls.get(), 1);
        assert_eq!(state.records[0].pin, pin_before);

        let incumbent_id = pin_before
            .as_ref()
            .expect("the one-item pin should remain available")
            .automation_node_id()
            .clone();
        *range_outcome.borrow_mut() = VirtualLayoutSemanticRangeProviderOutcome::Found(vec![
            semantic_range_entry_with_id(
                100,
                0,
                Rect::from_xy_size(0.0, 0.0, 10.0, 10.0),
                incumbent_id,
            ),
            semantic_range_entry(101, 1, Rect::from_xy_size(0.0, 10.0, 10.0, 10.0)),
        ]);
        assert!(matches!(
            state.admit_current_semantic_range(CONTAINER_ID, 0, 2),
            VirtualLayoutSemanticRangeQueryOutcome::Found(_)
        ));
        assert_eq!(range_calls.get(), 2);
        assert_eq!(state.records[0].pin, pin_before);

        *range_outcome.borrow_mut() = VirtualLayoutSemanticRangeProviderOutcome::Found(vec![
            semantic_range_entry(100, 0, Rect::from_xy_size(0.0, 0.0, f32::NAN, 10.0)),
            semantic_range_entry(101, 1, Rect::from_xy_size(0.0, 10.0, 10.0, 10.0)),
        ]);
        assert!(matches!(
            state.admit_current_semantic_range(CONTAINER_ID, 0, 2),
            VirtualLayoutSemanticRangeQueryOutcome::Rejected(
                VirtualLayoutSemanticRejectedReason::NonFiniteBounds
            )
        ));
        assert_eq!(state.records[0].pin, pin_before);
        assert_eq!(
            state.records[0].pin.as_ref().unwrap().reason(),
            VirtualLayoutPinReason::Semantic
        );
    }

    #[test]
    fn semantic_node_id_cross_key_reuse_is_allowed_but_same_key_drift_rejects() {
        let first = semantic_entry_with_id(
            7,
            AutomationNodeId::new("shared-node"),
            Rect::from_xy_size(0.0, 0.0, 10.0, 10.0),
        );
        let (provider, calls, outcome) = semantic_provider(
            VirtualLayoutSemanticQueryOutcome::Found(Box::new(first.clone())),
        );
        let mut state = semantic_state(provider, 3);
        assert!(matches!(
            state.admit_current_semantics(CONTAINER_ID, VirtualLayoutItemKey::new(7_u32)),
            VirtualLayoutSemanticQueryOutcome::Found(_)
        ));
        let cross_key = semantic_entry_with_id(
            9,
            AutomationNodeId::new("shared-node"),
            Rect::from_xy_size(0.0, 12.0, 10.0, 10.0),
        );
        *outcome.borrow_mut() =
            VirtualLayoutSemanticQueryOutcome::Found(Box::new(cross_key.clone()));
        assert_eq!(
            state.admit_current_semantics(CONTAINER_ID, VirtualLayoutItemKey::new(9_u32)),
            VirtualLayoutSemanticQueryOutcome::Found(Box::new(cross_key))
        );
        let pin_after_cross_key = state.records[0].pin.clone();
        let projection_after_cross_key = state.project_current_semantics(CONTAINER_ID);

        *outcome.borrow_mut() =
            VirtualLayoutSemanticQueryOutcome::Found(Box::new(semantic_entry_with_id(
                9,
                AutomationNodeId::new("drifted-node"),
                Rect::from_xy_size(1.0, 1.0, 12.0, 12.0),
            )));
        assert_eq!(
            state.admit_current_semantics(CONTAINER_ID, VirtualLayoutItemKey::new(9_u32)),
            VirtualLayoutSemanticQueryOutcome::Rejected(
                VirtualLayoutSemanticRejectedReason::SemanticNodeIdDrift
            )
        );
        assert_eq!(state.records[0].pin, pin_after_cross_key);
        assert_eq!(
            state.project_current_semantics(CONTAINER_ID),
            projection_after_cross_key
        );
        assert_eq!(calls.get(), 3);
    }

    #[test]
    fn surface_range_query_is_private_unmaterialized_and_has_no_runtime_side_effects() {
        let entries = valid_semantic_range_entries(0, 2);
        let (provider, calls, _) =
            semantic_range_provider(VirtualLayoutSemanticRangeProviderOutcome::Found(entries));
        let registration = registration(
            Rc::new(ReadyPolicy {
                calls: Rc::new(Cell::new(0)),
                key: 1,
            }),
            VirtualLayoutPolicyIdentity::new("semantic-range-policy"),
        )
        .with_semantic_range_provider(provider);
        let mut runtime = SurfaceRuntime::new(
            TestBridge {
                surface: surface(registration),
            },
            Vector2::new(160.0, 80.0),
        );
        let cached_item_count = runtime.virtual_layout.records[0]
            .cached_subtree
            .as_ref()
            .map(|subtree| subtree.items.len());
        let materialization_passes = runtime.virtual_layout.materialization_passes;
        let installation_count = runtime.declarative_owner_projection().installation_count();
        let source_ids = source_node_ids(runtime.surface());
        let active_keys = runtime.virtual_layout.records[0]
            .materialization
            .active_slots()
            .into_iter()
            .map(|slot| slot.item().key().clone())
            .collect::<Vec<_>>();
        let last_required_key = runtime.virtual_layout.records[0].last_required_key.clone();
        let retired = runtime.virtual_layout.records[0].retired;
        let focus_state = runtime.interaction.focus;
        let pointer_capture = (
            runtime.interaction.pointer.capture,
            runtime.interaction.pointer.capture_state,
            runtime.interaction.pointer.managed_capture,
            runtime.interaction.pointer.scroll_drag_capture,
        );
        let widget_hit_order = runtime.traversal.widgets.hit_order.clone();
        let focus_order = runtime.traversal.widgets.focusable.order().to_vec();
        let pointer_order = runtime.traversal.widgets.pointer.order().to_vec();
        let keyboard_focus_order = runtime.traversal.widgets.keyboard_focus.order().to_vec();
        let widget_paths = runtime.traversal.widgets.paths.current.clone();
        let scroll_container_order = runtime.traversal.containers.scroll.order().to_vec();
        let scroll_offset = runtime.layout_state.scroll_offset(CONTAINER_ID);
        let layout_before = runtime.layout.clone();
        let layout_root_before = runtime.layout_root.clone();
        let surface_root_before = runtime.surface().layout_node().clone();
        let refresh_counters = runtime.refresh_counters();
        let paint_observation = runtime.latest_paint_segment_observation();
        let paint_reuse = runtime.base_paint_plan_reuse_eligible();
        let automation_target_snapshot = runtime.automation_target_snapshot();

        let outcome = runtime.admit_virtual_layout_semantic_range(CONTAINER_ID, 0, 2);
        let VirtualLayoutSemanticRangeQueryOutcome::Found(batch) = outcome else {
            panic!("the surface range should be accepted");
        };
        assert_eq!(calls.get(), 1);
        assert!(
            batch
                .projections()
                .iter()
                .all(|projection| projection.authority()
                    == VirtualLayoutSemanticProjectionAuthority::Unmaterialized)
        );
        assert_eq!(
            runtime.virtual_layout.records[0]
                .cached_subtree
                .as_ref()
                .map(|subtree| subtree.items.len()),
            cached_item_count
        );
        assert_eq!(
            runtime.virtual_layout.materialization_passes,
            materialization_passes
        );
        assert_eq!(
            runtime.virtual_layout.records[0].last_required_key,
            last_required_key
        );
        assert_eq!(runtime.virtual_layout.records[0].retired, retired);
        assert_eq!(
            runtime.virtual_layout.records[0]
                .materialization
                .active_slots()
                .into_iter()
                .map(|slot| slot.item().key().clone())
                .collect::<Vec<_>>(),
            active_keys
        );
        assert_eq!(
            runtime.declarative_owner_projection().installation_count(),
            installation_count
        );
        assert_eq!(source_node_ids(runtime.surface()), source_ids);
        assert_eq!(runtime.interaction.focus, focus_state);
        assert_eq!(
            (
                runtime.interaction.pointer.capture,
                runtime.interaction.pointer.capture_state,
                runtime.interaction.pointer.managed_capture,
                runtime.interaction.pointer.scroll_drag_capture,
            ),
            pointer_capture
        );
        assert_eq!(runtime.traversal.widgets.hit_order, widget_hit_order);
        assert_eq!(runtime.traversal.widgets.focusable.order(), focus_order);
        assert_eq!(runtime.traversal.widgets.pointer.order(), pointer_order);
        assert_eq!(
            runtime.traversal.widgets.keyboard_focus.order(),
            keyboard_focus_order
        );
        assert_eq!(runtime.traversal.widgets.paths.current, widget_paths);
        assert_eq!(
            runtime.traversal.containers.scroll.order(),
            scroll_container_order
        );
        assert_eq!(
            runtime.layout_state.scroll_offset(CONTAINER_ID),
            scroll_offset
        );
        assert_eq!(runtime.layout, layout_before);
        assert_eq!(runtime.layout_root, layout_root_before);
        assert_eq!(runtime.surface().layout_node(), surface_root_before);
        assert_eq!(runtime.refresh_counters(), refresh_counters);
        assert_eq!(
            runtime.latest_paint_segment_observation(),
            paint_observation
        );
        assert_eq!(runtime.base_paint_plan_reuse_eligible(), paint_reuse);
        assert_eq!(
            runtime.automation_target_snapshot(),
            automation_target_snapshot
        );
    }

    #[test]
    fn current_semantic_admission_constructs_exact_current_request_and_pins_found() {
        let entry = semantic_entry(7, Rect::from_xy_size(4.0, 8.0, 24.0, 16.0));
        let (provider, calls, _) = semantic_provider(VirtualLayoutSemanticQueryOutcome::Found(
            Box::new(entry.clone()),
        ));
        let revisions = VirtualLayoutRegistrationRevisions {
            data: 11,
            policy: 12,
            measurement: 13,
            semantic: 14,
            ..Default::default()
        };
        let mut registration = registration(
            Rc::new(ReadyPolicy {
                calls: Rc::new(Cell::new(0)),
                key: 7,
            }),
            VirtualLayoutPolicyIdentity::new("current-policy".to_owned()),
        )
        .with_semantic_provider(provider.clone());
        registration.revisions = revisions;
        let mut runtime = SurfaceRuntime::new(
            TestBridge {
                surface: surface(registration),
            },
            Vector2::new(160.0, 80.0),
        );
        let mount_generation = runtime.virtual_layout.records[0].mount_generation;
        let expected_request =
            semantic_request_with_revisions("current-policy", mount_generation, revisions, 7);

        assert_eq!(
            runtime.admit_virtual_layout_semantics(CONTAINER_ID, VirtualLayoutItemKey::new(7_u32),),
            VirtualLayoutSemanticQueryOutcome::Found(Box::new(entry.clone()))
        );
        assert_eq!(calls.get(), 1);
        assert_eq!(
            provider.requests.borrow().as_slice(),
            std::slice::from_ref(&expected_request)
        );
        let pin = runtime.virtual_layout.records[0]
            .pin
            .as_ref()
            .expect("current semantic admission should retain one pin");
        assert_eq!(pin.reason(), VirtualLayoutPinReason::Semantic);
        assert_eq!(pin.request(), &expected_request);
        assert_eq!(pin.entry(), &entry);
        assert_eq!(pin.automation_node_id(), entry.automation_node_id());
    }

    #[test]
    fn current_semantic_admission_rejects_non_live_or_unstable_inputs_without_lookup() {
        let (provider, calls, _) = semantic_provider(VirtualLayoutSemanticQueryOutcome::Found(
            Box::new(semantic_entry(7, Rect::from_xy_size(0.0, 0.0, 10.0, 10.0))),
        ));
        let mut state = semantic_state(provider, 3);

        assert_eq!(
            state.admit_current_semantics(CONTAINER_ID + 1, VirtualLayoutItemKey::new(7_u32),),
            VirtualLayoutSemanticQueryOutcome::Rejected(
                VirtualLayoutSemanticRejectedReason::UnknownContainer
            )
        );
        assert_eq!(
            state.admit_current_semantics(CONTAINER_ID, VirtualLayoutItemKey::new(UnstableKey)),
            VirtualLayoutSemanticQueryOutcome::Rejected(
                VirtualLayoutSemanticRejectedReason::UnstableKey
            )
        );
        state.records[0].retire();
        assert_eq!(
            state.admit_current_semantics(CONTAINER_ID, VirtualLayoutItemKey::new(7_u32),),
            VirtualLayoutSemanticQueryOutcome::Rejected(
                VirtualLayoutSemanticRejectedReason::Retired
            )
        );
        assert_eq!(calls.get(), 0);
        assert!(state.records[0].pin.is_none());

        let mut missing_provider = RuntimeVirtualLayoutState::default();
        missing_provider
            .records
            .push(RuntimeVirtualLayoutRecord::new(
                registration(
                    Rc::new(ReadyPolicy {
                        calls: Rc::new(Cell::new(0)),
                        key: 7,
                    }),
                    VirtualLayoutPolicyIdentity::new("missing-provider-policy".to_owned()),
                ),
                SEMANTIC_MOUNT_GENERATION,
            ));
        assert_eq!(
            missing_provider
                .admit_current_semantics(CONTAINER_ID, VirtualLayoutItemKey::new(7_u32),),
            VirtualLayoutSemanticQueryOutcome::Unavailable(
                VirtualLayoutSemanticUnavailableReason::NoProvider
            )
        );
        assert!(missing_provider.records[0].pin.is_none());
    }

    #[test]
    fn project_current_semantics_preserves_identity_coordinate_entry_and_exact_fence() {
        let coordinate_spaces = [
            VirtualLayoutCoordinateSpace::logical(),
            VirtualLayoutCoordinateSpace::custom(VirtualLayoutPolicyIdentity::new(
                "semantic-coordinate-space",
            )),
        ];
        let bounds = Rect::from_xy_size(4.0, 8.0, 24.0, 16.0);
        let semantics =
            AutomationNodeSemantics::new(AutomationRole::Row).with_label("projected row");

        for coordinate_space in coordinate_spaces {
            let key = VirtualLayoutItemKey::new(17_u32);
            let entry = VirtualLayoutSemanticEntry::new(
                key.clone(),
                23,
                bounds,
                semantics.clone(),
                AutomationNodeId::new("projected-row"),
            );
            let (provider, _, _) = semantic_provider(VirtualLayoutSemanticQueryOutcome::Found(
                Box::new(entry.clone()),
            ));
            let mut state = semantic_state(provider, 5);
            state.records[0].registration.coordinate_space = coordinate_space.clone();
            let request = semantic_request("semantic-policy", SEMANTIC_MOUNT_GENERATION, 5, 17);

            assert_eq!(
                state.admit_current_semantics(CONTAINER_ID, key),
                VirtualLayoutSemanticQueryOutcome::Found(Box::new(entry.clone()))
            );

            let projection = state
                .project_current_semantics(CONTAINER_ID)
                .expect("a valid semantic pin should project");
            assert_eq!(projection.identity().container_id(), CONTAINER_ID);
            assert_eq!(
                projection.identity().key(),
                &VirtualLayoutItemKey::new(17_u32)
            );
            assert_eq!(projection.coordinate_space(), &coordinate_space);
            assert_eq!(projection.logical_index(), 23);
            assert_eq!(projection.bounds(), bounds);
            assert_eq!(projection.semantics(), &semantics);
            assert_eq!(
                projection.automation_node_id(),
                &AutomationNodeId::new("projected-row")
            );
            assert_eq!(
                serde_json::to_string(projection.automation_node_id())
                    .expect("AutomationNodeId should remain serializable"),
                "\"projected-row\""
            );
            assert_eq!(projection.request(), &request);
            assert_eq!(
                projection.authority(),
                VirtualLayoutSemanticProjectionAuthority::Unmaterialized
            );
        }
    }

    #[test]
    fn project_current_semantics_requires_a_live_semantic_pin() {
        let entry = semantic_entry(7, Rect::from_xy_size(0.0, 0.0, 10.0, 10.0));
        let (provider, _, outcome) = semantic_provider(VirtualLayoutSemanticQueryOutcome::Found(
            Box::new(entry.clone()),
        ));
        let mut state = semantic_state(provider, 3);
        let request = semantic_request("semantic-policy", SEMANTIC_MOUNT_GENERATION, 3, 7);

        assert!(state.project_current_semantics(CONTAINER_ID + 1).is_none());
        assert!(state.project_current_semantics(CONTAINER_ID).is_none());

        assert!(matches!(
            state.query_pin(&request, VirtualLayoutPinReason::Focus),
            VirtualLayoutSemanticQueryOutcome::Found(_)
        ));
        assert!(state.project_current_semantics(CONTAINER_ID).is_none());

        assert!(matches!(
            state.query_semantics(&request),
            VirtualLayoutSemanticQueryOutcome::Found(_)
        ));
        assert!(state.project_current_semantics(CONTAINER_ID).is_some());

        *outcome.borrow_mut() = VirtualLayoutSemanticQueryOutcome::NotFound;
        assert_eq!(
            state.admit_current_semantics(CONTAINER_ID, VirtualLayoutItemKey::new(7_u32)),
            VirtualLayoutSemanticQueryOutcome::NotFound
        );
        assert!(state.project_current_semantics(CONTAINER_ID).is_none());

        *outcome.borrow_mut() = VirtualLayoutSemanticQueryOutcome::Found(Box::new(entry));
        assert!(matches!(
            state.admit_current_semantics(CONTAINER_ID, VirtualLayoutItemKey::new(7_u32)),
            VirtualLayoutSemanticQueryOutcome::Found(_)
        ));
        state.records[0].retire();
        assert!(state.project_current_semantics(CONTAINER_ID).is_none());
    }

    #[test]
    fn semantic_query_pins_one_valid_entry_without_materialization_side_effects() {
        let entry = semantic_entry(7, Rect::from_xy_size(4.0, 8.0, 24.0, 16.0));
        let (provider, calls, _) = semantic_provider(VirtualLayoutSemanticQueryOutcome::Found(
            Box::new(entry.clone()),
        ));
        let mut state = semantic_state(provider, 3);
        let request = semantic_request("semantic-policy", SEMANTIC_MOUNT_GENERATION, 3, 7);
        let cached_before = state.records[0].cached_subtree.is_some();
        let passes_before = state.materialization_passes;

        assert_eq!(
            state.query_semantics(&request),
            VirtualLayoutSemanticQueryOutcome::Found(Box::new(entry.clone()))
        );
        assert_eq!(calls.get(), 1);
        assert_eq!(
            state.records[0].pin.as_ref().unwrap().reason(),
            VirtualLayoutPinReason::Semantic
        );
        assert_eq!(state.records[0].pin.as_ref().unwrap().request(), &request);
        assert_eq!(state.records[0].pin.as_ref().unwrap().entry(), &entry);
        assert_eq!(
            state.records[0].pin.as_ref().unwrap().automation_node_id(),
            entry.automation_node_id()
        );
        assert_eq!(state.records[0].cached_subtree.is_some(), cached_before);
        assert_eq!(state.materialization_passes, passes_before);
    }

    #[test]
    fn pin_reasons_are_valid_and_one_pin_replaces_in_query_order() {
        let first = semantic_entry(7, Rect::from_xy_size(0.0, 0.0, 10.0, 10.0));
        let second = semantic_entry_with_id(
            9,
            AutomationNodeId::new("semantic-second"),
            Rect::from_xy_size(0.0, 12.0, 10.0, 10.0),
        );
        let (provider, calls, outcome) = semantic_provider(
            VirtualLayoutSemanticQueryOutcome::Found(Box::new(first.clone())),
        );
        let mut state = semantic_state(provider, 3);
        let first_request = semantic_request("semantic-policy", SEMANTIC_MOUNT_GENERATION, 3, 7);

        assert!(matches!(
            state.query_pin(&first_request, VirtualLayoutPinReason::Focus),
            VirtualLayoutSemanticQueryOutcome::Found(_)
        ));
        assert_eq!(
            state.records[0].pin.as_ref().unwrap().reason(),
            VirtualLayoutPinReason::Focus
        );

        assert!(matches!(
            state.query_pin(&first_request, VirtualLayoutPinReason::PointerCapture),
            VirtualLayoutSemanticQueryOutcome::Found(_)
        ));
        assert_eq!(
            state.records[0].pin.as_ref().unwrap().reason(),
            VirtualLayoutPinReason::PointerCapture
        );

        assert!(matches!(
            state.query_semantics(&first_request),
            VirtualLayoutSemanticQueryOutcome::Found(_)
        ));
        assert_eq!(
            state.records[0].pin.as_ref().unwrap().reason(),
            VirtualLayoutPinReason::Semantic
        );

        *outcome.borrow_mut() = VirtualLayoutSemanticQueryOutcome::Found(Box::new(second.clone()));
        let second_request = semantic_request("semantic-policy", SEMANTIC_MOUNT_GENERATION, 3, 9);
        assert!(matches!(
            state.query_pin(&second_request, VirtualLayoutPinReason::PointerCapture),
            VirtualLayoutSemanticQueryOutcome::Found(_)
        ));
        let pin = state.records[0]
            .pin
            .as_ref()
            .expect("the one bounded pin should be retained");
        assert_eq!(pin.reason(), VirtualLayoutPinReason::PointerCapture);
        assert_eq!(pin.request(), &second_request);
        assert_eq!(pin.entry(), &second);
        assert_eq!(calls.get(), 4);
    }

    #[test]
    fn semantic_query_does_not_install_or_rebuild_the_runtime_tree() {
        let (provider, _, _) = semantic_provider(VirtualLayoutSemanticQueryOutcome::Found(
            Box::new(semantic_entry(1, Rect::from_xy_size(4.0, 8.0, 24.0, 16.0))),
        ));
        let registration = registration(
            Rc::new(ReadyPolicy {
                calls: Rc::new(Cell::new(0)),
                key: 1,
            }),
            VirtualLayoutPolicyIdentity::new("semantic-policy".to_owned()),
        )
        .with_semantic_provider(provider);
        let mut runtime = SurfaceRuntime::new(
            TestBridge {
                surface: surface(registration),
            },
            Vector2::new(160.0, 80.0),
        );
        let cached_item_count = runtime.virtual_layout.records[0]
            .cached_subtree
            .as_ref()
            .map(|subtree| subtree.items.len());
        let materialization_passes = runtime.virtual_layout.materialization_passes;
        let installation_count = runtime.declarative_owner_projection().installation_count();
        let source_ids = source_node_ids(runtime.surface());
        let active_keys = runtime.virtual_layout.records[0]
            .materialization
            .active_slots()
            .into_iter()
            .map(|slot| slot.item().key().clone())
            .collect::<Vec<_>>();
        let focus_state = runtime.interaction.focus;
        let pointer_capture = (
            runtime.interaction.pointer.capture,
            runtime.interaction.pointer.capture_state,
            runtime.interaction.pointer.managed_capture,
            runtime.interaction.pointer.scroll_drag_capture,
        );
        let widget_hit_order = runtime.traversal.widgets.hit_order.clone();
        let focus_order = runtime.traversal.widgets.focusable.order().to_vec();
        let pointer_order = runtime.traversal.widgets.pointer.order().to_vec();
        let keyboard_focus_order = runtime.traversal.widgets.keyboard_focus.order().to_vec();
        let widget_paths = runtime.traversal.widgets.paths.current.clone();
        let styled_container_order = runtime.traversal.containers.styled.order().to_vec();
        let scroll_container_order = runtime.traversal.containers.scroll.order().to_vec();
        let layout_before = runtime.layout.clone();
        let layout_root_before = runtime.layout_root.clone();
        let surface_root_before = runtime.surface().layout_node().clone();
        let refresh_counters = runtime.refresh_counters();
        let paint_observation = runtime.latest_paint_segment_observation();
        let paint_reuse = runtime.base_paint_plan_reuse_eligible();
        let automation_target_snapshot = runtime.automation_target_snapshot();

        assert!(matches!(
            runtime.admit_virtual_layout_semantics(CONTAINER_ID, VirtualLayoutItemKey::new(1_u32),),
            VirtualLayoutSemanticQueryOutcome::Found(_)
        ));
        assert_eq!(
            runtime
                .project_virtual_layout_semantics(CONTAINER_ID)
                .expect("semantic admission should expose private evidence")
                .authority(),
            VirtualLayoutSemanticProjectionAuthority::Unmaterialized
        );
        assert_eq!(
            runtime.virtual_layout.records[0]
                .cached_subtree
                .as_ref()
                .map(|subtree| subtree.items.len()),
            cached_item_count
        );
        assert_eq!(
            runtime.virtual_layout.materialization_passes,
            materialization_passes
        );
        assert_eq!(
            runtime.virtual_layout.records[0]
                .materialization
                .active_slots()
                .into_iter()
                .map(|slot| slot.item().key().clone())
                .collect::<Vec<_>>(),
            active_keys
        );
        assert_eq!(
            runtime.declarative_owner_projection().installation_count(),
            installation_count
        );
        assert_eq!(source_node_ids(runtime.surface()), source_ids);
        assert_eq!(runtime.interaction.focus, focus_state);
        assert_eq!(
            (
                runtime.interaction.pointer.capture,
                runtime.interaction.pointer.capture_state,
                runtime.interaction.pointer.managed_capture,
                runtime.interaction.pointer.scroll_drag_capture,
            ),
            pointer_capture
        );
        assert_eq!(runtime.traversal.widgets.hit_order, widget_hit_order);
        assert_eq!(runtime.traversal.widgets.focusable.order(), focus_order);
        assert_eq!(runtime.traversal.widgets.pointer.order(), pointer_order);
        assert_eq!(
            runtime.traversal.widgets.keyboard_focus.order(),
            keyboard_focus_order
        );
        assert_eq!(runtime.traversal.widgets.paths.current, widget_paths);
        assert_eq!(
            runtime.traversal.containers.styled.order(),
            styled_container_order
        );
        assert_eq!(
            runtime.traversal.containers.scroll.order(),
            scroll_container_order
        );
        assert_eq!(runtime.layout, layout_before);
        assert_eq!(runtime.layout_root, layout_root_before);
        assert_eq!(runtime.surface().layout_node(), surface_root_before);
        assert_eq!(runtime.refresh_counters(), refresh_counters);
        assert_eq!(
            runtime.latest_paint_segment_observation(),
            paint_observation
        );
        assert_eq!(runtime.base_paint_plan_reuse_eligible(), paint_reuse);
        assert_eq!(
            runtime.automation_target_snapshot(),
            automation_target_snapshot
        );
    }

    #[test]
    fn current_semantic_admission_replaces_pin_after_provider_or_revision_change() {
        let first = semantic_entry(7, Rect::from_xy_size(0.0, 0.0, 10.0, 10.0));
        let (first_provider, first_calls, _) = semantic_provider(
            VirtualLayoutSemanticQueryOutcome::Found(Box::new(first.clone())),
        );
        let mut state = semantic_state(first_provider.clone(), 3);
        assert_eq!(
            state.admit_current_semantics(CONTAINER_ID, VirtualLayoutItemKey::new(7_u32)),
            VirtualLayoutSemanticQueryOutcome::Found(Box::new(first.clone()))
        );
        assert_eq!(first_calls.get(), 1);
        assert_eq!(state.records[0].pin.as_ref().unwrap().entry(), &first);
        assert_eq!(
            state
                .project_current_semantics(CONTAINER_ID)
                .unwrap()
                .identity()
                .key(),
            &VirtualLayoutItemKey::new(7_u32)
        );

        let second = semantic_entry_with_id(
            9,
            AutomationNodeId::new("semantic-second"),
            Rect::from_xy_size(0.0, 12.0, 10.0, 10.0),
        );
        let (second_provider, second_calls, second_outcome) = semantic_provider(
            VirtualLayoutSemanticQueryOutcome::Found(Box::new(second.clone())),
        );
        let mut replacement = registration(
            Rc::new(ReadyPolicy {
                calls: Rc::new(Cell::new(0)),
                key: 1,
            }),
            VirtualLayoutPolicyIdentity::new("semantic-policy".to_owned()),
        )
        .with_semantic_provider(second_provider.clone());
        replacement.revisions.semantic = 3;
        state.records[0].update_registration(replacement);
        assert!(state.records[0].pin.is_none());
        assert!(state.project_current_semantics(CONTAINER_ID).is_none());

        assert_eq!(
            state.admit_current_semantics(CONTAINER_ID, VirtualLayoutItemKey::new(9_u32)),
            VirtualLayoutSemanticQueryOutcome::Found(Box::new(second.clone()))
        );
        assert_eq!(first_calls.get(), 1);
        assert_eq!(second_calls.get(), 1);
        let second_request = semantic_request("semantic-policy", SEMANTIC_MOUNT_GENERATION, 3, 9);
        assert_eq!(
            state.records[0].pin.as_ref().unwrap().request(),
            &second_request
        );
        assert_eq!(
            second_provider.requests.borrow().last(),
            Some(&second_request)
        );
        assert_eq!(
            state
                .project_current_semantics(CONTAINER_ID)
                .unwrap()
                .identity()
                .key(),
            &VirtualLayoutItemKey::new(9_u32)
        );

        let third = semantic_entry_with_id(
            9,
            AutomationNodeId::new("semantic-third"),
            Rect::from_xy_size(1.0, 13.0, 11.0, 12.0),
        );
        *second_outcome.borrow_mut() =
            VirtualLayoutSemanticQueryOutcome::Found(Box::new(third.clone()));
        let mut revised = state.records[0].registration.clone();
        revised.revisions.semantic = 4;
        state.records[0].update_registration(revised);
        assert!(state.records[0].pin.is_none());
        assert!(state.project_current_semantics(CONTAINER_ID).is_none());

        assert_eq!(
            state.admit_current_semantics(CONTAINER_ID, VirtualLayoutItemKey::new(9_u32)),
            VirtualLayoutSemanticQueryOutcome::Found(Box::new(third.clone()))
        );
        assert_eq!(second_calls.get(), 2);
        let revised_request = semantic_request("semantic-policy", SEMANTIC_MOUNT_GENERATION, 4, 9);
        assert_eq!(
            state.records[0].pin.as_ref().unwrap().request(),
            &revised_request
        );
        assert_eq!(
            second_provider.requests.borrow().last(),
            Some(&revised_request)
        );
    }

    #[test]
    fn semantic_query_rejects_stale_scope_or_revision_before_provider_invocation() {
        let (provider, calls, _) = semantic_provider(VirtualLayoutSemanticQueryOutcome::NotFound);
        let mut state = semantic_state(provider, 3);

        let cases = [
            (
                semantic_request("other-policy", SEMANTIC_MOUNT_GENERATION, 3, 7),
                VirtualLayoutSemanticRejectedReason::ScopeMismatch,
            ),
            (
                semantic_request("semantic-policy", SEMANTIC_MOUNT_GENERATION + 1, 3, 7),
                VirtualLayoutSemanticRejectedReason::Stale,
            ),
            (
                semantic_request("semantic-policy", SEMANTIC_MOUNT_GENERATION, 4, 7),
                VirtualLayoutSemanticRejectedReason::Stale,
            ),
            (
                semantic_request_with_revisions(
                    "semantic-policy",
                    SEMANTIC_MOUNT_GENERATION,
                    VirtualLayoutRegistrationRevisions {
                        data: 1,
                        ..Default::default()
                    },
                    7,
                ),
                VirtualLayoutSemanticRejectedReason::Stale,
            ),
            (
                semantic_request_with_revisions(
                    "semantic-policy",
                    SEMANTIC_MOUNT_GENERATION,
                    VirtualLayoutRegistrationRevisions {
                        policy: 1,
                        ..Default::default()
                    },
                    7,
                ),
                VirtualLayoutSemanticRejectedReason::Stale,
            ),
            (
                semantic_request_with_revisions(
                    "semantic-policy",
                    SEMANTIC_MOUNT_GENERATION,
                    VirtualLayoutRegistrationRevisions {
                        measurement: 1,
                        ..Default::default()
                    },
                    7,
                ),
                VirtualLayoutSemanticRejectedReason::Stale,
            ),
        ];
        for (request, reason) in cases {
            assert_eq!(
                state.query_semantics(&request),
                VirtualLayoutSemanticQueryOutcome::Rejected(reason)
            );
        }
        assert_eq!(calls.get(), 0);
        assert!(state.records[0].pin.is_none());
    }

    #[test]
    fn semantic_query_rejects_wrong_key_nonfinite_and_inverted_entries() {
        let (provider, calls, outcome) =
            semantic_provider(VirtualLayoutSemanticQueryOutcome::Found(Box::new(
                semantic_entry(7, Rect::from_xy_size(0.0, 0.0, 10.0, 10.0)),
            )));
        let mut state = semantic_state(provider, 3);

        assert!(matches!(
            state.admit_current_semantics(CONTAINER_ID, VirtualLayoutItemKey::new(7_u32)),
            VirtualLayoutSemanticQueryOutcome::Found(_)
        ));
        assert!(state.records[0].pin.is_some());

        *outcome.borrow_mut() = VirtualLayoutSemanticQueryOutcome::Found(Box::new(semantic_entry(
            8,
            Rect::from_xy_size(0.0, 0.0, 10.0, 10.0),
        )));

        assert_eq!(
            state.admit_current_semantics(CONTAINER_ID, VirtualLayoutItemKey::new(7_u32)),
            VirtualLayoutSemanticQueryOutcome::Rejected(
                VirtualLayoutSemanticRejectedReason::WrongKey
            )
        );
        *outcome.borrow_mut() = VirtualLayoutSemanticQueryOutcome::Found(Box::new(semantic_entry(
            7,
            Rect::from_xy_size(0.0, 0.0, f32::NAN, 10.0),
        )));
        assert_eq!(
            state.admit_current_semantics(CONTAINER_ID, VirtualLayoutItemKey::new(7_u32)),
            VirtualLayoutSemanticQueryOutcome::Rejected(
                VirtualLayoutSemanticRejectedReason::NonFiniteBounds
            )
        );
        *outcome.borrow_mut() = VirtualLayoutSemanticQueryOutcome::Found(Box::new(semantic_entry(
            7,
            Rect::from_min_max(Point::new(10.0, 0.0), Point::new(0.0, 10.0)),
        )));
        assert_eq!(
            state.admit_current_semantics(CONTAINER_ID, VirtualLayoutItemKey::new(7_u32)),
            VirtualLayoutSemanticQueryOutcome::Rejected(
                VirtualLayoutSemanticRejectedReason::InvertedBounds
            )
        );
        assert_eq!(calls.get(), 4);
        assert!(state.records[0].pin.is_none());
    }

    #[test]
    fn semantic_query_outcomes_clear_the_existing_pin() {
        let entry = semantic_entry(7, Rect::from_xy_size(0.0, 0.0, 10.0, 10.0));
        let (provider, _, outcome) = semantic_provider(VirtualLayoutSemanticQueryOutcome::Found(
            Box::new(entry.clone()),
        ));
        let mut state = semantic_state(provider, 3);
        assert!(matches!(
            state.admit_current_semantics(CONTAINER_ID, VirtualLayoutItemKey::new(7_u32)),
            VirtualLayoutSemanticQueryOutcome::Found(_)
        ));
        assert!(state.records[0].pin.is_some());

        for outcome_value in [
            VirtualLayoutSemanticQueryOutcome::NotFound,
            VirtualLayoutSemanticQueryOutcome::Deferred(
                VirtualLayoutSemanticDeferredReason::DataPending,
            ),
            VirtualLayoutSemanticQueryOutcome::Unavailable(
                VirtualLayoutSemanticUnavailableReason::DataUnavailable,
            ),
            VirtualLayoutSemanticQueryOutcome::Rejected(
                VirtualLayoutSemanticRejectedReason::ProviderRejected,
            ),
        ] {
            *outcome.borrow_mut() = outcome_value.clone();
            assert_eq!(
                state.admit_current_semantics(CONTAINER_ID, VirtualLayoutItemKey::new(7_u32)),
                outcome_value
            );
            assert!(state.records[0].pin.is_none());
            *outcome.borrow_mut() =
                VirtualLayoutSemanticQueryOutcome::Found(Box::new(entry.clone()));
            assert!(matches!(
                state.admit_current_semantics(CONTAINER_ID, VirtualLayoutItemKey::new(7_u32)),
                VirtualLayoutSemanticQueryOutcome::Found(_)
            ));
        }
    }

    #[test]
    fn semantic_pin_is_bounded_and_clears_on_all_revision_changes_and_retirement() {
        for revision in ["data", "policy", "measurement", "semantic"] {
            let entry = semantic_entry(7, Rect::from_xy_size(0.0, 0.0, 10.0, 10.0));
            let (provider, calls, _) = semantic_provider(VirtualLayoutSemanticQueryOutcome::Found(
                Box::new(entry.clone()),
            ));
            let mut state = semantic_state(provider, 3);
            let revisions = state.records[0].registration.revisions;
            let request = semantic_request_with_revisions(
                "semantic-policy",
                SEMANTIC_MOUNT_GENERATION,
                revisions,
                7,
            );
            assert!(matches!(
                state.query_semantics(&request),
                VirtualLayoutSemanticQueryOutcome::Found(_)
            ));
            assert_eq!(calls.get(), 1, "{revision} revision should initially query");
            assert!(state.records[0].pin.is_some());

            let mut next_registration = state.records[0].registration.clone();
            match revision {
                "data" => next_registration.revisions.data += 1,
                "policy" => next_registration.revisions.policy += 1,
                "measurement" => next_registration.revisions.measurement += 1,
                "semantic" => next_registration.revisions.semantic += 1,
                _ => unreachable!("the revision cases are exhaustive"),
            }
            state.records[0].update_registration(next_registration);
            assert!(
                state.records[0].pin.is_none(),
                "{revision} revision should clear the existing pin"
            );
            assert_eq!(
                state.query_semantics(&request),
                VirtualLayoutSemanticQueryOutcome::Rejected(
                    VirtualLayoutSemanticRejectedReason::Stale
                ),
                "{revision} revision should reject the stale request"
            );
            assert_eq!(
                calls.get(),
                1,
                "{revision} stale request must not invoke the provider"
            );
        }

        let first = semantic_entry(7, Rect::from_xy_size(0.0, 0.0, 10.0, 10.0));
        let second = semantic_entry_with_id(
            9,
            AutomationNodeId::new("semantic-after-revision"),
            Rect::from_xy_size(0.0, 12.0, 10.0, 10.0),
        );
        let (provider, _, outcome) = semantic_provider(VirtualLayoutSemanticQueryOutcome::Found(
            Box::new(first.clone()),
        ));
        let mut state = semantic_state(provider, 3);
        let first_request = semantic_request("semantic-policy", SEMANTIC_MOUNT_GENERATION, 3, 7);
        assert!(matches!(
            state.query_semantics(&first_request),
            VirtualLayoutSemanticQueryOutcome::Found(_)
        ));
        assert_eq!(state.records[0].pin.as_ref().unwrap().entry(), &first);

        let mut next_registration = state.records[0].registration.clone();
        next_registration.revisions.semantic = 4;
        state.records[0].update_registration(next_registration);
        assert!(state.records[0].pin.is_none());

        *outcome.borrow_mut() = VirtualLayoutSemanticQueryOutcome::Found(Box::new(second.clone()));
        let second_request = semantic_request("semantic-policy", SEMANTIC_MOUNT_GENERATION, 4, 9);
        assert_eq!(
            state.query_semantics(&second_request),
            VirtualLayoutSemanticQueryOutcome::Found(Box::new(second.clone()))
        );
        assert_eq!(state.records[0].pin.as_ref().unwrap().entry(), &second);

        state.records[0].retire();
        assert!(state.records[0].pin.is_none());
    }

    fn surface(registration: VirtualLayoutRegistration<()>) -> UiSurface<()> {
        UiSurface::new(
            SurfaceNode::container(
                CONTAINER_ID,
                ContainerPolicy {
                    kind: ContainerKind::ScrollView,
                    overflow: OverflowPolicy::Scroll,
                    ..ContainerPolicy::default()
                },
                Vec::<SurfaceChild<()>>::new(),
            )
            .with_virtual_layout_registration(registration),
        )
    }

    fn ordinary_surface() -> UiSurface<()> {
        UiSurface::new(SurfaceNode::container(
            CONTAINER_ID,
            ContainerPolicy::default(),
            vec![SurfaceChild::new(
                crate::layout::SlotParams {
                    size_main: crate::layout::SizeModeMain::Fixed(20.0),
                    size_cross: crate::layout::SizeModeCross::Fixed(48.0),
                    constraints: crate::layout::Constraints::unconstrained(),
                    margin: Default::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                SurfaceNode::text(
                    ORDINARY_CHILD_ID,
                    "ordinary child",
                    WidgetSizing::fixed(Vector2::new(48.0, 20.0)),
                ),
            )],
        ))
    }

    fn duplicate_surface(
        first: VirtualLayoutRegistration<()>,
        second: VirtualLayoutRegistration<()>,
    ) -> UiSurface<()> {
        let container = |registration| {
            SurfaceNode::container(
                CONTAINER_ID,
                ContainerPolicy {
                    kind: ContainerKind::ScrollView,
                    overflow: OverflowPolicy::Scroll,
                    ..ContainerPolicy::default()
                },
                Vec::<SurfaceChild<()>>::new(),
            )
            .with_virtual_layout_registration(registration)
        };
        UiSurface::new(SurfaceNode::container(
            ROOT_ID,
            ContainerPolicy::default(),
            vec![
                SurfaceChild::fill(container(first)),
                SurfaceChild::fill(container(second)),
            ],
        ))
    }

    #[test]
    fn duplicate_registration_rejects_all_candidates_without_a_winner() {
        let first_calls = Rc::new(Cell::new(0));
        let second_calls = Rc::new(Cell::new(0));
        let first = registration_with_parts(RegistrationParts {
            policy: Rc::new(ReadyPolicy {
                calls: Rc::clone(&first_calls),
                key: 9,
            }),
            policy_identity: VirtualLayoutPolicyIdentity::new("duplicate-first-policy"),
            revisions: Default::default(),
            shell: Rc::new(|| scroll(spacer::<()>())),
            item: Rc::new(|_| text::<()>("first item")),
            kind: Rc::new(|_| VirtualLayoutPolicyIdentity::new("duplicate-first-kind")),
        });
        let second = registration_with_parts(RegistrationParts {
            policy: Rc::new(ReadyPolicy {
                calls: Rc::clone(&second_calls),
                key: 10,
            }),
            policy_identity: VirtualLayoutPolicyIdentity::new("duplicate-second-policy"),
            revisions: Default::default(),
            shell: Rc::new(|| scroll(spacer::<()>())),
            item: Rc::new(|_| text::<()>("second item")),
            kind: Rc::new(|_| VirtualLayoutPolicyIdentity::new("duplicate-second-kind")),
        });
        let mut runtime = SurfaceRuntime::new(
            TestBridge {
                surface: duplicate_surface(first, second),
            },
            Vector2::new(160.0, 80.0),
        );

        assert_eq!(runtime.surface().layout_node().id(), ROOT_ID);
        assert!(runtime.virtual_layout.records.is_empty());
        assert_eq!(first_calls.get(), 0);
        assert_eq!(second_calls.get(), 0);
        let crate::layout::LayoutNode::Container(root) = runtime.surface().layout_node() else {
            panic!("duplicate-registration surface should retain its root container");
        };
        assert_eq!(root.children.len(), 2);

        runtime.refresh_with_scope(crate::runtime::RepaintScope::Projection);

        assert!(runtime.virtual_layout.records.is_empty());
        assert_eq!(first_calls.get(), 0);
        assert_eq!(second_calls.get(), 0);
        assert_eq!(runtime.surface().layout_node().id(), ROOT_ID);
    }

    struct TestBridge {
        surface: UiSurface<()>,
    }

    impl RuntimeBridge<()> for TestBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            crate::runtime::test_arc_surface(self.surface.clone())
        }

        fn pull_surface(&mut self) -> UiSurface<()> {
            self.surface.clone()
        }
    }

    fn source_node_ids(surface: &UiSurface<()>) -> Vec<u64> {
        surface
            .runtime_source_traversal_index()
            .records
            .into_iter()
            .map(|record| record.node_id)
            .collect()
    }

    fn assert_authoritative_source(runtime: &SurfaceRuntime<TestBridge, ()>) {
        let expected = source_node_ids(runtime.surface());
        let actual = runtime
            .scratch
            .projection_source
            .records
            .iter()
            .map(|record| record.node_id)
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }

    #[test]
    fn ordinary_startup_keeps_projection_source_authoritative() {
        let runtime = SurfaceRuntime::new(
            TestBridge {
                surface: ordinary_surface(),
            },
            Vector2::new(160.0, 80.0),
        );

        assert_authoritative_source(&runtime);
        assert_eq!(
            runtime.declarative_owner_projection().installation_count(),
            1
        );
    }

    #[test]
    fn runtime_admits_shell_and_complete_batch_before_installing_children() {
        let calls = Rc::new(Cell::new(0));
        let policy = Rc::new(ReadyPolicy {
            calls: Rc::clone(&calls),
            key: 1,
        });
        let runtime = SurfaceRuntime::new(
            TestBridge {
                surface: surface(registration(
                    policy,
                    VirtualLayoutPolicyIdentity::new("policy"),
                )),
            },
            Vector2::new(160.0, 80.0),
        );

        assert_eq!(calls.get(), 1);
        assert_eq!(runtime.virtual_layout.records.len(), 1);
        assert_eq!(
            runtime.virtual_layout.records[0]
                .materialization
                .active_slots()
                .len(),
            1
        );
        assert_eq!(runtime.surface().layout_node().id(), CONTAINER_ID);
        let crate::layout::LayoutNode::Container(root) = runtime.surface().layout_node() else {
            panic!("virtual shell should remain a layout container");
        };
        assert_eq!(root.children.len(), 2, "shell plus one admitted item");
        assert_authoritative_source(&runtime);
        assert_eq!(
            runtime.declarative_owner_projection().installation_count(),
            1
        );
    }

    #[test]
    fn runtime_forwards_one_required_key_before_materialization() {
        let calls = Rc::new(Cell::new(0));
        let runtime = SurfaceRuntime::new(
            TestBridge {
                surface: surface(registration_with_required_key(
                    Rc::new(RequiredKeyPolicy {
                        calls: Rc::clone(&calls),
                        required_key: 7,
                    }),
                    VirtualLayoutPolicyIdentity::new("required-key-policy"),
                    7,
                )),
            },
            Vector2::new(160.0, 80.0),
        );

        assert_eq!(calls.get(), 1);
        let slots = runtime.virtual_layout.records[0]
            .materialization
            .active_slots();
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].item().key(), &VirtualLayoutItemKey::new(7_u32));
    }

    #[test]
    fn virtual_geometry_relayout_keeps_projection_source_authoritative() {
        let mut runtime = SurfaceRuntime::new(
            TestBridge {
                surface: surface(registration(
                    Rc::new(ReadyPolicy {
                        calls: Rc::new(Cell::new(0)),
                        key: 3,
                    }),
                    VirtualLayoutPolicyIdentity::new("geometry-policy"),
                )),
            },
            Vector2::new(160.0, 80.0),
        );
        let source_capacity = runtime.scratch.projection_source.records.capacity();

        assert!(runtime.relayout_virtual_layout_for_geometry());

        assert_authoritative_source(&runtime);
        assert!(runtime.scratch.projection_source.records.capacity() >= source_capacity);
        assert_eq!(
            runtime.declarative_owner_projection().installation_count(),
            2
        );
    }

    #[test]
    fn unchanged_projection_reuses_the_active_window_without_requerying() {
        let calls = Rc::new(Cell::new(0));
        let shell_constructions = Rc::new(Cell::new(0_u32));
        let item_projections = Rc::new(Cell::new(0_u32));
        let kind_projections = Rc::new(Cell::new(0_u32));
        let policy = Rc::new(ReadyPolicy {
            calls: Rc::clone(&calls),
            key: 2,
        });
        let shell_counter = Rc::clone(&shell_constructions);
        let item_counter = Rc::clone(&item_projections);
        let kind_counter = Rc::clone(&kind_projections);
        let registration = registration_with_parts(RegistrationParts {
            policy,
            policy_identity: VirtualLayoutPolicyIdentity::new("policy"),
            revisions: Default::default(),
            shell: Rc::new(move || {
                shell_counter.set(shell_counter.get().saturating_add(1));
                scroll(spacer::<()>())
            }),
            item: Rc::new(move |_| {
                item_counter.set(item_counter.get().saturating_add(1));
                text::<()>("virtual item")
            }),
            kind: Rc::new(move |_| {
                kind_counter.set(kind_counter.get().saturating_add(1));
                VirtualLayoutPolicyIdentity::new("item-kind")
            }),
        });
        let mut runtime = SurfaceRuntime::new(
            TestBridge {
                surface: surface(registration),
            },
            Vector2::new(160.0, 80.0),
        );

        let expected_hit_order = runtime.traversal.widgets.hit_order.clone();
        let expected_focus_order = runtime.traversal.widgets.focusable.order().to_vec();
        let expected_widget_paths = runtime.traversal.widgets.paths.current.clone();
        let expected_virtual_registration_count = runtime
            .traversal
            .containers
            .virtual_layout_registrations
            .len();

        let before_refresh = (
            calls.get(),
            shell_constructions.get(),
            item_projections.get(),
            kind_projections.get(),
            runtime.refresh_counters().runtime_projection,
            runtime.refresh_counters().layout,
            runtime.virtual_layout.materialization_passes,
        );
        let owner_installations = runtime.declarative_owner_projection().installation_count();
        let source_capacity = runtime.scratch.projection_source.records.capacity();
        let stale_source_record = runtime
            .scratch
            .projection_source
            .records
            .first()
            .cloned()
            .expect("virtual startup should have source records");
        runtime
            .scratch
            .projection_source
            .records
            .push(stale_source_record);
        let _ = runtime.take_frame_refresh_diagnostics();
        runtime.refresh_with_scope(crate::runtime::RepaintScope::Projection);

        let frame = runtime.take_frame_refresh_diagnostics();
        assert_eq!(
            frame.effective_scope,
            crate::runtime::RepaintScope::Projection,
            "unchanged cached refresh evidence: {frame:?}"
        );
        assert_eq!(calls.get(), before_refresh.0);
        assert_eq!(shell_constructions.get(), before_refresh.1);
        assert_eq!(item_projections.get(), before_refresh.2);
        assert_eq!(kind_projections.get(), before_refresh.3);
        assert_eq!(
            runtime.refresh_counters().runtime_projection,
            before_refresh.4 + 1,
            "unchanged cached refresh must use only its initial runtime projection"
        );
        assert_eq!(runtime.refresh_counters().layout, before_refresh.5);
        assert_eq!(
            runtime.virtual_layout.materialization_passes,
            before_refresh.6
        );
        assert!(runtime.base_paint_plan_reuse_eligible());
        assert_authoritative_source(&runtime);
        assert!(runtime.scratch.projection_source.records.capacity() >= source_capacity);
        assert_eq!(
            runtime.declarative_owner_projection().installation_count(),
            owner_installations + 1
        );
        assert_eq!(
            frame.view_delta.effect,
            crate::runtime::surface::ViewDeltaEffect::Unchanged
        );
        assert_eq!(
            runtime.virtual_layout.records[0]
                .last_query
                .as_ref()
                .unwrap()
                .viewport_revision,
            0
        );
        assert_eq!(
            runtime.virtual_layout.records[0]
                .materialization
                .active_slots()
                .len(),
            1
        );
        assert!(runtime.virtual_layout.records[0].cached_subtree.is_some());
        assert_eq!(runtime.traversal.widgets.hit_order, expected_hit_order);
        assert_eq!(
            runtime.traversal.widgets.focusable.order(),
            expected_focus_order.as_slice()
        );
        assert_eq!(
            runtime.traversal.widgets.paths.current,
            expected_widget_paths
        );
        assert_eq!(
            runtime
                .traversal
                .containers
                .virtual_layout_registrations
                .len(),
            expected_virtual_registration_count
        );
        assert!(runtime.virtual_layout.projection_probe.is_some());
        runtime.virtual_layout.retire_all();
        assert!(runtime.virtual_layout.projection_probe.is_none());
    }

    #[test]
    fn provisional_virtual_probe_does_not_replace_accepted_owner_projection() {
        let registration = registration_with_parts(RegistrationParts {
            policy: Rc::new(ReadyPolicy {
                calls: Rc::new(Cell::new(0)),
                key: 12,
            }),
            policy_identity: VirtualLayoutPolicyIdentity::new("probe-isolation-policy"),
            revisions: Default::default(),
            shell: Rc::new(|| scroll(spacer::<()>())),
            item: Rc::new(|_| text::<()>("virtual item").key("probe-owner")),
            kind: Rc::new(|_| VirtualLayoutPolicyIdentity::new("item-kind")),
        });
        let mut runtime = SurfaceRuntime::new(
            TestBridge {
                surface: surface(registration),
            },
            Vector2::new(160.0, 80.0),
        );

        runtime.refresh_with_scope(crate::runtime::RepaintScope::Projection);
        let accepted_before_probe_mutation = runtime
            .declarative_owner_projection()
            .accepted_keyed_nodes()
            .to_vec();
        let installations_before_probe_mutation =
            runtime.declarative_owner_projection().installation_count();
        let accepted_keyed_node = accepted_before_probe_mutation
            .first()
            .expect("keyed virtual item should have accepted owner metadata");
        let accepted_identity =
            super::super::declarative_owner::DeclarativeOwnerIdentity::KeyedNode {
                structural_scope: accepted_keyed_node.identity.structural_scope,
            };
        let accepted_token_before = runtime
            .declarative_owner_ledger()
            .live_records()
            .iter()
            .find(|record| record.token.identity() == accepted_identity)
            .map(|record| record.token.clone())
            .expect("keyed virtual item should have a live owner token");
        let generation_before = accepted_token_before.generation();
        let next_generation_before = runtime.declarative_owner_ledger().next_generation();
        let reconciliations_before = runtime.declarative_owner_ledger().reconciliation_count();
        {
            let probe = runtime
                .virtual_layout
                .projection_probe
                .as_mut()
                .expect("unchanged virtual refresh should retain a provisional probe");
            probe.source.records.clear();
        }

        assert!(
            runtime
                .declarative_owner_ledger()
                .is_live(&accepted_token_before)
        );
        assert_eq!(
            runtime.declarative_owner_ledger().next_generation(),
            next_generation_before
        );
        assert_eq!(
            runtime.declarative_owner_ledger().reconciliation_count(),
            reconciliations_before
        );

        runtime.refresh_with_scope(crate::runtime::RepaintScope::Projection);

        let accepted_token_after = runtime
            .declarative_owner_ledger()
            .live_records()
            .iter()
            .find(|record| record.token.identity() == accepted_identity)
            .map(|record| record.token.clone())
            .expect("authoritative materialization should retain the keyed owner token");

        assert_eq!(
            runtime
                .declarative_owner_projection()
                .accepted_keyed_nodes(),
            accepted_before_probe_mutation.as_slice()
        );
        assert_eq!(
            runtime.declarative_owner_projection().installation_count(),
            installations_before_probe_mutation + 1
        );
        assert_eq!(accepted_token_after, accepted_token_before);
        assert_eq!(accepted_token_after.generation(), generation_before);
        assert_eq!(
            runtime.declarative_owner_ledger().next_generation(),
            next_generation_before
        );
        assert_eq!(
            runtime.declarative_owner_ledger().reconciliation_count(),
            reconciliations_before + 1
        );
        assert_authoritative_source(&runtime);
    }

    #[test]
    fn same_id_ordinary_container_replaces_admitted_virtual_container() {
        let calls = Rc::new(Cell::new(0));
        let registration = registration(
            Rc::new(ReadyPolicy {
                calls: Rc::clone(&calls),
                key: 11,
            }),
            VirtualLayoutPolicyIdentity::new("same-id-transition-policy"),
        );
        let mut runtime = SurfaceRuntime::new(
            TestBridge {
                surface: surface(registration),
            },
            Vector2::new(160.0, 80.0),
        );
        assert_eq!(calls.get(), 1);

        runtime.refresh_with_scope(crate::runtime::RepaintScope::Projection);
        assert!(runtime.virtual_layout.projection_probe.is_some());
        let calls_before_transition = calls.get();

        runtime.bridge_mut().surface = ordinary_surface();
        runtime.refresh_with_scope(crate::runtime::RepaintScope::Projection);

        assert!(runtime.virtual_layout.records.is_empty());
        assert!(runtime.virtual_layout.projection_probe.is_none());
        assert_eq!(calls.get(), calls_before_transition);

        let crate::layout::LayoutNode::Container(installed_root) = runtime.surface().layout_node()
        else {
            panic!("same-ID ordinary transition should retain its container");
        };
        assert_eq!(installed_root.id, CONTAINER_ID);
        assert_eq!(installed_root.children.len(), 1);
        assert_eq!(installed_root.children[0].child.id(), ORDINARY_CHILD_ID);

        let installed_projection = runtime.surface().runtime_projection();
        assert_eq!(
            installed_projection.layout_root,
            runtime.surface().layout_node()
        );
        assert!(
            installed_projection
                .traversal
                .widget_paint_order
                .contains(&ORDINARY_CHILD_ID)
        );
        assert!(
            installed_projection
                .traversal
                .virtual_layout_registrations
                .is_empty()
        );
        assert_authoritative_source(&runtime);

        let installed_traversal = runtime.surface().runtime_traversal_index();
        assert!(
            installed_traversal
                .widget_paint_order
                .contains(&ORDINARY_CHILD_ID)
        );
        assert!(installed_traversal.virtual_layout_registrations.is_empty());

        let crate::layout::LayoutNode::Container(layout_root) = &runtime.layout_root else {
            panic!("final layout root should retain the ordinary container");
        };
        assert_eq!(layout_root.id, CONTAINER_ID);
        assert_eq!(layout_root.children.len(), 1);
        assert_eq!(layout_root.children[0].child.id(), ORDINARY_CHILD_ID);
        assert!(runtime.layout().rects.contains_key(&CONTAINER_ID));
        assert!(runtime.layout().rects.contains_key(&ORDINARY_CHILD_ID));
    }

    #[test]
    fn conservative_shell_evidence_keeps_the_normal_fallback_path() {
        let calls = Rc::new(Cell::new(0));
        let shell_constructions = Rc::new(Cell::new(0_u32));
        let shell_counter = Rc::clone(&shell_constructions);
        let registration = registration_with_parts(RegistrationParts {
            policy: Rc::new(ReadyPolicy {
                calls: Rc::clone(&calls),
                key: 6,
            }),
            policy_identity: VirtualLayoutPolicyIdentity::new("conservative-shell-policy"),
            revisions: Default::default(),
            shell: Rc::new(move || {
                shell_counter.set(shell_counter.get().saturating_add(1));
                scroll(empty::<()>())
            }),
            item: Rc::new(|_| text::<()>("virtual item")),
            kind: Rc::new(|_| VirtualLayoutPolicyIdentity::new("item-kind")),
        });
        let mut runtime = SurfaceRuntime::new(
            TestBridge {
                surface: surface(registration),
            },
            Vector2::new(160.0, 80.0),
        );
        let before_layout = runtime.refresh_counters().layout;
        let before_materialization = runtime.virtual_layout.materialization_passes;
        let before_shells = shell_constructions.get();
        let before_calls = calls.get();

        let _ = runtime.take_frame_refresh_diagnostics();
        runtime.refresh_with_scope(crate::runtime::RepaintScope::Projection);

        let frame = runtime.take_frame_refresh_diagnostics();
        assert_eq!(frame.effective_scope, crate::runtime::RepaintScope::Surface);
        assert_eq!(
            frame.view_delta.effect,
            crate::runtime::surface::ViewDeltaEffect::Structural
        );
        assert_eq!(runtime.refresh_counters().layout, before_layout + 1);
        assert_eq!(
            runtime.virtual_layout.materialization_passes,
            before_materialization + 1
        );
        assert_eq!(shell_constructions.get(), before_shells);
        assert_eq!(calls.get(), before_calls);
        assert!(!runtime.base_paint_plan_reuse_eligible());
    }

    #[test]
    fn deferred_query_retains_only_a_complete_matching_fallback_window() {
        let calls = Rc::new(Cell::new(0));
        let controlled = Rc::new(ControlledPolicy {
            calls: Rc::clone(&calls),
            decision: Cell::new(VirtualLayoutPolicyDecision::Ready),
            key: 7,
        });
        let policy: Rc<dyn VirtualLayoutPolicy> = controlled.clone();
        let initial = registration_with_parts(RegistrationParts {
            policy: Rc::clone(&policy),
            policy_identity: VirtualLayoutPolicyIdentity::new("deferred-policy"),
            revisions: Default::default(),
            shell: Rc::new(|| scroll(spacer::<()>())),
            item: Rc::new(|_| text::<()>("virtual item")),
            kind: Rc::new(|_| VirtualLayoutPolicyIdentity::new("item-kind")),
        });
        let mut runtime = SurfaceRuntime::new(
            TestBridge {
                surface: surface(initial),
            },
            Vector2::new(160.0, 80.0),
        );
        assert_eq!(calls.get(), 1);
        let before_materialization = runtime.virtual_layout.materialization_passes;

        controlled
            .decision
            .set(VirtualLayoutPolicyDecision::Deferred(
                VirtualLayoutDeferredReason::DataPending,
            ));
        runtime.bridge_mut().surface = surface(registration_with_parts(RegistrationParts {
            policy,
            policy_identity: VirtualLayoutPolicyIdentity::new("deferred-policy"),
            revisions: VirtualLayoutRegistrationRevisions {
                viewport: 1,
                ..Default::default()
            },
            shell: Rc::new(|| scroll(spacer::<()>())),
            item: Rc::new(|_| text::<()>("virtual item")),
            kind: Rc::new(|_| VirtualLayoutPolicyIdentity::new("item-kind")),
        }));
        runtime.refresh_with_scope(crate::runtime::RepaintScope::Projection);

        assert_eq!(calls.get(), 2);
        assert_eq!(
            runtime.virtual_layout.materialization_passes,
            before_materialization + 1
        );
        assert!(!runtime.virtual_layout.records[0].retired);
        assert!(runtime.virtual_layout.records[0].cached_subtree.is_some());
        assert_eq!(
            runtime.virtual_layout.records[0]
                .materialization
                .active_slots()
                .len(),
            1
        );
        assert_eq!(
            runtime.virtual_layout.records[0]
                .last_query
                .as_ref()
                .expect("the deferred query must remain retryable")
                .viewport_revision,
            0
        );
        let crate::layout::LayoutNode::Container(root) = runtime.surface().layout_node() else {
            panic!("matching fallback should retain the active item");
        };
        assert_eq!(root.children.len(), 2);
    }

    #[test]
    fn unavailable_query_suppresses_stale_items_and_remains_retryable() {
        let calls = Rc::new(Cell::new(0));
        let controlled = Rc::new(ControlledPolicy {
            calls: Rc::clone(&calls),
            decision: Cell::new(VirtualLayoutPolicyDecision::Ready),
            key: 8,
        });
        let policy: Rc<dyn VirtualLayoutPolicy> = controlled.clone();
        let initial = registration_with_parts(RegistrationParts {
            policy: Rc::clone(&policy),
            policy_identity: VirtualLayoutPolicyIdentity::new("unavailable-policy"),
            revisions: Default::default(),
            shell: Rc::new(|| scroll(spacer::<()>())),
            item: Rc::new(|_| text::<()>("virtual item")),
            kind: Rc::new(|_| VirtualLayoutPolicyIdentity::new("item-kind")),
        });
        let mut runtime = SurfaceRuntime::new(
            TestBridge {
                surface: surface(initial),
            },
            Vector2::new(160.0, 80.0),
        );
        assert_eq!(calls.get(), 1);

        controlled
            .decision
            .set(VirtualLayoutPolicyDecision::Unavailable(
                VirtualLayoutUnavailableReason::DataUnavailable,
            ));
        runtime.bridge_mut().surface = surface(registration_with_parts(RegistrationParts {
            policy: Rc::clone(&policy),
            policy_identity: VirtualLayoutPolicyIdentity::new("unavailable-policy"),
            revisions: VirtualLayoutRegistrationRevisions {
                data: 1,
                ..Default::default()
            },
            shell: Rc::new(|| scroll(spacer::<()>())),
            item: Rc::new(|_| text::<()>("virtual item")),
            kind: Rc::new(|_| VirtualLayoutPolicyIdentity::new("item-kind")),
        }));
        runtime.refresh_with_scope(crate::runtime::RepaintScope::Projection);

        assert_eq!(calls.get(), 2);
        assert!(!runtime.virtual_layout.records[0].retired);
        assert!(runtime.virtual_layout.records[0].cached_subtree.is_some());
        assert_eq!(
            runtime.virtual_layout.records[0]
                .last_query
                .as_ref()
                .expect("the unavailable query must remain retryable")
                .data_revision,
            0
        );
        let crate::layout::LayoutNode::Container(root) = runtime.surface().layout_node() else {
            panic!("unavailable fallback should retain the shell container");
        };
        assert_eq!(root.children.len(), 1);

        controlled.decision.set(VirtualLayoutPolicyDecision::Ready);
        runtime.refresh_with_scope(crate::runtime::RepaintScope::Projection);

        assert_eq!(calls.get(), 3);
        assert!(!runtime.virtual_layout.records[0].retired);
        assert_eq!(
            runtime.virtual_layout.records[0]
                .last_query
                .as_ref()
                .expect("the retry should commit a new query")
                .data_revision,
            1
        );
        let crate::layout::LayoutNode::Container(root) = runtime.surface().layout_node() else {
            panic!("a successful retry should reinstall the active item");
        };
        assert_eq!(root.children.len(), 2);
    }

    #[test]
    fn invalid_shell_lowering_retires_and_suppresses_without_retrying() {
        let shell_constructions = Rc::new(Cell::new(0_u32));
        let policy_calls = Rc::new(Cell::new(0));
        let shell_counter = Rc::clone(&shell_constructions);
        let registration = registration_with_parts(RegistrationParts {
            policy: Rc::new(ReadyPolicy {
                calls: Rc::clone(&policy_calls),
                key: 3,
            }),
            policy_identity: VirtualLayoutPolicyIdentity::new("invalid-shell-policy"),
            revisions: Default::default(),
            shell: Rc::new(move || {
                shell_counter.set(shell_counter.get().saturating_add(1));
                text::<()>("invalid shell").id(CONTAINER_ID + 1)
            }),
            item: Rc::new(|_| text::<()>("virtual item")),
            kind: Rc::new(|_| VirtualLayoutPolicyIdentity::new("item-kind")),
        });
        let mut runtime = SurfaceRuntime::new(
            TestBridge {
                surface: surface(registration),
            },
            Vector2::new(160.0, 80.0),
        );

        assert_eq!(shell_constructions.get(), 1);
        assert_eq!(policy_calls.get(), 0);
        assert!(runtime.virtual_layout.records[0].retired);
        assert!(runtime.virtual_layout.records[0].cached_subtree.is_none());
        assert!(
            runtime.virtual_layout.records[0]
                .materialization
                .active_slots()
                .is_empty()
        );
        let crate::layout::LayoutNode::Container(root) = runtime.surface().layout_node() else {
            panic!("invalid shell test should retain the application container");
        };
        assert!(root.children.is_empty());

        runtime.refresh_with_scope(crate::runtime::RepaintScope::Projection);

        assert_eq!(shell_constructions.get(), 1);
        assert_eq!(policy_calls.get(), 0);
        assert!(runtime.virtual_layout.records[0].retired);
        let crate::layout::LayoutNode::Container(root) = runtime.surface().layout_node() else {
            panic!("invalid shell test should retain the application container");
        };
        assert!(root.children.is_empty());
    }

    #[test]
    fn materialize_retirement_resynchronizes_semantic_owner() {
        let (provider, calls, _) = semantic_provider(VirtualLayoutSemanticQueryOutcome::NotFound);
        let registration = registration(
            Rc::new(ReadyPolicy {
                calls: Rc::new(Cell::new(0)),
                key: 3,
            }),
            VirtualLayoutPolicyIdentity::new("semantic-retirement-policy"),
        )
        .with_semantic_provider(provider);
        let mut state = RuntimeVirtualLayoutState::default();
        let mut surface = surface(registration.clone());
        state.prepare_surface(&mut surface, &[registration]);
        assert!(!state.records[0].retired);

        let ticket = match state
            .semantic_demand
            .semantic_pin(CONTAINER_ID, VirtualLayoutItemKey::new(3_u32))
        {
            Ok(SemanticDemandAdmission::Started(ticket)) => ticket,
            other => panic!("semantic demand should start for the active record: {other:?}"),
        };

        // An inert layout with no viewport exercises the existing
        // materialize_surface retirement path.
        state.materialize_surface(
            &mut surface,
            &crate::gui::layout_core::LayoutOutput::default(),
        );

        assert!(state.records[0].retired);
        let stale_execution = state
            .semantic_demand
            .execute(ticket.clone())
            .expect("retired owner authority should make the ticket stale");
        assert!(matches!(stale_execution, SemanticProviderCompletion::Stale));
        assert!(matches!(
            state
                .semantic_demand
                .complete(SemanticProviderCompletion::RequiredItemPin {
                    ticket,
                    outcome: VirtualLayoutSemanticQueryOutcome::NotFound,
                }),
            SemanticDemandCompletion::Stale
        ));
        assert!(matches!(
            state
                .semantic_demand
                .semantic_pin(CONTAINER_ID, VirtualLayoutItemKey::new(3_u32)),
            Err(SemanticDemandAdmissionError::UnknownContainer)
        ));
        assert_eq!(calls.get(), 0);
    }

    #[test]
    fn invalid_complete_batch_admission_retires_and_suppresses_without_retrying() {
        let policy_calls = Rc::new(Cell::new(0));
        let invalid_item_projections = Rc::new(Cell::new(0_u32));
        let policy: Rc<dyn VirtualLayoutPolicy> = Rc::new(ReadyPolicy {
            calls: Rc::clone(&policy_calls),
            key: 4,
        });
        let valid_registration = registration_with_parts(RegistrationParts {
            policy: Rc::clone(&policy),
            policy_identity: VirtualLayoutPolicyIdentity::new("batch-policy"),
            revisions: Default::default(),
            shell: Rc::new(|| scroll(spacer::<()>())),
            item: Rc::new(|_| text::<()>("valid item")),
            kind: Rc::new(|_| VirtualLayoutPolicyIdentity::new("item-kind")),
        });
        let invalid_item_counter = Rc::clone(&invalid_item_projections);
        let invalid_registration = registration_with_parts(RegistrationParts {
            policy,
            policy_identity: VirtualLayoutPolicyIdentity::new("batch-policy"),
            revisions: VirtualLayoutRegistrationRevisions {
                data: 1,
                ..Default::default()
            },
            shell: Rc::new(|| scroll(spacer::<()>())),
            item: Rc::new(move |_| {
                invalid_item_counter.set(invalid_item_counter.get().saturating_add(1));
                text::<()>("invalid item").id(CONTAINER_ID + 1)
            }),
            kind: Rc::new(|_| VirtualLayoutPolicyIdentity::new("item-kind")),
        });
        let mut runtime = SurfaceRuntime::new(
            TestBridge {
                surface: surface(valid_registration),
            },
            Vector2::new(160.0, 80.0),
        );
        assert_eq!(policy_calls.get(), 1);
        let crate::layout::LayoutNode::Container(root) = runtime.surface().layout_node() else {
            panic!("valid virtual shell should remain a layout container");
        };
        assert_eq!(root.children.len(), 2);

        runtime.bridge_mut().surface = surface(invalid_registration);
        runtime.refresh_with_scope(crate::runtime::RepaintScope::Projection);

        assert_eq!(policy_calls.get(), 2);
        assert_eq!(invalid_item_projections.get(), 1);
        assert!(runtime.virtual_layout.records[0].retired);
        assert!(runtime.virtual_layout.records[0].cached_subtree.is_none());
        assert!(
            runtime.virtual_layout.records[0]
                .materialization
                .active_slots()
                .is_empty()
        );
        let crate::layout::LayoutNode::Container(root) = runtime.surface().layout_node() else {
            panic!("retired virtual batch should retain only the shell");
        };
        assert_eq!(root.children.len(), 1);

        runtime.refresh_with_scope(crate::runtime::RepaintScope::Projection);

        assert_eq!(policy_calls.get(), 2);
        assert_eq!(invalid_item_projections.get(), 1);
        assert!(runtime.virtual_layout.records[0].retired);
        let crate::layout::LayoutNode::Container(root) = runtime.surface().layout_node() else {
            panic!("retired virtual batch should remain suppressed");
        };
        assert!(root.children.len() <= 1);
    }

    #[test]
    fn coordinator_begin_error_retires_and_suppresses_without_retrying() {
        let policy_calls = Rc::new(Cell::new(0));
        let policy: Rc<dyn VirtualLayoutPolicy> = Rc::new(ReadyPolicy {
            calls: Rc::clone(&policy_calls),
            key: 5,
        });
        let initial_registration = registration_with_parts(RegistrationParts {
            policy: Rc::clone(&policy),
            policy_identity: VirtualLayoutPolicyIdentity::new("regression-policy"),
            revisions: VirtualLayoutRegistrationRevisions {
                data: 2,
                ..Default::default()
            },
            shell: Rc::new(|| scroll(spacer::<()>())),
            item: Rc::new(|_| text::<()>("virtual item")),
            kind: Rc::new(|_| VirtualLayoutPolicyIdentity::new("item-kind")),
        });
        let regressed_registration = registration_with_parts(RegistrationParts {
            policy,
            policy_identity: VirtualLayoutPolicyIdentity::new("regression-policy"),
            revisions: VirtualLayoutRegistrationRevisions {
                data: 1,
                ..Default::default()
            },
            shell: Rc::new(|| scroll(spacer::<()>())),
            item: Rc::new(|_| text::<()>("virtual item")),
            kind: Rc::new(|_| VirtualLayoutPolicyIdentity::new("item-kind")),
        });
        let mut runtime = SurfaceRuntime::new(
            TestBridge {
                surface: surface(initial_registration),
            },
            Vector2::new(160.0, 80.0),
        );
        assert_eq!(policy_calls.get(), 1);

        runtime.bridge_mut().surface = surface(regressed_registration);
        runtime.refresh_with_scope(crate::runtime::RepaintScope::Projection);

        assert_eq!(policy_calls.get(), 1);
        assert!(runtime.virtual_layout.records[0].retired);
        assert!(runtime.virtual_layout.records[0].cached_subtree.is_none());
        assert!(
            runtime.virtual_layout.records[0]
                .materialization
                .active_slots()
                .is_empty()
        );
        let crate::layout::LayoutNode::Container(root) = runtime.surface().layout_node() else {
            panic!("coordinator failure should retain only the shell");
        };
        assert_eq!(root.children.len(), 1);

        runtime.refresh_with_scope(crate::runtime::RepaintScope::Projection);

        assert_eq!(policy_calls.get(), 1);
        assert!(runtime.virtual_layout.records[0].retired);
        let crate::layout::LayoutNode::Container(root) = runtime.surface().layout_node() else {
            panic!("coordinator failure should remain suppressed");
        };
        assert!(root.children.len() <= 1);
    }

    #[test]
    fn registry_preserves_equal_scope_and_retires_changed_policy_scope() {
        let mut state = RuntimeVirtualLayoutState::default();
        let first = registration(
            Rc::new(ReadyPolicy {
                calls: Rc::new(Cell::new(0)),
                key: 3,
            }),
            VirtualLayoutPolicyIdentity::new("policy"),
        );
        let mut first_surface = surface(first.clone());
        state.prepare_surface(&mut first_surface, &[first]);
        assert_eq!(state.records[0].mount_generation, 1);

        let equal = registration(
            Rc::new(ReadyPolicy {
                calls: Rc::new(Cell::new(0)),
                key: 3,
            }),
            VirtualLayoutPolicyIdentity::new("policy"),
        );
        let mut equal_surface = surface(equal.clone());
        state.prepare_surface(&mut equal_surface, &[equal]);
        assert_eq!(state.records[0].mount_generation, 1);

        let changed = registration(
            Rc::new(ReadyPolicy {
                calls: Rc::new(Cell::new(0)),
                key: 4,
            }),
            VirtualLayoutPolicyIdentity::new("new-policy"),
        );
        let mut changed_surface = surface(changed.clone());
        state.prepare_surface(&mut changed_surface, &[changed]);
        assert_eq!(state.records[0].mount_generation, 2);
        assert!(!state.records[0].retired);
    }
}
