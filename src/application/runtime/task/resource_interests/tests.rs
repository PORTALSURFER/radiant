use super::*;

fn runtime() -> ResourceInterestRuntimeId {
    ResourceInterestRuntimeId::new(7)
}

fn key(name: &str) -> ResourceKey {
    ResourceKey::scoped("resource-interest-test", name)
}

fn live() -> ResourceInterestLiveness {
    Arc::new(AtomicBool::new(true))
}

fn admit(
    ledger: &ResourceInterestLedger,
    owner: u64,
    interest: u64,
    resource: ResourceKey,
    class: ResourceInterestClass,
    liveness: ResourceInterestLiveness,
) -> ResourceInterestLease {
    ledger
        .admit(
            runtime(),
            ResourceInterestOwnerId::new(owner),
            ResourceInterestId::new(interest),
            resource,
            class,
            liveness,
        )
        .expect("interest is admitted")
}

#[test]
fn multiple_owners_join_and_only_last_release_removes_resource() {
    let ledger = ResourceInterestLedger::new();
    let resource = key("joined");
    let first = admit(
        &ledger,
        1,
        1,
        resource.clone(),
        ResourceInterestClass::Visible,
        live(),
    );
    let second = admit(
        &ledger,
        2,
        1,
        resource.clone(),
        ResourceInterestClass::Prefetch,
        live(),
    );

    assert_eq!(ledger.live_count_for(&resource), 2);
    assert!(first.release());
    assert_eq!(ledger.live_count_for(&resource), 1);
    assert!(second.release());
    assert_eq!(ledger.live_count_for(&resource), 0);
    assert_eq!(ledger.retained_resource_count(), 0);
}

#[test]
fn class_transition_retains_exact_lease() {
    let ledger = ResourceInterestLedger::new();
    let resource = key("transition");
    let lease = admit(
        &ledger,
        1,
        9,
        resource.clone(),
        ResourceInterestClass::Visible,
        live(),
    );

    assert!(lease.set_class(ResourceInterestClass::Persistent));
    assert_eq!(lease.class(), Some(ResourceInterestClass::Persistent));
    assert_eq!(ledger.live_count_for(&resource), 1);
}

#[test]
fn duplicate_admission_and_clone_require_last_handle_drop() {
    let ledger = ResourceInterestLedger::new();
    let resource = key("clone");
    let first = admit(
        &ledger,
        1,
        1,
        resource.clone(),
        ResourceInterestClass::Visible,
        live(),
    );
    let duplicate = admit(
        &ledger,
        1,
        1,
        resource.clone(),
        ResourceInterestClass::Prefetch,
        live(),
    );
    let clone = first.clone();

    assert_eq!(duplicate.class(), Some(ResourceInterestClass::Visible));
    drop(first);
    drop(duplicate);
    assert_eq!(ledger.live_count_for(&resource), 1);
    drop(clone);
    assert_eq!(ledger.live_count_for(&resource), 0);
}

#[test]
fn weak_retirement_guard_does_not_extend_application_ownership() {
    let ledger = ResourceInterestLedger::new();
    let resource = key("weak-guard");
    let lease = admit(
        &ledger,
        1,
        1,
        resource.clone(),
        ResourceInterestClass::Visible,
        live(),
    );
    let guard = lease.downgrade();

    drop(lease);
    assert!(!guard.is_live());
    assert!(!guard.release());
    assert_eq!(ledger.live_count_for(&resource), 0);
}

#[test]
fn stale_release_cannot_remove_reinserted_interest() {
    let ledger = ResourceInterestLedger::new();
    let resource = key("stale");
    let old = admit(
        &ledger,
        1,
        1,
        resource.clone(),
        ResourceInterestClass::Visible,
        live(),
    );
    assert!(old.release());
    let replacement = admit(
        &ledger,
        1,
        1,
        resource.clone(),
        ResourceInterestClass::Prefetch,
        live(),
    );

    assert!(!old.release());
    assert_eq!(replacement.class(), Some(ResourceInterestClass::Prefetch));
    assert_eq!(ledger.live_count_for(&resource), 1);
}

#[test]
fn dead_owner_prune_uses_atomic_witness_and_retains_ready_metadata() {
    let ledger = ResourceInterestLedger::new();
    let resource = key("dead");
    ledger
        .set_metadata(
            resource.clone(),
            ResourceInterestMetadata { keep_ready: true },
        )
        .expect("metadata fits");
    let witness = live();
    let lease = admit(
        &ledger,
        1,
        1,
        resource.clone(),
        ResourceInterestClass::Visible,
        witness.clone(),
    );

    witness.store(false, Ordering::Release);
    ledger.prune_dead_owners();
    assert!(!lease.is_live());
    assert_eq!(ledger.live_lease_count(), 0);
    assert_eq!(ledger.retained_resource_count(), 1);
}

