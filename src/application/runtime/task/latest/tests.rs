use super::LatestTask;

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
