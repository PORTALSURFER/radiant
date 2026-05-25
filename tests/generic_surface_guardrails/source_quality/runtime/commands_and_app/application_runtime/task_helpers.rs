use super::*;

#[test]
fn application_task_helpers_keep_cancellation_completion_and_latest_state_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/application/runtime/task.rs"))
        .expect("application runtime task root should be readable");
    let cancellation =
        fs::read_to_string(manifest_dir.join("src/application/runtime/task/cancellation.rs"))
            .expect("application runtime cancellation token module should be readable");
    let completion =
        fs::read_to_string(manifest_dir.join("src/application/runtime/task/completion.rs"))
            .expect("application runtime task completion module should be readable");
    let latest = fs::read_to_string(manifest_dir.join("src/application/runtime/task/latest.rs"))
        .expect("application runtime latest task module should be readable");
    let keyed_latest =
        fs::read_to_string(manifest_dir.join("src/application/runtime/task/keyed_latest.rs"))
            .expect("application runtime keyed latest task module should be readable");
    let runtime = fs::read_to_string(manifest_dir.join("src/application/runtime.rs"))
        .expect("application runtime module should be readable");

    for required in [
        "mod cancellation;",
        "mod completion;",
        "mod keyed_latest;",
        "mod latest;",
        "pub use cancellation::CancellationToken;",
        "pub use completion::{KeyedTaskCompletion, TaskCompletion, TaskTicket};",
        "pub use keyed_latest::KeyedLatestTasks;",
        "pub use latest::LatestTask;",
    ] {
        assert!(
            root.contains(required),
            "application runtime task root should delegate `{required}`"
        );
    }
    assert!(
        !root.contains("pub struct CancellationToken")
            && !root.contains("pub struct TaskCompletion")
            && !root.contains("pub struct LatestTask")
            && !root.contains("pub struct KeyedLatestTasks"),
        "application runtime task root should re-export task helpers without owning implementation"
    );
    assert!(
        cancellation.contains("pub struct CancellationToken")
            && cancellation.contains("pub fn cancel(&self)")
            && cancellation.contains("pub fn is_cancelled(&self) -> bool")
            && cancellation.contains("#[path = \"cancellation/tests.rs\"]")
            && !cancellation.contains("fn cancellation_token_is_shared_across_clones"),
        "task cancellation token should live in application/runtime/task/cancellation.rs while behavior tests stay delegated"
    );
    assert!(
        completion.contains("pub struct TaskTicket")
            && completion.contains("pub struct TaskCompletion<Output>")
            && completion.contains("pub struct KeyedTaskCompletion<Key, Output>"),
        "task tickets and completion DTOs should live in application/runtime/task/completion.rs"
    );
    assert!(
        latest.contains("pub struct LatestTask")
            && latest.contains("pub fn begin(&mut self) -> TaskTicket")
            && latest.contains("pub fn finish(&mut self, ticket: TaskTicket) -> bool")
            && latest.contains("#[path = \"latest/tests.rs\"]")
            && !latest.contains("fn latest_task_rejects_stale_tickets_after_newer_begin"),
        "single-resource latest task state should live in application/runtime/task/latest.rs while behavior tests stay delegated"
    );
    assert!(
        keyed_latest.contains("pub struct KeyedLatestTasks<Key>")
            && keyed_latest.contains("pub fn begin(&mut self, key: Key) -> TaskTicket")
            && keyed_latest.contains("pub fn remove(&mut self, key: &Key) -> Option<LatestTask>")
            && keyed_latest.contains("#[path = \"keyed_latest/tests.rs\"]")
            && !keyed_latest.contains("fn keyed_latest_tasks_reject_stale_tickets_per_key"),
        "keyed latest task registry should live in application/runtime/task/keyed_latest.rs while behavior tests stay delegated"
    );
    assert!(
        runtime.contains("CancellationToken")
            && runtime.contains("KeyedLatestTasks")
            && runtime.contains("TaskCompletion")
            && runtime.contains("TaskTicket"),
        "application runtime facade should keep task helpers available through the public runtime path"
    );
}
