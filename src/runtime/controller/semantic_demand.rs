//! Crate-private semantic-demand ownership for one `SurfaceRuntime`.
//!
//! This module owns crate-private semantic demand, provider attempts, exact
//! retention, and the staged whole-surface publication kernel.  It deliberately
//! has no scheduler, native/product consumer, or snapshot integration.

#![allow(dead_code)]

use super::virtual_layout::VirtualLayoutSemanticClassificationInput;
use crate::{
    application::virtual_layout::VirtualLayoutSemanticCardinality,
    gui::automation::GuiAutomationSnapshot,
    gui::layout_core::{
        VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES, VirtualLayoutSemanticEntry,
        VirtualLayoutSemanticProjection, VirtualLayoutSemanticProjectionBatch,
        VirtualLayoutSemanticProvider, VirtualLayoutSemanticQueryOutcome,
        VirtualLayoutSemanticRange, VirtualLayoutSemanticRangeProvider,
        VirtualLayoutSemanticRangeProviderOutcome, VirtualLayoutSemanticRangeQueryOutcome,
        VirtualLayoutSemanticRangeRequest, VirtualLayoutSemanticRejectedReason,
        VirtualLayoutSemanticRequest, VirtualLayoutSemanticUnavailableReason,
    },
    layout::{NodeId, VirtualLayoutBudget, VirtualLayoutCoordinateSpace, VirtualLayoutItemKey},
    runtime::surface::{MAX_VIRTUAL_LAYOUT_REGISTRATIONS, VirtualLayoutRegistration},
};
use std::{marker::PhantomData, rc::Rc};

const MAX_ACTIVE_RANGE_DEMAND_ENTRIES: usize = VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES;

/// The only semantic-demand sources admitted by this owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SemanticDemandSource {
    Range,
    RequiredItemPin,
}

#[derive(Clone, Debug)]
pub(super) enum SemanticDemand {
    Range(VirtualLayoutSemanticRange),
    RequiredItemPin(VirtualLayoutItemKey),
}

impl SemanticDemand {
    fn source(&self) -> SemanticDemandSource {
        match self {
            Self::Range(_) => SemanticDemandSource::Range,
            Self::RequiredItemPin(_) => SemanticDemandSource::RequiredItemPin,
        }
    }

    fn same_exact(&self, other: &Self) -> Option<bool> {
        match (self, other) {
            (Self::Range(left), Self::Range(right)) => Some(left == right),
            (Self::RequiredItemPin(left), Self::RequiredItemPin(right)) => {
                left.stable_equals(right)
            }
            _ => Some(false),
        }
    }

    fn range_length(&self) -> usize {
        match self {
            Self::Range(range) => range.length(),
            Self::RequiredItemPin(_) => 0,
        }
    }
}

#[derive(Clone, Debug)]
enum SemanticDemandRequest {
    Range(VirtualLayoutSemanticRangeRequest),
    RequiredItemPin(VirtualLayoutSemanticRequest),
}

impl SemanticDemandRequest {
    fn demand(&self) -> SemanticDemand {
        match self {
            Self::Range(request) => SemanticDemand::Range(request.range()),
            Self::RequiredItemPin(request) => {
                SemanticDemand::RequiredItemPin(request.key().clone())
            }
        }
    }

    fn source(&self) -> SemanticDemandSource {
        self.demand().source()
    }
}

/// The owner-issued, non-pointer provider identity carried by one fence.
///
/// `Present` does not identify a handle by itself.  The source-qualified
/// checked `provider_generation` in the same fence identifies the exact handle
/// observed by this owner.  `Missing` is intentionally distinct from `Present`
/// so a missing provider is never treated as a wildcard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SemanticProviderIdentity {
    Missing,
    Present,
}

/// Exact provider-attempt authority owned by the semantic-demand kernel.
#[derive(Clone, Debug)]
pub(super) struct SemanticProviderFence {
    pub(super) container_id: NodeId,
    pub(super) policy_identity: crate::layout::VirtualLayoutPolicyIdentity,
    pub(super) registration_generation: u64,
    pub(super) mount_generation: u64,
    pub(super) data_revision: u64,
    pub(super) policy_revision: u64,
    pub(super) measurement_revision: u64,
    pub(super) semantic_revision: u64,
    pub(super) semantic_cardinality: Option<VirtualLayoutSemanticCardinality>,
    pub(super) coordinate_space: VirtualLayoutCoordinateSpace,
    pub(super) budget: VirtualLayoutBudget,
    pub(super) demand: SemanticDemand,
    pub(super) source: SemanticDemandSource,
    pub(super) provider_identity: SemanticProviderIdentity,
    pub(super) provider_generation: u64,
    pub(super) demand_generation: u64,
    pub(super) attempt: u64,
    pub(super) cancelled: bool,
}

impl SemanticProviderFence {
    /// Compare every field, including attempt, demand generation, and
    /// cancellation.  Unstable opaque equality never authorizes a match.
    pub(super) fn same_exact(&self, other: &Self) -> bool {
        self.container_id == other.container_id
            && stable_policy_identity_equals(&self.policy_identity, &other.policy_identity)
                == Some(true)
            && self.registration_generation == other.registration_generation
            && self.mount_generation == other.mount_generation
            && self.data_revision == other.data_revision
            && self.policy_revision == other.policy_revision
            && self.measurement_revision == other.measurement_revision
            && self.semantic_revision == other.semantic_revision
            && self.semantic_cardinality == other.semantic_cardinality
            && stable_coordinate_space_equals(&self.coordinate_space, &other.coordinate_space)
                == Some(true)
            && self.budget == other.budget
            && self.demand.same_exact(&other.demand) == Some(true)
            && self.source == other.source
            && self.provider_identity == other.provider_identity
            && self.provider_generation == other.provider_generation
            && self.demand_generation == other.demand_generation
            && self.attempt == other.attempt
            && self.cancelled == other.cancelled
    }

    /// Retention excludes exactly attempt, demand generation, and
    /// cancellation.  Every identity, live revision, coordinate, budget,
    /// demand, source, and provider field remains exact.
    fn same_retention_fence(&self, other: &Self) -> bool {
        self.container_id == other.container_id
            && stable_policy_identity_equals(&self.policy_identity, &other.policy_identity)
                == Some(true)
            && self.registration_generation == other.registration_generation
            && self.mount_generation == other.mount_generation
            && self.data_revision == other.data_revision
            && self.policy_revision == other.policy_revision
            && self.measurement_revision == other.measurement_revision
            && self.semantic_revision == other.semantic_revision
            && self.semantic_cardinality == other.semantic_cardinality
            && stable_coordinate_space_equals(&self.coordinate_space, &other.coordinate_space)
                == Some(true)
            && self.budget == other.budget
            && self.demand.same_exact(&other.demand) == Some(true)
            && self.source == other.source
            && self.provider_identity == other.provider_identity
            && self.provider_generation == other.provider_generation
            && !self.cancelled
            && !other.cancelled
    }
}

impl PartialEq for SemanticProviderFence {
    fn eq(&self, other: &Self) -> bool {
        self.same_exact(other)
    }
}

impl Eq for SemanticProviderFence {}

#[derive(Clone, Debug, PartialEq)]
enum SemanticEvidence {
    Pin(Box<SemanticPinEvidence>),
    Range(Box<SemanticRangeEvidence>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SemanticSlotStatus {
    Pending,
    Found,
    NotFound,
    Fallback,
    Withheld,
    Terminal,
}

#[derive(Clone, Debug, PartialEq)]
struct SemanticPinEvidence {
    entry: Option<VirtualLayoutSemanticEntry>,
    projection: Option<VirtualLayoutSemanticProjection>,
}

#[derive(Clone, Debug, PartialEq)]
struct SemanticRangeEvidence {
    entries: Vec<VirtualLayoutSemanticEntry>,
    projections: Vec<VirtualLayoutSemanticProjection>,
}

#[derive(Clone, Debug)]
struct SemanticRetainedEvidence {
    fence: SemanticProviderFence,
    evidence: SemanticEvidence,
}

#[derive(Clone, Debug)]
struct SemanticDemandSlot {
    request: SemanticDemandRequest,
    fence: SemanticProviderFence,
    executed: bool,
    completed: bool,
    evidence: Option<SemanticEvidence>,
    withheld: bool,
    retained: Option<SemanticRetainedEvidence>,
    /// Membership is retained independently of the current attempt/evidence.
    /// A terminal attempt therefore cannot make an active demand disappear.
    status: SemanticSlotStatus,
}

/// Exact composition authorities supplied by the current materialization and
/// ordinary-projection owners.  These are opaque generations here: the
/// materialization/classification consumers own how they advance them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SemanticPublicationAuthorities {
    pub(super) session_generation: u64,
    pub(super) materialization_authority: u64,
    pub(super) classification_authority: u64,
    pub(super) ordinary_projection_generation: u64,
}

/// Whole-surface publication fence for one provider member.  A complete
/// publication carries one exact fence for every active demand member.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SemanticPublicationFence {
    pub(super) provider_fence: SemanticProviderFence,
    pub(super) session_generation: u64,
    pub(super) materialization_authority: u64,
    pub(super) classification_authority: u64,
    pub(super) ordinary_projection_generation: u64,
    pub(super) complete_demand_set_generation: u64,
}

impl SemanticPublicationFence {
    fn new(
        provider_fence: SemanticProviderFence,
        authorities: SemanticPublicationAuthorities,
        complete_demand_set_generation: u64,
    ) -> Self {
        Self {
            provider_fence,
            session_generation: authorities.session_generation,
            materialization_authority: authorities.materialization_authority,
            classification_authority: authorities.classification_authority,
            ordinary_projection_generation: authorities.ordinary_projection_generation,
            complete_demand_set_generation,
        }
    }

    pub(super) fn same_exact(&self, other: &Self) -> bool {
        self.provider_fence.same_exact(&other.provider_fence)
            && self.session_generation == other.session_generation
            && self.materialization_authority == other.materialization_authority
            && self.classification_authority == other.classification_authority
            && self.ordinary_projection_generation == other.ordinary_projection_generation
            && self.complete_demand_set_generation == other.complete_demand_set_generation
    }
}

/// Provider evidence normalized for the materialization classifier.  Empty
/// evidence is authoritative `NotFound`; it has no classifier input.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum SemanticPublicationEvidence {
    Pin {
        request: VirtualLayoutSemanticRequest,
        projection: Box<VirtualLayoutSemanticProjection>,
    },
    Range {
        request: VirtualLayoutSemanticRangeRequest,
        projections: Vec<VirtualLayoutSemanticProjection>,
    },
}

impl SemanticPublicationEvidence {
    pub(super) fn pin_parts(
        &self,
    ) -> Option<(
        &VirtualLayoutSemanticRequest,
        &VirtualLayoutSemanticProjection,
    )> {
        match self {
            Self::Pin {
                request,
                projection,
            } => Some((request, projection.as_ref())),
            Self::Range { .. } => None,
        }
    }

    pub(super) fn range_parts(
        &self,
    ) -> Option<(
        &VirtualLayoutSemanticRangeRequest,
        &[VirtualLayoutSemanticProjection],
    )> {
        match self {
            Self::Range {
                request,
                projections,
            } => Some((request, projections)),
            Self::Pin { .. } => None,
        }
    }
}

/// One active demand member staged for whole-surface publication.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct SemanticPublicationMember {
    container_id: NodeId,
    source: SemanticDemandSource,
    fence: SemanticPublicationFence,
    evidence: Option<SemanticPublicationEvidence>,
    resolved: bool,
}

impl SemanticPublicationMember {
    pub(super) fn container_id(&self) -> NodeId {
        self.container_id
    }

    pub(super) fn source(&self) -> SemanticDemandSource {
        self.source
    }

    pub(super) fn fence(&self) -> &SemanticPublicationFence {
        &self.fence
    }

    pub(super) fn evidence(&self) -> Option<&SemanticPublicationEvidence> {
        self.evidence.as_ref()
    }

    pub(super) fn resolved(&self) -> bool {
        self.resolved
    }
}

/// Classification evidence paired with the exact publication fence that
/// admitted it.  The owner accepts this only when the second publication
/// phase still carries the same member fence.
#[derive(Clone, Debug)]
pub(super) struct SemanticPublicationClassification {
    fence: SemanticPublicationFence,
    input: VirtualLayoutSemanticClassificationInput,
}

impl SemanticPublicationClassification {
    pub(super) fn new(
        fence: SemanticPublicationFence,
        input: VirtualLayoutSemanticClassificationInput,
    ) -> Self {
        Self { fence, input }
    }

    pub(super) fn fence(&self) -> &SemanticPublicationFence {
        &self.fence
    }

    pub(super) fn input(&self) -> &VirtualLayoutSemanticClassificationInput {
        &self.input
    }
}

/// Immutable first phase of owner publication.  It contains every active
/// member, including unresolved terminal members, so an incomplete set cannot
/// be mistaken for a smaller complete set.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct SemanticPublicationPlan {
    authorities: SemanticPublicationAuthorities,
    complete_demand_set_generation: u64,
    members: Vec<SemanticPublicationMember>,
    complete: bool,
}

impl SemanticPublicationPlan {
    pub(super) fn members(&self) -> &[SemanticPublicationMember] {
        &self.members
    }

    pub(super) fn complete(&self) -> bool {
        self.complete
    }

    pub(super) fn complete_demand_set_generation(&self) -> u64 {
        self.complete_demand_set_generation
    }

    fn authorities(&self) -> SemanticPublicationAuthorities {
        self.authorities
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SemanticPublicationFallbackReason {
    IncompleteDemandSet,
    StalePlan,
    ClassificationRejected,
    CompositionRejected,
    CounterOverflow,
}

/// Result of one atomic publication attempt.  The ordinary baseline is
/// returned on every incomplete, stale, malformed, or composition-vetoed
/// attempt; no partial virtual tree is exposed.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum SemanticPublicationOutcome {
    Published(crate::runtime::controller::VirtualLayoutAutomationComposition),
    OrdinaryBaseline {
        composition: crate::runtime::controller::VirtualLayoutAutomationComposition,
        reason: SemanticPublicationFallbackReason,
    },
}

#[derive(Clone, Debug, PartialEq)]
struct SemanticCompleteCandidate {
    authorities: SemanticPublicationAuthorities,
    ordinary: GuiAutomationSnapshot,
    plan: SemanticPublicationPlan,
    composition: crate::runtime::controller::VirtualLayoutAutomationComposition,
}

/// A reduced live authority copied from one accepted runtime record.
///
/// This is semantic provider authority only; it is not a second virtual-layout
/// registration registry and does not retain shell, item, policy, or
/// materialization ownership.
#[derive(Clone)]
struct SemanticLiveAuthority {
    container_id: NodeId,
    policy_identity: crate::layout::VirtualLayoutPolicyIdentity,
    mount_generation: u64,
    data_revision: u64,
    policy_revision: u64,
    measurement_revision: u64,
    semantic_revision: u64,
    semantic_cardinality: Option<VirtualLayoutSemanticCardinality>,
    coordinate_space: VirtualLayoutCoordinateSpace,
    budget: VirtualLayoutBudget,
    semantic_provider: Option<Rc<dyn VirtualLayoutSemanticProvider>>,
    semantic_range_provider: Option<Rc<dyn VirtualLayoutSemanticRangeProvider>>,
    semantic_provider_token: Option<usize>,
    semantic_range_provider_token: Option<usize>,
}

impl SemanticLiveAuthority {
    fn from_registration<Message>(
        registration: &VirtualLayoutRegistration<Message>,
        mount_generation: u64,
    ) -> Self {
        Self {
            container_id: registration.container_id,
            policy_identity: registration.policy_identity.clone(),
            mount_generation,
            data_revision: registration.data_revision(),
            policy_revision: registration.policy_revision(),
            measurement_revision: registration.measurement_revision(),
            semantic_revision: registration.semantic_revision(),
            semantic_cardinality: registration.semantic_cardinality,
            coordinate_space: registration.coordinate_space.clone(),
            budget: registration.budget,
            semantic_provider: registration.semantic_provider_handle(),
            semantic_range_provider: registration.semantic_range_provider_handle(),
            semantic_provider_token: registration.semantic_provider_token(),
            semantic_range_provider_token: registration.semantic_range_provider_token(),
        }
    }

    fn same_scope<Message>(&self, registration: &VirtualLayoutRegistration<Message>) -> bool {
        self.container_id == registration.container_id
            && stable_policy_identity_equals(&self.policy_identity, &registration.policy_identity)
                == Some(true)
    }

    fn shared_live_changed<Message>(
        &self,
        registration: &VirtualLayoutRegistration<Message>,
        mount_generation: u64,
    ) -> bool {
        self.mount_generation != mount_generation
            || self.data_revision != registration.data_revision()
            || self.policy_revision != registration.policy_revision()
            || self.measurement_revision != registration.measurement_revision()
            || self.semantic_revision != registration.semantic_revision()
            || self.semantic_cardinality != registration.semantic_cardinality
            || stable_coordinate_space_equals(
                &self.coordinate_space,
                &registration.coordinate_space,
            ) != Some(true)
            || self.budget != registration.budget
    }

    fn update_from<Message>(
        &mut self,
        registration: &VirtualLayoutRegistration<Message>,
        mount_generation: u64,
    ) {
        *self = Self::from_registration(registration, mount_generation);
    }

