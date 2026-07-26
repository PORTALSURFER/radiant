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
