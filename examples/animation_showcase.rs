//! Frame-driven animation through the application builder.

use radiant::prelude::*;
use radiant::{
    gui::{
        paint::{BorderSides, FillCircle, FillRect, PaintFrame, Primitive, border_fill_rects},
        types::Rgba8,
    },
    layout::{Point, Rect},
    theme::ThemeTokens,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AnimationMessage {
    Toggle,
    Frame,
    Reset,
}

#[derive(Clone, Debug)]
struct AnimationState {
    running: bool,
    frame: u64,
    phase: f32,
}

impl Default for AnimationState {
    fn default() -> Self {
        Self {
            running: true,
            frame: 0,
            phase: 0.0,
        }
    }
}

impl AnimationState {
    fn status(&self) -> String {
        if self.running {
            format!("Running | frame {} | phase {:.2}", self.frame, self.phase)
        } else {
            format!("Paused | frame {} | phase {:.2}", self.frame, self.phase)
        }
    }

    fn tick(&mut self) {
        if self.running {
            self.frame = self.frame.saturating_add(1);
            self.phase = ((self.frame % 180) as f32) / 180.0;
        }
    }

    fn reset(&mut self) {
        self.running = false;
        self.frame = 0;
        self.phase = 0.0;
    }
}

fn main() -> radiant::Result {
    radiant::app(AnimationState::default())
        .title("Radiant Animation Showcase")
        .size(520, 220)
        .min_size(420, 180)
        .view(animation_view)
        .animation(|state| state.running)
        .on_frame(|| AnimationMessage::Frame)
        .retained_painter(30, |state, _descriptor, rect, _viewport| {
            Some(pulse_meter_frame(
                state.phase,
                rect,
                &ThemeTokens::default(),
            ))
        })
        .update(|state, message| match message {
            AnimationMessage::Toggle => state.running = !state.running,
            AnimationMessage::Frame => state.tick(),
            AnimationMessage::Reset => state.reset(),
        })
        .run()
}

fn animation_view(state: &mut AnimationState) -> View<AnimationMessage> {
    column([
        text("Animation Showcase").height(28.0).fill_width(),
        text(state.status()).id(20).height(26.0).fill_width(),
        phase_meter(state.phase),
        row([
            button(if state.running { "Pause" } else { "Run" })
                .primary()
                .message(AnimationMessage::Toggle)
                .id(40)
                .width(100.0)
                .height(32.0),
            button("Reset")
                .subtle()
                .message(AnimationMessage::Reset)
                .id(41)
                .width(100.0)
                .height(32.0),
        ])
        .spacing(10.0)
        .fill_width(),
    ])
    .style(WidgetStyle::default())
    .padding(16.0)
    .spacing(10.0)
    .fill_width()
    .fill_height()
}

fn phase_meter(phase: f32) -> View<AnimationMessage> {
    retained_canvas_with(30, pulse_meter_revision(phase), 0, true)
        .view()
        .height(42.0)
        .key("phase-meter")
        .fill_width()
}

fn pulse_meter_revision(phase: f32) -> u64 {
    (phase.clamp(0.0, 1.0) * 10_000.0).round() as u64
}

fn pulse_meter_frame(phase: f32, bounds: Rect, theme: &ThemeTokens) -> PaintFrame {
    let visual = PulseMeterVisual::resolve(phase);
    let track = inset(bounds, 2.0, 7.0);
    let rail = inset(track, 8.0, 10.0);
    let pulse_lane = inset(track, 8.0, 6.0);
    let center_y = (track.min.y + track.max.y) * 0.5;
    let mut frame = PaintFrame::default();
    frame.primitives.reserve(18);
    push_rect(&mut frame, track, theme.surface_base);
    push_rect(&mut frame, rail, with_alpha(theme.grid_soft, 95));
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
        push_ratio_bar(
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
        with_alpha(theme.highlight_orange, 70),
    );
    push_ratio_circle(
        &mut frame,
        track,
        visual.playhead_center,
        center_y,
        visual.playhead_radius,
        theme.highlight_orange,
    );
    push_ratio_rect(
        &mut frame,
        inset(track, 2.0, 3.0),
        visual.playhead_start,
        visual.playhead_width,
        theme.text_primary,
    );
    frame.primitives.extend(
        border_fill_rects(track, theme.border_emphasis, 1.0, BorderSides::ALL)
            .into_iter()
            .map(Primitive::Rect),
    );
    frame
}

fn push_ratio_bar(
    frame: &mut PaintFrame,
    lane: Rect,
    center_ratio: f32,
    width_ratio: f32,
    height_ratio: f32,
    color: Rgba8,
) {
    let width = width_ratio.max(0.0);
    if width <= 0.0 {
        return;
    }
    let center = center_ratio.clamp(0.0, 1.0);
    let start = (center - width * 0.5).clamp(0.0, 1.0);
    let end = (center + width * 0.5).clamp(0.0, 1.0);
    let height = lane.height() * height_ratio.clamp(0.0, 1.0);
    if end <= start || height <= 0.0 {
        return;
    }
    let center_y = (lane.min.y + lane.max.y) * 0.5;
    push_rect(
        frame,
        Rect::from_min_max(
            radiant::layout::Point::new(lane.min.x + lane.width() * start, center_y - height * 0.5),
            radiant::layout::Point::new(lane.min.x + lane.width() * end, center_y + height * 0.5),
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
            radiant::layout::Point::new(track.min.x + track.width() * start, track.min.y),
            radiant::layout::Point::new(track.min.x + track.width() * end, track.max.y),
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
        radiant::layout::Point::new(rect.min.x + x, rect.min.y + y),
        radiant::layout::Point::new(rect.max.x - x, rect.max.y - y),
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
    fn resolve(phase: f32) -> Self {
        let phase = phase.clamp(0.0, 1.0);
        let beat = smoothstep(0.0, 1.0, 1.0 - (phase * 2.0 - 1.0).abs());
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
                Self::bar(playhead_center - 0.18, 0.016, 0.36, 52),
                Self::bar(playhead_center - 0.11, 0.018, 0.52, 72),
                Self::bar(playhead_center - 0.055, 0.022, 0.70 + beat * 0.18, 104),
                Self::bar(playhead_center, 0.026, 0.84 + beat * 0.16, 150),
            ],
            playhead_center,
            playhead_radius: 4.8 + beat * 1.4,
            glow_radius: 9.0 + beat * 2.0,
            playhead_start,
            playhead_width,
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

    fn bar(center: f32, width: f32, height_ratio: f32, alpha: u8) -> PulseBar {
        PulseBar {
            center: center.clamp(0.0, 1.0),
            width,
            height_ratio,
            color: Rgba8 {
                r: 255,
                g: 116,
                b: 76,
                a: alpha,
            },
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
    use radiant::{
        layout::{Point, Rect, Vector2},
        runtime::{Event, PaintPrimitive, RuntimeBridge, SurfaceRuntime},
        theme::ThemeTokens,
        widgets::PointerButton,
    };

    #[test]
    fn animation_state_advances_meter_phase() {
        let mut state = AnimationState::default();

        state.tick();

        assert_eq!(state.frame, 1);
        assert!(state.phase > 0.0);
    }

    #[test]
    fn pulse_meter_resolves_visible_motion_geometry() {
        let start = PulseMeterVisual::resolve(0.0);
        let peak = PulseMeterVisual::resolve(0.25);
        let far_edge = PulseMeterVisual::resolve(0.5);
        let end = PulseMeterVisual::resolve(1.0);

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
    fn phase_meter_paints_rail_pulses_playhead_and_marker() {
        let frame = pulse_meter_frame(
            0.5,
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
    fn animation_controls_pause_resume_and_reset_state() {
        let bridge = animation_test_bridge(AnimationState {
            running: true,
            frame: 42,
            phase: 0.5,
        });
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(520.0, 220.0));

        click_widget(&mut runtime, 40);
        assert_status_contains(&runtime, "Paused | frame 42 | phase 0.50");

        click_widget(&mut runtime, 41);
        assert_status_contains(&runtime, "Paused | frame 0 | phase 0.00");

        click_widget(&mut runtime, 40);
        assert_status_contains(&runtime, "Running | frame 0 | phase 0.00");
    }

    #[test]
    fn animation_controls_disable_and_reset_frame_driven_updates() {
        let bridge = animation_test_bridge(AnimationState {
            running: true,
            frame: 42,
            phase: 0.5,
        });
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(520.0, 220.0));

        assert!(runtime.bridge_mut().needs_animation());
        let outcome = runtime.drain_runtime_messages();
        assert_eq!(outcome.messages_dispatched, 1);
        assert_status_contains(&runtime, "Running | frame 43 | phase 0.24");

        click_widget(&mut runtime, 40);
        assert!(!runtime.bridge_mut().needs_animation());
        let outcome = runtime.drain_runtime_messages();
        assert_eq!(outcome.messages_dispatched, 0);
        assert_status_contains(&runtime, "Paused | frame 43 | phase 0.24");

        click_widget(&mut runtime, 41);
        assert!(!runtime.bridge_mut().needs_animation());
        assert_status_contains(&runtime, "Paused | frame 0 | phase 0.00");
    }

    #[test]
    fn animation_control_labels_track_running_state() {
        let bridge = animation_test_bridge(AnimationState {
            running: true,
            frame: 12,
            phase: 0.25,
        });
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(520.0, 220.0));

        assert_button_label(&runtime, 40, "Pause");
        click_widget(&mut runtime, 40);
        assert_button_label(&runtime, 40, "Run");
        click_widget(&mut runtime, 41);
        assert_button_label(&runtime, 40, "Run");
        click_widget(&mut runtime, 40);
        assert_button_label(&runtime, 40, "Pause");
    }

    #[test]
    fn reset_stops_running_animation_on_first_frame() {
        let bridge = animation_test_bridge(AnimationState {
            running: true,
            frame: 88,
            phase: 0.75,
        });
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(520.0, 220.0));

        click_widget(&mut runtime, 41);

        assert!(!runtime.bridge_mut().needs_animation());
        assert_status_contains(&runtime, "Paused | frame 0 | phase 0.00");
        assert_button_label(&runtime, 40, "Run");
    }

    #[test]
    fn reset_ignores_already_queued_animation_frame() {
        let bridge = animation_test_bridge(AnimationState {
            running: true,
            frame: 88,
            phase: 0.75,
        });
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(520.0, 220.0));

        assert!(runtime.bridge_mut().needs_animation());
        click_widget(&mut runtime, 41);

        let outcome = runtime.drain_runtime_messages();

        assert_eq!(outcome.messages_dispatched, 1);
        assert!(!runtime.bridge_mut().needs_animation());
        assert_status_contains(&runtime, "Paused | frame 0 | phase 0.00");
        assert_button_label(&runtime, 40, "Run");
    }

    #[test]
    fn animation_controls_survive_pending_frame_between_press_and_release() {
        let bridge = animation_test_bridge(AnimationState {
            running: true,
            frame: 42,
            phase: 0.5,
        });
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(520.0, 220.0));
        let rect = runtime.layout().rects[&40];
        let point = rect.center();

        assert!(runtime.bridge_mut().needs_animation());
        runtime.dispatch_event(Event::PointerPress {
            position: point,
            button: PointerButton::Primary,
        });
        let outcome = runtime.drain_runtime_messages();
        assert_eq!(outcome.messages_dispatched, 1);
        runtime.dispatch_event(Event::PointerRelease {
            position: point,
            button: PointerButton::Primary,
        });

        assert!(!runtime.bridge_mut().needs_animation());
        assert_status_contains(&runtime, "Paused | frame 43 | phase 0.24");
    }

    fn animation_test_bridge(
        state: AnimationState,
    ) -> impl radiant::runtime::RuntimeBridge<AnimationMessage> {
        radiant::app(state)
            .view(animation_view)
            .animation(|state| state.running)
            .on_frame(|| AnimationMessage::Frame)
            .update(|state, message| match message {
                AnimationMessage::Toggle => state.running = !state.running,
                AnimationMessage::Frame => state.tick(),
                AnimationMessage::Reset => state.reset(),
            })
            .into_bridge()
    }

    fn click_widget<Bridge>(runtime: &mut SurfaceRuntime<Bridge, AnimationMessage>, widget_id: u64)
    where
        Bridge: radiant::runtime::RuntimeBridge<AnimationMessage>,
    {
        let rect = runtime.layout().rects[&widget_id];
        let point = rect.center();
        runtime.dispatch_event(Event::PointerPress {
            position: point,
            button: PointerButton::Primary,
        });
        runtime.dispatch_event(Event::PointerRelease {
            position: point,
            button: PointerButton::Primary,
        });
    }

    fn assert_status_contains<Bridge>(
        runtime: &SurfaceRuntime<Bridge, AnimationMessage>,
        expected: &str,
    ) where
        Bridge: radiant::runtime::RuntimeBridge<AnimationMessage>,
    {
        let plan = runtime.paint_plan(&ThemeTokens::default());
        assert!(
            plan.primitives.iter().any(|primitive| matches!(
                primitive,
                PaintPrimitive::Text(text) if text.widget_id == 20 && text.text == expected
            )),
            "expected status text {expected:?}"
        );
    }

    fn assert_button_label<Bridge>(
        runtime: &SurfaceRuntime<Bridge, AnimationMessage>,
        widget_id: u64,
        expected: &str,
    ) where
        Bridge: radiant::runtime::RuntimeBridge<AnimationMessage>,
    {
        let plan = runtime.paint_plan(&ThemeTokens::default());
        assert!(
            plan.primitives.iter().any(|primitive| matches!(
                primitive,
                PaintPrimitive::Text(text) if text.widget_id == widget_id && text.text == expected
            )),
            "expected button {widget_id} label {expected:?}"
        );
    }
}