    fn request_for_demand(&self, demand: SemanticDemand) -> SemanticDemandRequest {
        match demand {
            SemanticDemand::Range(range) => {
                SemanticDemandRequest::Range(VirtualLayoutSemanticRangeRequest::new(
                    self.container_id,
                    self.policy_identity.clone(),
                    self.mount_generation,
                    self.data_revision,
                    self.policy_revision,
                    self.measurement_revision,
                    self.semantic_revision,
                    self.coordinate_space.clone(),
                    self.budget,
                    range,
                ))
            }
            SemanticDemand::RequiredItemPin(key) => {
                SemanticDemandRequest::RequiredItemPin(VirtualLayoutSemanticRequest::new(
                    self.container_id,
                    self.policy_identity.clone(),
                    self.mount_generation,
                    self.data_revision,
                    self.policy_revision,
                    self.measurement_revision,
                    self.semantic_revision,
                    key,
                ))
            }
        }
    }

    fn provider_identity(&self, source: SemanticDemandSource) -> SemanticProviderIdentity {
        match source {
            SemanticDemandSource::Range => self
                .semantic_range_provider
                .as_ref()
                .map_or(SemanticProviderIdentity::Missing, |_| {
                    SemanticProviderIdentity::Present
                }),
            SemanticDemandSource::RequiredItemPin => self
                .semantic_provider
                .as_ref()
                .map_or(SemanticProviderIdentity::Missing, |_| {
                    SemanticProviderIdentity::Present
                }),
        }
    }
}

#[derive(Clone)]
struct SemanticDemandRecord {
    authority: SemanticLiveAuthority,
    registration_generation: u64,
    semantic_provider_generation: u64,
    semantic_range_provider_generation: u64,
    range: Option<SemanticDemandSlot>,
    pin: Option<SemanticDemandSlot>,
    retired: bool,
}

impl SemanticDemandRecord {
    fn has_members(&self) -> bool {
        self.range.is_some() || self.pin.is_some()
    }

    fn cancel_and_clear(&mut self) {
        [&mut self.range, &mut self.pin]
            .into_iter()
            .flatten()
            .for_each(|slot| slot.fence.cancelled = true);
        self.range = None;
        self.pin = None;
        self.retired = true;
    }

