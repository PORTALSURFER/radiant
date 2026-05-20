use super::*;

#[test]
fn scroll_commands_use_named_parts_for_reveal_requests() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let command = fs::read_to_string(manifest_dir.join("src/runtime/command.rs"))
        .expect("runtime command module should be readable");
    let command_repaint = fs::read_to_string(manifest_dir.join("src/runtime/command/repaint.rs"))
        .expect("runtime command repaint model should be readable");
    let command_scroll = fs::read_to_string(manifest_dir.join("src/runtime/command/scroll.rs"))
        .expect("runtime command scroll reveal models should be readable");
    let constructors = fs::read_to_string(manifest_dir.join("src/runtime/command/constructors.rs"))
        .expect("runtime command constructors should be readable");
    let scroll_constructors =
        fs::read_to_string(manifest_dir.join("src/runtime/command/constructors/scroll.rs"))
            .expect("runtime command scroll constructors should be readable");
    let update_context =
        fs::read_to_string(manifest_dir.join("src/application/runtime/update_context.rs"))
            .expect("application update context should be readable");
    let update_context_surface =
        fs::read_to_string(manifest_dir.join("src/application/runtime/update_context/surface.rs"))
            .expect("application update context surface helpers should be readable");
    let runtime =
        fs::read_to_string(manifest_dir.join("src/runtime/mod.rs")).expect("runtime module");
    let lib = fs::read_to_string(manifest_dir.join("src/lib.rs"))
        .expect("library module should be readable");

    assert!(
        command.contains("mod repaint;")
            && command.contains("mod scroll;")
            && command.contains("pub use repaint::RepaintScope;")
            && command
                .contains("pub use scroll::{ScrollFixedRowIntoViewParts, ScrollIntoViewParts};")
            && !command.contains("pub enum RepaintScope")
            && !command.contains("pub struct ScrollIntoViewParts")
            && command_repaint.contains("pub enum RepaintScope")
            && command_repaint.contains("pub const fn merge")
            && command_scroll.contains("pub struct ScrollIntoViewParts")
            && command_scroll.contains("pub struct ScrollFixedRowIntoViewParts"),
        "runtime command repaint and scroll reveal models should stay delegated while public exports remain stable"
    );
    assert!(
        constructors.contains("mod scroll;")
            && !constructors.contains(
                "pub const fn scroll_into_view_from_parts(parts: ScrollIntoViewParts) -> Self"
            )
            && scroll_constructors.contains(
                "pub const fn scroll_into_view_from_parts(parts: ScrollIntoViewParts) -> Self"
            )
            && scroll_constructors.contains("pub const fn scroll_fixed_row_into_view_from_parts")
            && scroll_constructors
                .contains("Self::scroll_into_view_from_parts(ScrollIntoViewParts {")
            && scroll_constructors.contains(
                "Self::scroll_fixed_row_into_view_from_parts(ScrollFixedRowIntoViewParts {"
            ),
        "scroll command constructors should stay in their focused module and delegate positional helpers through named request parts"
    );
    assert!(
        update_context.contains("mod surface;")
            && update_context_surface.contains(
                "pub fn scroll_into_view_from_parts(&mut self, parts: ScrollIntoViewParts)"
            )
            && update_context_surface.contains("pub fn scroll_fixed_row_into_view_from_parts")
            && runtime.contains("ScrollIntoViewParts")
            && runtime.contains("ScrollFixedRowIntoViewParts")
            && lib.contains("ScrollIntoViewParts")
            && lib.contains("ScrollFixedRowIntoViewParts"),
        "scroll reveal named request parts should be available through runtime and prelude paths"
    );
}

