use super::{
    WAVEFORM_WIDTH,
    source::{MIN_VISIBLE_FRAMES, SignalSource, WaveformViewport},
};
use radiant::gui::types::Vector2;
use std::sync::Arc;

#[derive(Debug)]
pub(super) struct WaveformApp {
    pub(super) source: Arc<SignalSource>,
    pub(super) viewport: WaveformViewport,
    pub(super) zoom_anchor_ratio: f32,
    pub(super) playing: bool,
    pub(super) playhead_ratio: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum WaveformInteraction {
    Wheel { delta: Vector2, anchor_ratio: f32 },
    ScrollTo { offset_fraction: f32 },
    Zoom { factor: f32 },
    Pan { visible_fraction: f32 },
    TogglePlayback,
    Reset,
}

impl WaveformApp {
    pub(super) fn apply_interaction(&mut self, interaction: WaveformInteraction) {
        match interaction {
            WaveformInteraction::Wheel {
                delta,
                anchor_ratio,
            } => {
                self.zoom_anchor_ratio = anchor_ratio;
                self.handle_wheel(delta, anchor_ratio);
            }
            WaveformInteraction::ScrollTo { offset_fraction } => {
                self.set_offset_fraction(offset_fraction)
            }
            WaveformInteraction::Zoom { factor } => {
                self.zoom_around_anchor(factor, self.zoom_anchor_ratio)
            }
            WaveformInteraction::Pan { visible_fraction } => {
                self.pan_by_visible_fraction(visible_fraction)
            }
            WaveformInteraction::TogglePlayback => {
                self.playing = !self.playing;
            }
            WaveformInteraction::Reset => {
                self.viewport = WaveformViewport::full(self.source.frames);
                self.playhead_ratio = 0.5;
            }
        }
    }

    fn handle_wheel(&mut self, delta: Vector2, anchor_ratio: f32) {
        if delta.x.abs() > delta.y.abs() && delta.x.abs() > f32::EPSILON {
            self.pan_by_visible_fraction(delta.x / WAVEFORM_WIDTH as f32);
            return;
        }
        if delta.y < -f32::EPSILON {
            self.zoom_around_anchor(0.82, anchor_ratio);
        } else if delta.y > f32::EPSILON {
            self.zoom_around_anchor(1.22, anchor_ratio);
        }
    }

    pub(super) fn zoom_around_anchor(&mut self, factor: f32, anchor_ratio: f32) {
        let total = self.source.frames.max(1);
        self.viewport =
            self.viewport
                .zoom_around_anchor(total, MIN_VISIBLE_FRAMES, factor, anchor_ratio);
    }

    pub(super) fn pan_by_visible_fraction(&mut self, fraction: f32) {
        let total = self.source.frames.max(1);
        self.viewport = self
            .viewport
            .pan_by_visible_fraction(total, MIN_VISIBLE_FRAMES, fraction);
    }

    fn set_offset_fraction(&mut self, offset_fraction: f32) {
        let total = self.source.frames.max(1);
        self.viewport =
            self.viewport
                .with_offset_fraction(total, MIN_VISIBLE_FRAMES, offset_fraction);
    }
}
