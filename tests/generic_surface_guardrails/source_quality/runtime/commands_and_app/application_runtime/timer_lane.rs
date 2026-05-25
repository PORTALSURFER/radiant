use super::*;

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
