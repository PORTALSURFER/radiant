use super::TimerLane;
use crate::application::runtime::{AppRuntime, timer::timer_sink};
use std::sync::Arc;

#[test]
fn timer_lane_rejects_work_when_worker_is_unavailable() {
    let lane = TimerLane::without_worker_for_test();
    let runtime = Arc::new(AppRuntime::<u32>::default());
    let identity = runtime.allocate_timer_identity(0);

    assert!(!lane.schedule(timer_sink(&runtime), std::time::Duration::ZERO, identity,));
}
