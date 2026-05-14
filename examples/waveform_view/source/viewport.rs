use super::MIN_VISIBLE_FRAMES;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WaveformViewport {
    pub(crate) start: usize,
    pub(crate) end: usize,
}

impl WaveformViewport {
    pub(crate) fn full(frames: usize) -> Self {
        Self {
            start: 0,
            end: frames.max(1),
        }
    }

    pub(crate) fn visible_frames(self) -> usize {
        self.end.saturating_sub(self.start).max(1)
    }

    pub(crate) fn visible_seconds(self, sample_rate: u32) -> f32 {
        self.visible_frames() as f32 / sample_rate.max(1) as f32
    }

    pub(crate) fn visible_fraction(self, total_frames: usize) -> f32 {
        self.visible_frames() as f32 / total_frames.max(1) as f32
    }

    pub(crate) fn offset_fraction(self, total_frames: usize) -> f32 {
        let total_frames = total_frames.max(1);
        let free_frames = total_frames.saturating_sub(self.visible_frames());
        if free_frames == 0 {
            0.0
        } else {
            self.start as f32 / free_frames as f32
        }
    }

    pub(crate) fn is_zoomed_in(self, total_frames: usize) -> bool {
        self.visible_frames() < total_frames.max(1)
    }

    pub(crate) fn clamp(self, total_frames: usize) -> Self {
        let total_frames = total_frames.max(1);
        let visible = self
            .visible_frames()
            .clamp(MIN_VISIBLE_FRAMES.min(total_frames), total_frames);
        let start = self.start.min(total_frames.saturating_sub(visible));
        Self {
            start,
            end: start + visible,
        }
    }
}
