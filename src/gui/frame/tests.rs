use super::FrameBuildResult;

#[test]
fn frame_build_result_defaults_to_no_work_observed() {
    let result = FrameBuildResult::default();

    assert_eq!(result.primitive_count, 0);
    assert_eq!(result.text_run_count, 0);
    assert!(!result.layout_rebuild);
    assert!(!result.static_rebuild);
    assert!(!result.state_overlay_rebuild);
    assert!(!result.motion_overlay_rebuild);
    assert!(!result.needs_animation);
    assert_eq!(result.frame_total_us, 0);
    assert_eq!(result.present_us, 0);
    assert_eq!(result.frame_budget_us, 0);
    assert!(!result.jank);
    assert!(!result.presented);
    assert!(!result.missed_present);
}
