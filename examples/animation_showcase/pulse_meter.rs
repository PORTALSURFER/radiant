#[path = "pulse_meter/visual.rs"]
mod visual;

use self::visual::{PulseMeterVisual, wrap01};
use radiant::{
    gui::{
        paint::{BorderSides, FillCircle, FillRect, PaintFrame, Primitive, border_fill_rects},
        types::Rgba8,
    },
    layout::{Point, Rect},
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
    let track = inset(bounds, 2.0, 7.0);
    let rail = inset(track, 8.0, 10.0);
    let pulse_lane = inset(track, 8.0, 6.0);
    let center_y = (track.min.y + track.max.y) * 0.5;
    let mut frame = PaintFrame::default();
    frame.primitives.reserve(22);
    push_rect(&mut frame, track, visual.track_color(theme));
    push_rect(&mut frame, rail, visual.rail_color(theme));
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
        inset(track, 2.0, 3.0),
        visual.playhead_start,
        visual.playhead_width,
        visual.playhead_line_color(theme),
    );
    frame.primitives.extend(
        border_fill_rects(track, theme.border_emphasis, 1.0, BorderSides::ALL)
            .into_iter()
            .map(Primitive::Rect),
    );
    frame
}

fn push_wrapped_ratio_bar(
    frame: &mut PaintFrame,
    lane: Rect,
    center_ratio: f32,
    width_ratio: f32,
    height_ratio: f32,
    color: Rgba8,
) {
    let width = width_ratio.max(0.0);
    let width = width.min(0.08);
    if width <= 0.0 {
        return;
    }
    let center = wrap01(center_ratio);
    let start = center - width * 0.5;
    let end = center + width * 0.5;
    if start < 0.0 {
        push_ratio_bar_segment(frame, lane, start + 1.0, 1.0, height_ratio, color);
        push_ratio_bar_segment(frame, lane, 0.0, end, height_ratio, color);
        return;
    }
    if end > 1.0 {
        push_ratio_bar_segment(frame, lane, start, 1.0, height_ratio, color);
        push_ratio_bar_segment(frame, lane, 0.0, end - 1.0, height_ratio, color);
        return;
    }
    push_ratio_bar_segment(frame, lane, start, end, height_ratio, color);
}

fn push_ratio_bar_segment(
    frame: &mut PaintFrame,
    lane: Rect,
    start: f32,
    end: f32,
    height_ratio: f32,
    color: Rgba8,
) {
    let start = start.clamp(0.0, 1.0);
    let end = end.clamp(0.0, 1.0);
    let height = lane.height() * height_ratio.clamp(0.0, 1.0);
    if end <= start || height <= 0.0 {
        return;
    }
    let center_y = (lane.min.y + lane.max.y) * 0.5;
    push_rect(
        frame,
        Rect::from_min_max(
            Point::new(lane.min.x + lane.width() * start, center_y - height * 0.5),
            Point::new(lane.min.x + lane.width() * end, center_y + height * 0.5),
        ),
        color,
    );
}

fn push_ratio_circle(
    frame: &mut PaintFrame,
    track: Rect,
    center_ratio: f32,
    center_y: f32,
    radius: f32,
    color: Rgba8,
) {
    if radius <= 0.0 {
        return;
    }
    let center_x = track.min.x + track.width() * center_ratio.clamp(0.0, 1.0);
    frame.primitives.push(Primitive::Circle(FillCircle {
        center: Point::new(center_x, center_y),
        radius,
        color,
    }));
}

fn push_ratio_rect(
    frame: &mut PaintFrame,
    track: Rect,
    start_ratio: f32,
    width_ratio: f32,
    color: Rgba8,
) {
    let start = start_ratio.clamp(0.0, 1.0);
    let end = (start + width_ratio.max(0.0)).clamp(0.0, 1.0);
    if end <= start {
        return;
    }
    push_rect(
        frame,
        Rect::from_min_max(
            Point::new(track.min.x + track.width() * start, track.min.y),
            Point::new(track.min.x + track.width() * end, track.max.y),
        ),
        color,
    );
}

fn push_rect(frame: &mut PaintFrame, rect: Rect, color: Rgba8) {
    frame
        .primitives
        .push(Primitive::Rect(FillRect { rect, color }));
}

fn inset(rect: Rect, x: f32, y: f32) -> Rect {
    Rect::from_min_max(
        Point::new(rect.min.x + x, rect.min.y + y),
        Point::new(rect.max.x - x, rect.max.y - y),
    )
}

#[cfg(test)]
#[path = "pulse_meter/tests.rs"]
mod tests;
