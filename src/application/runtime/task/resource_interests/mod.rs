//! Private ownership ledger for shared, application-managed resource work.
//!
//! This module deliberately records interests only. The application retains
//! resource values, task state, retry policy, and completion handling.

use crate::runtime::ResourceKey;
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicBool, Ordering},
    },
};

#[cfg(test)]
mod tests;

const DEFAULT_MAX_RESOURCES: usize = 256;
const DEFAULT_MAX_LEASES: usize = 1024;
const DEFAULT_MAX_LEASES_PER_RESOURCE: usize = 64;

/// Opaque identity for one controller runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ResourceInterestRuntimeId(u64);

impl ResourceInterestRuntimeId {
    pub(crate) const fn new(value: u64) -> Self {
        Self(value)
    }
}

/// Opaque identity for one accepted declarative owner generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ResourceInterestOwnerId(u64);

impl ResourceInterestOwnerId {
    pub(crate) const fn new(value: u64) -> Self {
        Self(value)
    }
}

/// Caller-stable identity for one declarative interest independent of class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ResourceInterestId(u64);

impl ResourceInterestId {
    pub(crate) const fn new(value: u64) -> Self {
        Self(value)
    }
}

/// Scheduling reason for a resource interest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResourceInterestClass {
    Visible,
    Prefetch,
    Persistent,
}

/// Atomic liveness evidence supplied by the controller adapter.
///
/// The ledger only loads this atomic witness while locked. It never invokes
/// application callbacks while mutating its state.
pub(crate) type ResourceInterestLiveness = Arc<AtomicBool>;

/// Metadata retained for an otherwise empty resource key.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ResourceInterestMetadata {
    pub(crate) keep_ready: bool,
}

/// Bounded admission failures for the pure interest ledger.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResourceInterestAdmissionError {
    Closed,
    OwnerRetired,
    RuntimeMismatch,
    ResourceCapacity,
    LeaseCapacity,
    PerResourceLeaseCapacity,
    LeaseIdExhausted,
}

/// Thread-safe application-owned registry of declarative resource interests.
#[derive(Clone)]
pub(crate) struct ResourceInterestLedger {
    state: Arc<Mutex<ResourceInterestState>>,
}

impl Default for ResourceInterestLedger {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceInterestLedger {
    pub(crate) fn new() -> Self {
        Self::with_limits(
            DEFAULT_MAX_RESOURCES,
            DEFAULT_MAX_LEASES,
            DEFAULT_MAX_LEASES_PER_RESOURCE,
        )
    }

    #[cfg(test)]
    pub(crate) fn with_test_limits(
        max_resources: usize,
        max_leases: usize,
        max_leases_per_resource: usize,
    ) -> Self {
        Self::with_limits(max_resources, max_leases, max_leases_per_resource)
    }

    fn with_limits(
        max_resources: usize,
        max_leases: usize,
        max_leases_per_resource: usize,
    ) -> Self {
        assert!(max_resources > 0);
        assert!(max_leases > 0);
        assert!(max_leases_per_resource > 0);
        Self {
            state: Arc::new(Mutex::new(ResourceInterestState {
                limits: ResourceInterestLimits {
                    max_resources,
                    max_leases,
                    max_leases_per_resource,
                },
                ..ResourceInterestState::default()
            })),
        }
    }

