use super::super::{ColorRamp, ColorRampStop};
use crate::gui::types::Rgba8;

#[test]
fn color_ramp_samples_between_normalized_stops() {
    let stops = [
        ColorRampStop::new(0.0, Rgba8::new(0, 0, 0, 255)),
        ColorRampStop::new(0.5, Rgba8::new(100, 150, 200, 255)),
        ColorRampStop::new(1.0, Rgba8::new(200, 50, 0, 255)),
    ];
    let ramp = ColorRamp::new(&stops);

    assert_eq!(ramp.sample(-1.0), Rgba8::new(0, 0, 0, 255));
    assert_eq!(ramp.sample(0.5), Rgba8::new(100, 150, 200, 255));
    assert_eq!(ramp.sample(2.0), Rgba8::new(200, 50, 0, 255));
}

#[test]
fn color_ramp_interpolates_alpha_and_channels() {
    let stops = [
        ColorRampStop::byte(0, Rgba8::new(10, 20, 30, 40)),
        ColorRampStop::byte(100, Rgba8::new(110, 220, 130, 240)),
    ];

    assert_eq!(
        ColorRamp::new(&stops).sample(50.0 / 255.0),
        Rgba8::new(60, 120, 80, 140)
    );
}

#[test]
fn color_ramp_handles_empty_and_duplicate_stops_without_panicking() {
    assert_eq!(ColorRamp::new(&[]).sample(0.5), Rgba8::default());

    let stops = [
        ColorRampStop::new(f32::NAN, Rgba8::new(1, 2, 3, 4)),
        ColorRampStop::new(0.0, Rgba8::new(5, 6, 7, 8)),
    ];

    assert_eq!(
        ColorRamp::new(&stops).sample(f32::NAN),
        Rgba8::new(1, 2, 3, 4)
    );
    assert_eq!(ColorRamp::new(&stops).sample(0.1), Rgba8::new(5, 6, 7, 8));
}
