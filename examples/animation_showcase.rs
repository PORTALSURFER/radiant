//! Frame-driven animation through the application builder.

use radiant::prelude::*;

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
    let visual = PulseMeterVisual::resolve(phase);
    stack([
        row(Vec::<View<AnimationMessage>>::new())
            .style(WidgetStyle {
                tone: WidgetTone::Neutral,
                prominence: WidgetProminence::Subtle,
            })
            .id(30)
            .height(42.0)
            .fill_width(),
        row([
            row(Vec::<View<AnimationMessage>>::new())
                .primary()
                .id(31)
                .width_percent(visual.fill_width)
                .height(42.0),
            spacer().fill_width(),
        ])
        .id(32)
        .spacing(0.0)
        .height(42.0)
        .fill_width(),
        row([
            spacer().width_percent(visual.glow_offset),
            row(Vec::<View<AnimationMessage>>::new())
                .style(WidgetStyle {
                    tone: WidgetTone::Warning,
                    prominence: WidgetProminence::Strong,
                })
                .id(33)
                .width_percent(visual.glow_width)
                .height(34.0),
            spacer().fill_width(),
        ])
        .id(34)
        .spacing(0.0)
        .padding_y(4.0)
        .height(42.0)
        .fill_width(),
        row([
            spacer().width_percent(visual.marker_offset),
            row(Vec::<View<AnimationMessage>>::new())
                .style(WidgetStyle {
                    tone: WidgetTone::Neutral,
                    prominence: WidgetProminence::Strong,
                })
                .id(35)
                .width_percent(visual.marker_width)
                .height(42.0),
            spacer().fill_width(),
        ])
        .id(36)
        .spacing(0.0)
        .height(42.0)
        .fill_width(),
    ])
    .height(42.0)
    .key("phase-meter")
    .fill_width()
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PulseMeterVisual {
    fill_width: f32,
    glow_offset: f32,
    glow_width: f32,
    marker_offset: f32,
    marker_width: f32,
}

impl PulseMeterVisual {
    fn resolve(phase: f32) -> Self {
        let phase = phase.clamp(0.0, 1.0);
        let pulse = (phase * std::f32::consts::TAU).sin() * 0.5 + 0.5;
        let fill_width = (phase * 0.96).clamp(0.0, 0.96);
        let marker_width = 0.018 + pulse * 0.008;
        let marker_offset = (phase * (1.0 - marker_width)).clamp(0.0, 1.0 - marker_width);
        let glow_width = 0.08 + pulse * 0.08;
        let marker_center = marker_offset + marker_width * 0.5;
        let glow_offset = (marker_center - glow_width * 0.5).clamp(0.0, 1.0 - glow_width);

        Self {
            fill_width,
            glow_offset,
            glow_width,
            marker_offset,
            marker_width,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::{
        layout::{Point, Rect, Vector2},
        prelude::IntoView,
        runtime::{Event, PaintPrimitive, SurfaceRuntime},
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

        assert!(peak.fill_width > start.fill_width + 0.2);
        assert!(end.fill_width > far_edge.fill_width + 0.4);
        assert!(peak.glow_width > start.glow_width);
        assert!(peak.marker_width > start.marker_width);
        assert!(far_edge.marker_offset > start.marker_offset + 0.45);
        assert!(end.marker_offset > far_edge.marker_offset + 0.45);
    }

    #[test]
    fn phase_meter_paints_track_fill_glow_and_marker() {
        let surface = phase_meter(0.5).into_surface();
        let frame = surface.frame(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(420.0, 48.0)),
            &Default::default(),
        );
        let fills: Vec<_> = frame
            .paint_plan
            .primitives
            .iter()
            .filter_map(|primitive| match primitive {
                PaintPrimitive::FillRect(fill) if [30, 31, 33, 35].contains(&fill.widget_id) => {
                    Some((fill.widget_id, fill))
                }
                _ => None,
            })
            .collect();

        assert_eq!(fills.len(), 4);
        assert!(
            fills
                .iter()
                .any(|(id, fill)| *id == 30 && fill.rect.width() == 420.0)
        );
        assert!(
            fills
                .iter()
                .any(|(id, fill)| *id == 31 && fill.rect.width() > 180.0)
        );
        assert!(
            fills
                .iter()
                .any(|(id, fill)| *id == 33 && fill.rect.width() > 30.0)
        );
        assert!(
            fills
                .iter()
                .any(|(id, fill)| *id == 35 && fill.rect.width() >= 4.0)
        );
        assert_ne!(fills[0].1.color, fills[1].1.color);
    }

    #[test]
    fn animation_controls_pause_resume_and_reset_state() {
        let bridge = radiant::app(AnimationState {
            running: true,
            frame: 42,
            phase: 0.5,
        })
        .view(animation_view)
        .animation(|state| state.running)
        .on_frame(|| AnimationMessage::Frame)
        .update(|state, message| match message {
            AnimationMessage::Toggle => state.running = !state.running,
            AnimationMessage::Frame => state.tick(),
            AnimationMessage::Reset => state.reset(),
        })
        .into_bridge();
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(520.0, 220.0));

        click_widget(&mut runtime, 40);
        assert_status_contains(&runtime, "Paused | frame 42 | phase 0.50");

        click_widget(&mut runtime, 41);
        assert_status_contains(&runtime, "Paused | frame 0 | phase 0.00");

        click_widget(&mut runtime, 40);
        assert_status_contains(&runtime, "Running | frame 0 | phase 0.00");
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