    fn provider_generation(&self, source: SemanticDemandSource) -> u64 {
        match source {
            SemanticDemandSource::Range => self.semantic_range_provider_generation,
            SemanticDemandSource::RequiredItemPin => self.semantic_provider_generation,
        }
    }
}

/// Typed result of explicit owner-level demand admission.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum SemanticDemandAdmission {
    Started(SemanticAttemptTicket),
    Unchanged,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SemanticDemandAdmissionError {
    DuplicateSource,
    UnknownContainer,
    Retired,
    ScopeMismatch,
    InvalidKey,
    InvalidRange(VirtualLayoutSemanticRejectedReason),
    CustomCoordinate,
    AggregateBudgetExceeded,
    CounterOverflow,
    NoActiveDemand,
    Reentrant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SemanticDemandExecutionError {
    AlreadyExecuted,
    Reentrant,
}

#[derive(Clone, Debug)]
pub(super) enum SemanticProviderCompletion {
    RequiredItemPin {
        ticket: SemanticAttemptTicket,
        outcome: VirtualLayoutSemanticQueryOutcome,
    },
    Range {
        ticket: SemanticAttemptTicket,
        outcome: VirtualLayoutSemanticRangeProviderOutcome,
    },
    Stale,
}

#[derive(Clone, Debug)]
pub(super) enum SemanticDemandCompletion {
    RequiredItemPin(VirtualLayoutSemanticQueryOutcome),
    Range(VirtualLayoutSemanticRangeQueryOutcome),
    Stale,
}

/// One semantic-demand owner for one `SurfaceRuntime`.
pub(super) struct SemanticDemandOwner<Message> {
    records: Vec<SemanticDemandRecord>,
    next_registration_generation: u64,
    next_semantic_provider_generation: u64,
    next_semantic_range_provider_generation: u64,
    next_demand_generation: u64,
    next_demand_set_generation: u64,
    demand_set_generation: u64,
    provider_call_in_progress: bool,
    last_complete_candidate: Option<SemanticCompleteCandidate>,
    _message: PhantomData<fn() -> Message>,
}

impl<Message> Clone for SemanticDemandOwner<Message> {
    fn clone(&self) -> Self {
        Self {
            records: self.records.clone(),
            next_registration_generation: self.next_registration_generation,
            next_semantic_provider_generation: self.next_semantic_provider_generation,
            next_semantic_range_provider_generation: self.next_semantic_range_provider_generation,
            next_demand_generation: self.next_demand_generation,
            next_demand_set_generation: self.next_demand_set_generation,
            demand_set_generation: self.demand_set_generation,
            provider_call_in_progress: self.provider_call_in_progress,
            last_complete_candidate: self.last_complete_candidate.clone(),
            _message: PhantomData,
        }
    }
}

impl<Message> Default for SemanticDemandOwner<Message> {
    fn default() -> Self {
        Self {
            records: Vec::new(),
            next_registration_generation: 0,
            next_semantic_provider_generation: 0,
            next_semantic_range_provider_generation: 0,
            next_demand_generation: 0,
            next_demand_set_generation: 0,
            demand_set_generation: 0,
            provider_call_in_progress: false,
            last_complete_candidate: None,
            _message: PhantomData,
        }
    }
}

impl<Message> SemanticDemandOwner<Message> {
    /// Synchronize only the reduced semantic authority of accepted mounted
    /// records.  New capability registration creates no demand, attempt,
    /// provider call, or publication.  Existing active demand is restarted
    /// only when an exact live fence or its source provider handle changes.
    pub(super) fn synchronize(
        &mut self,
        registrations: &[(VirtualLayoutRegistration<Message>, u64)],
    ) -> Vec<SemanticAttemptTicket> {
        self.synchronize_with_change(registrations).1
    }

    /// Synchronize semantic authority and report whether the live authority
    /// changed alongside the pending restart tickets.
    pub(super) fn synchronize_with_change(
        &mut self,
        registrations: &[(VirtualLayoutRegistration<Message>, u64)],
    ) -> (bool, Vec<SemanticAttemptTicket>) {
        if registrations.len() > MAX_VIRTUAL_LAYOUT_REGISTRATIONS
            || contains_duplicate_container(registrations)
        {
            self.retire_all();
            return (true, Vec::new());
        }

        let active_ids: Vec<NodeId> = registrations
            .iter()
            .map(|(registration, _)| registration.container_id)
            .collect();
        let mut changed = false;
        let mut index = 0;
        while index < self.records.len() {
            if active_ids.contains(&self.records[index].authority.container_id) {
                index += 1;
            } else {
                changed = true;
                if self.records[index].has_members()
                    && (self.ensure_demand_set_capacity().is_err()
                        || self.advance_demand_set_generation().is_err())
                {
                    self.retire_all();
                    return (true, Vec::new());
                }
                let mut record = self.records.remove(index);
                record.cancel_and_clear();
            }
        }

        let mut refresh_tickets = Vec::new();
        for (registration, mount_generation) in registrations {
            let authority =
                SemanticLiveAuthority::from_registration(registration, *mount_generation);
            let Some(existing_index) = self.record_index(registration.container_id) else {
                changed = true;
                let Some(record) = self.new_record(authority) else {
                    self.retire_all();
                    return (true, Vec::new());
                };
                self.records.push(record);
                continue;
            };

            if !self.records[existing_index]
                .authority
                .same_scope(registration)
            {
                changed = true;
                if self.records[existing_index].has_members()
                    && (self.ensure_demand_set_capacity().is_err()
                        || self.advance_demand_set_generation().is_err())
                {
                    self.retire_all();
                    return (true, Vec::new());
                }
                let mut old = self.records.remove(existing_index);
                old.cancel_and_clear();
                let Some(record) = self.new_record(authority) else {
                    continue;
                };
                self.records.insert(existing_index, record);
                continue;
            }

            let shared_live_changed = self.records[existing_index]
                .authority
                .shared_live_changed(registration, *mount_generation);
            let semantic_provider_changed = self.records[existing_index]
                .authority
                .semantic_provider_token
                != registration.semantic_provider_token();
            let semantic_range_provider_changed = self.records[existing_index]
                .authority
                .semantic_range_provider_token
                != registration.semantic_range_provider_token();
            let live_changed =
                shared_live_changed || semantic_provider_changed || semantic_range_provider_changed;
            if !live_changed {
                self.records[existing_index]
                    .authority
                    .update_from(registration, *mount_generation);
                continue;
            }

            changed = true;
            self.records[existing_index]
                .authority
                .update_from(registration, *mount_generation);
            if semantic_provider_changed {
                let Some(next) = checked_next(&mut self.next_semantic_provider_generation) else {
                    let mut record = self.records.remove(existing_index);
                    record.cancel_and_clear();
                    continue;
                };
                self.records[existing_index].semantic_provider_generation = next;
            }
            if semantic_range_provider_changed {
                let Some(next) = checked_next(&mut self.next_semantic_range_provider_generation)
                else {
                    let mut record = self.records.remove(existing_index);
                    record.cancel_and_clear();
                    continue;
                };
                self.records[existing_index].semantic_range_provider_generation = next;
            }

            // The owner is logical-only.  A live transition to a custom
            // coordinate cancels existing attempts and clears the authority;
            // it never gets an identity-transform fallback ticket.
            if !is_logical_coordinate(&self.records[existing_index].authority.coordinate_space) {
                self.clear_record_slots(existing_index);
                continue;
            }

            let sources = [
                (
                    SemanticDemandSource::Range,
                    shared_live_changed || semantic_range_provider_changed,
                ),
                (
                    SemanticDemandSource::RequiredItemPin,
                    shared_live_changed || semantic_provider_changed,
                ),
            ];
            for (source, should_restart) in sources {
                if !should_restart {
                    continue;
                }
                if self.slot(existing_index, source).is_none() {
                    continue;
                }
                match self.restart_slot(existing_index, source) {
                    Ok(ticket) => refresh_tickets.push(ticket),
                    Err(_) => self.clear_slot(existing_index, source),
                }
            }
        }
        (changed, refresh_tickets)
    }

    /// Retire all semantic authority and cancel active attempts before clearing
    /// the owner state.
    pub(super) fn retire_all(&mut self) {
        for record in &mut self.records {
            record.cancel_and_clear();
        }
        self.records.clear();
        self.last_complete_candidate = None;
    }

    /// Atomically replace the complete active demand set. Validation and
    /// counter allocation happen on a staged owner, so a rejected member or
    /// overflow cannot leave a partially applied set behind.
    pub(super) fn replace_demand_set(
        &mut self,
        demands: &[(NodeId, SemanticDemand)],
    ) -> Result<Vec<SemanticAttemptTicket>, SemanticDemandAdmissionError> {
        if self.provider_call_in_progress {
            return Err(SemanticDemandAdmissionError::Reentrant);
        }

        let mut staged = self.clone();
        let mut seen = Vec::with_capacity(demands.len());
        let mut aggregate_range_length = 0_usize;
        for (container_id, demand) in demands {
            let index = staged.authority_index(*container_id)?;
            staged.validate_intent_authority(index)?;
            let source = demand.source();
            if seen
                .iter()
                .any(|(id, previous)| *id == *container_id && *previous == source)
            {
                return Err(SemanticDemandAdmissionError::DuplicateSource);
            }
            if let SemanticDemand::Range(range) = demand {
                range
                    .validate_budget(staged.records[index].authority.budget)
                    .map_err(SemanticDemandAdmissionError::InvalidRange)?;
                aggregate_range_length = aggregate_range_length
                    .checked_add(range.length())
                    .ok_or(SemanticDemandAdmissionError::CounterOverflow)?;
            } else if let SemanticDemand::RequiredItemPin(key) = demand
                && key.stable_equals(key) != Some(true)
            {
                return Err(SemanticDemandAdmissionError::InvalidKey);
            }
            seen.push((*container_id, source));
        }
        if aggregate_range_length > MAX_ACTIVE_RANGE_DEMAND_ENTRIES {
            return Err(SemanticDemandAdmissionError::AggregateBudgetExceeded);
        }

        let current_member_count = staged
            .records
            .iter()
            .map(|record| usize::from(record.range.is_some()) + usize::from(record.pin.is_some()))
            .sum::<usize>();
        let mut changed = current_member_count != demands.len();
        if !changed {
            changed = demands.iter().any(|(container_id, demand)| {
                let Some(index) = staged.record_index(*container_id) else {
                    return true;
                };
                staged
                    .slot(index, demand.source())
                    .and_then(|slot| slot.request.demand().same_exact(demand))
                    != Some(true)
            });
        }

        if !changed {
            let tickets = demands
                .iter()
                .filter_map(|(container_id, demand)| {
                    let index = staged.record_index(*container_id)?;
                    let slot = staged.slot(index, demand.source())?;
                    (slot.status == SemanticSlotStatus::Pending && !slot.executed).then(|| {
                        SemanticAttemptTicket {
                            fence: slot.fence.clone(),
                        }
                    })
                })
                .collect();
            return Ok(tickets);
        }

        staged.ensure_demand_set_capacity()?;
        for index in 0..staged.records.len() {
            for source in [
                SemanticDemandSource::Range,
                SemanticDemandSource::RequiredItemPin,
            ] {
                if staged.slot(index, source).is_some()
                    && !seen.iter().any(|(id, member_source)| {
                        *id == staged.records[index].authority.container_id
                            && *member_source == source
                    })
                {
                    staged.remove_slot(index, source);
                }
            }
        }

        let mut tickets = Vec::with_capacity(demands.len());
        for (container_id, demand) in demands {
            let index = staged
                .record_index(*container_id)
                .ok_or(SemanticDemandAdmissionError::UnknownContainer)?;
            let source = demand.source();
            let unchanged = staged
                .slot(index, source)
                .and_then(|slot| slot.request.demand().same_exact(demand))
                == Some(true);
            if unchanged {
                if let Some(slot) = staged.slot(index, source)
                    && slot.status == SemanticSlotStatus::Pending
                    && !slot.executed
                {
                    tickets.push(SemanticAttemptTicket {
                        fence: slot.fence.clone(),
                    });
                }
                continue;
            }
            let request = staged.records[index]
                .authority
                .request_for_demand(demand.clone());
            tickets.push(staged.start_slot(index, request, None)?);
        }
        staged.advance_demand_set_generation()?;
        *self = staged;
        Ok(tickets)
    }

    /// Explicitly restart every active source without executing any provider.
    /// The complete owner is staged so a counter failure leaves all old slots
    /// and their retained evidence untouched.
    pub(super) fn retry_all(
        &mut self,
    ) -> Result<Vec<SemanticAttemptTicket>, SemanticDemandAdmissionError> {
        if self.provider_call_in_progress {
            return Err(SemanticDemandAdmissionError::Reentrant);
        }
        let mut staged = self.clone();
        let active = staged
            .records
            .iter()
            .enumerate()
            .flat_map(|(index, record)| {
                [
                    (index, SemanticDemandSource::Range, record.range.is_some()),
                    (
                        index,
                        SemanticDemandSource::RequiredItemPin,
                        record.pin.is_some(),
                    ),
                ]
                .into_iter()
                .filter_map(|(index, source, present)| present.then_some((index, source)))
                .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        if active.is_empty() {
            return Err(SemanticDemandAdmissionError::NoActiveDemand);
        }

        let mut tickets = Vec::with_capacity(active.len());
        for (index, source) in active {
            tickets.push(staged.restart_slot(index, source)?);
        }
        *self = staged;
        Ok(tickets)
    }

    /// Cancel all active demand membership while retaining live authorities.
    pub(super) fn clear_demands(&mut self) -> Result<(), SemanticDemandAdmissionError> {
        if self.provider_call_in_progress {
            return Err(SemanticDemandAdmissionError::Reentrant);
        }
        let mut staged = self.clone();
        let had_members = staged.records.iter().any(SemanticDemandRecord::has_members);
        if had_members {
            staged.ensure_demand_set_capacity()?;
            for index in 0..staged.records.len() {
                staged.remove_slot(index, SemanticDemandSource::Range);
                staged.remove_slot(index, SemanticDemandSource::RequiredItemPin);
            }
            staged.advance_demand_set_generation()?;
        }
        staged.last_complete_candidate = None;
        *self = staged;
        Ok(())
    }

    /// Admit or repeat one exact logical range demand.  All fence fields are
    /// constructed from the current accepted runtime authority.
    pub(super) fn range(
        &mut self,
        container_id: NodeId,
        start_index: usize,
        length: usize,
    ) -> Result<SemanticDemandAdmission, SemanticDemandAdmissionError> {
        if self.provider_call_in_progress {
            return Err(SemanticDemandAdmissionError::Reentrant);
        }
        let range = VirtualLayoutSemanticRange::new(start_index, length)
            .map_err(SemanticDemandAdmissionError::InvalidRange)?;
        let index = self.authority_index(container_id)?;
        self.validate_intent_authority(index)?;
        range
            .validate_budget(self.records[index].authority.budget)
            .map_err(SemanticDemandAdmissionError::InvalidRange)?;

        let current_length = self
            .slot(index, SemanticDemandSource::Range)
            .filter(|slot| slot.status != SemanticSlotStatus::Terminal)
            .map(|slot| slot.request.demand().range_length())
            .unwrap_or(0);
        let aggregate = self
            .active_range_length()
            .ok_or(SemanticDemandAdmissionError::CounterOverflow)?
            .checked_sub(current_length)
            .and_then(|active| active.checked_add(range.length()))
            .ok_or(SemanticDemandAdmissionError::CounterOverflow)?;
        if aggregate > MAX_ACTIVE_RANGE_DEMAND_ENTRIES {
            return Err(SemanticDemandAdmissionError::AggregateBudgetExceeded);
        }

        let request = SemanticDemandRequest::Range(VirtualLayoutSemanticRangeRequest::new(
            container_id,
            self.records[index].authority.policy_identity.clone(),
            self.records[index].authority.mount_generation,
            self.records[index].authority.data_revision,
            self.records[index].authority.policy_revision,
            self.records[index].authority.measurement_revision,
            self.records[index].authority.semantic_revision,
            self.records[index].authority.coordinate_space.clone(),
            self.records[index].authority.budget,
            range,
        ));
        self.start_new_demand(index, request)
    }

    /// Admit or repeat one exact semantic pin demand.  The caller supplies no
    /// policy, revision, mount, provider, budget, or coordinate fence fields.
    pub(super) fn semantic_pin(
        &mut self,
        container_id: NodeId,
        key: VirtualLayoutItemKey,
    ) -> Result<SemanticDemandAdmission, SemanticDemandAdmissionError> {
        if self.provider_call_in_progress {
            return Err(SemanticDemandAdmissionError::Reentrant);
        }
        if key.stable_equals(&key) != Some(true) {
            return Err(SemanticDemandAdmissionError::InvalidKey);
        }
        let index = self.authority_index(container_id)?;
        self.validate_intent_authority(index)?;

        let request = SemanticDemandRequest::RequiredItemPin(VirtualLayoutSemanticRequest::new(
            container_id,
            self.records[index].authority.policy_identity.clone(),
            self.records[index].authority.mount_generation,
            self.records[index].authority.data_revision,
            self.records[index].authority.policy_revision,
            self.records[index].authority.measurement_revision,
            self.records[index].authority.semantic_revision,
            key,
        ));
        self.start_new_demand(index, request)
    }

    /// Explicitly retry the unchanged range demand.  A retry changes only the
    /// attempt and leaves the demand generation and retention fence intact.
    pub(super) fn retry_range(
        &mut self,
        container_id: NodeId,
    ) -> Result<SemanticAttemptTicket, SemanticDemandAdmissionError> {
        self.retry_source(container_id, SemanticDemandSource::Range)
    }

    /// Explicitly retry the unchanged semantic pin demand.
    pub(super) fn retry_semantic_pin(
        &mut self,
        container_id: NodeId,
    ) -> Result<SemanticAttemptTicket, SemanticDemandAdmissionError> {
        self.retry_source(container_id, SemanticDemandSource::RequiredItemPin)
    }

    /// Remove the active range membership without invoking a provider.  The
    /// complete demand-set generation advances even when the member's last
    /// attempt was terminal, so a failed member cannot silently vanish from a
    /// publication set.
    pub(super) fn remove_range_demand(
        &mut self,
        container_id: NodeId,
    ) -> Result<(), SemanticDemandAdmissionError> {
        self.remove_source(container_id, SemanticDemandSource::Range)
    }

    /// Remove the independent one-item semantic-pin membership without
    /// invoking a provider.
    pub(super) fn remove_semantic_pin(
        &mut self,
        container_id: NodeId,
    ) -> Result<(), SemanticDemandAdmissionError> {
        self.remove_source(container_id, SemanticDemandSource::RequiredItemPin)
    }

    pub(super) fn demand_set_generation(&self) -> u64 {
        self.demand_set_generation
    }

    fn remove_source(
        &mut self,
        container_id: NodeId,
        source: SemanticDemandSource,
    ) -> Result<(), SemanticDemandAdmissionError> {
        if self.provider_call_in_progress {
            return Err(SemanticDemandAdmissionError::Reentrant);
        }
        let index = self.authority_index(container_id)?;
        if self.slot(index, source).is_none() {
            return Err(SemanticDemandAdmissionError::NoActiveDemand);
        }
        self.ensure_demand_set_capacity()?;
        self.remove_slot(index, source);
        self.advance_demand_set_generation()
    }

    /// Execute one admitted ticket at most once.  This is a direct owner
    /// operation; no scheduler or queue is introduced here.
    pub(super) fn execute(
        &mut self,
        ticket: SemanticAttemptTicket,
    ) -> Result<SemanticProviderCompletion, SemanticDemandExecutionError> {
        let Some(index) = self.ticket_index(&ticket) else {
            return Ok(SemanticProviderCompletion::Stale);
        };
        let source = ticket.fence.source;
        if self.slot(index, source).is_some_and(|slot| slot.executed) {
            return Err(SemanticDemandExecutionError::AlreadyExecuted);
        }
        if self.provider_call_in_progress {
            return Err(SemanticDemandExecutionError::Reentrant);
        }
        let Some(slot) = self.slot_mut(index, source) else {
            return Ok(SemanticProviderCompletion::Stale);
        };
        slot.executed = true;
        let request = slot.request.clone();

        self.provider_call_in_progress = true;
        let completion = match request {
            SemanticDemandRequest::RequiredItemPin(request) => {
                let provider = self.records[index].authority.semantic_provider.clone();
                let outcome = provider.map_or(
                    VirtualLayoutSemanticQueryOutcome::Unavailable(
                        VirtualLayoutSemanticUnavailableReason::NoProvider,
                    ),
                    |provider| provider.lookup(&request),
                );
                SemanticProviderCompletion::RequiredItemPin { ticket, outcome }
            }
            SemanticDemandRequest::Range(request) => {
                let provider = self.records[index]
                    .authority
                    .semantic_range_provider
                    .clone();
                let outcome = provider.map_or(
                    VirtualLayoutSemanticRangeProviderOutcome::Unavailable(
                        VirtualLayoutSemanticUnavailableReason::NoProvider,
                    ),
                    |provider| provider.lookup_range(&request),
                );
                SemanticProviderCompletion::Range { ticket, outcome }
            }
        };
        self.provider_call_in_progress = false;
        Ok(completion)
    }

    /// Complete one provider return only if every current fence field still
    /// matches.  Stale/cancelled completion returns without changing owner
    /// state.
    pub(super) fn complete(
        &mut self,
        completion: SemanticProviderCompletion,
    ) -> SemanticDemandCompletion {
        let (ticket, outcome, source) = match completion {
            SemanticProviderCompletion::RequiredItemPin { ticket, outcome } => (
                ticket,
                SemanticOutcome::Pin(outcome),
                SemanticDemandSource::RequiredItemPin,
            ),
            SemanticProviderCompletion::Range { ticket, outcome } => (
                ticket,
                SemanticOutcome::Range(outcome),
                SemanticDemandSource::Range,
            ),
            SemanticProviderCompletion::Stale => return SemanticDemandCompletion::Stale,
        };
        let Some(index) = self.ticket_index(&ticket) else {
            return SemanticDemandCompletion::Stale;
        };
        let Some(slot) = self.slot(index, source) else {
            return SemanticDemandCompletion::Stale;
        };
        if slot.completed || !slot.executed || !slot.fence.same_exact(&ticket.fence) {
            return SemanticDemandCompletion::Stale;
        }
        if let Some(slot) = self.slot_mut(index, source) {
            slot.completed = true;
        }

        match outcome {
            SemanticOutcome::Pin(outcome) => {
                SemanticDemandCompletion::RequiredItemPin(self.complete_pin(index, ticket, outcome))
            }
            SemanticOutcome::Range(outcome) => {
                SemanticDemandCompletion::Range(self.complete_range(index, ticket, outcome))
            }
        }
    }

    /// Build the first, side-effect-free phase of whole-surface publication.
    /// Every active membership is represented, including a terminal or
    /// withheld member that makes the plan incomplete.
    pub(super) fn publication_plan(
        &self,
        authorities: SemanticPublicationAuthorities,
    ) -> SemanticPublicationPlan {
        let mut members = Vec::new();
        let mut complete = true;
        for record in &self.records {
            for source in [
                SemanticDemandSource::Range,
                SemanticDemandSource::RequiredItemPin,
            ] {
                let Some(slot) = self.slot_for_record(record, source) else {
                    continue;
                };
                let member = self.publication_member(slot, source, authorities);
                complete &= member.resolved();
                members.push(member);
            }
        }
        SemanticPublicationPlan {
            authorities,
            complete_demand_set_generation: self.demand_set_generation,
            members,
            complete,
        }
    }

    /// Complete the second publication phase.  The plan and all classifier
    /// outputs are rechecked against current membership/evidence before the
    /// compositor is called.  Only a successful compositor result updates the
    /// retained complete candidate.
    pub(super) fn finish_publication(
        &mut self,
        ordinary: &GuiAutomationSnapshot,
        plan: SemanticPublicationPlan,
        classifications: &[SemanticPublicationClassification],
    ) -> SemanticPublicationOutcome {
        let baseline = |reason| SemanticPublicationOutcome::OrdinaryBaseline {
            composition: super::automation_compositor::ordinary_virtual_layout_automation_snapshot(
                ordinary,
            ),
            reason,
        };

        if plan.complete_demand_set_generation != self.demand_set_generation {
            return baseline(SemanticPublicationFallbackReason::StalePlan);
        }
        if !plan.complete() {
            if let Some(candidate) = &self.last_complete_candidate
                && self.previous_candidate_is_eligible(candidate, ordinary, &plan)
            {
                return SemanticPublicationOutcome::Published(candidate.composition.clone());
            }
            return baseline(SemanticPublicationFallbackReason::IncompleteDemandSet);
        }

        let current = self.publication_plan(plan.authorities());
        if !same_publication_plan(&current, &plan) || !current.complete() {
            return baseline(SemanticPublicationFallbackReason::StalePlan);
        }
        let expected_inputs = current
            .members
            .iter()
            .filter(|member| member.evidence.is_some() && member.resolved)
            .count();
        if classifications.len() != expected_inputs
            || !current
                .members
                .iter()
                .filter(|member| member.evidence.is_some() && member.resolved)
                .zip(classifications)
                .all(|(member, classification)| {
                    self.classification_matches_member(member, classification)
                })
        {
            return baseline(SemanticPublicationFallbackReason::ClassificationRejected);
        }

        let composition =
            match super::automation_compositor::compose_virtual_layout_automation_publication(
                ordinary,
                classifications,
            ) {
                Ok(composition) => composition,
                Err(_) => return baseline(SemanticPublicationFallbackReason::CompositionRejected),
            };
        self.last_complete_candidate = Some(SemanticCompleteCandidate {
            authorities: current.authorities,
            ordinary: ordinary.clone(),
            plan: current,
            composition: composition.clone(),
        });
        SemanticPublicationOutcome::Published(composition)
    }

    fn publication_member(
        &self,
        slot: &SemanticDemandSlot,
        source: SemanticDemandSource,
        authorities: SemanticPublicationAuthorities,
    ) -> SemanticPublicationMember {
        let (provider_fence, evidence, resolved) = match slot.status {
            SemanticSlotStatus::Found | SemanticSlotStatus::NotFound => {
                (slot.fence.clone(), slot.evidence.clone(), true)
            }
            SemanticSlotStatus::Fallback | SemanticSlotStatus::Pending => slot
                .retained
                .as_ref()
                .filter(|retained| retained.fence.same_retention_fence(&slot.fence))
                .map_or((slot.fence.clone(), None, false), |retained| {
                    (
                        retained.fence.clone(),
                        Some(retained.evidence.clone()),
                        true,
                    )
                }),
            SemanticSlotStatus::Withheld | SemanticSlotStatus::Terminal => {
                (slot.fence.clone(), None, false)
            }
        };
        let (evidence, evidence_resolved) = publication_evidence(evidence);
        SemanticPublicationMember {
            container_id: provider_fence.container_id,
            source,
            fence: SemanticPublicationFence::new(
                provider_fence,
                authorities,
                self.demand_set_generation,
            ),
            evidence,
            resolved: resolved && evidence_resolved,
        }
    }

    fn classification_matches_member(
        &self,
        member: &SemanticPublicationMember,
        classification: &SemanticPublicationClassification,
    ) -> bool {
        member.fence.same_exact(classification.fence())
            && same_publication_input(member.evidence.as_ref(), classification.input())
    }

    fn previous_candidate_is_eligible(
        &self,
        candidate: &SemanticCompleteCandidate,
        ordinary: &GuiAutomationSnapshot,
        current_plan: &SemanticPublicationPlan,
    ) -> bool {
        candidate.authorities == current_plan.authorities
            && candidate.ordinary == *ordinary
            && candidate.plan.complete == current_plan.complete
            && candidate.plan.complete_demand_set_generation
                == current_plan.complete_demand_set_generation
            && same_publication_members(&candidate.plan.members, &current_plan.members)
    }

    fn complete_pin(
        &mut self,
        index: usize,
        ticket: SemanticAttemptTicket,
        outcome: VirtualLayoutSemanticQueryOutcome,
    ) -> VirtualLayoutSemanticQueryOutcome {
        match outcome {
            VirtualLayoutSemanticQueryOutcome::Found(entry) => {
                let Some(request) = self.pin_request(index) else {
                    return self.reject_and_clear(
                        index,
                        SemanticDemandSource::RequiredItemPin,
                        VirtualLayoutSemanticRejectedReason::Stale,
                    );
                };
                if let Err(reason) = entry.validate_for(&request) {
                    return self.reject_and_clear(
                        index,
                        SemanticDemandSource::RequiredItemPin,
                        reason,
                    );
                }
                let pin = crate::gui::layout_core::VirtualLayoutPin::new(
                    crate::gui::layout_core::VirtualLayoutPinReason::Semantic,
                    request.clone(),
                    entry.as_ref().clone(),
                );
                let Some(projection) = VirtualLayoutSemanticProjection::from_validated_semantic_pin(
                    &pin,
                    self.records[index].authority.coordinate_space.clone(),
                ) else {
                    return self.reject_and_clear(
                        index,
                        SemanticDemandSource::RequiredItemPin,
                        VirtualLayoutSemanticRejectedReason::ProviderRejected,
                    );
                };
                let evidence = SemanticEvidence::Pin(Box::new(SemanticPinEvidence {
                    entry: Some(entry.as_ref().clone()),
                    projection: Some(projection),
                }));
                self.store_evidence(
                    index,
                    SemanticDemandSource::RequiredItemPin,
                    ticket,
                    evidence,
                );
                VirtualLayoutSemanticQueryOutcome::Found(entry)
            }
            VirtualLayoutSemanticQueryOutcome::NotFound => {
                let evidence = SemanticEvidence::Pin(Box::new(SemanticPinEvidence {
                    entry: None,
                    projection: None,
                }));
                self.store_evidence(
                    index,
                    SemanticDemandSource::RequiredItemPin,
                    ticket,
                    evidence,
                );
                VirtualLayoutSemanticQueryOutcome::NotFound
            }
            VirtualLayoutSemanticQueryOutcome::Deferred(reason) => {
                self.apply_fallback_or_withhold(
                    index,
                    SemanticDemandSource::RequiredItemPin,
                    &ticket.fence,
                );
                VirtualLayoutSemanticQueryOutcome::Deferred(reason)
            }
            VirtualLayoutSemanticQueryOutcome::Unavailable(
                reason @ VirtualLayoutSemanticUnavailableReason::DataUnavailable,
            ) => {
                self.apply_fallback_or_withhold(
                    index,
                    SemanticDemandSource::RequiredItemPin,
                    &ticket.fence,
                );
                VirtualLayoutSemanticQueryOutcome::Unavailable(reason)
            }
            VirtualLayoutSemanticQueryOutcome::Unavailable(
                reason @ (VirtualLayoutSemanticUnavailableReason::NoProvider
                | VirtualLayoutSemanticUnavailableReason::Unsupported),
            ) => {
                self.clear_slot(index, SemanticDemandSource::RequiredItemPin);
                VirtualLayoutSemanticQueryOutcome::Unavailable(reason)
            }
            VirtualLayoutSemanticQueryOutcome::Rejected(reason) => {
                self.clear_slot(index, SemanticDemandSource::RequiredItemPin);
                VirtualLayoutSemanticQueryOutcome::Rejected(reason)
            }
        }
    }

    fn complete_range(
        &mut self,
        index: usize,
        ticket: SemanticAttemptTicket,
        outcome: VirtualLayoutSemanticRangeProviderOutcome,
    ) -> VirtualLayoutSemanticRangeQueryOutcome {
        match outcome {
            VirtualLayoutSemanticRangeProviderOutcome::Found(entries) => {
                let Some(request) = self.range_request(index) else {
                    return self
                        .reject_range_and_clear(index, VirtualLayoutSemanticRejectedReason::Stale);
                };
                if let Err(reason) = validate_range_entries(&request, &entries) {
                    return self.reject_range_and_clear(index, reason);
                }
                let coordinate_space = self.records[index].authority.coordinate_space.clone();
                let Some(projections) = entries
                    .iter()
                    .map(|entry| {
                        VirtualLayoutSemanticProjection::from_validated_semantic_range_entry(
                            &request,
                            entry,
                            coordinate_space.clone(),
                        )
                    })
                    .collect::<Option<Vec<_>>>()
                else {
                    return self.reject_range_and_clear(
                        index,
                        VirtualLayoutSemanticRejectedReason::ProviderRejected,
                    );
                };
                let batch = VirtualLayoutSemanticProjectionBatch::new(request, projections.clone());
                let evidence = SemanticEvidence::Range(Box::new(SemanticRangeEvidence {
                    entries,
                    projections,
                }));
                self.store_evidence(index, SemanticDemandSource::Range, ticket, evidence);
                VirtualLayoutSemanticRangeQueryOutcome::Found(batch)
            }
            VirtualLayoutSemanticRangeProviderOutcome::NotFound => {
                let evidence = SemanticEvidence::Range(Box::new(SemanticRangeEvidence {
                    entries: Vec::new(),
                    projections: Vec::new(),
                }));
                self.store_evidence(index, SemanticDemandSource::Range, ticket, evidence);
                VirtualLayoutSemanticRangeQueryOutcome::NotFound
            }
            VirtualLayoutSemanticRangeProviderOutcome::Deferred(reason) => {
                self.apply_fallback_or_withhold(index, SemanticDemandSource::Range, &ticket.fence);
                VirtualLayoutSemanticRangeQueryOutcome::Deferred(reason)
            }
            VirtualLayoutSemanticRangeProviderOutcome::Unavailable(
                reason @ VirtualLayoutSemanticUnavailableReason::DataUnavailable,
            ) => {
                self.apply_fallback_or_withhold(index, SemanticDemandSource::Range, &ticket.fence);
                VirtualLayoutSemanticRangeQueryOutcome::Unavailable(reason)
            }
            VirtualLayoutSemanticRangeProviderOutcome::Unavailable(
                reason @ (VirtualLayoutSemanticUnavailableReason::NoProvider
                | VirtualLayoutSemanticUnavailableReason::Unsupported),
            ) => {
                self.clear_slot(index, SemanticDemandSource::Range);
                VirtualLayoutSemanticRangeQueryOutcome::Unavailable(reason)
            }
            VirtualLayoutSemanticRangeProviderOutcome::Rejected(reason) => {
                self.clear_slot(index, SemanticDemandSource::Range);
                VirtualLayoutSemanticRangeQueryOutcome::Rejected(reason)
            }
        }
    }

    fn store_evidence(
        &mut self,
        index: usize,
        source: SemanticDemandSource,
        ticket: SemanticAttemptTicket,
        evidence: SemanticEvidence,
    ) {
        let retained = SemanticRetainedEvidence {
            fence: ticket.fence.clone(),
            evidence: evidence.clone(),
        };
        if let Some(slot) = self.slot_mut(index, source) {
            let status = match &evidence {
                SemanticEvidence::Pin(pin) if pin.entry.is_some() => SemanticSlotStatus::Found,
                SemanticEvidence::Range(range) if !range.entries.is_empty() => {
                    SemanticSlotStatus::Found
                }
                SemanticEvidence::Pin(_) | SemanticEvidence::Range(_) => {
                    SemanticSlotStatus::NotFound
                }
            };
            slot.evidence = Some(evidence);
            slot.withheld = false;
            slot.retained = Some(retained);
            slot.status = status;
        }
    }

    fn apply_fallback_or_withhold(
        &mut self,
        index: usize,
        source: SemanticDemandSource,
        current_fence: &SemanticProviderFence,
    ) {
        let fallback = self
            .slot(index, source)
            .and_then(|slot| slot.retained.as_ref())
            .filter(|retained| retained.fence.same_retention_fence(current_fence))
            .map(|retained| retained.evidence.clone());
        if let Some(slot) = self.slot_mut(index, source) {
            slot.withheld = fallback.is_none();
            slot.evidence = fallback;
            slot.status = if slot.withheld {
                SemanticSlotStatus::Withheld
            } else {
                SemanticSlotStatus::Fallback
            };
        }
    }

    fn validate_intent_authority(&self, index: usize) -> Result<(), SemanticDemandAdmissionError> {
        let record = &self.records[index];
        if record.retired {
            return Err(SemanticDemandAdmissionError::Retired);
        }
        if stable_policy_identity_equals(
            &record.authority.policy_identity,
            &record.authority.policy_identity,
        ) != Some(true)
        {
            return Err(SemanticDemandAdmissionError::ScopeMismatch);
        }
        if !is_logical_coordinate(&record.authority.coordinate_space) {
            return Err(SemanticDemandAdmissionError::CustomCoordinate);
        }
        Ok(())
    }

    fn start_new_demand(
        &mut self,
        index: usize,
        request: SemanticDemandRequest,
    ) -> Result<SemanticDemandAdmission, SemanticDemandAdmissionError> {
        let source = request.source();
        if self
            .slot(index, source)
            .and_then(|slot| slot.request.demand().same_exact(&request.demand()))
            == Some(true)
        {
            return Ok(SemanticDemandAdmission::Unchanged);
        }
        self.ensure_demand_set_capacity()?;
        let ticket = self.start_slot(index, request, None)?;
        self.advance_demand_set_generation()?;
        Ok(SemanticDemandAdmission::Started(ticket))
    }

    fn retry_source(
        &mut self,
        container_id: NodeId,
        source: SemanticDemandSource,
    ) -> Result<SemanticAttemptTicket, SemanticDemandAdmissionError> {
        if self.provider_call_in_progress {
            return Err(SemanticDemandAdmissionError::Reentrant);
        }
        let index = self.authority_index(container_id)?;
        self.validate_intent_authority(index)?;
        let old = self
            .slot(index, source)
            .cloned()
            .ok_or(SemanticDemandAdmissionError::NoActiveDemand)?;
        let attempt = old
            .fence
            .attempt
            .checked_add(1)
            .ok_or(SemanticDemandAdmissionError::CounterOverflow)?;
        let demand_generation = old.fence.demand_generation;
        self.replace_slot(
            index,
            source,
            old.request,
            demand_generation,
            attempt,
            old.retained,
        )
    }

    fn start_slot(
        &mut self,
        index: usize,
        request: SemanticDemandRequest,
        retained: Option<SemanticRetainedEvidence>,
    ) -> Result<SemanticAttemptTicket, SemanticDemandAdmissionError> {
        let demand_generation = checked_next(&mut self.next_demand_generation)
            .ok_or(SemanticDemandAdmissionError::CounterOverflow)?;
        self.replace_slot(
            index,
            request.source(),
            request,
            demand_generation,
            1,
            retained,
        )
    }

    fn restart_slot(
        &mut self,
        index: usize,
        source: SemanticDemandSource,
    ) -> Result<SemanticAttemptTicket, SemanticDemandAdmissionError> {
        let old = self
            .slot(index, source)
            .cloned()
            .ok_or(SemanticDemandAdmissionError::NoActiveDemand)?;
        let demand = old.request.demand();
        if let SemanticDemand::Range(range) = &demand {
            (*range)
                .validate_budget(self.records[index].authority.budget)
                .map_err(SemanticDemandAdmissionError::InvalidRange)?;
        }
        let demand_generation = checked_next(&mut self.next_demand_generation)
            .ok_or(SemanticDemandAdmissionError::CounterOverflow)?;
        let request = self.records[index].authority.request_for_demand(demand);
        self.replace_slot(index, source, request, demand_generation, 1, old.retained)
    }

    fn replace_slot(
        &mut self,
        index: usize,
        source: SemanticDemandSource,
        request: SemanticDemandRequest,
        demand_generation: u64,
        attempt: u64,
        retained: Option<SemanticRetainedEvidence>,
    ) -> Result<SemanticAttemptTicket, SemanticDemandAdmissionError> {
        let record = &self.records[index];
        let fence = SemanticProviderFence {
            container_id: record.authority.container_id,
            policy_identity: record.authority.policy_identity.clone(),
            registration_generation: record.registration_generation,
            mount_generation: record.authority.mount_generation,
            data_revision: record.authority.data_revision,
            policy_revision: record.authority.policy_revision,
            measurement_revision: record.authority.measurement_revision,
            semantic_revision: record.authority.semantic_revision,
            semantic_cardinality: record.authority.semantic_cardinality,
            coordinate_space: record.authority.coordinate_space.clone(),
            budget: record.authority.budget,
            demand: request.demand(),
            source,
            provider_identity: record.authority.provider_identity(source),
            provider_generation: record.provider_generation(source),
            demand_generation,
            attempt,
            cancelled: false,
        };
        if let Some(previous) = self.slot_mut(index, source) {
            previous.fence.cancelled = true;
        }
        let slot = SemanticDemandSlot {
            request,
            fence: fence.clone(),
            executed: false,
            completed: false,
            evidence: None,
            withheld: false,
            retained,
            status: SemanticSlotStatus::Pending,
        };
        self.set_slot(index, source, Some(slot));
        Ok(SemanticAttemptTicket { fence })
    }

    fn ensure_demand_set_capacity(&self) -> Result<(), SemanticDemandAdmissionError> {
        self.next_demand_set_generation
            .checked_add(1)
            .ok_or(SemanticDemandAdmissionError::CounterOverflow)
            .map(|_| ())
    }

    fn advance_demand_set_generation(&mut self) -> Result<(), SemanticDemandAdmissionError> {
        let next = self
            .next_demand_set_generation
            .checked_add(1)
            .ok_or(SemanticDemandAdmissionError::CounterOverflow)?;
        self.next_demand_set_generation = next;
        self.demand_set_generation = next;
        Ok(())
    }

    fn reject_and_clear(
        &mut self,
        index: usize,
        source: SemanticDemandSource,
        reason: VirtualLayoutSemanticRejectedReason,
    ) -> VirtualLayoutSemanticQueryOutcome {
        self.clear_slot(index, source);
        VirtualLayoutSemanticQueryOutcome::Rejected(reason)
    }

    fn reject_range_and_clear(
        &mut self,
        index: usize,
        reason: VirtualLayoutSemanticRejectedReason,
    ) -> VirtualLayoutSemanticRangeQueryOutcome {
        self.clear_slot(index, SemanticDemandSource::Range);
        VirtualLayoutSemanticRangeQueryOutcome::Rejected(reason)
    }

    fn clear_record_slots(&mut self, index: usize) {
        self.clear_slot(index, SemanticDemandSource::Range);
        self.clear_slot(index, SemanticDemandSource::RequiredItemPin);
    }

    fn clear_slot(&mut self, index: usize, source: SemanticDemandSource) {
        if let Some(slot) = self.slot_mut(index, source) {
            slot.fence.cancelled = true;
            slot.executed = false;
            slot.completed = true;
            slot.evidence = None;
            slot.withheld = true;
            slot.status = SemanticSlotStatus::Terminal;
        }
    }

    fn remove_slot(&mut self, index: usize, source: SemanticDemandSource) {
        if let Some(slot) = self.slot_mut(index, source) {
            slot.fence.cancelled = true;
        }
        self.set_slot(index, source, None);
    }

    fn new_record(&mut self, authority: SemanticLiveAuthority) -> Option<SemanticDemandRecord> {
        let registration_generation = checked_next(&mut self.next_registration_generation)?;
        let semantic_provider_generation =
            checked_next(&mut self.next_semantic_provider_generation)?;
        let semantic_range_provider_generation =
            checked_next(&mut self.next_semantic_range_provider_generation)?;
        Some(SemanticDemandRecord {
            authority,
            registration_generation,
            semantic_provider_generation,
            semantic_range_provider_generation,
            range: None,
            pin: None,
            retired: false,
        })
    }

    fn authority_index(&self, container_id: NodeId) -> Result<usize, SemanticDemandAdmissionError> {
        self.record_index(container_id)
            .ok_or(SemanticDemandAdmissionError::UnknownContainer)
    }

    fn record_index(&self, container_id: NodeId) -> Option<usize> {
        self.records
            .iter()
            .position(|record| record.authority.container_id == container_id)
    }

    fn ticket_index(&self, ticket: &SemanticAttemptTicket) -> Option<usize> {
        self.records.iter().position(|record| {
            !record.retired
                && record.authority.container_id == ticket.fence.container_id
                && self
                    .slot_for_record(record, ticket.fence.source)
                    .is_some_and(|slot| slot.fence.same_exact(&ticket.fence))
        })
    }

    fn active_range_length(&self) -> Option<usize> {
        self.records
            .iter()
            .filter_map(|record| {
                record
                    .range
                    .as_ref()
                    .filter(|slot| slot.status != SemanticSlotStatus::Terminal)
            })
            .try_fold(0_usize, |total, slot| {
                total.checked_add(slot.request.demand().range_length())
            })
    }

    fn slot_for_record<'a>(
        &self,
        record: &'a SemanticDemandRecord,
        source: SemanticDemandSource,
    ) -> Option<&'a SemanticDemandSlot> {
        match source {
            SemanticDemandSource::Range => record.range.as_ref(),
            SemanticDemandSource::RequiredItemPin => record.pin.as_ref(),
        }
    }

    fn slot(&self, index: usize, source: SemanticDemandSource) -> Option<&SemanticDemandSlot> {
        match source {
            SemanticDemandSource::Range => self.records[index].range.as_ref(),
            SemanticDemandSource::RequiredItemPin => self.records[index].pin.as_ref(),
        }
    }

    fn slot_mut(
        &mut self,
        index: usize,
        source: SemanticDemandSource,
    ) -> Option<&mut SemanticDemandSlot> {
        match source {
            SemanticDemandSource::Range => self.records[index].range.as_mut(),
            SemanticDemandSource::RequiredItemPin => self.records[index].pin.as_mut(),
        }
    }

    fn set_slot(
        &mut self,
        index: usize,
        source: SemanticDemandSource,
        slot: Option<SemanticDemandSlot>,
    ) {
        match source {
            SemanticDemandSource::Range => self.records[index].range = slot,
            SemanticDemandSource::RequiredItemPin => self.records[index].pin = slot,
        }
    }

    fn pin_request(&self, index: usize) -> Option<VirtualLayoutSemanticRequest> {
        match self
            .slot(index, SemanticDemandSource::RequiredItemPin)?
            .request
            .clone()
        {
            SemanticDemandRequest::RequiredItemPin(request) => Some(request),
            SemanticDemandRequest::Range(_) => None,
        }
    }

    fn range_request(&self, index: usize) -> Option<VirtualLayoutSemanticRangeRequest> {
        match self
            .slot(index, SemanticDemandSource::Range)?
            .request
            .clone()
        {
            SemanticDemandRequest::Range(request) => Some(request),
            SemanticDemandRequest::RequiredItemPin(_) => None,
        }
    }
}

fn publication_evidence(
    evidence: Option<SemanticEvidence>,
) -> (Option<SemanticPublicationEvidence>, bool) {
    let Some(evidence) = evidence else {
        return (None, false);
    };
    match evidence {
        SemanticEvidence::Pin(pin) => {
            let Some(projection) = pin.projection else {
                return (None, true);
            };
            (
                Some(SemanticPublicationEvidence::Pin {
                    request: projection.request().clone(),
                    projection: Box::new(projection),
                }),
                true,
            )
        }
        SemanticEvidence::Range(range) => {
            if range.projections.is_empty() {
                return (None, true);
            }
            let Some(request) = range
                .projections
                .first()
                .and_then(|projection| projection.range_request())
                .cloned()
            else {
                return (None, false);
            };
            (
                Some(SemanticPublicationEvidence::Range {
                    request,
                    projections: range.projections,
                }),
                true,
            )
        }
    }
}

fn same_publication_plan(left: &SemanticPublicationPlan, right: &SemanticPublicationPlan) -> bool {
    left.authorities == right.authorities
        && left.complete_demand_set_generation == right.complete_demand_set_generation
        && left.complete == right.complete
        && same_publication_members(&left.members, &right.members)
}

fn same_publication_members(
    left: &[SemanticPublicationMember],
    right: &[SemanticPublicationMember],
) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| same_publication_member(left, right))
}