    /// Admit or deduplicate one exact resource/owner/interest triple.
    pub(crate) fn admit(
        &self,
        runtime: ResourceInterestRuntimeId,
        owner: ResourceInterestOwnerId,
        interest: ResourceInterestId,
        key: ResourceKey,
        class: ResourceInterestClass,
        owner_live: ResourceInterestLiveness,
    ) -> Result<ResourceInterestLease, ResourceInterestAdmissionError> {
        let mut state = lock_state(&self.state);
        state.prune_dead();
        if state.closed {
            return Err(ResourceInterestAdmissionError::Closed);
        }
        if !owner_live.load(Ordering::Acquire) {
            return Err(ResourceInterestAdmissionError::OwnerRetired);
        }
        if state.runtime.is_some_and(|bound| bound != runtime) {
            return Err(ResourceInterestAdmissionError::RuntimeMismatch);
        }

        let identity = ResourceInterestIdentity { owner, interest };
        if let Some(existing) = state.live_handle_for(&key, identity) {
            return Ok(ResourceInterestLease { inner: existing });
        }
        // A last-handle drop can mark the lease released before it acquires
        // this lock. Remove that exact stale generation before considering
        // capacity or inserting a replacement.
        state.remove_nonlive_exact(&key, identity);

        let creating_resource = !state.resources.contains_key(&key);
        if creating_resource && state.resources.len() >= state.limits.max_resources {
            return Err(ResourceInterestAdmissionError::ResourceCapacity);
        }
        if state.lease_count >= state.limits.max_leases {
            return Err(ResourceInterestAdmissionError::LeaseCapacity);
        }
        if state
            .resources
            .get(&key)
            .is_some_and(|entry| entry.leases.len() >= state.limits.max_leases_per_resource)
        {
            return Err(ResourceInterestAdmissionError::PerResourceLeaseCapacity);
        }
        let generation = state.allocate_lease_generation()?;
        // Do not publish or bind a runtime if the controller retired the
        // accepted owner while this admission was being checked.
        if !owner_live.load(Ordering::Acquire) {
            return Err(ResourceInterestAdmissionError::OwnerRetired);
        }
        let inner = Arc::new(ResourceInterestLeaseInner {
            state: Arc::downgrade(&self.state),
            key: key.clone(),
            identity,
            generation,
            released: Arc::new(AtomicBool::new(false)),
        });
        let resource = state.resources.entry(key).or_default();
        if resource.leases.is_empty() {
            resource.demand_generation = Some(generation);
        }
        resource.leases.insert(
            identity,
            ResourceInterestEntry {
                generation,
                class,
                owner_live,
                released: Arc::clone(&inner.released),
                handle: Arc::downgrade(&inner),
            },
        );
        state.lease_count += 1;
        state.runtime.get_or_insert(runtime);
        Ok(ResourceInterestLease { inner })
    }

    /// Set metadata for a key without attaching resource values to the ledger.
    pub(crate) fn set_metadata(
        &self,
        key: ResourceKey,
        metadata: ResourceInterestMetadata,
    ) -> Result<(), ResourceInterestAdmissionError> {
        let mut state = lock_state(&self.state);
        state.prune_dead();
        if state.closed {
            return Err(ResourceInterestAdmissionError::Closed);
        }
        if !state.resources.contains_key(&key)
            && state.resources.len() >= state.limits.max_resources
        {
            return Err(ResourceInterestAdmissionError::ResourceCapacity);
        }
        let remove_after = {
            let entry = state.resources.entry(key.clone()).or_default();
            entry.metadata = metadata;
            entry.leases.is_empty() && !entry.metadata.keep_ready
        };
        if remove_after {
            state.resources.remove(&key);
        }
        Ok(())
    }

    /// Remove all interests. Existing handles become stale and release harmlessly.
    pub(crate) fn shutdown(&self) {
        let mut state = lock_state(&self.state);
        state.closed = true;
        state.resources.clear();
        state.lease_count = 0;
    }

    #[cfg(test)]
    pub(crate) fn prune_dead_owners(&self) {
        lock_state(&self.state).prune_dead();
    }

    pub(crate) fn live_lease_count(&self) -> usize {
        let mut state = lock_state(&self.state);
        state.prune_dead();
        state.lease_count
    }

    #[cfg(test)]
    pub(crate) fn retained_resource_count(&self) -> usize {
        let mut state = lock_state(&self.state);
        state.prune_dead();
        state.resources.len()
    }

    #[cfg(test)]
    pub(crate) fn live_count_for(&self, key: &ResourceKey) -> usize {
        let mut state = lock_state(&self.state);
        state.prune_dead();
        state.resources.get(key).map_or(0, |entry| {
            entry
                .leases
                .values()
                .filter(|lease| lease.is_live())
                .count()
        })
    }

    /// Return whether this key retains ready bookkeeping after its last lease.
    pub(crate) fn keeps_ready(&self, key: &ResourceKey) -> bool {
        let mut state = lock_state(&self.state);
        state.prune_dead();
        state
            .resources
            .get(key)
            .is_some_and(|entry| entry.metadata.keep_ready)
    }

    /// Return the current shared-operation fence for one live resource demand.
    pub(crate) fn demand_generation(&self, key: &ResourceKey) -> Option<u64> {
        let mut state = lock_state(&self.state);
        state.prune_dead();
        state.resources.get(key)?.demand_generation
    }

    pub(crate) fn is_bound_to(&self, runtime: ResourceInterestRuntimeId) -> bool {
        lock_state(&self.state).runtime == Some(runtime)
    }
}

/// One cloneable RAII handle for a deduplicated resource interest.
pub(crate) struct ResourceInterestLease {
    inner: Arc<ResourceInterestLeaseInner>,
}

impl Clone for ResourceInterestLease {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl ResourceInterestLease {
    pub(crate) fn class(&self) -> Option<ResourceInterestClass> {
        let state = self.inner.state.upgrade()?;
        let mut state = lock_state(&state);
        state.prune_dead();
        state.class_for(&self.inner)
    }

