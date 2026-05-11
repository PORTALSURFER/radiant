//! Frame-driven animation through the application builder.

use radiant::prelude::*;
use radiant::{
    gui::{
        paint::{BorderSides, FillRect, PaintFrame, Primitive, border_fill_rects},
        types::Rgba8,
    },
    layout::Rect,
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
            self.phase = ((self.frame % 240) as f32) / 240.0;
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
    let mut frame = PaintFrame::default();
    frame.primitives.reserve(13);
    push_rect(&mut frame, track, theme.surface_base);
    push_rect(
        &mut frame,
        inset(track, 1.0, 9.0),
        with_alpha(theme.grid_soft, 120),
    );
    for echo in visual.echoes {
        push_ratio_rect(
            &mut frame,
            inset(track, 0.0, 8.0),
            echo.start,
            echo.width,
            echo.color,
        );
    }
    push_ratio_rect(
        &mut frame,
        inset(track, 0.0, 5.0),
        visual.glow_start,
        visual.glow_width,
        visual.glow_color,
    );
    push_ratio_rect(
        &mut frame,
        inset(track, 0.0, 3.0),
        visual.core_start,
        visual.core_width,
        theme.highlight_orange,
    );
    push_ratio_rect(
        &mut frame,
        track,
        visual.marker_start,
        visual.marker_width,
        theme.text_primary,
    );
    frame.primitives.extend(
        border_fill_rects(track, theme.border_emphasis, 1.0, BorderSides::ALL)
            .into_iter()
            .map(Primitive::Rect),
    );
    frame
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
    echoes: [PulseEcho; 3],
    glow_start: f32,
    glow_width: f32,
    glow_color: Rgba8,
    core_start: f32,
    core_width: f32,
    marker_start: f32,
    marker_width: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PulseEcho {
    start: f32,
    width: f32,
    color: Rgba8,
}

impl PulseMeterVisual {
    fn resolve(phase: f32) -> Self {
        let phase = phase.clamp(0.0, 1.0);
        let pulse = (phase * std::f32::consts::TAU).sin() * 0.5 + 0.5;
        let marker_width = 0.01;
        let core_width = 0.045 + pulse * 0.02;
        let glow_width = core_width * 1.85;
        let marker_center = phase * (1.0 - marker_width) + marker_width * 0.5;
        let marker_start = (marker_center - marker_width * 0.5).clamp(0.0, 1.0 - marker_width);
        let core_start = (marker_center - core_width * 0.5).clamp(0.0, 1.0 - core_width);
        let glow_start = (marker_center - glow_width * 0.5).clamp(0.0, 1.0 - glow_width);

        Self {
            echoes: [
                Self::echo(marker_center, 0.10, 0.035, 55),
                Self::echo(marker_center, 0.18, 0.028, 38),
                Self::echo(marker_center, 0.26, 0.022, 26),
            ],
            glow_start,
            glow_width,
            glow_color: Rgba8 {
                r: 255,
                g: 104,
                b: 60,
                a: 96,
            },
            core_start,
            core_width,
            marker_start,
            marker_width,
        }
    }

    fn echo(marker_center: f32, delay: f32, width: f32, alpha: u8) -> PulseEcho {
        let center = (marker_center - delay).max(0.0);
        let start = (center - width * 0.5).clamp(0.0, 1.0 - width);
        PulseEcho {
            start,
            width,
            color: Rgba8 {
                r: 255,
                g: 112,
                b: 72,
                a: alpha,
            },
        }
    }
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

        assert!(peak.marker_start > start.marker_start + 0.20);
        assert!(far_edge.marker_start > peak.marker_start + 0.20);
        assert!(end.marker_start > far_edge.marker_start + 0.45);
        assert!(peak.core_width > start.core_width);
        assert!(peak.glow_width > peak.core_width);
        assert_eq!(start.marker_width, end.marker_width);
        assert!(far_edge.echoes[0].start > start.echoes[0].start + 0.20);
        assert!(start.echoes[0].color.a > start.echoes[1].color.a);
    }

    #[test]
    fn phase_meter_paints_track_trail_core_and_marker() {
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

        assert_eq!(fills.len(), 12);
        assert!(fills.iter().any(|fill| fill.rect.width() > 410.0));
        assert!(
            fills
                .iter()
                .any(|fill| fill.rect.width() > 25.0 && fill.rect.width() < 45.0)
        );
        assert!(fills.iter().any(|fill| fill.color.a < 70));
        assert!(fills.iter().any(|fill| fill.rect.width() < 6.0));
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
        assert_status_contains(&runtime, "Running | frame 43 | phase 0.18");

        click_widget(&mut runtime, 40);
        assert!(!runtime.bridge_mut().needs_animation());
        let outcome = runtime.drain_runtime_messages();
        assert_eq!(outcome.messages_dispatched, 0);
        assert_status_contains(&runtime, "Paused | frame 43 | phase 0.18");

        click_widget(&mut runtime, 41);
        assert!(!runtime.bridge_mut().needs_animation());
        assert_status_contains(&runtime, "Paused | frame 0 | phase 0.00");
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
}
