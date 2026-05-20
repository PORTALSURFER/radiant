use super::*;

#[test]
fn retained_invalidation_primitives_stay_in_focused_modules() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/gui/invalidation.rs"))
        .expect("invalidation root should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/gui/invalidation/tests.rs"))
        .expect("invalidation behavior tests should be readable");
    let mask = fs::read_to_string(manifest_dir.join("src/gui/invalidation/mask.rs"))
        .expect("invalidation mask module should be readable");
    let retained_mask =
        fs::read_to_string(manifest_dir.join("src/gui/invalidation/retained_mask.rs"))
            .expect("retained mask module should be readable");
    let segment = fs::read_to_string(manifest_dir.join("src/gui/invalidation/segment.rs"))
        .expect("retained segment module should be readable");

    for required in [
        "mod mask;",
        "mod retained_mask;",
        "mod segment;",
        "#[path = \"invalidation/tests.rs\"]",
        "pub use mask::InvalidationMask;",
        "pub use retained_mask::RetainedSegmentMask;",
    ] {
        assert!(
            root.contains(required),
            "invalidation root should delegate `{required}`"
        );
    }
    assert!(
        root.contains("RetainedSegmentPlan")
            && root.contains("RetainedSegmentRevisions")
            && !root.contains("pub struct InvalidationMask")
            && !root.contains("pub struct RetainedSegmentMask")
            && !root.contains("pub struct RetainedSegmentPlan")
            && !root.contains("fn invalidation_mask_clips_to_valid_bits"),
        "invalidation root should re-export public primitives and delegate behavior tests without owning implementations"
    );
    assert!(
        tests.contains("fn invalidation_mask_clips_to_valid_bits")
            && tests.contains("fn retained_segment_plan_names_groups_and_bumps_revisions"),
        "invalidation behavior coverage should live in gui/invalidation/tests.rs"
    );
    assert!(
        mask.contains("pub struct InvalidationMask")
            && mask.contains("pub const fn from_bits")
            && mask.contains("pub fn insert"),
        "raw invalidation bit operations should live in invalidation/mask.rs"
    );
    assert!(
        retained_mask.contains("pub struct RetainedSegmentMask")
            && retained_mask.contains("pub const fn requires_static_rebuild")
            && retained_mask.contains("pub const fn requires_overlay_rebuild"),
        "typed retained segment masks should live in invalidation/retained_mask.rs"
    );
    assert!(
        segment.contains("pub struct RetainedSegmentPlan")
            && segment.contains("pub struct RetainedSegmentRevisions")
            && segment.contains("pub enum RetainedSegmentKind")
            && segment.contains("pub fn bump_revisions"),
        "retained segment metadata, plans, and revisions should live in invalidation/segment.rs"
    );
}

#[test]
fn retained_cache_support_keeps_fingerprint_storage_and_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fingerprint = fs::read_to_string(manifest_dir.join("src/gui/fingerprint.rs"))
        .expect("stable fingerprint source should be readable");
    let fingerprint_tests = fs::read_to_string(manifest_dir.join("src/gui/fingerprint/tests.rs"))
        .expect("stable fingerprint tests should be readable");
    let retained = fs::read_to_string(manifest_dir.join("src/gui/retained.rs"))
        .expect("retained storage source should be readable");
    let retained_tests = fs::read_to_string(manifest_dir.join("src/gui/retained/tests.rs"))
        .expect("retained storage tests should be readable");

    assert!(
        fingerprint.contains("pub struct StableFingerprint")
            && fingerprint.contains("pub fn mix_rgba8")
            && fingerprint.contains("#[path = \"fingerprint/tests.rs\"]")
            && !fingerprint.contains("fn fingerprints_are_stable_for_identical_inputs"),
        "stable fingerprint mixing should live in gui/fingerprint.rs while behavior tests stay delegated"
    );
    assert!(
        fingerprint_tests.contains("fn fingerprints_are_stable_for_identical_inputs")
            && fingerprint_tests.contains("fn color_channels_affect_fingerprint"),
        "fingerprint behavior coverage should live in gui/fingerprint/tests.rs"
    );
    assert!(
        retained.contains("pub struct RetainedVec")
            && retained.contains("pub fn make_mut")
            && retained.contains("#[path = \"retained/tests.rs\"]")
            && !retained.contains("fn retained_vec_clones_share_storage_until_mutation"),
        "retained vector storage should live in gui/retained.rs while behavior tests stay delegated"
    );
    assert!(
        retained_tests.contains("fn retained_vec_clones_share_storage_until_mutation"),
        "retained storage behavior coverage should live in gui/retained/tests.rs"
    );
}
