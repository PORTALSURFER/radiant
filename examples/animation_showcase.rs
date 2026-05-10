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
            self.phase = ((self.frame % 120) as f32) / 119.0;
        }
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
            AnimationMessage::Reset => {
                state.frame = 0;
                state.phase = 0.0;
            }
        })
        .run()
}

fn animation_view(state: &mut AnimationState) -> View<AnimationMessage> {
    column([
        text("Animation Showcase").height(28.0).fill_width(),
        text(state.status()).height(26.0).fill_width(),
        phase_meter(state.phase),
        row([
            button(if state.running { "Pause" } else { "Run" })
                .primary()
                .message(AnimationMessage::Toggle)
                .width(100.0)
                .height(32.0),
            button("Reset")
                .subtle()
                .message(AnimationMessage::Reset)
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
            .height(36.0)
            .fill_width(),
        row([
            row(Vec::<View<AnimationMessage>>::new())
                .primary()
                .id(31)
                .width(visual.fill_width)
                .height(36.0),
            spacer().fill_width(),
        ])
        .id(32)
        .spacing(0.0)
        .height(36.0)
        .fill_width(),
        row([
            spacer().width(visual.glow_offset),
            row(Vec::<View<AnimationMessage>>::new())
                .style(WidgetStyle {
                    tone: WidgetTone::Warning,
                    prominence: WidgetProminence::Strong,
                })
                .id(33)
                .width(visual.glow_width)
                .height(30.0),
            spacer().fill_width(),
        ])
        .id(34)
        .spacing(0.0)
        .padding_y(3.0)
        .height(36.0)
        .fill_width(),
        row([
            spacer().width(visual.marker_offset),
            row(Vec::<View<AnimationMessage>>::new())
                .style(WidgetStyle {
                    tone: WidgetTone::Neutral,
                    prominence: WidgetProminence::Strong,
                })
                .id(35)
                .width(visual.marker_width)
                .height(36.0),
            spacer().fill_width(),
        ])
        .id(36)
        .spacing(0.0)
        .height(36.0)
        .fill_width(),
    ])
    .height(36.0)
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
        let eased = smoothstep(phase);
        let pulse = (phase * std::f32::consts::TAU).sin() * 0.5 + 0.5;
        let track_width = 388.0;
        let fill_width = (64.0 + eased * 304.0).clamp(32.0, track_width);
        let marker_width = 5.0 + pulse * 7.0;
        let marker_offset = (fill_width - marker_width * 0.5).clamp(0.0, track_width);
        let glow_width = 56.0 + pulse * 44.0;
        let glow_offset = (marker_offset + marker_width * 0.5 - glow_width * 0.5)
            .clamp(0.0, track_width - glow_width);

        Self {
            fill_width,
            glow_offset,
            glow_width,
            marker_offset,
            marker_width,
        }
    }
}

fn smoothstep(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    value * value * (3.0 - 2.0 * value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::{
        layout::{Point, Rect, Vector2},
        prelude::IntoView,
        runtime::PaintPrimitive,
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
        let mid = PulseMeterVisual::resolve(0.5);
        let end = PulseMeterVisual::resolve(1.0);

        assert!(mid.fill_width > start.fill_width + 100.0);
        assert!(end.fill_width > mid.fill_width + 100.0);
        assert!(mid.glow_width >= 56.0);
        assert!(mid.marker_width >= 5.0);
        assert!(mid.marker_offset > mid.glow_offset);
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
                .any(|(id, fill)| *id == 31 && fill.rect.width() > 180.0)
        );
        assert!(
            fills
                .iter()
                .any(|(id, fill)| *id == 33 && fill.rect.width() > 56.0)
        );
        assert!(
            fills
                .iter()
                .any(|(id, fill)| *id == 35 && fill.rect.width() >= 5.0)
        );
        assert_ne!(fills[0].1.color, fills[1].1.color);
    }
}
