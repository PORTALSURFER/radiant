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
