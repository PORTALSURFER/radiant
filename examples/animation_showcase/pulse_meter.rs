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

fn wrap01(value: f32) -> f32 {
    value.rem_euclid(1.0)
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

fn with_alpha(mut color: Rgba8, alpha: u8) -> Rgba8 {
    color.a = alpha;
    color
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PulseMeterVisual {
    beat_markers: [PulseMarker; 5],
    pulses: [PulseBar; 4],
    playhead_center: f32,
    playhead_radius: f32,
    glow_radius: f32,
    playhead_start: f32,
    playhead_width: f32,
    running: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PulseMarker {
    center: f32,
    width: f32,
    color: Rgba8,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PulseBar {
    center: f32,
    width: f32,
    height_ratio: f32,
    color: Rgba8,
}

impl PulseMeterVisual {
    fn resolve(phase: f32, running: bool) -> Self {
        let phase = phase.clamp(0.0, 1.0);
        let beat = smoothstep(0.0, 1.0, 1.0 - (phase * 2.0 - 1.0).abs());
        let pulse = smoothstep(0.0, 1.0, (phase * std::f32::consts::TAU).sin() * 0.5 + 0.5);
        let playhead_width = 0.012;
        let playhead_center = phase * (1.0 - playhead_width) + playhead_width * 0.5;
        let playhead_start =
            (playhead_center - playhead_width * 0.5).clamp(0.0, 1.0 - playhead_width);

        Self {
            beat_markers: [
                Self::marker(0.125, 48),
                Self::marker(0.3125, 40),
                Self::marker(0.5, 58),
                Self::marker(0.6875, 40),
                Self::marker(0.875, 48),
            ],
            pulses: [
                Self::bar(playhead_center - 0.18, 0.007, 0.30, 54, running),
                Self::bar(playhead_center - 0.11, 0.009, 0.46, 84, running),
                Self::bar(
                    playhead_center - 0.052,
                    0.011,
                    0.62 + pulse * 0.18,
                    120,
                    running,
                ),
                Self::bar(playhead_center, 0.014, 0.78 + beat * 0.16, 190, running),
            ],
            playhead_center,
            playhead_radius: if running { 4.8 + beat * 1.4 } else { 4.2 },
            glow_radius: if running { 9.0 + beat * 2.0 } else { 6.5 },
            playhead_start,
            playhead_width,
            running,
        }
    }

    fn marker(center: f32, alpha: u8) -> PulseMarker {
        PulseMarker {
            center,
            width: 0.0035,
            color: Rgba8 {
                r: 176,
                g: 182,
                b: 194,
                a: alpha,
            },
        }
    }

    fn bar(center: f32, width: f32, height_ratio: f32, alpha: u8, running: bool) -> PulseBar {
        PulseBar {
            center: wrap01(center),
            width,
            height_ratio,
            color: Rgba8 {
                r: 255,
                g: 116,
                b: 76,
                a: if running { alpha } else { alpha / 3 },
            },
        }
    }

    fn track_color(self, theme: &ThemeTokens) -> Rgba8 {
        if self.running {
            theme.surface_base
        } else {
            with_alpha(theme.surface_base, 210)
        }
    }

    fn rail_color(self, theme: &ThemeTokens) -> Rgba8 {
        if self.running {
            with_alpha(theme.grid_soft, 95)
        } else {
            with_alpha(theme.grid_soft, 48)
        }
    }

    fn glow_color(self, theme: &ThemeTokens) -> Rgba8 {
        if self.running {
            with_alpha(theme.highlight_orange, 70)
        } else {
            with_alpha(theme.highlight_orange, 32)
        }
    }

    fn playhead_color(self, theme: &ThemeTokens) -> Rgba8 {
        if self.running {
            theme.highlight_orange
        } else {
            with_alpha(theme.highlight_orange, 115)
        }
    }

    fn playhead_line_color(self, theme: &ThemeTokens) -> Rgba8 {
        if self.running {
            theme.text_primary
        } else {
            with_alpha(theme.text_primary, 150)
        }
    }
}

fn smoothstep(edge0: f32, edge1: f32, value: f32) -> f32 {
    let t = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::layout::Vector2;

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
                Primitive::Rect(fill) if fill.color.r > 240 && fill.color.g < 140 => {
                    Some(fill.color.a)
                }
                Primitive::Circle(fill) if fill.color.r > 240 && fill.color.g < 140 => {
                    Some(fill.color.a)
                }
                _ => None,
            })
            .max()
            .unwrap_or(0)
    }
}
