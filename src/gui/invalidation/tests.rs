use super::{
    InvalidationMask, RetainedSegment, RetainedSegmentMask, RetainedSegmentPlan,
    RetainedSegmentRevisions,
};

const VALID_MASK: u16 = 0b0111;

#[test]
fn invalidation_mask_clips_to_valid_bits() {
    let mask = InvalidationMask::from_bits(0b1111, VALID_MASK);

    assert_eq!(mask.bits(), VALID_MASK);
}

#[test]
fn invalidation_mask_reports_intersections() {
    let mask = InvalidationMask::from_bits(0b0101, VALID_MASK);

    assert!(mask.intersects(0b0001));
    assert!(!mask.intersects(0b0010));
}

#[test]
fn invalidation_mask_insert_preserves_only_valid_bits() {
    let mut mask = InvalidationMask::empty();

    mask.insert(0b1010, VALID_MASK);

    assert_eq!(mask.bits(), 0b0010);
}

#[test]
fn retained_segment_mask_tracks_static_overlay_and_valid_bits() {
    type Mask = RetainedSegmentMask<0b1111, 0b0011, 0b1100>;

    let mut mask = Mask::from_bits(0b1_1111);
    assert_eq!(mask.bits(), 0b1111);
    assert!(mask.requires_static_rebuild());
    assert!(mask.requires_overlay_rebuild());

    mask = Mask::empty();
    mask.insert(0b10000);
    assert!(mask.is_empty());
    mask.insert(0b0100);
    assert!(!mask.requires_static_rebuild());
    assert!(mask.requires_overlay_rebuild());
}

#[test]
fn retained_segment_revisions_report_and_bump_changed_segments() {
    let mut revisions = RetainedSegmentRevisions::<3>::default();

    assert!(!revisions.has_revisions());
    revisions.bump_for_bits(0b101, [0b001, 0b010, 0b100]);

    assert_eq!(revisions.revisions, [1, 0, 1]);
    assert!(revisions.has_revisions());
}

#[test]
fn retained_segment_plan_names_groups_and_bumps_revisions() {
    const PLAN: RetainedSegmentPlan<3> = RetainedSegmentPlan::new([
        RetainedSegment::static_segment("base"),
        RetainedSegment::overlay("hover"),
        RetainedSegment::overlay("playhead"),
    ]);
    let mut revisions = RetainedSegmentRevisions::<3>::default();

    assert_eq!(PLAN.valid_mask(), 0b111);
    assert_eq!(PLAN.static_mask(), 0b001);
    assert_eq!(PLAN.overlay_mask(), 0b110);

    let hover = PLAN.mask_for_name("hover").expect("hover segment");
    assert!(!PLAN.requires_static_rebuild(hover));
    assert!(PLAN.requires_overlay_rebuild(hover));

    PLAN.bump_revisions(&mut revisions, hover);
    assert_eq!(revisions.revisions, [0, 1, 0]);
    assert_eq!(PLAN.mask(0b1000).bits(), 0);
}