    /// Update class without retiring or replacing this shared operation identity.
    pub(crate) fn set_class(&self, class: ResourceInterestClass) -> bool {
        let Some(state) = self.inner.state.upgrade() else {
            return false;
        };
        let mut state = lock_state(&state);
        state.prune_dead();
        let Some(entry) = state.entry_mut_for(&self.inner) else {
            return false;
        };
        entry.class = class;
        true
    }

    /// Release this exact generation. Repeated or stale releases are harmless.
    pub(crate) fn release(&self) -> bool {
        self.inner.release()
    }

    pub(crate) fn is_live(&self) -> bool {
        self.class().is_some()
    }

    /// Create a non-owning controller-retirement guard.
    ///
    /// Runtime reconciliation must retain this weak guard rather than extending
    /// the application interest's lifetime.
    pub(crate) fn downgrade(&self) -> ResourceInterestLeaseWeak {
        ResourceInterestLeaseWeak {
            lease: Arc::downgrade(&self.inner),
            state: self.inner.state.clone(),
            key: self.inner.key.clone(),
            identity: self.inner.identity,
            generation: self.inner.generation,
            released: Arc::clone(&self.inner.released),
        }
    }
}

/// Non-owning controller guard that may retire an application-held lease.
#[derive(Clone)]
pub(crate) struct ResourceInterestLeaseWeak {
    lease: Weak<ResourceInterestLeaseInner>,
    state: Weak<Mutex<ResourceInterestState>>,
    key: ResourceKey,
    identity: ResourceInterestIdentity,
    generation: u64,
    released: Arc<AtomicBool>,
}

impl ResourceInterestLeaseWeak {
    /// Return whether two registry guards refer to the same exact lease allocation.
    pub(crate) fn is_same(&self, other: &Self) -> bool {
        Weak::ptr_eq(&self.lease, &other.lease)
    }

    /// Retire the exact live lease without retaining application ownership.
    pub(crate) fn release(&self) -> bool {
        if self.released.swap(true, Ordering::AcqRel) {
            return false;
        }
        let Some(state) = self.state.upgrade() else {
            return false;
        };
        lock_state(&state).release_exact(&self.key, self.identity, self.generation)
    }

