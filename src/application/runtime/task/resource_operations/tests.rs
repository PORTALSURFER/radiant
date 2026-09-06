use super::super::resource_interests::{
    ResourceInterestClass, ResourceInterestId, ResourceInterestLedger, ResourceInterestOwnerId,
    ResourceInterestRuntimeId,
};
use super::*;
use std::sync::{atomic::AtomicBool, Arc};

fn key(name: &str) -> ResourceKey {
    ResourceKey::scoped("operation-test", name)
}
fn live() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(true))
}
fn admit(
    ledger: &ResourceInterestLedger,
    owner: u64,
    resource: ResourceKey,
) -> super::super::resource_interests::ResourceInterestLease {
    ledger
        .admit(
            ResourceInterestRuntimeId::new(1),
            ResourceInterestOwnerId::new(owner),
            ResourceInterestId::new(1),
            resource,
            ResourceInterestClass::Visible,
            live(),
        )
        .unwrap()
}
fn reserved(
    registry: &ResourceOperationRegistry,
    resource: ResourceKey,
) -> ResourceOperationReservation {
    match registry
        .reserve(resource, ResourceOperationReplaceMode::Join)
        .unwrap()
    {
        ResourceOperationReserve::Reserved(value) => value,
        _ => panic!("new demand reserves work"),
    }
}

#[test]
fn multiowner_survives_one_release_then_final_release_and_reacquire_fences_old_work() {
    let ledger = ResourceInterestLedger::new();
    let registry = ResourceOperationRegistry::with_ledger(ledger.clone());
    let resource = key("owners");
    let first = admit(&ledger, 1, resource.clone());
    let second = admit(&ledger, 2, resource.clone());
    let operation = reserved(&registry, resource.clone());
    let probe = operation.currentness_probe();
    operation.transaction().accept();
    assert!(probe());
    first.release();
    assert!(probe(), "a surviving owner retains current work");
    second.release();
    assert!(!probe(), "final release invalidates in-flight work");
    let replacement = admit(&ledger, 3, resource.clone());
    let next = reserved(&registry, resource.clone());
    assert_ne!(
        operation.current().demand_generation(),
        next.current().demand_generation()
    );
    assert!(!operation.currentness_probe()());
    replacement.release();
}

#[test]
fn foreign_broker_cannot_finish_a_matching_current() {
    let ledger = ResourceInterestLedger::new();
    let first = ResourceOperationRegistry::with_ledger(ledger.clone());
    let second = ResourceOperationRegistry::with_ledger(ledger.clone());
    let resource = key("foreign");
    let _lease = admit(&ledger, 1, resource.clone());
    let a = reserved(&first, resource.clone());
    let b = reserved(&second, resource);
    a.transaction().accept();
    b.transaction().accept();
    assert!(!second.finish_ready(a.current()));
    assert!(first.finish_ready(a.current()));
}

#[test]
fn only_one_unsettled_replacement_is_admitted_and_rejection_restores_its_predecessor() {
    let ledger = ResourceInterestLedger::new();
    let registry = ResourceOperationRegistry::with_ledger(ledger.clone());
    let resource = key("replacement");
    let _lease = admit(&ledger, 1, resource.clone());
    let first = reserved(&registry, resource.clone());
    first.transaction().accept();
    let first_probe = first.currentness_probe();
    let replacement = match registry
        .reserve(resource.clone(), ResourceOperationReplaceMode::Replace)
        .unwrap()
    {
        ResourceOperationReserve::Reserved(value) => value,
        _ => unreachable!(),
    };
    assert!(matches!(
        registry.reserve(resource.clone(), ResourceOperationReplaceMode::Join),
        Ok(ResourceOperationReserve::Joined)
    ));
    assert!(matches!(
        registry.reserve(resource.clone(), ResourceOperationReplaceMode::Replace),
        Err(ResourceOperationAdmissionError::PendingAdmission)
    ));
    assert!(!first_probe());
    drop(replacement);
    assert!(first_probe(), "only the exact predecessor is restored");
    assert_eq!(registry.rollback_entry_count(&resource), 0);
}

