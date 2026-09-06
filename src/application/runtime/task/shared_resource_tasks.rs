//! Shared resource demand, with application-owned values and completion policy.

use super::resource_interests::{
    ResourceInterestAdmissionError, ResourceInterestClass as InterestClass, ResourceInterestId,
    ResourceInterestLease, ResourceInterestLedger, ResourceInterestLiveness,
    ResourceInterestOwnerId, ResourceInterestRuntimeId,
};
use super::resource_operations::{
    ResourceOperationAdmissionError, ResourceOperationCurrent, ResourceOperationRegistry,
};
use crate::runtime::ResourceKey;

/// Whether an existing resource operation should be reused or replaced.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SharedResourceTaskMode {
    /// Share running work, retained ready state, or scheduled backoff.
    Join,
    /// Explicitly refresh, replacing accepted work or bypassing backoff.
    Refresh,
}

/// A bounded shared operation could not be reserved.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SharedResourceTaskError {
    /// Ready-state retention was rejected by the interest registry.
    Interest(ResourceInterestError),
    /// The broker has shut down.
    Closed,
    /// No accepted consumer currently needs this resource.
    NoInterest,
    /// The bounded operation registry is full.
    Capacity,
    /// An earlier replacement still awaits host admission or rejection.
    PendingAdmission,
    /// The monotonic operation identity space is exhausted.
    IdentityExhausted,
}

impl From<ResourceOperationAdmissionError> for SharedResourceTaskError {
    fn from(value: ResourceOperationAdmissionError) -> Self {
        match value {
            ResourceOperationAdmissionError::Interest(error) => Self::Interest(error.into()),
            ResourceOperationAdmissionError::Closed => Self::Closed,
            ResourceOperationAdmissionError::NoLiveInterest => Self::NoInterest,
            ResourceOperationAdmissionError::Capacity => Self::Capacity,
            ResourceOperationAdmissionError::PendingAdmission => Self::PendingAdmission,
            ResourceOperationAdmissionError::EpochExhausted => Self::IdentityExhausted,
        }
    }
}

/// A worker output and the exact shared-operation fence that produced it.
///
/// Apply output in the UI reducer through [`SharedResourceTasks::finish_ready`]
/// or [`SharedResourceTasks::finish_failed`]. The mapper itself should only
/// construct a message, so the runtime can still check its post-mapping fence.
#[derive(Debug)]
pub struct SharedResourceCompletion<Output> {
    /// Application-owned output. Resource values and errors never enter the broker.
    pub output: Output,
    pub(crate) current: ResourceOperationCurrent,
}

impl<Output> SharedResourceCompletion<Output> {
    /// Resource key of the completed operation.
    pub fn key(&self) -> &ResourceKey {
        self.current.key()
    }
    /// Monotonic operation identity within this broker.
    pub fn operation_id(&self) -> u64 {
        self.current.operation_epoch()
    }
}

/// Why a consumer currently needs a shared resource.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceInterestKind {
    /// The resource contributes to current visible UI.
    Visible,
    /// The consumer anticipates needing the resource soon.
    Prefetch,
    /// Explicit application policy keeps interest beyond visibility.
    Persistent,
}

impl From<ResourceInterestKind> for InterestClass {
    fn from(value: ResourceInterestKind) -> Self {
        match value {
            ResourceInterestKind::Visible => Self::Visible,
            ResourceInterestKind::Prefetch => Self::Prefetch,
            ResourceInterestKind::Persistent => Self::Persistent,
        }
    }
}

impl From<InterestClass> for ResourceInterestKind {
    fn from(value: InterestClass) -> Self {
        match value {
            InterestClass::Visible => Self::Visible,
            InterestClass::Prefetch => Self::Prefetch,
            InterestClass::Persistent => Self::Persistent,
        }
    }
}

/// Failure to attach an interest to an accepted runtime owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceInterestError {
    /// The broker has shut down.
    Closed,
    /// A shared broker cannot span distinct runtime instances.
    RuntimeMismatch,
    /// The selected declarative owner is absent, ambiguous, or retired.
    OwnerUnavailable,
    /// The broker's retained resource-key bound has been reached.
    ResourceCapacity,
    /// The broker's total interest bound has been reached.
    InterestCapacity,
    /// The resource's interest bound has been reached.
    PerResourceCapacity,
    /// The runtime's aggregate interest bound has been reached.
    RuntimeCapacity,
    /// Monotonic identity space is exhausted.
    IdentityExhausted,
}

impl From<ResourceInterestAdmissionError> for ResourceInterestError {
    fn from(value: ResourceInterestAdmissionError) -> Self {
        match value {
            ResourceInterestAdmissionError::Closed => Self::Closed,
            ResourceInterestAdmissionError::RuntimeMismatch => Self::RuntimeMismatch,
            ResourceInterestAdmissionError::OwnerRetired => Self::OwnerUnavailable,
            ResourceInterestAdmissionError::ResourceCapacity => Self::ResourceCapacity,
            ResourceInterestAdmissionError::LeaseCapacity => Self::InterestCapacity,
            ResourceInterestAdmissionError::PerResourceLeaseCapacity => Self::PerResourceCapacity,
            ResourceInterestAdmissionError::LeaseIdExhausted => Self::IdentityExhausted,
        }
    }
}

