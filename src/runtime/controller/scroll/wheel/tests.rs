use super::{ScrollUpdateMetadata, WheelWidgetDispatch};
use crate::{
    gui::types::Vector2,
    widgets::{PointerModifiers, WheelDelta, WheelPhase, WheelSample},
};

#[test]
fn retained_no_output_is_distinct_from_unhandled_fallback() {
    assert!(WheelWidgetDispatch::RetainedNoOutput.retained());
    assert!(!WheelWidgetDispatch::Unhandled.retained());
    assert!(!WheelWidgetDispatch::Handled { retained: false }.retained());
    assert!(WheelWidgetDispatch::Handled { retained: true }.retained());
    assert_ne!(
        WheelWidgetDispatch::RetainedNoOutput,
        WheelWidgetDispatch::Unhandled
    );
}

#[test]
fn exact_sample_preserves_units_phase_and_scroll_metadata() {
    let modifiers = PointerModifiers {
        command: true,
        shift: false,
        alt: true,
    };
    let sample = WheelSample::new_with_metadata(
        WheelDelta::Lines(Vector2::new(0.25, -2.0)),
        Some(WheelPhase::Changed),
        modifiers,
        None,
        None,
    )
    .expect("finite line sample");

    assert_eq!(sample.delta(), WheelDelta::Lines(Vector2::new(0.25, -2.0)));
    assert_eq!(sample.phase(), Some(WheelPhase::Changed));
    assert_eq!(sample.modifiers(), modifiers);
    assert_eq!(
        sample.delta().to_logical_pixels(),
        Some(Vector2::new(10.0, -80.0))
    );
    assert_eq!(
        ScrollUpdateMetadata::from(sample),
        ScrollUpdateMetadata {
            modifiers,
            timestamp: None,
            sequence_range: None,
        }
    );
}

#[test]
fn all_explicit_phases_and_phase_less_samples_are_constructible() {
    let delta = WheelDelta::pixels(Vector2::new(0.0, 1.0)).expect("finite pixel sample");
    for phase in [
        None,
        Some(WheelPhase::Started),
        Some(WheelPhase::Changed),
        Some(WheelPhase::Ended),
        Some(WheelPhase::Cancelled),
        Some(WheelPhase::Discrete),
    ] {
        assert_eq!(
            WheelSample::new(delta, phase, PointerModifiers::default())
                .expect("phase should be accepted")
                .phase(),
            phase
        );
    }
}
