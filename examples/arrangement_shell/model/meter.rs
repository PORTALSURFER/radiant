#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TrackMeter {
    pub(crate) track: usize,
    pub(crate) level: f32,
    pub(crate) peak: f32,
}

impl TrackMeter {
    pub(super) fn new(track: usize) -> Self {
        Self {
            track,
            level: 0.0,
            peak: 0.0,
        }
    }

    pub(super) fn tick(&mut self, frame: u64) {
        let phase = frame as f32 * (0.030 + self.track as f32 * 0.006);
        let pulse = (phase.sin() * 0.5 + 0.5).powf(1.8);
        let accent = if (frame + self.track as u64 * 9) % (42 + self.track as u64 * 4) < 5 {
            0.35
        } else {
            0.0
        };
        let target = (0.10 + pulse * 0.62 + accent).min(1.0);
        self.level = self.level * 0.70 + target * 0.30;
        self.peak = if target > self.peak {
            target
        } else {
            (self.peak - 0.012).max(self.level)
        };
    }
}
