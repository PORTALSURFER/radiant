use super::{
    FrameAnimationRequest, FrameBuildCounts, FrameBuildResult, FrameBuildTiming,
    FramePresentResult, FrameRebuildFlags,
};

#[test]
fn frame_build_result_defaults_to_no_work_observed() {
    let result = FrameBuildResult::default();

    assert_eq!(result.counts.primitive_count, 0);
    assert_eq!(result.counts.text_run_count, 0);
    assert!(!result.rebuilds.layout_rebuild);
    assert!(!result.rebuilds.static_rebuild);
    assert!(!result.rebuilds.state_overlay_rebuild);
    assert!(!result.rebuilds.motion_overlay_rebuild);
    assert!(!result.animation.needs_animation);
    assert_eq!(result.timing.frame_total_us, 0);
    assert_eq!(result.timing.present_us, 0);
    assert_eq!(result.timing.frame_budget_us, 0);
    assert!(!result.timing.jank);
    assert!(!result.presentation.presented);
    assert!(!result.presentation.missed_present);
}

#[test]
fn frame_build_result_groups_related_feedback() {
    let result = FrameBuildResult {
        counts: FrameBuildCounts {
            primitive_count: 7,
            text_run_count: 3,
        },
        rebuilds: FrameRebuildFlags {
            layout_rebuild: true,
            static_rebuild: true,
            state_overlay_rebuild: false,
            motion_overlay_rebuild: true,
        },
        animation: FrameAnimationRequest {
            needs_animation: true,
        },
        timing: FrameBuildTiming {
            frame_total_us: 1_500,
            present_us: 400,
            frame_budget_us: 16_667,
            jank: false,
        },
        presentation: FramePresentResult {
            presented: true,
            missed_present: false,
        },
    };

    assert_eq!(result.counts.primitive_count, 7);
    assert!(result.rebuilds.layout_rebuild);
    assert!(result.animation.needs_animation);
    assert_eq!(result.timing.present_us, 400);
    assert!(result.presentation.presented);
}
