use super::*;
use std::sync::{Arc, atomic::AtomicBool};

fn key(name: &str) -> ResourceKey {
    ResourceKey::scoped("resource-operation-view", name)
}

fn admit(tasks: &SharedResourceTasks, resource: ResourceKey) -> ResourceInterest {
    tasks
        .admit_interest(
            1,
            1,
            1,
            resource,
            ResourceInterestKind::Visible,
            Arc::new(AtomicBool::new(true)),
        )
        .unwrap()
}

fn start(
    tasks: &SharedResourceTasks,
    resource: ResourceKey,
) -> super::super::resource_operations::ResourceOperationReservation {
    match tasks
        .operations
        .reserve(
            resource,
            super::super::resource_operations::ResourceOperationReplaceMode::Join,
        )
        .unwrap()
    {
        super::super::resource_operations::ResourceOperationReserve::Reserved(value) => value,
        _ => panic!("live demand starts one operation"),
    }
}

#[test]
fn observing_an_absent_operation_never_starts_work() {
    let tasks = SharedResourceTasks::new();
    let resource = key("observer");
    let _interest = admit(&tasks, resource.clone());

    assert!(tasks.operation(&resource).is_none());
    assert_eq!(tasks.operations.slot_count(), 0);
}

#[test]
fn snapshots_reject_foreign_broker_completions_with_same_numeric_identity() {
    let resource = key("foreign");
    let first = SharedResourceTasks::new();
    let second = SharedResourceTasks::new();
    let _first_interest = admit(&first, resource.clone());
    let _second_interest = admit(&second, resource.clone());
    let first_reservation = start(&first, resource.clone());
    let second_reservation = start(&second, resource.clone());
    first_reservation.transaction().accept();
    second_reservation.transaction().accept();
    let snapshot = first.operation(&resource).unwrap();
    let foreign = SharedResourceCompletion {
        output: (),
        current: second_reservation.current(),
    };

    assert_eq!(snapshot.operation_id(), foreign.operation_id());
    assert!(snapshot.same_operation(&snapshot.clone()));
    assert!(!snapshot.matches_completion(&foreign));
}

#[test]
fn snapshots_invalidate_after_completion_cancellation_and_final_interest_loss() {
    let tasks = SharedResourceTasks::new();
    let resource = key("invalidation");
    let interest = admit(&tasks, resource.clone());

    let completed = start(&tasks, resource.clone());
    completed.transaction().accept();
    let completion_snapshot = tasks.operation(&resource).unwrap();
    let completion = SharedResourceCompletion {
        output: (),
        current: completed.current(),
    };
    assert!(completion_snapshot.matches_completion(&completion));
    assert_eq!(tasks.finish_ready(completion), Some(()));
    assert!(!completion_snapshot.is_current());
    assert!(tasks.operation(&resource).is_none());

    // Joining ready state intentionally starts nothing; clear it explicitly.
    assert!(tasks.cancel(&resource));
    let cancelled = start(&tasks, resource.clone());
    cancelled.transaction().accept();
    let cancelled_snapshot = tasks.operation(&resource).unwrap();
    assert!(tasks.cancel(&resource));
    assert!(!cancelled_snapshot.is_current());

    let dropped = start(&tasks, resource.clone());
    dropped.transaction().accept();
    let dropped_snapshot = tasks.operation(&resource).unwrap();
    interest.release();
    assert!(!dropped_snapshot.is_current());
    assert!(tasks.operation(&resource).is_none());
}

#[test]
fn stale_snapshot_cannot_cancel_a_newer_replacement() {
    let tasks = SharedResourceTasks::new();
    let resource = key("stale-cancel");
    let _interest = admit(&tasks, resource.clone());
    let first = start(&tasks, resource.clone());
    first.transaction().accept();
    let stale = tasks.operation(&resource).unwrap();

    let replacement = match tasks
        .operations
        .reserve(
            resource.clone(),
            super::super::resource_operations::ResourceOperationReplaceMode::Replace,
        )
        .unwrap()
    {
        super::super::resource_operations::ResourceOperationReserve::Reserved(value) => value,
        _ => panic!("replacement reserves current work"),
    };
    replacement.transaction().accept();
    let current = tasks.operation(&resource).unwrap();

    assert!(!tasks.cancel_operation(&stale));
    assert!(current.is_current());
    assert!(tasks.cancel_operation(&current));
}

#[test]
fn rejected_snapshot_keeps_its_settlement_evidence_across_a_later_replacement() {
    let tasks = SharedResourceTasks::new();
    let resource = key("rejected-snapshot");
    let interest = admit(&tasks, resource.clone());
    let first = start(&tasks, resource.clone());
    first.transaction().accept();

    let rejected_reservation = match tasks
        .operations
        .reserve(
            resource.clone(),
            super::super::resource_operations::ResourceOperationReplaceMode::Replace,
        )
        .unwrap()
    {
        super::super::resource_operations::ResourceOperationReserve::Reserved(value) => value,
        _ => panic!("replacement reserves current work"),
    };
    let rejected = tasks.operation(&resource).unwrap();
    rejected_reservation.transaction().reject();
    assert!(rejected.was_rejected());

    let replacement = match tasks
        .operations
        .reserve(
            resource.clone(),
            super::super::resource_operations::ResourceOperationReplaceMode::Replace,
        )
        .unwrap()
    {
        super::super::resource_operations::ResourceOperationReserve::Reserved(value) => value,
        _ => panic!("later replacement reserves restored predecessor"),
    };
    replacement.transaction().accept();
    assert!(rejected.was_rejected());
    assert!(!rejected.is_current());

    interest.release();
    drop(tasks);
    assert!(rejected.was_rejected());
    assert!(!rejected.is_current());
}
