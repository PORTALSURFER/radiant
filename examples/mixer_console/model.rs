pub(crate) const CHANNEL_COUNT: usize = 8;
pub(crate) const MIN_GAIN_DB: f32 = -60.0;
const MAX_GAIN_DB: f32 = 6.0;
pub(crate) const CHANNEL_LABELS: [&str; CHANNEL_COUNT] =
    ["Kick", "Snare", "Hat", "Bass", "Keys", "Pad", "Lead", "Vox"];

const DEFAULT_GAINS: [f32; CHANNEL_COUNT] = [-5.0, -7.5, -12.0, -8.0, -9.0, -14.0, -10.0, -6.5];
const DEFAULT_PANS: [f32; CHANNEL_COUNT] = [0.0, 0.0, -0.34, -0.08, 0.22, 0.38, -0.18, 0.06];

#[derive(Clone, Debug)]
pub(crate) struct MixerState {
    pub(crate) running: bool,
    pub(crate) frame: u64,
    pub(crate) selected_channel: usize,
    pub(crate) channels: [MixerChannel; CHANNEL_COUNT],
}

impl Default for MixerState {
    fn default() -> Self {
        let mut state = Self {
            running: true,
            frame: 0,
            selected_channel: 0,
            channels: std::array::from_fn(MixerChannel::new),
        };
        state.tick();
        state
    }
}

impl MixerState {
    pub(crate) fn tick(&mut self) {
        if !self.running {
            return;
        }
        self.frame = self.frame.saturating_add(1);
        for channel in &mut self.channels {
            channel.tick(self.frame);
        }
    }

    pub(crate) fn reset(&mut self) {
        self.frame = 0;
        self.running = true;
        self.channels = std::array::from_fn(MixerChannel::new);
        self.tick();
    }

    pub(crate) fn selected(&self) -> MixerChannel {
        self.channels[self.selected_channel]
    }

    pub(crate) fn status(&self) -> String {
        let selected = self.selected();
        let transport = if self.running { "running" } else { "paused" };
        format!(
            "{transport} | {} | fader {:+.1} dB | meter {:+.1} dB | synthetic GUI data",
            selected.label, selected.gain_db, selected.meter_db
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MixerChannel {
    pub(crate) id: usize,
    pub(crate) label: &'static str,
    pub(crate) gain_db: f32,
    pub(crate) pan: f32,
    pub(crate) meter_db: f32,
    pub(crate) peak_db: f32,
    pub(crate) muted: bool,
    pub(crate) solo: bool,
    pub(crate) armed: bool,
}

impl MixerChannel {
    fn new(id: usize) -> Self {
        Self {
            id,
            label: CHANNEL_LABELS[id],
            gain_db: DEFAULT_GAINS[id],
            pan: DEFAULT_PANS[id],
            meter_db: MIN_GAIN_DB,
            peak_db: MIN_GAIN_DB,
            muted: false,
            solo: false,
            armed: id == 0,
        }
    }

    fn tick(&mut self, frame: u64) {
        let level = synthetic_level(frame, self.id, self.gain_db, self.muted);
        let target_db = level_to_db(level);
        self.meter_db = self.meter_db * 0.72 + target_db * 0.28;
        self.peak_db = if target_db > self.peak_db {
            target_db
        } else {
            (self.peak_db - 0.42).max(self.meter_db)
        };
    }

    pub(crate) fn set_gain_from_ratio(&mut self, ratio: f32) {
        self.gain_db = gain_for_ratio(ratio);
        if ratio <= 0.001 {
            self.meter_db = MIN_GAIN_DB;
            self.peak_db = MIN_GAIN_DB;
        }
    }

    pub(crate) fn gain_ratio(&self) -> f32 {
        ratio_for_gain(self.gain_db)
    }

    pub(crate) fn is_visually_dimmed_by_solo(&self, solo_active: bool) -> bool {
        solo_active && !self.solo
    }
}

fn synthetic_level(frame: u64, channel: usize, gain_db: f32, muted: bool) -> f32 {
    if muted {
        return 0.0;
    }
    let phase = frame as f32 * (0.034 + channel as f32 * 0.004);
    let pulse = (phase.sin() * 0.5 + 0.5).powf(1.7);
    let wobble = ((phase * 0.37 + channel as f32).cos() * 0.5 + 0.5) * 0.32;
    let transient = if (frame + channel as u64 * 11) % (34 + channel as u64 * 3) < 4 {
        0.38
    } else {
        0.0
    };
    let fader_gain = db_to_linear(gain_db);
    ((0.08 + pulse * 0.50 + wobble + transient).min(1.0) * fader_gain).min(1.0)
}

fn level_to_db(level: f32) -> f32 {
    if level <= 0.001 {
        MIN_GAIN_DB
    } else {
        (20.0 * level.clamp(0.001, 1.0).log10()).clamp(MIN_GAIN_DB, MAX_GAIN_DB)
    }
}

pub(crate) fn ratio_for_meter_db(db: f32) -> f32 {
    ((db.clamp(MIN_GAIN_DB, 0.0) - MIN_GAIN_DB) / (0.0 - MIN_GAIN_DB)).clamp(0.0, 1.0)
}

pub(crate) fn ratio_for_gain(db: f32) -> f32 {
    ((db.clamp(MIN_GAIN_DB, MAX_GAIN_DB) - MIN_GAIN_DB) / (MAX_GAIN_DB - MIN_GAIN_DB))
        .clamp(0.0, 1.0)
}

fn gain_for_ratio(ratio: f32) -> f32 {
    MIN_GAIN_DB + (MAX_GAIN_DB - MIN_GAIN_DB) * ratio.clamp(0.0, 1.0)
}

fn db_to_linear(db: f32) -> f32 {
    10.0_f32.powf(db.clamp(MIN_GAIN_DB, MAX_GAIN_DB) / 20.0)
}
