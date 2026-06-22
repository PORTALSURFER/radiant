use super::{KeyedLatestTasks, KeyedTaskCompletion};

#[test]
fn keyed_latest_tasks_reject_stale_tickets_per_key() {
    let mut tasks = KeyedLatestTasks::new();

    let first_a = tasks.begin("a");
    let current_a = tasks.begin("a");
    let only_b = tasks.begin("b");

    assert!(!tasks.is_active(&"a", first_a));
    assert!(tasks.is_active(&"a", current_a));
    assert!(tasks.is_active(&"b", only_b));

    assert!(!tasks.finish(&"a", first_a));
    assert!(tasks.finish(&"a", current_a));
    assert!(tasks.finish(&"b", only_b));
    assert_eq!(tasks.active(&"a"), None);
    assert_eq!(tasks.active(&"b"), None);
}

#[test]
fn keyed_latest_tasks_can_cancel_and_remove_keys() {
    let mut tasks = KeyedLatestTasks::new();

    let ticket = tasks.begin(String::from("preview"));
    assert_eq!(tasks.len(), 1);
    assert!(tasks.cancel(&String::from("preview")));
    assert!(!tasks.is_active(&String::from("preview"), ticket));
    assert!(tasks.remove(&String::from("preview")).is_some());
    assert!(tasks.is_empty());
}

#[test]
fn keyed_latest_tasks_finish_completion_returns_only_current_output() {
    let mut tasks = KeyedLatestTasks::new();

    let stale = tasks.begin("a");
    let current = tasks.begin("a");

    let stale_completion = KeyedTaskCompletion {
        key: "a",
        ticket: stale,
        output: "stale",
    };
    assert!(!tasks.is_active_completion(&stale_completion));
    assert_eq!(tasks.finish_completion(stale_completion), None);

    assert_eq!(
        tasks.finish_completion(KeyedTaskCompletion {
            key: "a",
            ticket: current,
            output: "current",
        }),
        Some("current")
    );
    assert_eq!(tasks.active(&"a"), None);
}