#[test]
fn sequential_rejected_replacements_do_not_accumulate_latest_rollback_history() {
    let ledger = ResourceInterestLedger::new();
    let registry = ResourceOperationRegistry::with_ledger(ledger.clone());
    let resource = key("history");
    let _lease = admit(&ledger, 1, resource.clone());
    let first = reserved(&registry, resource.clone());
    first.transaction().accept();
    for _ in 0..32 {
        let replacement = match registry
            .reserve(resource.clone(), ResourceOperationReplaceMode::Replace)
            .unwrap()
        {
            ResourceOperationReserve::Reserved(value) => value,
            _ => unreachable!(),
        };
        drop(replacement);
        assert_eq!(registry.rollback_entry_count(&resource), 0);
    }
}

#[test]
fn ready_retention_counts_against_ledger_capacity_and_reacquire_reserves_fresh_work() {
    let ledger = ResourceInterestLedger::with_test_limits(1, 4, 2);
    let registry = ResourceOperationRegistry::with_ledger(ledger.clone());
    let resource = key("ready");
    let lease = admit(&ledger, 1, resource.clone());
    registry.set_keep_ready(resource.clone(), true).unwrap();
    let first = reserved(&registry, resource.clone());
    first.transaction().accept();
    assert!(registry.finish_ready(first.current()));
    lease.release();
    assert_eq!(registry.slot_count(), 1);
    assert!(matches!(
        registry.set_keep_ready(key("other"), true),
        Err(ResourceOperationAdmissionError::Interest(
            super::super::resource_interests::ResourceInterestAdmissionError::ResourceCapacity
        ))
    ));
    let _next_lease = admit(&ledger, 2, resource.clone());
    let previous_fence = registry.slot_fence(&resource).unwrap();
    assert!(matches!(
        registry.reserve(resource.clone(), ResourceOperationReplaceMode::Join),
        Ok(ResourceOperationReserve::Ready)
    ));
    let next_fence = registry.slot_fence(&resource).unwrap();
    assert_ne!(previous_fence, next_fence);
    assert_ne!(first.current().demand_generation(), next_fence.1);
    assert!(!registry.finish_ready(first.current()));
}

#[test]
fn retry_is_consumed_once_and_join_does_not_bypass_backoff() {
    let ledger = ResourceInterestLedger::new();
    let registry = ResourceOperationRegistry::with_ledger(ledger.clone());
    let resource = key("retry");
    let _lease = admit(&ledger, 1, resource.clone());
    let first = reserved(&registry, resource.clone());
    first.transaction().accept();
    assert!(registry.schedule_retry(first.current(), 10));
    assert_eq!(registry.backoff_deadline(&resource), Some(10));
    assert!(matches!(
        registry.reserve(resource.clone(), ResourceOperationReplaceMode::Join),
        Ok(ResourceOperationReserve::Backoff)
    ));
    assert!(registry.take_retry(&resource, 9).is_none());
    let retry = match registry.take_retry(&resource, 10) {
        Some(ResourceOperationReserve::Reserved(value)) => value,
        _ => panic!("one due retry reserves work"),
    };
    assert!(registry.take_retry(&resource, 10).is_none());
    retry.transaction().accept();
    assert!(registry.finish_idle(retry.current()));
}

