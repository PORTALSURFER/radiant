use super::TimerLane;

#[test]
fn timer_lane_rejects_work_when_worker_is_unavailable() {
    let lane = TimerLane::<u32>::without_worker_for_test();

    assert!(!lane.schedule(std::sync::Weak::new(), std::time::Duration::ZERO, 1));
}