fn same_publication_member(
    left: &SemanticPublicationMember,
    right: &SemanticPublicationMember,
) -> bool {
    left.container_id == right.container_id
        && left.source == right.source
        && left.resolved == right.resolved
        && left.fence.same_exact(&right.fence)
        && same_publication_evidence(left.evidence.as_ref(), right.evidence.as_ref())
}

fn same_publication_evidence(
    left: Option<&SemanticPublicationEvidence>,
    right: Option<&SemanticPublicationEvidence>,
) -> bool {
    match (left, right) {
        (None, None) => true,
        (
            Some(SemanticPublicationEvidence::Pin {
                request: left_request,
                projection: left_projection,
            }),
            Some(SemanticPublicationEvidence::Pin {
                request: right_request,
                projection: right_projection,
            }),
        ) => {
            same_semantic_request(left_request, right_request)
                && same_semantic_projection(left_projection.as_ref(), right_projection.as_ref())
        }
        (
            Some(SemanticPublicationEvidence::Range {
                request: left_request,
                projections: left_projections,
            }),
            Some(SemanticPublicationEvidence::Range {
                request: right_request,
                projections: right_projections,
            }),
        ) => {
            same_semantic_range_request(left_request, right_request)
                && left_projections.len() == right_projections.len()
                && left_projections
                    .iter()
                    .zip(right_projections)
                    .all(|(left, right)| same_semantic_projection(left, right))
        }
        _ => false,
    }
}