#[test]
fn update_context_keeps_followup_command_groups_in_focused_modules() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/application/runtime/update_context.rs"))
        .expect("application update context root should be readable");
    let commands =
        fs::read_to_string(manifest_dir.join("src/application/runtime/update_context/commands.rs"))
            .expect("application update context command helpers should be readable");
    let platform =
        fs::read_to_string(manifest_dir.join("src/application/runtime/update_context/platform.rs"))
            .expect("application update context platform helpers should be readable");
    let tasks =
        fs::read_to_string(manifest_dir.join("src/application/runtime/update_context/tasks.rs"))
            .expect("application update context task helpers should be readable");
    let surface =
        fs::read_to_string(manifest_dir.join("src/application/runtime/update_context/surface.rs"))
            .expect("application update context surface helpers should be readable");

    for required in [
        "mod commands;",
        "mod platform;",
        "mod surface;",
        "mod tasks;",
        "pub struct UpdateContext<Message>",
        "fn into_command(self) -> Command<Message>",
    ] {
        assert!(
            root.contains(required),
            "update context root should own the queue and delegate `{required}`"
        );
    }
    assert!(
        commands.contains("pub fn request_repaint")
            && commands.contains("pub fn request_paint_only")
            && commands.contains("pub fn repaint")
            && commands.contains("pub fn after")
            && commands.contains("pub fn exit"),
        "basic command and repaint helpers should live in update_context/commands.rs"
    );
    assert!(
        platform.contains("pub fn begin_external_drag")
            && platform.contains("pub fn platform_request")
            && platform.contains("pub fn pick_folder")
            && platform.contains("pub fn confirm"),
        "platform and external-drag helpers should live in update_context/platform.rs"
    );
    assert!(
        tasks.contains("pub fn spawn<Output>")
            && tasks.contains("pub fn spawn_cancellable")
            && tasks.contains("pub fn spawn_latest")
            && tasks.contains("pub fn spawn_resource"),
        "runtime task and resource helpers should live in update_context/tasks.rs"
    );
    assert!(
        surface.contains("pub fn focus")
            && surface.contains("pub fn scroll_to")
            && surface.contains("pub fn scroll_into_view_from_parts")
            && surface.contains("pub fn scroll_fixed_row_into_view_from_parts"),
        "focus and scroll helpers should live in update_context/surface.rs"
    );
}

#[test]
fn application_id_generation_keeps_policy_and_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ids = fs::read_to_string(manifest_dir.join("src/application/ids.rs"))
        .expect("application id generation module should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/application/ids/tests.rs"))
        .expect("application id generation tests should be readable");

    assert!(
        ids.contains("pub(in crate::application) struct IdGenerator")
            && ids.contains("enum ReservedIds")
            && ids.contains("fn reserved_id_range(reserved: &[NodeId])")
            && ids.contains("pub(in crate::application) fn scoped_key_id")
            && ids.contains("#[path = \"ids/tests.rs\"]")
            && !ids.contains("fn id_generator_skips_dense_reserved_runs_after_collision"),
        "application id allocation should live in application/ids.rs while behavior tests stay delegated"
    );
    assert!(
        tests.contains("fn id_generator_skips_dense_reserved_runs_after_collision")
            && tests.contains("fn id_generator_keeps_sorted_reserved_ids_for_small_sets")
            && tests.contains("fn id_generator_skips_probing_after_reserved_range_is_exhausted"),
        "application id generation behavior coverage should live in application/ids/tests.rs"
    );
}

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

#[test]
fn application_runtime_timer_lane_keeps_worker_policy_and_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let timer = fs::read_to_string(manifest_dir.join("src/application/runtime/timer.rs"))
        .expect("application runtime timer root should be readable");
    let lane = fs::read_to_string(manifest_dir.join("src/application/runtime/timer/lane.rs"))
        .expect("application runtime timer lane should be readable");
    let tests =
        fs::read_to_string(manifest_dir.join("src/application/runtime/timer/lane/tests.rs"))
            .expect("application runtime timer lane tests should be readable");

    assert!(
        timer.contains("mod lane;")
            && timer.contains("mod queue;")
            && timer.contains("mod worker;")
            && timer.contains("pub(super) use lane::TimerLane;"),
        "application runtime timer root should delegate lane, queue, and worker responsibilities"
    );
    assert!(
        lane.contains("pub(in crate::application::runtime) struct TimerLane<Message>")
            && lane.contains("pub(in crate::application::runtime) fn schedule(")
            && lane.contains("pub(in crate::application::runtime) fn schedule_interval(")
            && lane.contains("#[path = \"lane/tests.rs\"]")
            && !lane.contains("fn timer_lane_rejects_work_when_worker_is_unavailable"),
        "timer lane worker policy should live in timer/lane.rs while behavior tests stay delegated"
    );
    assert!(
        tests.contains("fn timer_lane_rejects_work_when_worker_is_unavailable"),
        "timer lane behavior coverage should live in timer/lane/tests.rs"
    );
}