#[test]
fn capacity_stale_retry_and_shutdown_fail_closed() {
    let ledger = ResourceInterestLedger::with_test_limits(2, 4, 2);
    let registry = ResourceOperationRegistry::with_test_limit(ledger.clone(), 1);
    let one = key("one");
    let two = key("two");
    let _one = admit(&ledger, 1, one.clone());
    let operation = reserved(&registry, one.clone());
    operation.transaction().accept();
    assert!(registry.schedule_retry(operation.current(), 2));
    let _two = admit(&ledger, 2, two.clone());
    assert!(matches!(
        registry.reserve(two, ResourceOperationReplaceMode::Join),
        Err(ResourceOperationAdmissionError::Capacity)
    ));
    let retry = match registry.take_retry(&one, 2) {
        Some(ResourceOperationReserve::Reserved(value)) => value,
        _ => panic!("due retry is reserved"),
    };
    retry.transaction().accept();
    assert!(registry.schedule_retry(retry.current(), 3));
    let newer = match registry
        .reserve(one.clone(), ResourceOperationReplaceMode::Replace)
        .unwrap()
    {
        ResourceOperationReserve::Reserved(value) => value,
        _ => unreachable!(),
    };
    assert!(registry.take_retry(&one, 3).is_none());
    newer.transaction().accept();
    registry.shutdown();
    assert_eq!(registry.slot_count(), 0);
    assert!(matches!(
        registry.reserve(one, ResourceOperationReplaceMode::Join),
        Err(ResourceOperationAdmissionError::Closed)
    ));
}

#[test]
fn cancelled_accepted_work_is_stale_and_join_reserves_a_retry() {
    use crate::application::CancellationToken;

    let ledger = ResourceInterestLedger::new();
    let registry = ResourceOperationRegistry::with_ledger(ledger.clone());
    let resource = key("cancelled-accepted");
    let _lease = admit(&ledger, 1, resource.clone());
    let first = reserved(&registry, resource.clone());
    let probe = first.currentness_probe();
    let token = CancellationToken::new();
    first.attach_cancellation(token.clone());
    first.transaction().accept();
    token.cancel();
    assert!(!probe(), "the attached token fences delivery immediately");
    assert!(!registry.finish_ready(first.current()));
    assert!(matches!(
        registry.reserve(resource, ResourceOperationReplaceMode::Join),
        Ok(ResourceOperationReserve::Reserved(_))
    ));
}

#[test]
fn cancelled_unsettled_work_stays_pending_until_rejection_then_retries() {
    use crate::application::CancellationToken;

    let ledger = ResourceInterestLedger::new();
    let registry = ResourceOperationRegistry::with_ledger(ledger.clone());
    let resource = key("cancelled-pending");
    let _lease = admit(&ledger, 1, resource.clone());
    let pending = reserved(&registry, resource.clone());
    let probe = pending.currentness_probe();
    let token = CancellationToken::new();
    pending.attach_cancellation(token.clone());
    token.cancel();
    assert!(!probe());
    assert!(
        matches!(
            registry.reserve(resource.clone(), ResourceOperationReplaceMode::Join),
            Ok(ResourceOperationReserve::Joined)
        ),
        "a cancelled unsettled transaction still owns the one pending slot"
    );
    drop(pending);
    assert_eq!(registry.rollback_entry_count(&resource), 0);
    assert!(matches!(
        registry.reserve(resource, ResourceOperationReplaceMode::Join),
        Ok(ResourceOperationReserve::Reserved(_))
    ));
}

#[test]
fn explicit_cancel_invalidates_settled_work_without_opening_a_second_pending_slot() {
    let ledger = ResourceInterestLedger::new();
    let registry = ResourceOperationRegistry::with_ledger(ledger.clone());
    let resource = key("explicit-cancel");
    let _lease = admit(&ledger, 1, resource.clone());
    let operation = reserved(&registry, resource.clone());
    let probe = operation.currentness_probe();
    operation.transaction().accept();
    assert!(registry.cancel(&resource));
    assert!(!probe());
    assert!(!registry.finish_ready(operation.current()));
    assert!(matches!(
        registry.reserve(resource, ResourceOperationReplaceMode::Join),
        Ok(ResourceOperationReserve::Reserved(_))
    ));
}

