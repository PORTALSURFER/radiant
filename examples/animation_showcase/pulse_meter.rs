#[path = "pulse_meter/paint_geometry.rs"]
mod paint_geometry;
#[path = "pulse_meter/visual.rs"]
mod visual;

use self::paint_geometry::{push_ratio_circle, push_ratio_rect, push_wrapped_ratio_bar};
use self::visual::{PulseMeterVisual, wrap01};
use radiant::{
    gui::paint::{BorderSides, PaintFrame},
    layout::Rect,
    theme::ThemeTokens,
};

pub(super) fn pulse_meter_revision(phase: f32, running: bool) -> u64 {
    let phase_revision = (phase.clamp(0.0, 1.0) * 10_000.0).round() as u64;
    (phase_revision << 1) | u64::from(running)
}

pub(super) fn pulse_meter_frame(
    phase: f32,
    running: bool,
    bounds: Rect,
    theme: &ThemeTokens,
) -> PaintFrame {
    let visual = PulseMeterVisual::resolve(phase, running);
    let track = bounds.inset_symmetric_saturating(2.0, 7.0);
    let rail = track.inset_symmetric_saturating(8.0, 10.0);
    let pulse_lane = track.inset_symmetric_saturating(8.0, 6.0);
    let center_y = (track.min.y + track.max.y) * 0.5;
    let mut frame = PaintFrame::default();
    frame.primitives.reserve(22);
    frame.push_rect(track, visual.track_color(theme));
    frame.push_rect(rail, visual.rail_color(theme));
    for marker in visual.beat_markers {
        push_ratio_rect(
            &mut frame,
            rail,
            marker.center - marker.width * 0.5,
            marker.width,
            marker.color,
        );
    }
    for pulse in visual.pulses {
        push_wrapped_ratio_bar(
            &mut frame,
            pulse_lane,
            pulse.center,
            pulse.width,
            pulse.height_ratio,
            pulse.color,
        );
    }
    push_ratio_circle(
        &mut frame,
        track,
        visual.playhead_center,
        center_y,
        visual.glow_radius,
        visual.glow_color(theme),
    );
    push_ratio_circle(
        &mut frame,
        track,
        visual.playhead_center,
        center_y,
        visual.playhead_radius,
        visual.playhead_color(theme),
    );
    push_ratio_rect(
        &mut frame,
        track.inset_symmetric_saturating(2.0, 3.0),
        visual.playhead_start,
        visual.playhead_width,
        visual.playhead_line_color(theme),
    );
    frame.push_border_rects(track, theme.border_emphasis, 1.0, BorderSides::ALL);
    frame
}

#[cfg(test)]
#[path = "pulse_meter/tests.rs"]
mod tests;