#[test]
fn controller_commands_keep_outcome_drain_and_dispatch_in_focused_modules() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/runtime/controller/commands.rs"))
        .expect("runtime controller command root should be readable");
    let outcome =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/commands/outcome.rs"))
            .expect("runtime command outcome module should be readable");
    let drain = fs::read_to_string(manifest_dir.join("src/runtime/controller/commands/drain.rs"))
        .expect("runtime command drain module should be readable");
    let dispatch =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/commands/dispatch.rs"))
            .expect("runtime command dispatch module should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/runtime/controller/commands/tests.rs"))
        .expect("runtime command test root should be readable");
    let test_batching =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/commands/tests/batching.rs"))
            .expect("runtime command batching tests should be readable");
    let test_drain =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/commands/tests/drain.rs"))
            .expect("runtime command drain tests should be readable");
    let test_external_drag = fs::read_to_string(
        manifest_dir.join("src/runtime/controller/commands/tests/external_drag.rs"),
    )
    .expect("runtime command external drag tests should be readable");
    let test_platform =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/commands/tests/platform.rs"))
            .expect("runtime command platform tests should be readable");
    let test_fixtures =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/commands/tests/fixtures.rs"))
            .expect("runtime command test fixtures should be readable");

    for required in [
        "mod dispatch;",
        "mod drain;",
        "mod outcome;",
        "pub use outcome::CommandOutcome;",
    ] {
        assert!(
            root.contains(required),
            "runtime controller command root should delegate `{required}`"
        );
    }
    assert!(
        outcome.contains("pub struct CommandOutcome")
            && outcome.contains("fn finish_command_outcome")
            && !root.contains("pub struct CommandOutcome"),
        "command pass result and finalization should live in commands/outcome.rs"
    );
    assert!(
        drain.contains("pub fn drain_runtime_messages")
            && drain.contains("take_runtime_command_batch_into")
            && !root.contains("pub fn drain_runtime_messages"),
        "runtime work draining should live in commands/drain.rs"
    );
    assert!(
        dispatch.contains("fn execute_command_inner")
            && dispatch.contains("Command::PlatformRequest")
            && dispatch.contains("Command::ScrollFixedRowIntoView")
            && !root.contains("fn execute_command_inner"),
        "command execution branches should live in commands/dispatch.rs"
    );
    assert!(
        tests.contains("mod batching;")
            && tests.contains("mod drain;")
            && tests.contains("mod external_drag;")
            && tests.contains("mod platform;")
            && tests.contains("mod fixtures;")
            && !tests.contains("fn runtime_command_batch_preserves_order_and_keeps_remainder"),
        "runtime controller command test root should index focused behavior groups instead of owning all cases"
    );
    assert!(
        test_batching.contains("fn runtime_command_batch_preserves_order_and_keeps_remainder")
            && test_drain
                .contains("fn runtime_command_drains_are_bounded_and_request_followup_wakeup")
            && test_external_drag
                .contains("fn external_drag_command_arms_and_clears_native_session")
            && test_platform.contains("fn platform_request_dispatches_through_bridge_completion")
            && test_fixtures.contains("struct QueuedCommandBridge"),
        "runtime controller command tests should stay grouped by batching, drain, external drag, platform, and fixtures concerns"
    );
}
