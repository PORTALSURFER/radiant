use super::*;

#[test]
fn runtime_animation_activity_keeps_frame_messages_bound_to_paint_frames() {
    assert!(!RuntimeAnimationActivity::idle().needs_animation());
    assert!(!RuntimeAnimationActivity::idle().needs_frame_message());
    assert!(RuntimeAnimationActivity::paint_only().needs_animation());
    assert!(!RuntimeAnimationActivity::paint_only().needs_frame_message());
    assert!(RuntimeAnimationActivity::frame_messages().needs_animation());
    assert!(RuntimeAnimationActivity::frame_messages().needs_frame_message());
    assert!(!RuntimeAnimationActivity::new(false, true).needs_frame_message());
}

#[test]
fn runtime_animation_activity_uses_named_demands_for_policy() {
    assert_eq!(
        RuntimeAnimationActivity::from_demand(RuntimeAnimationDemand::Idle),
        RuntimeAnimationActivity::idle()
    );
    assert_eq!(
        RuntimeAnimationActivity::from_demand(RuntimeAnimationDemand::PaintOnly),
        RuntimeAnimationActivity::paint_only()
    );
    assert_eq!(
        RuntimeAnimationActivity::from_demand(RuntimeAnimationDemand::FrameMessages),
        RuntimeAnimationActivity::frame_messages()
    );
    assert_eq!(
        RuntimeAnimationActivity::new(true, true),
        RuntimeAnimationActivity::from_demand(RuntimeAnimationDemand::FrameMessages)
    );
}

#[test]
fn runtime_animation_activity_carries_optional_frame_rate_policy() {
    assert_eq!(RuntimeAnimationActivity::idle().target_fps(), None);
    assert_eq!(
        RuntimeAnimationActivity::paint_only_at(24).target_fps(),
        Some(24)
    );
    assert_eq!(
        RuntimeAnimationActivity::frame_messages_at(30).target_fps(),
        Some(30)
    );
    assert_eq!(
        RuntimeAnimationActivity::idle()
            .with_target_fps(60)
            .target_fps(),
        None
    );
}

#[test]
fn runtime_animation_activity_merges_message_and_paint_demands() {
    let activity =
        RuntimeAnimationActivity::frame_messages_at(24).merge(RuntimeAnimationActivity::idle());

    assert!(activity.needs_animation());
    assert!(activity.needs_frame_message());
    assert_eq!(activity.target_fps(), Some(24));
}

#[test]
fn runtime_animation_activity_merge_preserves_fastest_capped_source() {
    let activity = RuntimeAnimationActivity::paint_only_at(24)
        .merge(RuntimeAnimationActivity::frame_messages_at(60));

    assert!(activity.needs_animation());
    assert!(activity.needs_frame_message());
    assert_eq!(activity.target_fps(), Some(60));
}

#[test]
fn runtime_animation_activity_merge_keeps_uncapped_source_uncapped() {
    let activity = RuntimeAnimationActivity::paint_only_at(24)
        .merge(RuntimeAnimationActivity::frame_messages());

    assert!(activity.needs_animation());
    assert!(activity.needs_frame_message());
    assert_eq!(activity.target_fps(), None);
}