#[test]
fn capacities_are_independently_enforced_after_dead_pruning() {
    let ledger = ResourceInterestLedger::with_test_limits(1, 2, 1);
    let first_key = key("one");
    let first = admit(
        &ledger,
        1,
        1,
        first_key.clone(),
        ResourceInterestClass::Visible,
        live(),
    );
    assert!(matches!(
        ledger.admit(
            runtime(),
            ResourceInterestOwnerId::new(2),
            ResourceInterestId::new(1),
            first_key.clone(),
            ResourceInterestClass::Visible,
            live(),
        ),
        Err(ResourceInterestAdmissionError::PerResourceLeaseCapacity)
    ));
    assert!(matches!(
        ledger.admit(
            runtime(),
            ResourceInterestOwnerId::new(1),
            ResourceInterestId::new(2),
            key("two"),
            ResourceInterestClass::Visible,
            live(),
        ),
        Err(ResourceInterestAdmissionError::ResourceCapacity)
    ));
    drop(first);
    assert!(ledger.live_lease_count() == 0);
}

#[test]
fn total_lease_capacity_is_independent_of_resource_capacity() {
    let ledger = ResourceInterestLedger::with_test_limits(2, 1, 2);
    let resource = key("total-capacity");
    let _first = admit(
        &ledger,
        1,
        1,
        resource.clone(),
        ResourceInterestClass::Visible,
        live(),
    );

    assert!(matches!(
        ledger.admit(
            runtime(),
            ResourceInterestOwnerId::new(2),
            ResourceInterestId::new(1),
            resource,
            ResourceInterestClass::Prefetch,
            live(),
        ),
        Err(ResourceInterestAdmissionError::LeaseCapacity)
    ));
}

#[test]
fn runtime_is_bound_once_and_shutdown_invalidates_all_handles() {
    let ledger = ResourceInterestLedger::new();
    let resource = key("runtime");
    let lease = admit(
        &ledger,
        1,
        1,
        resource.clone(),
        ResourceInterestClass::Visible,
        live(),
    );

    assert!(matches!(
        ledger.admit(
            ResourceInterestRuntimeId::new(8),
            ResourceInterestOwnerId::new(2),
            ResourceInterestId::new(1),
            key("other"),
            ResourceInterestClass::Visible,
            live(),
        ),
        Err(ResourceInterestAdmissionError::RuntimeMismatch)
    ));
    ledger.shutdown();
    assert!(!lease.is_live());
    assert!(!lease.release());
    assert_eq!(ledger.live_lease_count(), 0);
    assert!(matches!(
        ledger.admit(
            runtime(),
            ResourceInterestOwnerId::new(2),
            ResourceInterestId::new(1),
            resource,
            ResourceInterestClass::Visible,
            live(),
        ),
        Err(ResourceInterestAdmissionError::Closed)
    ));
}

#[test]
fn retired_owner_is_never_admitted_or_reported_live() {
    let ledger = ResourceInterestLedger::new();
    let resource = key("retired-owner");
    let retired = Arc::new(AtomicBool::new(false));

    assert!(matches!(
        ledger.admit(
            runtime(),
            ResourceInterestOwnerId::new(1),
            ResourceInterestId::new(1),
            resource.clone(),
            ResourceInterestClass::Visible,
            retired,
        ),
        Err(ResourceInterestAdmissionError::OwnerRetired)
    ));
    assert!(!ledger.is_bound_to(runtime()));

    let witness = live();
    let lease = admit(
        &ledger,
        1,
        2,
        resource.clone(),
        ResourceInterestClass::Visible,
        witness.clone(),
    );
    witness.store(false, Ordering::Release);
    assert_eq!(lease.class(), None);
    assert!(!lease.is_live());
    assert!(!lease.set_class(ResourceInterestClass::Persistent));
    assert_eq!(ledger.live_count_for(&resource), 0);
}