#[test]
fn replacement_token_cancellation_restores_an_uncancelled_predecessor() {
    use crate::application::CancellationToken;

    let ledger = ResourceInterestLedger::new();
    let registry = ResourceOperationRegistry::with_ledger(ledger.clone());
    let resource = key("replacement-token");
    let _lease = admit(&ledger, 1, resource.clone());
    let first = reserved(&registry, resource.clone());
    let predecessor_probe = first.currentness_probe();
    first.transaction().accept();
    let replacement = match registry
        .reserve(resource, ResourceOperationReplaceMode::Replace)
        .unwrap()
    {
        ResourceOperationReserve::Reserved(value) => value,
        _ => unreachable!(),
    };
    let replacement_token = CancellationToken::new();
    replacement.attach_cancellation(replacement_token.clone());
    replacement_token.cancel();
    drop(replacement);
    assert!(
        predecessor_probe(),
        "replacement token cancellation does not cancel its predecessor"
    );
}

#[test]
fn cancelled_predecessor_or_explicit_key_cancel_cannot_revive_on_rejection() {
    use crate::application::CancellationToken;

    let ledger = ResourceInterestLedger::new();
    let registry = ResourceOperationRegistry::with_ledger(ledger.clone());
    let resource = key("rollback-cancel");
    let _lease = admit(&ledger, 1, resource.clone());
    let first = reserved(&registry, resource.clone());
    let predecessor_probe = first.currentness_probe();
    let predecessor_token = CancellationToken::new();
    first.attach_cancellation(predecessor_token.clone());
    first.transaction().accept();
    let replacement = match registry
        .reserve(resource.clone(), ResourceOperationReplaceMode::Replace)
        .unwrap()
    {
        ResourceOperationReserve::Reserved(value) => value,
        _ => unreachable!(),
    };
    predecessor_token.cancel();
    drop(replacement);
    assert!(!predecessor_probe());
    assert!(matches!(
        registry.reserve(resource.clone(), ResourceOperationReplaceMode::Join),
        Ok(ResourceOperationReserve::Reserved(_))
    ));

    let next = reserved(&registry, resource.clone());
    let next_probe = next.currentness_probe();
    next.transaction().accept();
    let replacement = match registry
        .reserve(resource.clone(), ResourceOperationReplaceMode::Replace)
        .unwrap()
    {
        ResourceOperationReserve::Reserved(value) => value,
        _ => unreachable!(),
    };
    assert!(registry.cancel(&resource));
    drop(replacement);
    assert!(
        !next_probe(),
        "explicit key cancellation also cancels rollback state"
    );
    assert!(matches!(
        registry.reserve(resource, ResourceOperationReplaceMode::Join),
        Ok(ResourceOperationReserve::Reserved(_))
    ));
}

#[test]
fn late_completed_token_cancellation_preserves_ready_and_due_retry_state() {
    use crate::application::CancellationToken;

    let ledger = ResourceInterestLedger::new();
    let registry = ResourceOperationRegistry::with_ledger(ledger.clone());
    let ready_key = key("late-ready-token");
    let retry_key = key("late-retry-token");
    let _ready_lease = admit(&ledger, 1, ready_key.clone());
    let _retry_lease = admit(&ledger, 2, retry_key.clone());

    let ready = reserved(&registry, ready_key.clone());
    let ready_token = CancellationToken::new();
    ready.attach_cancellation(ready_token.clone());
    ready.transaction().accept();
    assert!(registry.finish_ready(ready.current()));
    ready_token.cancel();
    assert!(matches!(
        registry.reserve(ready_key, ResourceOperationReplaceMode::Join),
        Ok(ResourceOperationReserve::Ready)
    ));

    let retry = reserved(&registry, retry_key.clone());
    let retry_token = CancellationToken::new();
    retry.attach_cancellation(retry_token.clone());
    retry.transaction().accept();
    assert!(registry.schedule_retry(retry.current(), 4));
    retry_token.cancel();
    let due = match registry.take_retry(&retry_key, 4) {
        Some(ResourceOperationReserve::Reserved(value)) => value,
        _ => panic!("late cancellation must not consume the due retry"),
    };
    assert!(due.currentness_probe()());
}
