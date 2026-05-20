use super::*;

#[test]
fn zoom_and_pan_keep_viewport_inside_sample() {
    let mono_samples = vec![0.0; 20_000];
    let file = Arc::new(synthetic_file(mono_samples, 48_000, 1));
    let mut app = WaveformApp {
        file,
        viewport: WaveformViewport::full(20_000),
        zoom_anchor_ratio: 0.5,
        playing: false,
        playhead_ratio: 0.5,
    };

    app.zoom_around_anchor(0.5, 0.5);
    assert!(app.viewport.visible_items() < 20_000);
    app.pan_by_visible_fraction(100.0);
    assert_eq!(app.viewport.end, 20_000);
    app.pan_by_visible_fraction(-100.0);
    assert_eq!(app.viewport.start, 0);
}

#[test]
fn wheel_zoom_and_scrollbar_offset_update_viewport() {
    let mono_samples = vec![0.0; 20_000];
    let file = Arc::new(synthetic_file(mono_samples, 48_000, 1));
    let mut app = WaveformApp {
        file,
        viewport: WaveformViewport::full(20_000),
        zoom_anchor_ratio: 0.5,
        playing: false,
        playhead_ratio: 0.5,
    };

    app.apply_interaction(WaveformInteraction::Wheel {
        delta: Vector2::new(0.0, -40.0),
        anchor_ratio: 0.25,
    });
    assert!(app.viewport.is_zoomed_in(20_000));

    app.apply_interaction(WaveformInteraction::ScrollTo {
        offset_fraction: 1.0,
    });
    assert_eq!(app.viewport.end, 20_000);
}

#[test]
fn zoom_around_anchor_keeps_anchor_frame_at_same_ratio() {
    let mono_samples = vec![0.0; 20_000];
    let file = Arc::new(synthetic_file(mono_samples, 48_000, 1));
    let mut app = WaveformApp {
        file,
        viewport: WaveformViewport {
            start: 2_000,
            end: 12_000,
        },
        zoom_anchor_ratio: 0.5,
        playing: false,
        playhead_ratio: 0.5,
    };
    let ratio = 0.25;
    let before_anchor = app.viewport.start as f32 + app.viewport.visible_items() as f32 * ratio;

    app.zoom_around_anchor(0.5, ratio);

    let after_anchor = app.viewport.start as f32 + app.viewport.visible_items() as f32 * ratio;
    assert!((before_anchor - after_anchor).abs() <= 1.0);
}
