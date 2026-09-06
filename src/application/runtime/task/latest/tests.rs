use super::{LatestTask, TaskCompletion};

#[test]
fn latest_task_rejects_stale_tickets_after_newer_begin() {
    let mut task = LatestTask::new();
    let first = task.begin();
    let second = task.begin();

    assert!(!task.is_active(first));
    assert!(task.is_active(second));
    assert!(!task.finish(first));
    assert!(task.finish(second));
    assert_eq!(task.active(), None);
}

#[test]
fn latest_task_finish_completion_returns_only_current_output() {
    let mut task = LatestTask::new();
    let stale = task.begin();
    let current = task.begin();

    assert!(!task.is_active_completion(&TaskCompletion {
        ticket: stale,
        output: "stale",
    }));
    assert_eq!(
        task.finish_completion(TaskCompletion {
            ticket: stale,
            output: "stale"
        }),
        None
    );
    assert_eq!(
        task.finish_completion(TaskCompletion {
            ticket: current,
            output: "current"
        }),
        Some("current")
    );
    assert_eq!(task.active(), None);
}

#[test]
fn cloned_latest_tasks_have_isolated_effect_identities() {
    const EMPTY: LatestTask = LatestTask::new();
    assert_eq!(EMPTY.active(), None);

    let mut original = LatestTask::new();
    let mut first = original.clone();
    let mut second = original.clone();

    assert_eq!(original, first);
    assert_eq!(first, second);
    assert_ne!(first.effect_id(), second.effect_id());
    assert_ne!(first.effect_id(), original.effect_id());

    let first_ticket = first.begin();
    let second_ticket = second.begin();
    assert_eq!(first_ticket.id(), second_ticket.id());
    assert!(first.is_active(first_ticket));
    assert!(second.is_active(second_ticket));
    assert!(first.finish(first_ticket));
    assert!(second.is_active(second_ticket));
}

#[test]
fn timer_replacement_publishes_and_accepts_latest_ticket() {
    let mut task = LatestTask::new();
    let transaction = task.begin_timer_replacement();
    let ticket = transaction.replacement();

    assert_eq!(task.active(), Some(ticket));
    assert!(transaction.is_active());
    assert!(task.finish(ticket));
    assert!(!transaction.is_active());
}

#[test]
fn rejected_timer_replacements_restore_the_observable_chain() {
    let mut task = LatestTask::new();
    let first = task.begin_timer_replacement();
    let first_ticket = first.replacement();
    let second = task.begin_timer_replacement();
    let second_ticket = second.replacement();
    assert_eq!(task.active(), Some(second_ticket));

    second.reject();
    assert_eq!(task.active(), Some(first_ticket));
    assert!(first.is_active());

    let third = task.begin_timer_replacement();
    let third_ticket = third.replacement();
    let fourth = task.begin_timer_replacement();
    let fourth_ticket = fourth.replacement();
    assert_eq!(task.active(), Some(fourth_ticket));
    fourth.reject();
    assert_eq!(task.active(), Some(third_ticket));
    third.reject();
    assert_eq!(task.active(), Some(first_ticket));
}

#[test]
fn settlement_hook_runs_after_latest_state_and_cleanup_bounds_rejections() {
    use super::LatestTaskTransactionSettlement;
    use std::sync::{Arc, Mutex};

    let mut task = LatestTask::new();
    let settled_after_rejection = Arc::new(Mutex::new(None));
    for _ in 0..32 {
        let transaction = task.begin_replacement();
        let cancelled = transaction.cancellation_probe();
        let observed = Arc::clone(&settled_after_rejection);
        let transaction = transaction.with_settlement_hook(Arc::new(move |state| {
            if state == LatestTaskTransactionSettlement::Rejected {
                *observed.lock().unwrap() = Some(cancelled());
            }
        }));
        let ticket = transaction.replacement();
        transaction.reject();
        task.clear_resolved_replacement(ticket);
        assert_eq!(task.rollback_entry_count(), 0);
    }
    assert_eq!(*settled_after_rejection.lock().unwrap(), Some(true));
}
