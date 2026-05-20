use super::*;

#[test]
fn fingerprints_are_stable_for_identical_inputs() {
    let mut first = StableFingerprint::new();
    first.mix_str("button");
    first.mix_u32(42);
    first.mix_bool(true);

    let mut second = StableFingerprint::new();
    second.mix_str("button");
    second.mix_u32(42);
    second.mix_bool(true);

    assert_eq!(first.finish(), second.finish());
}

#[test]
fn option_presence_changes_fingerprint() {
    let mut present = StableFingerprint::new();
    present.mix_option_str(Some("hover"));

    let mut missing = StableFingerprint::new();
    missing.mix_option_str(None);

    assert_ne!(present.finish(), missing.finish());
}

#[test]
fn color_channels_affect_fingerprint() {
    let mut first = StableFingerprint::new();
    first.mix_rgba8(Rgba8 {
        r: 1,
        g: 2,
        b: 3,
        a: 4,
    });

    let mut changed = StableFingerprint::new();
    changed.mix_rgba8(Rgba8 {
        r: 1,
        g: 2,
        b: 3,
        a: 5,
    });

    assert_ne!(first.finish(), changed.finish());
}