fn same_publication_input(
    evidence: Option<&SemanticPublicationEvidence>,
    input: &VirtualLayoutSemanticClassificationInput,
) -> bool {
    match (evidence, input) {
        (
            Some(SemanticPublicationEvidence::Range {
                request,
                projections,
            }),
            VirtualLayoutSemanticClassificationInput::Range(batch),
        ) => {
            same_semantic_range_request(request, batch.request())
                && projections.len() == batch.classifications().len()
                && projections.iter().zip(batch.classifications()).all(
                    |(projection, classification)| {
                        same_semantic_projection(projection, classification.projection())
                    },
                )
        }
        (
            Some(SemanticPublicationEvidence::Pin {
                request,
                projection,
            }),
            VirtualLayoutSemanticClassificationInput::Pin(pin),
        ) => {
            same_semantic_request(request, pin.request())
                && same_semantic_projection(projection.as_ref(), pin.classification().projection())
        }
        _ => false,
    }
}

fn same_semantic_request(
    left: &VirtualLayoutSemanticRequest,
    right: &VirtualLayoutSemanticRequest,
) -> bool {
    left.container_id() == right.container_id()
        && left.mount_generation() == right.mount_generation()
        && left.data_revision() == right.data_revision()
        && left.policy_revision() == right.policy_revision()
        && left.measurement_revision() == right.measurement_revision()
        && left.semantic_revision() == right.semantic_revision()
        && stable_policy_identity_equals(left.policy_identity(), right.policy_identity())
            == Some(true)
        && left.key().stable_equals(right.key()) == Some(true)
}

fn same_semantic_range_request(
    left: &VirtualLayoutSemanticRangeRequest,
    right: &VirtualLayoutSemanticRangeRequest,
) -> bool {
    left.container_id() == right.container_id()
        && left.mount_generation() == right.mount_generation()
        && left.data_revision() == right.data_revision()
        && left.policy_revision() == right.policy_revision()
        && left.measurement_revision() == right.measurement_revision()
        && left.semantic_revision() == right.semantic_revision()
        && left.budget() == right.budget()
        && left.range() == right.range()
        && stable_policy_identity_equals(left.policy_identity(), right.policy_identity())
            == Some(true)
        && stable_coordinate_space_equals(left.coordinate_space(), right.coordinate_space())
            == Some(true)
}

fn same_semantic_projection(
    left: &VirtualLayoutSemanticProjection,
    right: &VirtualLayoutSemanticProjection,
) -> bool {
    left.identity().container_id() == right.identity().container_id()
        && left.logical_index() == right.logical_index()
        && left.bounds().min.x.to_bits() == right.bounds().min.x.to_bits()
        && left.bounds().min.y.to_bits() == right.bounds().min.y.to_bits()
        && left.bounds().max.x.to_bits() == right.bounds().max.x.to_bits()
        && left.bounds().max.y.to_bits() == right.bounds().max.y.to_bits()
        && left.semantics() == right.semantics()
        && left.automation_node_id() == right.automation_node_id()
        && left.authority() == right.authority()
        && left.identity().key().stable_equals(right.identity().key()) == Some(true)
        && stable_coordinate_space_equals(left.coordinate_space(), right.coordinate_space())
            == Some(true)
        && same_semantic_request(left.request(), right.request())
        && match (left.range_request(), right.range_request()) {
            (None, None) => true,
            (Some(left), Some(right)) => same_semantic_range_request(left, right),
            _ => false,
        }
}

impl<Message> Drop for SemanticDemandOwner<Message> {
    fn drop(&mut self) {
        self.retire_all();
    }
}

