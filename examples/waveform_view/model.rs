use super::{
    WAVEFORM_WIDTH,
    source::{MIN_VISIBLE_FRAMES, WaveformFile, WaveformViewport},
};
use radiant::gui::types::Vector2;
use std::sync::Arc;

#[derive(Debug)]
pub(super) struct WaveformApp {
    pub(super) file: Arc<WaveformFile>,
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
                self.viewport = WaveformViewport::full(self.file.frames);
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
        let total = self.file.frames.max(1);
        let current = self.viewport.clamp(total);
        let anchor_ratio = anchor_ratio.clamp(0.0, 1.0);
        let anchor_frame = current.start as f32 + current.visible_frames() as f32 * anchor_ratio;
        let next_visible = ((current.visible_frames() as f32) * factor)
            .round()
            .clamp(MIN_VISIBLE_FRAMES.min(total) as f32, total as f32)
            as usize;
        let start = (anchor_frame - next_visible as f32 * anchor_ratio)
            .round()
            .max(0.0) as usize;
        self.viewport = WaveformViewport {
            start,
            end: start + next_visible,
        }
        .clamp(total);
    }

    pub(super) fn pan_by_visible_fraction(&mut self, fraction: f32) {
        let total = self.file.frames.max(1);
        let current = self.viewport.clamp(total);
        let delta = (current.visible_frames() as f32 * fraction).round() as isize;
        let start = current.start.saturating_add_signed(delta);
        self.viewport = WaveformViewport {
            start,
            end: start + current.visible_frames(),
        }
        .clamp(total);
    }

    fn set_offset_fraction(&mut self, offset_fraction: f32) {
        let total = self.file.frames.max(1);
        let current = self.viewport.clamp(total);
        let visible = current.visible_frames();
        let free_frames = total.saturating_sub(visible);
        let start = (free_frames as f32 * offset_fraction.clamp(0.0, 1.0)).round() as usize;
        self.viewport = WaveformViewport {
            start,
            end: start + visible,
        }
        .clamp(total);
    }
}
