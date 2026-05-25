use super::*;

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
fn app_bridge_groups_lifecycle_hooks_and_runtime_flags() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let bridge = fs::read_to_string(manifest_dir.join("src/application/runtime/bridge.rs"))
        .expect("application runtime bridge should be readable");
    let animation = fs::read_to_string(
        manifest_dir.join("src/application/runtime/bridge/adapter/animation.rs"),
    )
    .expect("application runtime bridge animation adapter should be readable");

    assert!(
        bridge.contains("pub(in crate::application) lifecycle: AppBridgeLifecycle<State, Message>")
            && bridge.contains("pub(in crate::application) runtime_flags: AppBridgeRuntimeFlags")
            && bridge.contains("pub(in crate::application) struct AppBridgeRuntimeFlags"),
        "app bridge should keep lifecycle hooks and mutable runtime flags in focused field groups"
    );
    assert!(
        bridge.contains("use super::{")
            && bridge.contains("AppAnimation")
            && bridge.contains("AppRuntime")
            && bridge.contains("AppStartup")
            && bridge.contains("RetainedPainter")
            && bridge.contains("TransientOverlayPainter")
            && bridge.contains("UpdateContext")
            && bridge.contains("use crate::{application::IntoView, runtime::Command};")
            && !bridge.contains("use super::*;"),
        "app bridge should name runtime lifecycle, queue, retained painter, update context, and command dependencies explicitly"
    );
    let bridge_fields = bridge
        .split("pub(in crate::application) struct AppBridgeRuntimeFlags")
        .next()
        .expect("app bridge source should include fields before runtime flags");
    for flattened_field in [
        "pub(in crate::application) animation:",
        "pub(in crate::application) frame_message:",
        "pub(in crate::application) subscriptions:",
        "pub(in crate::application) retained_painters:",
        "pub(in crate::application) startup_ran:",
    ] {
        assert!(
            !bridge_fields.contains(flattened_field),
            "app bridge should not flatten lifecycle or runtime flag field `{flattened_field}`"
        );
    }
    assert!(
        animation.contains(".lifecycle")
            && animation.contains(".animation")
            && animation.contains("self.lifecycle.frame_message")
            && animation.contains("self.runtime_flags.pending_animation_frame_activity"),
        "animation adapter should route through the grouped lifecycle and runtime flag state"
    );
}

#[test]
fn app_runtime_api_lifecycle_tests_stay_grouped_by_runtime_concern() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("tests/app_runtime_api.rs"))
        .expect("app runtime API test root should be readable");
    let lifecycle = fs::read_to_string(manifest_dir.join("tests/app_runtime_api/lifecycle.rs"))
        .expect("app runtime lifecycle test root should be readable");
    let startup = fs::read_to_string(
        manifest_dir.join("tests/app_runtime_api/lifecycle/startup_and_exit.rs"),
    )
    .expect("app runtime startup and exit tests should be readable");
    let animation =
        fs::read_to_string(manifest_dir.join("tests/app_runtime_api/lifecycle/animation.rs"))
            .expect("app runtime animation lifecycle tests should be readable");

    assert!(
        root.contains("#[path = \"app_runtime_api/lifecycle.rs\"]"),
        "app runtime API root should keep lifecycle coverage as a named module"
    );
    assert!(
        lifecycle.contains("#[path = \"lifecycle/startup_and_exit.rs\"]")
            && lifecycle.contains("#[path = \"lifecycle/animation.rs\"]")
            && lifecycle.contains("fn drain_until_messages<Bridge>")
            && !lifecycle.contains("fn app_startup_commands_use_full_runtime_dispatch")
            && !lifecycle
                .contains("fn active_animation_frame_messages_are_coalesced_until_drained"),
        "lifecycle test root should own shared fixtures and delegate startup/exit and animation behavior"
    );
    assert!(
        startup.contains("fn app_startup_commands_use_full_runtime_dispatch")
            && startup.contains("fn app_startup_runs_once_when_repaint_signal_is_reinstalled")
            && startup.contains("fn app_runtime_effects_stop_after_runtime_exit"),
        "startup and exit lifecycle coverage should stay in the startup_and_exit module"
    );
    assert!(
        animation.contains("fn active_animation_frame_messages_are_coalesced_until_drained")
            && animation.contains("fn animation_activity_poll_is_reused_for_frame_queue")
            && animation.contains("fn polling_animation_activity_does_not_queue_frame_messages"),
        "animation lifecycle coverage should stay in the animation module"
    );
}
