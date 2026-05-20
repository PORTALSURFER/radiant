use super::*;
use radiant::{
    gui::paint::{PaintFrame, Primitive},
    layout::{Point, Vector2},
};

#[test]
fn pulse_meter_resolves_visible_motion_geometry() {
    let start = PulseMeterVisual::resolve(0.0, true);
    let peak = PulseMeterVisual::resolve(0.25, true);
    let far_edge = PulseMeterVisual::resolve(0.5, true);
    let end = PulseMeterVisual::resolve(1.0, true);

    assert!(peak.playhead_start > start.playhead_start + 0.20);
    assert!(far_edge.playhead_start > peak.playhead_start + 0.20);
    assert!(end.playhead_start > far_edge.playhead_start + 0.45);
    assert!(peak.playhead_radius > start.playhead_radius);
    assert!(peak.glow_radius > start.glow_radius);
    assert_eq!(start.playhead_width, end.playhead_width);
    assert!(start.beat_markers[0].center < start.beat_markers[4].center);
    assert!(far_edge.pulses[3].center > peak.pulses[3].center + 0.20);
    assert_eq!(start.pulses[3].color.a, peak.pulses[3].color.a);
    assert!(start.pulses[0].color.a < start.pulses[3].color.a);
}

#[test]
fn pulse_meter_wraps_trailing_pulses_without_edge_clamping() {
    let early = PulseMeterVisual::resolve(0.02, true);
    let late = PulseMeterVisual::resolve(0.96, true);

    assert!(early.pulses[0].center > 0.75);
    assert!(early.pulses[1].center > 0.80);
    assert!(late.pulses[3].center > 0.90);
    assert!(
        early
            .pulses
            .iter()
            .chain(late.pulses.iter())
            .all(|pulse| (0.0..1.0).contains(&pulse.center))
    );
}

#[test]
fn paused_pulse_meter_keeps_position_but_dims_activity() {
    let running = PulseMeterVisual::resolve(0.45, true);
    let paused = PulseMeterVisual::resolve(0.45, false);

    assert_eq!(running.playhead_center, paused.playhead_center);
    assert_eq!(running.playhead_start, paused.playhead_start);
    assert!(paused.playhead_radius < running.playhead_radius);
    assert!(paused.glow_radius < running.glow_radius);
    assert!(paused.pulses[3].color.a < running.pulses[3].color.a);
}

#[test]
fn phase_meter_paints_rail_pulses_playhead_and_marker() {
    let frame = pulse_meter_frame(
        0.5,
        true,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(420.0, 48.0)),
        &ThemeTokens::default(),
    );
    let fills: Vec<_> = frame
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Rect(fill) => Some(fill),
            _ => None,
        })
        .collect();
    let circles: Vec<_> = frame
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Circle(fill) => Some(fill),
            _ => None,
        })
        .collect();

    assert_eq!(fills.len(), 16);
    assert_eq!(circles.len(), 2);
    assert!(fills.iter().any(|fill| fill.rect.width() > 410.0));
    assert!(
        fills
            .iter()
            .filter(|fill| fill.color.r > 240 && fill.color.g < 140)
            .count()
            >= 4
    );
    assert!(circles.iter().any(|circle| circle.radius > 6.0));
    assert!(circles.iter().any(|circle| circle.color.a <= 70));
    assert!(
        fills
            .iter()
            .all(|fill| fill.color != ThemeTokens::default().highlight_orange),
        "pulse paint should not collapse into one large filled slab"
    );
    assert!(
        fills
            .iter()
            .filter(|fill| fill.color.r > 240 && fill.color.g < 140)
            .all(|fill| fill.rect.width() < 16.0),
        "pulse accents should stay localized instead of reading as a progress bar"
    );
    assert_ne!(fills[0].color, fills[1].color);
}

#[test]
fn late_phase_meter_keeps_activity_localized_near_playhead() {
    let frame = pulse_meter_frame(
        0.85,
        true,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(740.0, 42.0)),
        &ThemeTokens::default(),
    );
    let orange_fills: Vec<_> = frame
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Rect(fill) if fill.color.r > 240 && fill.color.g < 150 => Some(fill),
            _ => None,
        })
        .collect();

    assert!(
        orange_fills.iter().all(|fill| fill.rect.width() < 18.0),
        "late-phase activity should stay as ticks instead of painting a progress slab"
    );
    assert!(
        orange_fills
            .iter()
            .any(|fill| fill.rect.min.x > 570.0 && fill.rect.max.x < 700.0),
        "late-phase playhead activity should remain near the visible playhead"
    );
    assert!(
        orange_fills.iter().all(|fill| fill.rect.width() > 0.0),
        "tick clamping should not create inverted retained-canvas rectangles"
    );
}

#[test]
fn paused_phase_meter_paints_dimmed_activity() {
    let running = pulse_meter_frame(
        0.5,
        true,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(420.0, 48.0)),
        &ThemeTokens::default(),
    );
    let paused = pulse_meter_frame(
        0.5,
        false,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(420.0, 48.0)),
        &ThemeTokens::default(),
    );

    let running_orange_alpha = max_orange_alpha(&running);
    let paused_orange_alpha = max_orange_alpha(&paused);

    assert!(paused_orange_alpha < running_orange_alpha);
    assert_ne!(
        pulse_meter_revision(0.5, true),
        pulse_meter_revision(0.5, false)
    );
}

fn max_orange_alpha(frame: &PaintFrame) -> u8 {
    frame
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Rect(fill) if fill.color.r > 240 && fill.color.g < 140 => Some(fill.color.a),
            Primitive::Circle(fill) if fill.color.r > 240 && fill.color.g < 140 => {
                Some(fill.color.a)
            }
            _ => None,
        })
        .max()
        .unwrap_or(0)
}