/// An application-held interest in one shared resource.
///
/// Clones refer to the same interest. Dropping the final clone releases it;
/// explicit release retires all clones. Runtime owner retirement also releases
/// the interest, even when the application still holds this handle.
#[derive(Clone)]
pub struct ResourceInterest {
    pub(crate) lease: ResourceInterestLease,
    key: ResourceKey,
}

impl ResourceInterest {
    /// The resource this interest refers to.
    pub fn key(&self) -> &ResourceKey {
        &self.key
    }

    /// The current kind, or `None` after release or owner retirement.
    pub fn kind(&self) -> Option<ResourceInterestKind> {
        self.lease.class().map(Into::into)
    }

    /// Change visibility policy while preserving the shared operation.
    pub fn set_kind(&self, kind: ResourceInterestKind) -> bool {
        self.lease.set_class(kind.into())
    }

    /// Release this exact interest. Repeated release is harmless.
    pub fn release(&self) -> bool {
        self.lease.release()
    }

    /// Whether this interest still has a live accepted owner.
    pub fn is_live(&self) -> bool {
        self.lease.is_live()
    }
}

/// Application-owned coordination for shared resource work.
///
/// Clones share the same broker. This differs deliberately from the isolated
/// latest/exclusive task snapshots provided by [`super::ResourceTasks`].
/// Actual values, errors, and cache policy remain in application state.
/// A broker admits at most 256 resource keys, 1,024 interests, and 64 interests
/// per key. It binds to one runtime on successful interest admission.
#[derive(Clone)]
pub struct SharedResourceTasks {
    pub(crate) ledger: ResourceInterestLedger,
    pub(crate) operations: ResourceOperationRegistry,
}

impl Default for SharedResourceTasks {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedResourceTasks {
    /// Create an empty shared broker.
    pub fn new() -> Self {
        let ledger = ResourceInterestLedger::new();
        let operations = ResourceOperationRegistry::with_ledger(ledger.clone());
        Self { ledger, operations }
    }

    /// Cancel all demand and work, permanently closing this broker.
    pub fn shutdown(&self) {
        self.ledger.shutdown();
        self.operations.shutdown();
    }

    /// Count current distinct interests after pruning retired owners.
    pub fn interest_count(&self) -> usize {
        self.ledger.live_lease_count()
    }

    /// Cancel current work or retry state while preserving consumer interests.
    ///
    /// A subsequent join can start fresh work after pending host admission settles.
    pub fn cancel(&self, key: &ResourceKey) -> bool {
        self.operations.cancel(key)
    }

    /// Retain ready bookkeeping when interests disappear; values stay in application state.
    pub fn retain_ready(
        &self,
        key: impl Into<ResourceKey>,
        retain: bool,
    ) -> Result<(), SharedResourceTaskError> {
        self.operations
            .set_keep_ready(key.into(), retain)
            .map_err(Into::into)
    }

    /// Accept a successful current completion and return its application value.
    pub fn finish_ready<Output>(
        &self,
        completion: SharedResourceCompletion<Output>,
    ) -> Option<Output> {
        self.operations
            .finish_ready(completion.current)
            .then_some(completion.output)
    }

    /// Accept a current failure without retaining the error in the broker.
    pub fn finish_failed<Output>(
        &self,
        completion: SharedResourceCompletion<Output>,
    ) -> Option<Output> {
        self.operations
            .finish_idle(completion.current)
            .then_some(completion.output)
    }

    /// Schedule a bounded retry deadline using the application's logical clock.
    ///
    /// This accepts the failed completion. A due retry is taken explicitly with
    /// `Effect::resource_retry`; no timer or provider is started by observation.
    pub fn schedule_retry<Output>(
        &self,
        completion: &SharedResourceCompletion<Output>,
        deadline: u64,
    ) -> bool {
        self.operations
            .schedule_retry(completion.current.clone(), deadline)
    }

    /// Request one interest after the runtime has accepted the selected owner.
    ///
    /// `interest_id` is stable within that owner and resource, independently of
    /// the interest kind. Repeated requests share one lease. Keep the returned
    /// handle in application state; discarded results do not keep work alive.
    pub fn interest<Message: 'static>(
        &self,
        key: impl Into<ResourceKey>,
        owner: crate::runtime::EffectOwner,
        interest_id: u64,
        kind: ResourceInterestKind,
        on_completed: impl FnOnce(Result<ResourceInterest, ResourceInterestError>) -> Message + 'static,
    ) -> crate::runtime::Command<Message> {
        crate::runtime::Command::AcquireResourceInterest(crate::runtime::ResourceInterestEffect {
            tasks: self.clone(),
            key: key.into(),
            owner,
            interest_id,
            kind,
            on_completed: Box::new(on_completed),
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn admit_interest(
        &self,
        runtime: u64,
        owner_generation: u64,
        interest_id: u64,
        key: ResourceKey,
        kind: ResourceInterestKind,
        live: ResourceInterestLiveness,
    ) -> Result<ResourceInterest, ResourceInterestError> {
        let lease = self.ledger.admit(
            ResourceInterestRuntimeId::new(runtime),
            ResourceInterestOwnerId::new(owner_generation),
            ResourceInterestId::new(interest_id),
            key.clone(),
            kind.into(),
            live,
        )?;
        Ok(ResourceInterest { lease, key })
    }
}