#[test]
fn released_entry_is_removed_before_replacement_capacity_accounting() {
    let ledger = ResourceInterestLedger::with_test_limits(1, 1, 1);
    let resource = key("released-entry");
    let old = admit(
        &ledger,
        1,
        1,
        resource.clone(),
        ResourceInterestClass::Visible,
        live(),
    );
    // Model a final Drop that marked the lease released just before blocking
    // on the ledger mutex. The entry remains until admission cleans it up.
    old.inner.released.store(true, Ordering::Release);

    let replacement = admit(
        &ledger,
        1,
        1,
        resource.clone(),
        ResourceInterestClass::Prefetch,
        live(),
    );
    assert_eq!(ledger.live_lease_count(), 1);
    assert_eq!(replacement.class(), Some(ResourceInterestClass::Prefetch));
    assert!(!old.release());
    assert_eq!(replacement.class(), Some(ResourceInterestClass::Prefetch));
}

#[test]
fn lease_generation_exhaustion_fails_closed() {
    let ledger = ResourceInterestLedger::new();
    {
        let mut state = lock_state(&ledger.state);
        state.next_lease_generation = u64::MAX;
    }
    let first = admit(
        &ledger,
        1,
        1,
        key("last-generation"),
        ResourceInterestClass::Visible,
        live(),
    );
    assert!(first.is_live());
    drop(first);

    assert!(matches!(
        ledger.admit(
            runtime(),
            ResourceInterestOwnerId::new(1),
            ResourceInterestId::new(2),
            key("exhausted"),
            ResourceInterestClass::Visible,
            live(),
        ),
        Err(ResourceInterestAdmissionError::LeaseIdExhausted)
    ));
}

#[test]
fn capacity_rejection_does_not_bind_a_first_runtime() {
    let ledger = ResourceInterestLedger::with_test_limits(1, 1, 1);
    ledger
        .set_metadata(
            key("reserved-key"),
            ResourceInterestMetadata { keep_ready: true },
        )
        .expect("metadata consumes the sole key slot");

    assert!(matches!(
        ledger.admit(
            runtime(),
            ResourceInterestOwnerId::new(1),
            ResourceInterestId::new(1),
            key("blocked-admission"),
            ResourceInterestClass::Visible,
            live(),
        ),
        Err(ResourceInterestAdmissionError::ResourceCapacity)
    ));
    assert!(!ledger.is_bound_to(runtime()));
}

#[test]
fn demand_generation_stays_stable_until_the_final_owner_releases() {
    let ledger = ResourceInterestLedger::new();
    let resource = key("shared-demand");
    let first = admit(
        &ledger,
        1,
        1,
        resource.clone(),
        ResourceInterestClass::Visible,
        live(),
    );
    let generation = ledger.demand_generation(&resource).expect("first demand");
    let second = admit(
        &ledger,
        2,
        1,
        resource.clone(),
        ResourceInterestClass::Prefetch,
        live(),
    );

    assert_eq!(ledger.demand_generation(&resource), Some(generation));
    drop(first);
    assert_eq!(ledger.demand_generation(&resource), Some(generation));
    drop(second);
    assert_eq!(ledger.demand_generation(&resource), None);
}

#[test]
fn retained_ready_key_gets_a_new_demand_generation_after_reacquisition() {
    let ledger = ResourceInterestLedger::new();
    let resource = key("retained-demand");
    ledger
        .set_metadata(
            resource.clone(),
            ResourceInterestMetadata { keep_ready: true },
        )
        .expect("metadata fits");
    let first = admit(
        &ledger,
        1,
        1,
        resource.clone(),
        ResourceInterestClass::Visible,
        live(),
    );
    let old_generation = ledger.demand_generation(&resource).expect("first demand");
    drop(first);
    assert_eq!(ledger.retained_resource_count(), 1);
    assert_eq!(ledger.demand_generation(&resource), None);

    let second = admit(
        &ledger,
        1,
        1,
        resource.clone(),
        ResourceInterestClass::Persistent,
        live(),
    );
    let new_generation = ledger
        .demand_generation(&resource)
        .expect("reacquired demand");
    assert_ne!(new_generation, old_generation);
    assert!(second.is_live());
}

#[test]
fn liveness_pruning_never_upgrades_an_orphaned_handle_while_locked() {
    let ledger = ResourceInterestLedger::new();
    let resource = key("orphaned-handle");
    let lease = admit(
        &ledger,
        1,
        1,
        resource.clone(),
        ResourceInterestClass::Visible,
        live(),
    );
    // Simulate a final Drop that has marked itself released before it can
    // acquire the ledger lock. Its later Drop must not re-enter from a
    // liveness predicate, and the weak entry must be pruned.
    lease.inner.released.store(true, Ordering::Release);
    drop(lease);

    assert_eq!(ledger.live_count_for(&resource), 0);
    assert_eq!(ledger.live_lease_count(), 0);
}