    pub(crate) fn is_live(&self) -> bool {
        if self.released.load(Ordering::Acquire) {
            return false;
        }
        let Some(state) = self.state.upgrade() else {
            return false;
        };
        let mut state = lock_state(&state);
        state.prune_dead();
        state.is_generation_live(&self.key, self.identity, self.generation)
    }
}

impl Drop for ResourceInterestLeaseInner {
    fn drop(&mut self) {
        let _ = self.release();
    }
}

struct ResourceInterestLeaseInner {
    state: Weak<Mutex<ResourceInterestState>>,
    key: ResourceKey,
    identity: ResourceInterestIdentity,
    generation: u64,
    released: Arc<AtomicBool>,
}

impl ResourceInterestLeaseInner {
    fn release(&self) -> bool {
        if self.released.swap(true, Ordering::AcqRel) {
            return false;
        }
        let Some(state) = self.state.upgrade() else {
            return false;
        };
        lock_state(&state).release_exact(&self.key, self.identity, self.generation)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ResourceInterestIdentity {
    owner: ResourceInterestOwnerId,
    interest: ResourceInterestId,
}

#[derive(Default)]
struct ResourceInterestState {
    runtime: Option<ResourceInterestRuntimeId>,
    closed: bool,
    next_lease_generation: u64,
    lease_ids_exhausted: bool,
    lease_count: usize,
    limits: ResourceInterestLimits,
    resources: HashMap<ResourceKey, ResourceInterestResource>,
}

#[derive(Default)]
struct ResourceInterestLimits {
    max_resources: usize,
    max_leases: usize,
    max_leases_per_resource: usize,
}

#[derive(Default)]
struct ResourceInterestResource {
    metadata: ResourceInterestMetadata,
    demand_generation: Option<u64>,
    leases: HashMap<ResourceInterestIdentity, ResourceInterestEntry>,
}

struct ResourceInterestEntry {
    generation: u64,
    class: ResourceInterestClass,
    owner_live: ResourceInterestLiveness,
    released: Arc<AtomicBool>,
    handle: Weak<ResourceInterestLeaseInner>,
}

impl ResourceInterestEntry {
    /// This deliberately avoids `Weak::upgrade`: an upgrade dropped while the
    /// ledger mutex is held could become the final lease handle, whose Drop
    /// re-enters the ledger to release it.
    fn is_live(&self) -> bool {
        self.owner_live.load(Ordering::Acquire)
            && !self.released.load(Ordering::Acquire)
            && self.handle.strong_count() > 0
    }
}

impl ResourceInterestState {
    fn allocate_lease_generation(&mut self) -> Result<u64, ResourceInterestAdmissionError> {
        if self.lease_ids_exhausted {
            return Err(ResourceInterestAdmissionError::LeaseIdExhausted);
        }
        let generation = self.next_lease_generation.max(1);
        match generation.checked_add(1) {
            Some(next) => self.next_lease_generation = next,
            None => self.lease_ids_exhausted = true,
        }
        Ok(generation)
    }

    fn live_handle_for(
        &self,
        key: &ResourceKey,
        identity: ResourceInterestIdentity,
    ) -> Option<Arc<ResourceInterestLeaseInner>> {
        let entry = self.resources.get(key)?.leases.get(&identity)?;
        if !entry.is_live() {
            return None;
        }
        // If a concurrent final Drop marked the handle released after the
        // liveness check, dropping this upgraded Arc is safe: its Drop sees
        // the shared release witness and never re-enters this mutex.
        let lease = entry.handle.upgrade()?;
        (!lease.released.load(Ordering::Acquire)).then_some(lease)
    }

    fn remove_nonlive_exact(&mut self, key: &ResourceKey, identity: ResourceInterestIdentity) {
        let remove = self
            .resources
            .get(key)
            .and_then(|resource| resource.leases.get(&identity))
            .is_some_and(|entry| !entry.is_live());
        if remove {
            let _ = self.release_exact_generation(key, identity);
        }
    }

    fn entry_mut_for(
        &mut self,
        lease: &ResourceInterestLeaseInner,
    ) -> Option<&mut ResourceInterestEntry> {
        if lease.released.load(Ordering::Acquire) {
            return None;
        }
        self.resources
            .get_mut(&lease.key)?
            .leases
            .get_mut(&lease.identity)
            .filter(|entry| entry.generation == lease.generation && entry.is_live())
    }

    fn class_for(&self, lease: &ResourceInterestLeaseInner) -> Option<ResourceInterestClass> {
        if lease.released.load(Ordering::Acquire) {
            return None;
        }
        self.resources
            .get(&lease.key)?
            .leases
            .get(&lease.identity)
            .filter(|entry| entry.generation == lease.generation && entry.is_live())
            .map(|entry| entry.class)
    }

    fn is_generation_live(
        &self,
        key: &ResourceKey,
        identity: ResourceInterestIdentity,
        generation: u64,
    ) -> bool {
        self.resources
            .get(key)
            .and_then(|resource| resource.leases.get(&identity))
            .is_some_and(|entry| entry.generation == generation && entry.is_live())
    }

    fn release_exact(
        &mut self,
        key: &ResourceKey,
        identity: ResourceInterestIdentity,
        generation: u64,
    ) -> bool {
        let matches_generation = self
            .resources
            .get(key)
            .and_then(|resource| resource.leases.get(&identity))
            .is_some_and(|entry| entry.generation == generation);
        matches_generation && self.release_exact_generation(key, identity).is_some()
    }

    fn release_exact_generation(
        &mut self,
        key: &ResourceKey,
        identity: ResourceInterestIdentity,
    ) -> Option<u64> {
        let removed = self
            .resources
            .get_mut(key)
            .and_then(|resource| resource.leases.remove(&identity))?;
        self.lease_count = self.lease_count.saturating_sub(1);
        self.clear_empty_demand(key);
        self.remove_empty_unretained(key);
        Some(removed.generation)
    }

    fn prune_dead(&mut self) {
        let keys = self.resources.keys().cloned().collect::<Vec<_>>();
        for key in keys {
            let Some(resource) = self.resources.get_mut(&key) else {
                continue;
            };
            let before = resource.leases.len();
            resource.leases.retain(|_, lease| lease.is_live());
            self.lease_count = self
                .lease_count
                .saturating_sub(before.saturating_sub(resource.leases.len()));
            self.clear_empty_demand(&key);
            self.remove_empty_unretained(&key);
        }
    }

    fn clear_empty_demand(&mut self, key: &ResourceKey) {
        let Some(resource) = self.resources.get_mut(key) else {
            return;
        };
        if resource.leases.is_empty() {
            resource.demand_generation = None;
        }
    }

    fn remove_empty_unretained(&mut self, key: &ResourceKey) {
        if self
            .resources
            .get(key)
            .is_some_and(|resource| resource.leases.is_empty() && !resource.metadata.keep_ready)
        {
            self.resources.remove(key);
        }
    }
}

fn lock_state(
    state: &Mutex<ResourceInterestState>,
) -> std::sync::MutexGuard<'_, ResourceInterestState> {
    state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