enum SemanticOutcome {
    Pin(VirtualLayoutSemanticQueryOutcome),
    Range(VirtualLayoutSemanticRangeProviderOutcome),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SemanticAttemptTicket {
    fence: SemanticProviderFence,
}

impl SemanticAttemptTicket {
    pub(super) fn fence(&self) -> &SemanticProviderFence {
        &self.fence
    }
}

fn checked_next(counter: &mut u64) -> Option<u64> {
    let next = counter.checked_add(1)?;
    *counter = next;
    Some(next)
}

fn contains_duplicate_container<Message>(
    registrations: &[(VirtualLayoutRegistration<Message>, u64)],
) -> bool {
    registrations
        .iter()
        .enumerate()
        .any(|(index, (registration, _))| {
            registrations[..index]
                .iter()
                .any(|(previous, _)| previous.container_id == registration.container_id)
        })
}

fn stable_policy_identity_equals(
    left: &crate::layout::VirtualLayoutPolicyIdentity,
    right: &crate::layout::VirtualLayoutPolicyIdentity,
) -> Option<bool> {
    left.stable_equals(right)
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

fn is_logical_coordinate(coordinate_space: &VirtualLayoutCoordinateSpace) -> bool {
    matches!(coordinate_space, VirtualLayoutCoordinateSpace::Logical)
}

fn validate_range_entries(
    request: &VirtualLayoutSemanticRangeRequest,
    entries: &[VirtualLayoutSemanticEntry],
) -> Result<(), VirtualLayoutSemanticRejectedReason> {
    let range = request.range();
    if entries.len() != range.length() {
        return Err(VirtualLayoutSemanticRejectedReason::RangeCountMismatch);
    }
    for (offset, entry) in entries.iter().enumerate() {
        if range.expected_index(offset) != Some(entry.logical_index()) {
            return Err(VirtualLayoutSemanticRejectedReason::WrongLogicalIndex);
        }
        if offset > 0 && entry.logical_index() <= entries[offset - 1].logical_index() {
            return Err(VirtualLayoutSemanticRejectedReason::RangeOutOfOrder);
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
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::virtual_layout::{
        VirtualLayoutSemanticClassification, VirtualLayoutSemanticClassificationBatch,
        VirtualLayoutSemanticClassificationOrigin, VirtualLayoutSemanticPinClassification,
    };
    use super::*;
    use crate::{
        application::virtual_layout::VirtualLayoutSemanticCardinality,
        application::{scroll, spacer, text},
        gui::{
            automation::{
                AutomationBounds, AutomationNodeId, AutomationNodeSemantics,
                AutomationNodeSnapshot, AutomationRole, GuiAutomationSnapshot,
            },
            types::Rect,
        },
        layout::{
            VirtualLayoutBoundsConfidence, VirtualLayoutExtentCandidate,
            VirtualLayoutItemCandidate, VirtualLayoutOverscan, VirtualLayoutPolicy,
            VirtualLayoutPolicyDecision, VirtualLayoutPolicyIdentity, VirtualLayoutQueryInput,
            VirtualLayoutQuerySink, VirtualLayoutVisibility,
        },
        runtime::surface::VirtualLayoutRegistrationRevisions,
    };
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    const CONTAINER_ID: u64 = 7;
    const MOUNT_GENERATION: u64 = 11;

    struct TestPolicy;

    impl VirtualLayoutPolicy for TestPolicy {
        fn query(
            &self,
            _input: &VirtualLayoutQueryInput,
            sink: &mut VirtualLayoutQuerySink,
        ) -> VirtualLayoutPolicyDecision {
            let _ = sink.visit(VirtualLayoutItemCandidate::new(
                VirtualLayoutItemKey::new(0_u32),
                0,
                Rect::from_xy_size(0.0, 0.0, 10.0, 10.0),
                VirtualLayoutVisibility::Visible,
                VirtualLayoutBoundsConfidence::Exact,
            ));
            let _ = sink.set_extent(VirtualLayoutExtentCandidate::exact(
                crate::gui::types::Vector2::new(10.0, 10.0),
            ));
            VirtualLayoutPolicyDecision::Ready
        }
    }

    struct PinProvider {
        calls: Cell<usize>,
        outcome: RefCell<VirtualLayoutSemanticQueryOutcome>,
    }

    impl VirtualLayoutSemanticProvider for PinProvider {
        fn lookup(
            &self,
            _request: &VirtualLayoutSemanticRequest,
        ) -> VirtualLayoutSemanticQueryOutcome {
            self.calls.set(self.calls.get() + 1);
            self.outcome.borrow().clone()
        }
    }

    struct RangeProvider {
        calls: Cell<usize>,
        outcome: RefCell<VirtualLayoutSemanticRangeProviderOutcome>,
    }

    impl VirtualLayoutSemanticRangeProvider for RangeProvider {
        fn lookup_range(
            &self,
            _request: &VirtualLayoutSemanticRangeRequest,
        ) -> VirtualLayoutSemanticRangeProviderOutcome {
            self.calls.set(self.calls.get() + 1);
            self.outcome.borrow().clone()
        }
    }

    struct RequestRecordingPinProvider {
        requests: RefCell<Vec<VirtualLayoutSemanticRequest>>,
    }

    impl VirtualLayoutSemanticProvider for RequestRecordingPinProvider {
        fn lookup(
            &self,
            request: &VirtualLayoutSemanticRequest,
        ) -> VirtualLayoutSemanticQueryOutcome {
            self.requests.borrow_mut().push(request.clone());
            VirtualLayoutSemanticQueryOutcome::NotFound
        }
    }

    struct RequestRecordingRangeProvider {
        requests: RefCell<Vec<VirtualLayoutSemanticRangeRequest>>,
    }

    impl VirtualLayoutSemanticRangeProvider for RequestRecordingRangeProvider {
        fn lookup_range(
            &self,
            request: &VirtualLayoutSemanticRangeRequest,
        ) -> VirtualLayoutSemanticRangeProviderOutcome {
            self.requests.borrow_mut().push(request.clone());
            VirtualLayoutSemanticRangeProviderOutcome::NotFound
        }
    }

    fn pin_entry(key: u32, node_id: &str) -> VirtualLayoutSemanticEntry {
        pin_entry_at(key, key as usize, node_id, "item")
    }

    fn pin_entry_at(
        key: u32,
        logical_index: usize,
        node_id: &str,
        label: &str,
    ) -> VirtualLayoutSemanticEntry {
        VirtualLayoutSemanticEntry::new(
            VirtualLayoutItemKey::new(key),
            logical_index,
            Rect::from_xy_size(0.0, 0.0, 10.0, 10.0),
            AutomationNodeSemantics::new(AutomationRole::Button).with_label(label),
            AutomationNodeId::new(node_id),
        )
    }

    fn range_entry(key: u32, logical_index: usize) -> VirtualLayoutSemanticEntry {
        VirtualLayoutSemanticEntry::new(
            VirtualLayoutItemKey::new(key),
            logical_index,
            Rect::from_xy_size(0.0, logical_index as f32, 10.0, 10.0),
            AutomationNodeSemantics::new(AutomationRole::Button).with_label("range item"),
            AutomationNodeId::new(format!("range-{logical_index}")),
        )
    }

    fn registration(
        policy_identity: &str,
        budget: usize,
        revisions: VirtualLayoutRegistrationRevisions,
        pin_provider: Option<Rc<dyn VirtualLayoutSemanticProvider>>,
        range_provider: Option<Rc<dyn VirtualLayoutSemanticRangeProvider>>,
    ) -> VirtualLayoutRegistration<()> {
        let mut registration = VirtualLayoutRegistration::new(
            CONTAINER_ID,
            VirtualLayoutPolicyIdentity::new(policy_identity.to_owned()),
            Rc::new(TestPolicy),
            VirtualLayoutCoordinateSpace::logical(),
            VirtualLayoutOverscan::new(0.0, 0.0).expect("finite overscan"),
            VirtualLayoutBudget::new(budget),
            revisions,
            Rc::new(|| scroll(spacer::<()>())),
            Rc::new(|_| text::<()>("item")),
            Rc::new(|_| VirtualLayoutPolicyIdentity::new("item-kind")),
        );
        if let Some(provider) = pin_provider {
            registration = registration.with_semantic_provider(provider);
        }
        if let Some(provider) = range_provider {
            registration = registration.with_semantic_range_provider(provider);
        }
        registration
    }

    fn registration_with_cardinality(
        policy_identity: &str,
        cardinality: Option<VirtualLayoutSemanticCardinality>,
        pin_provider: Rc<dyn VirtualLayoutSemanticProvider>,
    ) -> VirtualLayoutRegistration<()> {
        let mut registration = registration(
            policy_identity,
            8,
            Default::default(),
            Some(pin_provider),
            None,
        );
        registration.semantic_cardinality = cardinality;
        registration
    }

    fn sync(owner: &mut SemanticDemandOwner<()>, registration: VirtualLayoutRegistration<()>) {
        let _ = owner.synchronize(&[(registration, MOUNT_GENERATION)]);
    }

    fn started(admission: SemanticDemandAdmission) -> SemanticAttemptTicket {
        match admission {
            SemanticDemandAdmission::Started(ticket) => ticket,
            SemanticDemandAdmission::Unchanged => panic!("demand unexpectedly unchanged"),
        }
    }

    fn assert_terminal_slot(owner: &SemanticDemandOwner<()>, source: SemanticDemandSource) {
        let slot = owner.slot(0, source).expect("terminal demand membership");
        assert_eq!(slot.status, SemanticSlotStatus::Terminal);
        assert!(slot.evidence.is_none());
        assert!(slot.withheld);
    }

    fn complete_pin(
        owner: &mut SemanticDemandOwner<()>,
        ticket: SemanticAttemptTicket,
    ) -> VirtualLayoutSemanticQueryOutcome {
        let completion = owner.execute(ticket).expect("ticket executes once");
        match owner.complete(completion) {
            SemanticDemandCompletion::RequiredItemPin(outcome) => outcome,
            SemanticDemandCompletion::Range(_) | SemanticDemandCompletion::Stale => {
                panic!("expected pin completion")
            }
        }
    }

    fn complete_range(
        owner: &mut SemanticDemandOwner<()>,
        ticket: SemanticAttemptTicket,
    ) -> VirtualLayoutSemanticRangeQueryOutcome {
        let completion = owner.execute(ticket).expect("ticket executes once");
        match owner.complete(completion) {
            SemanticDemandCompletion::Range(outcome) => outcome,
            SemanticDemandCompletion::RequiredItemPin(_) | SemanticDemandCompletion::Stale => {
                panic!("expected range completion")
            }
        }
    }

    fn ordinary_publication_snapshot() -> GuiAutomationSnapshot {
        let container = AutomationNodeSnapshot::from_semantics(
            AutomationNodeId::new(CONTAINER_ID.to_string()),
            AutomationBounds {
                x: 0.0,
                y: 0.0,
                width: 160.0,
                height: 80.0,
            },
            AutomationNodeSemantics::new(AutomationRole::Group),
        );
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
        root.children.push(container);
        GuiAutomationSnapshot {
            schema_version: 2,
            viewport_width: 160,
            viewport_height: 80,
            root,
        }
    }

    fn publication_authorities(seed: u64) -> SemanticPublicationAuthorities {
        SemanticPublicationAuthorities {
            session_generation: seed,
            materialization_authority: seed,
            classification_authority: seed + 1,
            ordinary_projection_generation: seed + 2,
        }
    }

    fn classifications_for_plan(
        plan: &SemanticPublicationPlan,
    ) -> Vec<SemanticPublicationClassification> {
        plan.members()
            .iter()
            .filter_map(|member| {
                let evidence = member.evidence()?;
                let input = match evidence {
                    SemanticPublicationEvidence::Range {
                        request,
                        projections,
                    } => VirtualLayoutSemanticClassificationInput::Range(
                        VirtualLayoutSemanticClassificationBatch::new(
                            request.clone(),
                            projections
                                .iter()
                                .map(|projection| {
                                    VirtualLayoutSemanticClassification::new(
                                        projection.clone(),
                                        VirtualLayoutSemanticClassificationOrigin::Unmaterialized,
                                    )
                                })
                                .collect(),
                        ),
                    ),
                    SemanticPublicationEvidence::Pin {
                        request,
                        projection,
                    } => VirtualLayoutSemanticClassificationInput::Pin(Box::new(
                        VirtualLayoutSemanticPinClassification::new(
                            request.clone(),
                            VirtualLayoutSemanticClassification::new(
                                projection.as_ref().clone(),
                                VirtualLayoutSemanticClassificationOrigin::Unmaterialized,
                            ),
                        ),
                    )),
                };
                Some(SemanticPublicationClassification::new(
                    member.fence().clone(),
                    input,
                ))
            })
            .collect()
    }

    #[test]
    fn registration_sync_is_capability_only_and_lifecycle_is_bounded() {
        let pin = Rc::new(PinProvider {
            calls: Cell::new(0),
            outcome: RefCell::new(VirtualLayoutSemanticQueryOutcome::NotFound),
        });
        let initial_registration =
            registration("policy", 8, Default::default(), Some(pin.clone()), None);
        let mut owner = SemanticDemandOwner::default();
        assert!(
            owner
                .synchronize(&[(initial_registration.clone(), MOUNT_GENERATION)])
                .is_empty()
        );
        assert_eq!(pin.calls.get(), 0);
        assert!(owner.records[0].range.is_none());
        assert!(owner.records[0].pin.is_none());

        let ticket = started(
            owner
                .semantic_pin(CONTAINER_ID, VirtualLayoutItemKey::new(1_u32))
                .expect("admit"),
        );
        assert!(owner.synchronize(&[]).is_empty());
        assert!(owner.records.is_empty());
        assert!(matches!(
            owner.execute(ticket),
            Ok(SemanticProviderCompletion::Stale)
        ));

        sync(&mut owner, initial_registration);
        assert!(
            owner
                .semantic_pin(CONTAINER_ID, VirtualLayoutItemKey::new(1_u32))
                .is_ok()
        );
        let over_capacity = (0..=MAX_VIRTUAL_LAYOUT_REGISTRATIONS)
            .map(|index| {
                let mut candidate = registration("policy", 8, Default::default(), None, None);
                candidate.container_id = index as NodeId + 100;
                (candidate, MOUNT_GENERATION)
            })
            .collect::<Vec<_>>();
        assert!(owner.synchronize(&over_capacity).is_empty());
        assert!(owner.records.is_empty());
    }

    #[test]
    fn cardinality_is_preserved_through_registration_and_exact_live_fences() {
        let old_pin = Rc::new(PinProvider {
            calls: Cell::new(0),
            outcome: RefCell::new(VirtualLayoutSemanticQueryOutcome::NotFound),
        });
        let old_calls = &old_pin.calls;
        let new_pin = Rc::new(PinProvider {
            calls: Cell::new(0),
            outcome: RefCell::new(VirtualLayoutSemanticQueryOutcome::NotFound),
        });
        let new_calls = &new_pin.calls;
        let zero = Some(VirtualLayoutSemanticCardinality::new(0, 17));
        let large = Some(VirtualLayoutSemanticCardinality::new(usize::MAX, 18));
        let base = registration_with_cardinality("cardinality-policy", zero, old_pin.clone());
        let mut owner = SemanticDemandOwner::default();

        assert!(
            owner
                .synchronize(&[(base.clone(), MOUNT_GENERATION)])
                .is_empty()
        );
        assert_eq!(owner.records[0].authority.semantic_cardinality, zero);
        assert_eq!(old_calls.get(), 0);

        let initial = started(
            owner
                .semantic_pin(CONTAINER_ID, VirtualLayoutItemKey::new(1_u32))
                .expect("initial demand"),
        );
        assert_eq!(initial.fence().semantic_cardinality, zero);

        let replaced = base.with_semantic_provider(new_pin.clone());
        let provider_refresh = owner.synchronize(&[(replaced.clone(), MOUNT_GENERATION)]);
        assert_eq!(provider_refresh.len(), 1);
        let provider_ticket = &provider_refresh[0];
        assert_ne!(
            provider_ticket.fence().provider_generation,
            initial.fence().provider_generation
        );
        assert_eq!(provider_ticket.fence().semantic_cardinality, zero);
        assert_eq!(new_calls.get(), 0);

        let mut cardinality_changed = replaced;
        cardinality_changed.semantic_cardinality = large;
        let cardinality_refresh = owner.synchronize(&[(cardinality_changed, MOUNT_GENERATION)]);
        assert_eq!(cardinality_refresh.len(), 1);
        let cardinality_ticket = &cardinality_refresh[0];
        assert_eq!(cardinality_ticket.fence().semantic_cardinality, large);
        assert_eq!(
            cardinality_ticket.fence().provider_generation,
            provider_ticket.fence().provider_generation,
            "cardinality changes invalidate the live fence without replacing the provider"
        );
        assert_eq!(new_calls.get(), 0);
    }

    #[test]
    fn range_bounds_budget_and_aggregate_are_rejected_before_provider() {
        let range_provider = Rc::new(RangeProvider {
            calls: Cell::new(0),
            outcome: RefCell::new(VirtualLayoutSemanticRangeProviderOutcome::NotFound),
        });
        let mut owner = SemanticDemandOwner::default();
        sync(
            &mut owner,
            registration(
                "policy",
                4,
                Default::default(),
                None,
                Some(range_provider.clone()),
            ),
        );
        assert!(matches!(
            owner.range(CONTAINER_ID, 0, 0),
            Err(SemanticDemandAdmissionError::InvalidRange(
                VirtualLayoutSemanticRejectedReason::RangeLengthZero
            ))
        ));
        assert!(matches!(
            owner.range(CONTAINER_ID, usize::MAX, 2),
            Err(SemanticDemandAdmissionError::InvalidRange(
                VirtualLayoutSemanticRejectedReason::RangeIndexOverflow
            ))
        ));
        assert!(matches!(
            owner.range(CONTAINER_ID, 0, 5),
            Err(SemanticDemandAdmissionError::InvalidRange(
                VirtualLayoutSemanticRejectedReason::RangeOverBudget
            ))
        ));
        assert_eq!(range_provider.calls.get(), 0);

        let mut full_owner = SemanticDemandOwner::default();
        let mut first = registration(
            "policy",
            VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES,
            Default::default(),
            None,
            Some(range_provider.clone()),
        );
        first.container_id = 20;
        let mut second = first.clone();
        second.container_id = 21;
        let _ = full_owner.synchronize(&[(first, MOUNT_GENERATION), (second, MOUNT_GENERATION)]);
        assert!(full_owner.range(20, 0, 1024).is_ok());
        assert!(matches!(
            full_owner.range(21, 0, 1),
            Err(SemanticDemandAdmissionError::AggregateBudgetExceeded)
        ));
        assert_eq!(range_provider.calls.get(), 0);
    }

    #[test]
    fn range_and_pin_slots_are_independent_and_repeated_requests_are_unchanged() {
        let pin = Rc::new(PinProvider {
            calls: Cell::new(0),
            outcome: RefCell::new(VirtualLayoutSemanticQueryOutcome::NotFound),
        });
        let range = Rc::new(RangeProvider {
            calls: Cell::new(0),
            outcome: RefCell::new(VirtualLayoutSemanticRangeProviderOutcome::NotFound),
        });
        let mut owner = SemanticDemandOwner::default();
        sync(
            &mut owner,
            registration(
                "policy",
                8,
                Default::default(),
                Some(pin.clone()),
                Some(range.clone()),
            ),
        );
        let pin_ticket = started(
            owner
                .semantic_pin(CONTAINER_ID, VirtualLayoutItemKey::new(1))
                .expect("pin"),
        );
        let range_ticket = started(owner.range(CONTAINER_ID, 1, 2).expect("range"));
        assert!(matches!(
            owner.semantic_pin(CONTAINER_ID, VirtualLayoutItemKey::new(1)),
            Ok(SemanticDemandAdmission::Unchanged)
        ));
        assert!(matches!(
            owner.range(CONTAINER_ID, 1, 2),
            Ok(SemanticDemandAdmission::Unchanged)
        ));
        assert!(owner.records[0].pin.is_some());
        assert!(owner.records[0].range.is_some());
        assert_eq!(
            complete_pin(&mut owner, pin_ticket),
            VirtualLayoutSemanticQueryOutcome::NotFound
        );
        assert_eq!(
            complete_range(&mut owner, range_ticket),
            VirtualLayoutSemanticRangeQueryOutcome::NotFound
        );
        assert!(owner.records[0].pin.is_some());
        assert!(owner.records[0].range.is_some());
        assert_eq!(pin.calls.get(), 1);
        assert_eq!(range.calls.get(), 1);
    }

    #[test]
    fn one_item_pin_is_first_class_and_does_not_call_range_provider() {
        let pin = Rc::new(PinProvider {
            calls: Cell::new(0),
            outcome: RefCell::new(VirtualLayoutSemanticQueryOutcome::Found(Box::new(
                pin_entry(4, "pin-4"),
            ))),
        });
        let range = Rc::new(RangeProvider {
            calls: Cell::new(0),
            outcome: RefCell::new(VirtualLayoutSemanticRangeProviderOutcome::NotFound),
        });
        let mut owner = SemanticDemandOwner::default();
        sync(
            &mut owner,
            registration(
                "policy",
                8,
                Default::default(),
                Some(pin.clone()),
                Some(range.clone()),
            ),
        );

        let ticket = started(
            owner
                .semantic_pin(CONTAINER_ID, VirtualLayoutItemKey::new(4_u32))
                .expect("pin demand"),
        );
        assert!(matches!(
            complete_pin(&mut owner, ticket),
            VirtualLayoutSemanticQueryOutcome::Found(_)
        ));

        let plan = owner.publication_plan(publication_authorities(1));
        assert!(plan.complete());
        assert_eq!(plan.members().len(), 1);
        assert_eq!(
            plan.members()[0].source(),
            SemanticDemandSource::RequiredItemPin
        );
        assert!(matches!(
            plan.members()[0].evidence(),
            Some(SemanticPublicationEvidence::Pin { .. })
        ));
        assert_eq!(pin.calls.get(), 1);
        assert_eq!(range.calls.get(), 0);
    }

    #[test]
    fn incomplete_range_and_pin_do_not_publish_partial_state() {
        let pin = Rc::new(PinProvider {
            calls: Cell::new(0),
            outcome: RefCell::new(VirtualLayoutSemanticQueryOutcome::Found(Box::new(
                pin_entry(2, "pin-2"),
            ))),
        });
        let range = Rc::new(RangeProvider {
            calls: Cell::new(0),
            outcome: RefCell::new(VirtualLayoutSemanticRangeProviderOutcome::Found(vec![
                range_entry(1, 0),
            ])),
        });
        let mut owner = SemanticDemandOwner::default();
        sync(
            &mut owner,
            registration(
                "policy",
                8,
                Default::default(),
                Some(pin.clone()),
                Some(range.clone()),
            ),
        );
        let _pending_pin = started(
            owner
                .semantic_pin(CONTAINER_ID, VirtualLayoutItemKey::new(2_u32))
                .expect("pin demand"),
        );
        let range_ticket = started(owner.range(CONTAINER_ID, 0, 1).expect("range demand"));
        assert!(matches!(
            complete_range(&mut owner, range_ticket),
            VirtualLayoutSemanticRangeQueryOutcome::Found(_)
        ));

        let ordinary = ordinary_publication_snapshot();
        let plan = owner.publication_plan(publication_authorities(10));
        assert!(!plan.complete());
        assert_eq!(plan.members().len(), 2);
        let outcome =
            owner.finish_publication(&ordinary, plan.clone(), &classifications_for_plan(&plan));
        let SemanticPublicationOutcome::OrdinaryBaseline {
            composition,
            reason,
        } = outcome
        else {
            panic!("an incomplete surface must not publish the completed range");
        };
        assert_eq!(
            reason,
            SemanticPublicationFallbackReason::IncompleteDemandSet
        );
        assert_eq!(composition.snapshot(), &ordinary);
        assert_eq!(range.calls.get(), 1);
        assert_eq!(pin.calls.get(), 0);
    }

    #[test]
    fn complete_range_and_pin_commit_atomically_as_one_publication() {
        let pin = Rc::new(PinProvider {
            calls: Cell::new(0),
            outcome: RefCell::new(VirtualLayoutSemanticQueryOutcome::Found(Box::new(
                pin_entry_at(2, 0, "range-0", "range item"),
            ))),
        });
        let range = Rc::new(RangeProvider {
            calls: Cell::new(0),
            outcome: RefCell::new(VirtualLayoutSemanticRangeProviderOutcome::Found(vec![
                range_entry(2, 0),
            ])),
        });
        let mut owner = SemanticDemandOwner::default();
        let mut registration = registration(
            "policy",
            8,
            Default::default(),
            Some(pin.clone()),
            Some(range.clone()),
        );
        registration.semantic_cardinality = Some(VirtualLayoutSemanticCardinality::new(17, 23));
        sync(&mut owner, registration);
        let pin_ticket = started(
            owner
                .semantic_pin(CONTAINER_ID, VirtualLayoutItemKey::new(2_u32))
                .expect("pin demand"),
        );
        let range_ticket = started(owner.range(CONTAINER_ID, 0, 1).expect("range demand"));
        assert!(matches!(
            complete_range(&mut owner, range_ticket),
            VirtualLayoutSemanticRangeQueryOutcome::Found(_)
        ));
        assert!(matches!(
            complete_pin(&mut owner, pin_ticket),
            VirtualLayoutSemanticQueryOutcome::Found(_)
        ));

        let ordinary = ordinary_publication_snapshot();
        let plan = owner.publication_plan(publication_authorities(20));
        assert!(plan.complete());
        let classifications = classifications_for_plan(&plan);
        assert_eq!(classifications.len(), 2);
        let outcome = owner.finish_publication(&ordinary, plan.clone(), &classifications);
        let SemanticPublicationOutcome::Published(composition) = outcome else {
            panic!("a complete range and pin set should publish together");
        };
        assert_eq!(
            composition.snapshot().root.children[0]
                .children
                .iter()
                .map(|child| child.id.clone())
                .collect::<Vec<_>>(),
            vec![AutomationNodeId::new("range-0")]
        );
        assert_eq!(
            composition.unmaterialized_ids(),
            &[AutomationNodeId::new("range-0")].into_iter().collect()
        );
        assert_eq!(composition.normalized_sidecar().entries().len(), 1);
        let sidecar_entry = &composition.normalized_sidecar().entries()[0];
        assert_eq!(sidecar_entry.container_id(), CONTAINER_ID);
        assert_eq!(sidecar_entry.logical_index(), 0);
        assert_eq!(sidecar_entry.provider(), &AutomationNodeId::new("range-0"));
        assert_eq!(sidecar_entry.normalized_path(), &[0, 0]);
        assert_eq!(
            sidecar_entry.materialization_authority(),
            VirtualLayoutSemanticClassificationOrigin::Unmaterialized
        );
        assert_eq!(
            sidecar_entry.publication_fences().range.as_ref(),
            Some(classifications[0].fence())
        );
        assert_eq!(
            sidecar_entry
                .publication_fences()
                .required_item_pin
                .as_ref(),
            Some(classifications[1].fence())
        );
        assert_ne!(
            sidecar_entry.publication_fences().range.as_ref(),
            sidecar_entry
                .publication_fences()
                .required_item_pin
                .as_ref()
        );
        let range_fence = sidecar_entry
            .publication_fences()
            .range
            .as_ref()
            .expect("range fence");
        assert_eq!(
            range_fence.provider_fence.semantic_cardinality,
            Some(VirtualLayoutSemanticCardinality::new(17, 23))
        );
        assert_eq!(range_fence.session_generation, 20);
        assert_eq!(range_fence.materialization_authority, 20);
        assert_eq!(range_fence.classification_authority, 21);
        assert_eq!(range_fence.ordinary_projection_generation, 22);
        assert_eq!(
            range_fence.complete_demand_set_generation,
            plan.complete_demand_set_generation()
        );
        assert_eq!(range.calls.get(), 1);
        assert_eq!(pin.calls.get(), 1);
    }

    #[test]
    fn stale_complete_demand_set_plan_returns_empty_ordinary_baseline() {
        let range = Rc::new(RangeProvider {
            calls: Cell::new(0),
            outcome: RefCell::new(VirtualLayoutSemanticRangeProviderOutcome::Found(vec![
                range_entry(4, 0),
            ])),
        });
        let mut owner = SemanticDemandOwner::default();
        sync(
            &mut owner,
            registration("policy", 8, Default::default(), None, Some(range.clone())),
        );
        let ticket = started(owner.range(CONTAINER_ID, 0, 1).expect("range demand"));
        assert!(matches!(
            complete_range(&mut owner, ticket),
            VirtualLayoutSemanticRangeQueryOutcome::Found(_)
        ));

        let ordinary = ordinary_publication_snapshot();
        let stale_plan = owner.publication_plan(publication_authorities(40));
        let classifications = classifications_for_plan(&stale_plan);
        let first = owner.finish_publication(&ordinary, stale_plan.clone(), &classifications);
        let SemanticPublicationOutcome::Published(composition) = first else {
            panic!("a complete publication should retain its normalized sidecar");
        };
        assert!(!composition.normalized_sidecar().entries().is_empty());

        owner
            .remove_range_demand(CONTAINER_ID)
            .expect("remove range demand");
        let later_plan = owner.publication_plan(publication_authorities(40));
        assert_ne!(
            stale_plan.complete_demand_set_generation(),
            later_plan.complete_demand_set_generation()
        );

        let outcome = owner.finish_publication(&ordinary, stale_plan, &classifications);
        let SemanticPublicationOutcome::OrdinaryBaseline {
            composition,
            reason,
        } = outcome
        else {
            panic!("a stale complete plan must return the ordinary baseline");
        };
        assert_eq!(reason, SemanticPublicationFallbackReason::StalePlan);
        assert_eq!(composition.snapshot(), &ordinary);
        assert!(composition.normalized_sidecar().entries().is_empty());
        assert_eq!(range.calls.get(), 1);
    }

    #[test]
    fn same_source_publication_fence_drift_rejects_without_partial_sidecar() {
        let range = Rc::new(RangeProvider {
            calls: Cell::new(0),
            outcome: RefCell::new(VirtualLayoutSemanticRangeProviderOutcome::Found(vec![
                range_entry(4, 0),
            ])),
        });
        let mut owner = SemanticDemandOwner::default();
        sync(
            &mut owner,
            registration("policy", 8, Default::default(), None, Some(range)),
        );
        let ticket = started(owner.range(CONTAINER_ID, 0, 1).expect("range demand"));
        assert!(matches!(
            complete_range(&mut owner, ticket),
            VirtualLayoutSemanticRangeQueryOutcome::Found(_)
        ));

        let ordinary = ordinary_publication_snapshot();
        let plan = owner.publication_plan(publication_authorities(40));
        let classifications = classifications_for_plan(&plan);
        assert_eq!(classifications.len(), 1);
        let first = classifications[0].clone();
        let mut drifted = first.clone();
        drifted.fence.complete_demand_set_generation += 1;

        let result =
            super::super::automation_compositor::compose_virtual_layout_automation_publication(
                &ordinary,
                &[first, drifted],
            );
        assert_eq!(
            result,
            Err(
                super::super::automation_compositor::
                    VirtualLayoutAutomationCompositionError::PublicationFenceDrift
            )
        );
        assert_eq!(ordinary, ordinary_publication_snapshot());
    }

    #[test]
    fn terminal_membership_blocks_subset_publication() {
        let pin = Rc::new(PinProvider {
            calls: Cell::new(0),
            outcome: RefCell::new(VirtualLayoutSemanticQueryOutcome::Found(Box::new(
                pin_entry(2, "pin-2"),
            ))),
        });
        let range = Rc::new(RangeProvider {
            calls: Cell::new(0),
            outcome: RefCell::new(VirtualLayoutSemanticRangeProviderOutcome::Unavailable(
                VirtualLayoutSemanticUnavailableReason::Unsupported,
            )),
        });
        let mut owner = SemanticDemandOwner::default();
        sync(
            &mut owner,
            registration(
                "policy",
                8,
                Default::default(),
                Some(pin),
                Some(range.clone()),
            ),
        );
        let pin_ticket = started(
            owner
                .semantic_pin(CONTAINER_ID, VirtualLayoutItemKey::new(2_u32))
                .expect("pin demand"),
        );
        let range_ticket = started(owner.range(CONTAINER_ID, 0, 1).expect("range demand"));
        assert!(matches!(
            complete_pin(&mut owner, pin_ticket),
            VirtualLayoutSemanticQueryOutcome::Found(_)
        ));
        assert!(matches!(
            complete_range(&mut owner, range_ticket),
            VirtualLayoutSemanticRangeQueryOutcome::Unavailable(
                VirtualLayoutSemanticUnavailableReason::Unsupported
            )
        ));

        let ordinary = ordinary_publication_snapshot();
        let plan = owner.publication_plan(publication_authorities(30));
        assert_eq!(plan.members().len(), 2);
        assert!(plan.members().iter().any(|member| {
            member.source() == SemanticDemandSource::Range && !member.resolved()
        }));
        assert!(plan.members().iter().any(|member| {
            member.source() == SemanticDemandSource::RequiredItemPin && member.resolved()
        }));
        let outcome =
            owner.finish_publication(&ordinary, plan.clone(), &classifications_for_plan(&plan));
        let SemanticPublicationOutcome::OrdinaryBaseline {
            composition,
            reason,
        } = outcome
        else {
            panic!("a terminal member must prevent subset publication");
        };
        assert_eq!(
            reason,
            SemanticPublicationFallbackReason::IncompleteDemandSet
        );
        assert_eq!(composition.snapshot(), &ordinary);
        assert_eq!(range.calls.get(), 1);
    }

    #[test]
    fn cancelled_completion_after_membership_removal_is_inert() {
        let pin = Rc::new(PinProvider {
            calls: Cell::new(0),
            outcome: RefCell::new(VirtualLayoutSemanticQueryOutcome::NotFound),
        });
        let range = Rc::new(RangeProvider {
            calls: Cell::new(0),
            outcome: RefCell::new(VirtualLayoutSemanticRangeProviderOutcome::NotFound),
        });
        let mut owner = SemanticDemandOwner::default();
        sync(
            &mut owner,
            registration(
                "policy",
                8,
                Default::default(),
                Some(pin.clone()),
                Some(range),
            ),
        );
        let pin_ticket = started(
            owner
                .semantic_pin(CONTAINER_ID, VirtualLayoutItemKey::new(2_u32))
                .expect("pin demand"),
        );
        let _range_ticket = started(owner.range(CONTAINER_ID, 0, 1).expect("range demand"));
        let completion = owner.execute(pin_ticket).expect("pin executes once");
        let range_fence = owner.records[0]
            .range
            .as_ref()
            .expect("range membership")
            .fence
            .clone();
        owner
            .remove_semantic_pin(CONTAINER_ID)
            .expect("remove pin membership");
        let generation_after_removal = owner.demand_set_generation();
        assert!(owner.records[0].pin.is_none());

        assert!(matches!(
            owner.complete(completion),
            SemanticDemandCompletion::Stale
        ));
        assert_eq!(owner.demand_set_generation(), generation_after_removal);
        assert_eq!(
            owner.records[0]
                .range
                .as_ref()
                .expect("range membership remains")
                .fence,
            range_fence
        );
        assert_eq!(pin.calls.get(), 1);
    }

    #[test]
    fn logical_only_and_unstable_inputs_are_rejected_before_provider() {
        struct Unstable;
        impl PartialEq for Unstable {
            fn eq(&self, _: &Self) -> bool {
                false
            }
        }
        impl Eq for Unstable {}

        let pin = Rc::new(PinProvider {
            calls: Cell::new(0),
            outcome: RefCell::new(VirtualLayoutSemanticQueryOutcome::NotFound),
        });
        let mut owner = SemanticDemandOwner::default();
        let mut custom = registration("policy", 8, Default::default(), Some(pin.clone()), None);
        custom.coordinate_space =
            VirtualLayoutCoordinateSpace::custom(VirtualLayoutPolicyIdentity::new("custom"));
        sync(&mut owner, custom);
        assert!(matches!(
            owner.semantic_pin(CONTAINER_ID, VirtualLayoutItemKey::new(1)),
            Err(SemanticDemandAdmissionError::CustomCoordinate)
        ));
        assert_eq!(pin.calls.get(), 0);

        let mut unstable = registration("policy", 8, Default::default(), Some(pin.clone()), None);
        unstable.policy_identity = VirtualLayoutPolicyIdentity::new(Unstable);
        sync(&mut owner, unstable);
        assert!(matches!(
            owner.semantic_pin(CONTAINER_ID, VirtualLayoutItemKey::new(1)),
            Err(SemanticDemandAdmissionError::ScopeMismatch)
        ));
        assert!(matches!(
            owner.semantic_pin(CONTAINER_ID, VirtualLayoutItemKey::new(Unstable)),
            Err(SemanticDemandAdmissionError::InvalidKey)
        ));
        assert_eq!(pin.calls.get(), 0);
    }

    #[test]
    fn initial_changed_retry_and_live_refresh_have_exact_generation_semantics() {
        let pin = Rc::new(PinProvider {
            calls: Cell::new(0),
            outcome: RefCell::new(VirtualLayoutSemanticQueryOutcome::NotFound),
        });
        let mut owner = SemanticDemandOwner::default();
        let registration = registration("policy", 8, Default::default(), Some(pin.clone()), None);
        sync(&mut owner, registration.clone());

        let first = started(
            owner
                .semantic_pin(CONTAINER_ID, VirtualLayoutItemKey::new(1))
                .expect("first"),
        );
        assert_eq!(first.fence().demand_generation, 1);
        assert_eq!(first.fence().attempt, 1);
        assert_eq!(
            complete_pin(&mut owner, first),
            VirtualLayoutSemanticQueryOutcome::NotFound
        );
        let retry = owner.retry_semantic_pin(CONTAINER_ID).expect("retry");
        assert_eq!(retry.fence().demand_generation, 1);
        assert_eq!(retry.fence().attempt, 2);
        assert_eq!(
            complete_pin(&mut owner, retry),
            VirtualLayoutSemanticQueryOutcome::NotFound
        );

        let changed = started(
            owner
                .semantic_pin(CONTAINER_ID, VirtualLayoutItemKey::new(2))
                .expect("changed"),
        );
        assert_eq!(changed.fence().demand_generation, 2);
        assert_eq!(changed.fence().attempt, 1);
        assert_eq!(
            complete_pin(&mut owner, changed),
            VirtualLayoutSemanticQueryOutcome::NotFound
        );

        let mut revised = registration;
        revised.revisions.data = 1;
        let refresh = owner.synchronize(&[(revised, MOUNT_GENERATION)]);
        assert_eq!(refresh.len(), 1);
        assert_eq!(refresh[0].fence().demand_generation, 3);
        assert_eq!(refresh[0].fence().attempt, 1);
        assert_eq!(
            complete_pin(&mut owner, refresh.into_iter().next().expect("refresh")),
            VirtualLayoutSemanticQueryOutcome::NotFound
        );
        assert_eq!(pin.calls.get(), 4);
    }

    #[test]
    fn live_refresh_rebuilds_pin_and_range_requests_from_current_authority() {
        let pin = Rc::new(RequestRecordingPinProvider {
            requests: RefCell::new(Vec::new()),
        });
        let range = Rc::new(RequestRecordingRangeProvider {
            requests: RefCell::new(Vec::new()),
        });
        let mut owner = SemanticDemandOwner::default();
        sync(
            &mut owner,
            registration(
                "policy",
                4,
                Default::default(),
                Some(pin.clone()),
                Some(range.clone()),
            ),
        );
        let _initial_pin = started(
            owner
                .semantic_pin(CONTAINER_ID, VirtualLayoutItemKey::new(7_u32))
                .expect("initial pin"),
        );
        let _initial_range = started(owner.range(CONTAINER_ID, 3, 2).expect("initial range"));

        let refreshed_revisions = VirtualLayoutRegistrationRevisions {
            viewport: 2,
            data: 3,
            policy: 4,
            measurement: 5,
            semantic: 6,
        };
        let refresh = owner.synchronize(&[(
            registration(
                "policy",
                9,
                refreshed_revisions,
                Some(pin.clone()),
                Some(range.clone()),
            ),
            MOUNT_GENERATION + 1,
        )]);
        assert_eq!(refresh.len(), 2);

        let pin_ticket = refresh
            .iter()
            .find(|ticket| ticket.fence().source == SemanticDemandSource::RequiredItemPin)
            .cloned()
            .expect("pin refresh");
        let range_ticket = refresh
            .iter()
            .find(|ticket| ticket.fence().source == SemanticDemandSource::Range)
            .cloned()
            .expect("range refresh");

        assert_eq!(pin_ticket.fence().mount_generation, MOUNT_GENERATION + 1);
        assert_eq!(pin_ticket.fence().data_revision, 3);
        assert_eq!(pin_ticket.fence().policy_revision, 4);
        assert_eq!(pin_ticket.fence().measurement_revision, 5);
        assert_eq!(pin_ticket.fence().semantic_revision, 6);
        assert_eq!(
            pin_ticket.fence().coordinate_space,
            VirtualLayoutCoordinateSpace::logical()
        );
        assert_eq!(pin_ticket.fence().budget, VirtualLayoutBudget::new(9));
        assert_eq!(range_ticket.fence().mount_generation, MOUNT_GENERATION + 1);
        assert_eq!(range_ticket.fence().data_revision, 3);
        assert_eq!(range_ticket.fence().policy_revision, 4);
        assert_eq!(range_ticket.fence().measurement_revision, 5);
        assert_eq!(range_ticket.fence().semantic_revision, 6);
        assert_eq!(
            range_ticket.fence().coordinate_space,
            VirtualLayoutCoordinateSpace::logical()
        );
        assert_eq!(range_ticket.fence().budget, VirtualLayoutBudget::new(9));

        let pin_completion = owner.execute(pin_ticket).expect("execute pin refresh");
        let range_completion = owner.execute(range_ticket).expect("execute range refresh");
        assert!(matches!(
            owner.complete(pin_completion),
            SemanticDemandCompletion::RequiredItemPin(VirtualLayoutSemanticQueryOutcome::NotFound)
        ));
        assert!(matches!(
            owner.complete(range_completion),
            SemanticDemandCompletion::Range(VirtualLayoutSemanticRangeQueryOutcome::NotFound)
        ));

        let pin_requests = pin.requests.borrow();
        assert_eq!(pin_requests.len(), 1);
        let pin_request = &pin_requests[0];
        assert_eq!(pin_request.container_id(), CONTAINER_ID);
        assert_eq!(pin_request.mount_generation(), MOUNT_GENERATION + 1);
        assert_eq!(pin_request.data_revision(), 3);
        assert_eq!(pin_request.policy_revision(), 4);
        assert_eq!(pin_request.measurement_revision(), 5);
        assert_eq!(pin_request.semantic_revision(), 6);
        assert_eq!(pin_request.key(), &VirtualLayoutItemKey::new(7_u32));

        let range_requests = range.requests.borrow();
        assert_eq!(range_requests.len(), 1);
        let range_request = &range_requests[0];
        assert_eq!(range_request.container_id(), CONTAINER_ID);
        assert_eq!(range_request.mount_generation(), MOUNT_GENERATION + 1);
        assert_eq!(range_request.data_revision(), 3);
        assert_eq!(range_request.policy_revision(), 4);
        assert_eq!(range_request.measurement_revision(), 5);
        assert_eq!(range_request.semantic_revision(), 6);
        assert_eq!(
            range_request.coordinate_space(),
            &VirtualLayoutCoordinateSpace::logical()
        );
        assert_eq!(range_request.budget(), VirtualLayoutBudget::new(9));
        assert_eq!(
            range_request.range(),
            VirtualLayoutSemanticRange::new(3, 2).unwrap()
        );
    }

    #[test]
    fn live_refresh_budget_veto_withholds_range_but_preserves_membership() {
        let pin = Rc::new(PinProvider {
            calls: Cell::new(0),
            outcome: RefCell::new(VirtualLayoutSemanticQueryOutcome::NotFound),
        });
        let range = Rc::new(RangeProvider {
            calls: Cell::new(0),
            outcome: RefCell::new(VirtualLayoutSemanticRangeProviderOutcome::NotFound),
        });
        let mut owner = SemanticDemandOwner::default();
        sync(
            &mut owner,
            registration(
                "policy",
                2,
                Default::default(),
                Some(pin.clone()),
                Some(range.clone()),
            ),
        );
        let _pin = started(
            owner
                .semantic_pin(CONTAINER_ID, VirtualLayoutItemKey::new(7_u32))
                .expect("pin"),
        );
        let _range = started(owner.range(CONTAINER_ID, 0, 2).expect("range"));

        let refresh = owner.synchronize(&[(
            registration(
                "policy",
                1,
                Default::default(),
                Some(pin),
                Some(range.clone()),
            ),
            MOUNT_GENERATION,
        )]);
        assert_eq!(refresh.len(), 1);
        assert!(
            refresh
                .iter()
                .all(|ticket| ticket.fence().source != SemanticDemandSource::Range)
        );
        assert_terminal_slot(&owner, SemanticDemandSource::Range);
        assert!(owner.records[0].pin.is_some());
        assert_eq!(owner.active_range_length(), Some(0));
        assert_eq!(range.calls.get(), 0);
    }

    #[test]
    fn stale_completion_and_execution_are_side_effect_free_and_exactly_once() {
        let pin = Rc::new(PinProvider {
            calls: Cell::new(0),
            outcome: RefCell::new(VirtualLayoutSemanticQueryOutcome::NotFound),
        });
        let mut owner = SemanticDemandOwner::default();
        sync(
            &mut owner,
            registration("policy", 8, Default::default(), Some(pin.clone()), None),
        );
        let old = started(
            owner
                .semantic_pin(CONTAINER_ID, VirtualLayoutItemKey::new(1))
                .expect("old"),
        );
        let completion = owner.execute(old.clone()).expect("execute");
        assert_eq!(pin.calls.get(), 1);
        assert!(matches!(
            owner.execute(old.clone()),
            Err(SemanticDemandExecutionError::AlreadyExecuted)
        ));
        let newer = started(
            owner
                .semantic_pin(CONTAINER_ID, VirtualLayoutItemKey::new(2))
                .expect("newer"),
        );
        assert!(matches!(
            owner.complete(completion),
            SemanticDemandCompletion::Stale
        ));
        assert_eq!(pin.calls.get(), 1);
        assert_eq!(
            complete_pin(&mut owner, newer),
            VirtualLayoutSemanticQueryOutcome::NotFound
        );
    }

    #[test]
    fn outcomes_are_typed_and_terminal_or_fallback_as_specified() {
        let pin = Rc::new(PinProvider {
            calls: Cell::new(0),
            outcome: RefCell::new(VirtualLayoutSemanticQueryOutcome::Found(Box::new(
                pin_entry(1, "one"),
            ))),
        });
        let mut owner = SemanticDemandOwner::default();
        sync(
            &mut owner,
            registration("policy", 8, Default::default(), Some(pin.clone()), None),
        );
        let first = started(
            owner
                .semantic_pin(CONTAINER_ID, VirtualLayoutItemKey::new(1_u32))
                .expect("found"),
        );
        let first_outcome = complete_pin(&mut owner, first);
        assert!(matches!(
            first_outcome,
            VirtualLayoutSemanticQueryOutcome::Found(_)
        ));
        *pin.outcome.borrow_mut() = VirtualLayoutSemanticQueryOutcome::Deferred(
            crate::gui::layout_core::VirtualLayoutSemanticDeferredReason::DataPending,
        );
        let retry = owner.retry_semantic_pin(CONTAINER_ID).expect("retry");
        assert!(matches!(
            complete_pin(&mut owner, retry),
            VirtualLayoutSemanticQueryOutcome::Deferred(_)
        ));
        assert!(
            owner.records[0]
                .pin
                .as_ref()
                .is_some_and(|slot| slot.evidence.is_some())
        );

        *pin.outcome.borrow_mut() = VirtualLayoutSemanticQueryOutcome::Unavailable(
            VirtualLayoutSemanticUnavailableReason::Unsupported,
        );
        let terminal = owner
            .retry_semantic_pin(CONTAINER_ID)
            .expect("terminal retry");
        assert!(matches!(
            complete_pin(&mut owner, terminal),
            VirtualLayoutSemanticQueryOutcome::Unavailable(
                VirtualLayoutSemanticUnavailableReason::Unsupported
            )
        ));
        assert_terminal_slot(&owner, SemanticDemandSource::RequiredItemPin);

        *pin.outcome.borrow_mut() = VirtualLayoutSemanticQueryOutcome::Unavailable(
            VirtualLayoutSemanticUnavailableReason::NoProvider,
        );
        assert!(matches!(
            owner.semantic_pin(CONTAINER_ID, VirtualLayoutItemKey::new(1_u32)),
            Ok(SemanticDemandAdmission::Unchanged)
        ));
        let no_provider = owner
            .retry_semantic_pin(CONTAINER_ID)
            .expect("retry terminal demand");
        assert!(matches!(
            complete_pin(&mut owner, no_provider),
            VirtualLayoutSemanticQueryOutcome::Unavailable(
                VirtualLayoutSemanticUnavailableReason::NoProvider
            )
        ));
        assert_terminal_slot(&owner, SemanticDemandSource::RequiredItemPin);

        let mut no_provider_owner = SemanticDemandOwner::default();
        sync(
            &mut no_provider_owner,
            registration("policy", 8, Default::default(), None, None),
        );
        let ticket = started(
            no_provider_owner
                .semantic_pin(CONTAINER_ID, VirtualLayoutItemKey::new(1))
                .expect("missing provider"),
        );
        assert!(matches!(
            complete_pin(&mut no_provider_owner, ticket),
            VirtualLayoutSemanticQueryOutcome::Unavailable(
                VirtualLayoutSemanticUnavailableReason::NoProvider
            )
        ));
        assert_terminal_slot(&no_provider_owner, SemanticDemandSource::RequiredItemPin);
    }

    #[test]
    fn malformed_pin_and_range_results_retain_only_their_exact_slot() {
        let pin = Rc::new(PinProvider {
            calls: Cell::new(0),
            outcome: RefCell::new(VirtualLayoutSemanticQueryOutcome::Found(Box::new(
                pin_entry(2, "wrong"),
            ))),
        });
        let range = Rc::new(RangeProvider {
            calls: Cell::new(0),
            outcome: RefCell::new(VirtualLayoutSemanticRangeProviderOutcome::Found(vec![
                range_entry(1, 0),
            ])),
        });
        let mut owner = SemanticDemandOwner::default();
        sync(
            &mut owner,
            registration("policy", 8, Default::default(), Some(pin), Some(range)),
        );
        let pin_ticket = started(
            owner
                .semantic_pin(CONTAINER_ID, VirtualLayoutItemKey::new(1))
                .expect("pin"),
        );
        let range_ticket = started(owner.range(CONTAINER_ID, 0, 2).expect("range"));
        assert!(matches!(
            complete_pin(&mut owner, pin_ticket),
            VirtualLayoutSemanticQueryOutcome::Rejected(
                VirtualLayoutSemanticRejectedReason::WrongKey
            )
        ));
        assert_terminal_slot(&owner, SemanticDemandSource::RequiredItemPin);
        assert!(owner.records[0].range.is_some());
        assert!(matches!(
            complete_range(&mut owner, range_ticket),
            VirtualLayoutSemanticRangeQueryOutcome::Rejected(
                VirtualLayoutSemanticRejectedReason::RangeCountMismatch
            )
        ));
        assert_terminal_slot(&owner, SemanticDemandSource::Range);
    }

    #[test]
    fn range_outcomes_retain_complete_evidence_and_terminal_failures_retain_membership() {
        let range = Rc::new(RangeProvider {
            calls: Cell::new(0),
            outcome: RefCell::new(VirtualLayoutSemanticRangeProviderOutcome::Found(vec![
                range_entry(1, 0),
                range_entry(2, 1),
            ])),
        });
        let mut owner = SemanticDemandOwner::default();
        sync(
            &mut owner,
            registration("policy", 8, Default::default(), None, Some(range.clone())),
        );
        let first = started(owner.range(CONTAINER_ID, 0, 2).expect("range"));
        match complete_range(&mut owner, first) {
            VirtualLayoutSemanticRangeQueryOutcome::Found(batch) => {
                assert_eq!(batch.projections().len(), 2);
                assert_eq!(batch.request().range().start_index(), 0);
            }
            other => panic!("unexpected range outcome: {other:?}"),
        }
        assert!(
            owner.records[0]
                .range
                .as_ref()
                .is_some_and(|slot| slot.evidence.is_some() && slot.retained.is_some())
        );

        *range.outcome.borrow_mut() = VirtualLayoutSemanticRangeProviderOutcome::Deferred(
            crate::gui::layout_core::VirtualLayoutSemanticDeferredReason::DataPending,
        );
        let deferred = owner.retry_range(CONTAINER_ID).expect("deferred retry");
        assert!(matches!(
            complete_range(&mut owner, deferred),
            VirtualLayoutSemanticRangeQueryOutcome::Deferred(_)
        ));
        assert!(
            owner.records[0]
                .range
                .as_ref()
                .is_some_and(|slot| slot.evidence.is_some() && !slot.withheld)
        );

        *range.outcome.borrow_mut() = VirtualLayoutSemanticRangeProviderOutcome::Unavailable(
            VirtualLayoutSemanticUnavailableReason::DataUnavailable,
        );
        let unavailable = owner.retry_range(CONTAINER_ID).expect("data retry");
        assert!(matches!(
            complete_range(&mut owner, unavailable),
            VirtualLayoutSemanticRangeQueryOutcome::Unavailable(
                VirtualLayoutSemanticUnavailableReason::DataUnavailable
            )
        ));
        assert!(
            owner.records[0]
                .range
                .as_ref()
                .is_some_and(|slot| slot.evidence.is_some())
        );

        *range.outcome.borrow_mut() = VirtualLayoutSemanticRangeProviderOutcome::Rejected(
            VirtualLayoutSemanticRejectedReason::ProviderRejected,
        );
        let rejected = owner.retry_range(CONTAINER_ID).expect("rejected retry");
        assert!(matches!(
            complete_range(&mut owner, rejected),
            VirtualLayoutSemanticRangeQueryOutcome::Rejected(
                VirtualLayoutSemanticRejectedReason::ProviderRejected
            )
        ));
        assert_terminal_slot(&owner, SemanticDemandSource::Range);

        *range.outcome.borrow_mut() = VirtualLayoutSemanticRangeProviderOutcome::Unavailable(
            VirtualLayoutSemanticUnavailableReason::Unsupported,
        );
        let terminal = owner
            .retry_range(CONTAINER_ID)
            .expect("retry rejected range demand");
        assert!(matches!(
            complete_range(&mut owner, terminal),
            VirtualLayoutSemanticRangeQueryOutcome::Unavailable(
                VirtualLayoutSemanticUnavailableReason::Unsupported
            )
        ));
        assert_terminal_slot(&owner, SemanticDemandSource::Range);
        assert_eq!(range.calls.get(), 5);
    }

    #[test]
    fn provider_handles_and_generations_are_independent_by_source() {
        let old_pin = Rc::new(PinProvider {
            calls: Cell::new(0),
            outcome: RefCell::new(VirtualLayoutSemanticQueryOutcome::NotFound),
        });
        let new_pin = Rc::new(PinProvider {
            calls: Cell::new(0),
            outcome: RefCell::new(VirtualLayoutSemanticQueryOutcome::NotFound),
        });
        let range = Rc::new(RangeProvider {
            calls: Cell::new(0),
            outcome: RefCell::new(VirtualLayoutSemanticRangeProviderOutcome::NotFound),
        });
        let base = registration(
            "policy",
            8,
            Default::default(),
            Some(old_pin.clone()),
            Some(range.clone()),
        );
        let mut owner = SemanticDemandOwner::default();
        sync(&mut owner, base.clone());
        let old_pin_ticket = started(
            owner
                .semantic_pin(CONTAINER_ID, VirtualLayoutItemKey::new(1_u32))
                .expect("pin"),
        );
        let range_ticket = started(owner.range(CONTAINER_ID, 0, 1).expect("range"));
        let old_range_generation = range_ticket.fence().provider_generation;

        let next = base.with_semantic_provider(new_pin.clone());
        let refresh = owner.synchronize(&[(next, MOUNT_GENERATION)]);
        assert_eq!(refresh.len(), 1);
        assert_eq!(
            refresh[0].fence().source,
            SemanticDemandSource::RequiredItemPin
        );
        assert_ne!(
            refresh[0].fence().provider_generation,
            old_pin_ticket.fence().provider_generation
        );
        assert_eq!(
            owner.records[0]
                .range
                .as_ref()
                .expect("range slot")
                .fence
                .provider_generation,
            old_range_generation
        );
        assert!(matches!(
            owner.execute(old_pin_ticket),
            Ok(SemanticProviderCompletion::Stale)
        ));
        assert_eq!(
            complete_range(&mut owner, range_ticket),
            VirtualLayoutSemanticRangeQueryOutcome::NotFound
        );
        assert_eq!(
            complete_pin(&mut owner, refresh.into_iter().next().expect("pin refresh")),
            VirtualLayoutSemanticQueryOutcome::NotFound
        );
        assert_eq!(old_pin.calls.get(), 0);
        assert_eq!(new_pin.calls.get(), 1);
        assert_eq!(range.calls.get(), 1);
    }

    #[test]
    fn retention_vetoes_each_non_retry_fence_field() {
        let pin = Rc::new(PinProvider {
            calls: Cell::new(0),
            outcome: RefCell::new(VirtualLayoutSemanticQueryOutcome::NotFound),
        });
        let mut owner = SemanticDemandOwner::default();
        let mut registration = registration("policy", 8, Default::default(), Some(pin), None);
        registration.semantic_cardinality = Some(VirtualLayoutSemanticCardinality::new(1, 1));
        sync(&mut owner, registration);
        let ticket = started(
            owner
                .semantic_pin(CONTAINER_ID, VirtualLayoutItemKey::new(1_u32))
                .expect("pin"),
        );
        let base = ticket.fence().clone();

        let mut changed = base.clone();
        changed.container_id += 1;
        assert!(!base.same_retention_fence(&changed));
        let mut changed = base.clone();
        changed.policy_identity = VirtualLayoutPolicyIdentity::new("other");
        assert!(!base.same_retention_fence(&changed));
        let mut changed = base.clone();
        changed.registration_generation += 1;
        assert!(!base.same_retention_fence(&changed));
        let mut changed = base.clone();
        changed.mount_generation += 1;
        assert!(!base.same_retention_fence(&changed));
        let mut changed = base.clone();
        changed.data_revision += 1;
        assert!(!base.same_retention_fence(&changed));
        let mut changed = base.clone();
        changed.policy_revision += 1;
        assert!(!base.same_retention_fence(&changed));
        let mut changed = base.clone();
        changed.measurement_revision += 1;
        assert!(!base.same_retention_fence(&changed));
        let mut changed = base.clone();
        changed.semantic_revision += 1;
        assert!(!base.same_retention_fence(&changed));
        let mut changed = base.clone();
        changed
            .semantic_cardinality
            .as_mut()
            .expect("semantic cardinality")
            .logical_item_count += 1;
        assert!(!base.same_retention_fence(&changed));
        let mut changed = base.clone();
        changed
            .semantic_cardinality
            .as_mut()
            .expect("semantic cardinality")
            .cardinality_revision += 1;
        assert!(!base.same_retention_fence(&changed));
        let mut changed = base.clone();
        changed.coordinate_space =
            VirtualLayoutCoordinateSpace::custom(VirtualLayoutPolicyIdentity::new("custom"));
        assert!(!base.same_retention_fence(&changed));
        let mut changed = base.clone();
        changed.budget = VirtualLayoutBudget::new(7);
        assert!(!base.same_retention_fence(&changed));
        let mut changed = base.clone();
        changed.demand = SemanticDemand::RequiredItemPin(VirtualLayoutItemKey::new(2_u32));
        assert!(!base.same_retention_fence(&changed));
        let mut changed = base.clone();
        changed.source = SemanticDemandSource::Range;
        assert!(!base.same_retention_fence(&changed));
        let mut changed = base.clone();
        changed.provider_identity = SemanticProviderIdentity::Missing;
        assert!(!base.same_retention_fence(&changed));
        let mut changed = base.clone();
        changed.provider_generation += 1;
        assert!(!base.same_retention_fence(&changed));

        let mut retry = base.clone();
        retry.attempt += 1;
        assert!(base.same_retention_fence(&retry));
        let mut retry = base.clone();
        retry.demand_generation += 1;
        assert!(base.same_retention_fence(&retry));
        let mut cancelled = base.clone();
        cancelled.cancelled = true;
        assert!(!base.same_retention_fence(&cancelled));
    }

    #[test]
    fn retention_requires_every_fence_field_but_allows_only_retry_fields() {
        let pin = Rc::new(PinProvider {
            calls: Cell::new(0),
            outcome: RefCell::new(VirtualLayoutSemanticQueryOutcome::Found(Box::new(
                pin_entry(1, "one"),
            ))),
        });
        let mut owner = SemanticDemandOwner::default();
        let base = registration("policy", 8, Default::default(), Some(pin.clone()), None);
        sync(&mut owner, base.clone());
        let first = started(
            owner
                .semantic_pin(CONTAINER_ID, VirtualLayoutItemKey::new(1_u32))
                .expect("first"),
        );
        let first_outcome = complete_pin(&mut owner, first);
        assert!(matches!(
            first_outcome,
            VirtualLayoutSemanticQueryOutcome::Found(_)
        ));

        *pin.outcome.borrow_mut() = VirtualLayoutSemanticQueryOutcome::Deferred(
            crate::gui::layout_core::VirtualLayoutSemanticDeferredReason::Retry,
        );
        let retry = owner.retry_semantic_pin(CONTAINER_ID).expect("retry");
        assert!(matches!(
            complete_pin(&mut owner, retry),
            VirtualLayoutSemanticQueryOutcome::Deferred(_)
        ));
        assert!(
            owner.records[0]
                .pin
                .as_ref()
                .is_some_and(|slot| slot.evidence.is_some())
        );

        let mut changed = base.clone();
        changed.revisions.data = 1;
        let tickets = owner.synchronize(&[(changed, MOUNT_GENERATION)]);
        assert_eq!(tickets.len(), 1);
        assert!(matches!(
            complete_pin(&mut owner, tickets[0].clone()),
            VirtualLayoutSemanticQueryOutcome::Deferred(_)
        ));
        assert!(
            owner.records[0]
                .pin
                .as_ref()
                .is_some_and(|slot| slot.evidence.is_none() && slot.withheld)
        );

        let mut replaced = base;
        replaced.policy_identity = VirtualLayoutPolicyIdentity::new("other");
        let _ = owner.synchronize(&[(replaced, MOUNT_GENERATION)]);
        assert!(owner.records[0].pin.is_none());
        assert_eq!(pin.calls.get(), 3);
    }

    #[test]
    fn viewport_overscan_and_provider_reads_do_not_create_demand() {
        let pin = Rc::new(PinProvider {
            calls: Cell::new(0),
            outcome: RefCell::new(VirtualLayoutSemanticQueryOutcome::NotFound),
        });
        let mut owner = SemanticDemandOwner::default();
        let base = registration("policy", 8, Default::default(), Some(pin.clone()), None);
        sync(&mut owner, base.clone());
        let first = started(
            owner
                .semantic_pin(CONTAINER_ID, VirtualLayoutItemKey::new(1))
                .expect("first"),
        );
        assert_eq!(
            complete_pin(&mut owner, first),
            VirtualLayoutSemanticQueryOutcome::NotFound
        );
        let before_generation = owner.records[0]
            .pin
            .as_ref()
            .expect("slot")
            .fence
            .demand_generation;
        let mut viewport_only = base.clone();
        viewport_only.revisions.viewport = 9;
        viewport_only.overscan = VirtualLayoutOverscan::new(2.0, 3.0).expect("overscan");
        assert!(
            owner
                .synchronize(&[(viewport_only, MOUNT_GENERATION)])
                .is_empty()
        );
        assert_eq!(
            owner.records[0]
                .pin
                .as_ref()
                .expect("slot")
                .fence
                .demand_generation,
            before_generation
        );
        assert_eq!(pin.calls.get(), 1);
        assert!(matches!(
            owner.semantic_pin(CONTAINER_ID, VirtualLayoutItemKey::new(1)),
            Ok(SemanticDemandAdmission::Unchanged)
        ));
    }
}
