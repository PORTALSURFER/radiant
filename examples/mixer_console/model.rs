use radiant::prelude::{ListSelectionController, ListSelectionModifiers};

pub(crate) const CHANNEL_COUNT: usize = 32;
pub(crate) const SEND_COUNT: usize = 3;
pub(crate) const GROUP_COUNT: usize = 4;
pub(crate) const MIN_GAIN_DB: f32 = -60.0;
pub(crate) const MAX_GAIN_DB: f32 = 6.0;
pub(crate) const CHANNEL_LABELS: [&str; CHANNEL_COUNT] = [
    "Kik", "Snr", "Hat", "Tom", "Rid", "Clp", "Shk", "Per", "Bass", "Sub", "Gtr1", "Gtr2", "Keys",
    "Pno", "Org", "Pad", "Ld1", "Ld2", "Plk", "Arp", "Vox1", "Vox2", "Bgv1", "Bgv2", "FX1", "FX2",
    "Amb", "Loop", "BusA", "BusB", "Print", "Ref",
];

#[derive(Clone, Debug)]
pub(crate) struct MixerState {
    pub(crate) running: bool,
    pub(crate) frame: u64,
    pub(crate) selected_channel: usize,
    pub(crate) selection: ListSelectionController,
    pub(crate) channels: [MixerChannel; CHANNEL_COUNT],
}

impl Default for MixerState {
    fn default() -> Self {
        let mut selection = ListSelectionController::new();
        selection.select(0, CHANNEL_COUNT, ListSelectionModifiers::new());
        let mut state = Self {
            running: true,
            frame: 0,
            selected_channel: 0,
            selection,
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
        self.selected_channel = 0;
        self.selection.clear();
        self.selection.select(
            self.selected_channel,
            CHANNEL_COUNT,
            ListSelectionModifiers::new(),
        );
        self.channels = std::array::from_fn(MixerChannel::new);
        self.tick();
    }

    pub(crate) fn selected(&self) -> MixerChannel {
        self.channels[self.selected_channel]
    }

    pub(crate) fn status(&self) -> String {
        let selected = self.selected();
        let transport = if self.running { "running" } else { "paused" };
        let selected_count = self.selection.selected_indices().len().max(1);
        format!(
            "{transport} | {selected_count} selected | {} | group {} | fader {:+.1} dB | send A {:.0}% | meter {:+.1} dB | synthetic GUI data",
            selected.label,
            selected.group() + 1,
            selected.controls.gain_db,
            selected.controls.sends[0] * 100.0,
            selected.meter.meter_db
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MixerChannel {
    pub(crate) id: usize,
    pub(crate) label: &'static str,
    pub(crate) controls: MixerChannelControls,
    pub(crate) meter: MixerMeterState,
    pub(crate) flags: MixerChannelFlags,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MixerChannelControls {
    pub(crate) gain_db: f32,
    pub(crate) pan: f32,
    pub(crate) sends: [f32; SEND_COUNT],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MixerMeterState {
    pub(crate) meter_db: f32,
    pub(crate) peak_db: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MixerChannelFlags {
    pub(crate) muted: bool,
    pub(crate) solo: bool,
    pub(crate) armed: bool,
}

impl MixerChannel {
    fn new(id: usize) -> Self {
        Self {
            id,
            label: CHANNEL_LABELS[id],
            controls: MixerChannelControls {
                gain_db: default_gain(id),
                pan: default_pan(id),
                sends: default_sends(id),
            },
            meter: MixerMeterState {
                meter_db: MIN_GAIN_DB,
                peak_db: MIN_GAIN_DB,
            },
            flags: MixerChannelFlags {
                muted: false,
                solo: false,
                armed: id == 0,
            },
        }
    }

    fn tick(&mut self, frame: u64) {
        let level = synthetic_level(frame, self.id, self.controls.gain_db, self.flags.muted);
        let target_db = level_to_db(level);
        self.meter.meter_db = self.meter.meter_db * 0.72 + target_db * 0.28;
        self.meter.peak_db = if target_db > self.meter.peak_db {
            target_db
        } else {
            (self.meter.peak_db - 0.42).max(self.meter.meter_db)
        };
    }

    pub(crate) fn set_gain_from_ratio(&mut self, ratio: f32) {
        self.set_gain_from_db(gain_for_ratio(ratio));
        if ratio <= 0.001 {
            self.silence_meter();
        }
    }

    pub(crate) fn set_gain_from_db(&mut self, db: f32) {
        self.controls.gain_db = db.clamp(MIN_GAIN_DB, MAX_GAIN_DB);
        if self.controls.gain_db <= MIN_GAIN_DB + 0.001 {
            self.silence_meter();
        }
    }

    fn silence_meter(&mut self) {
        self.meter.meter_db = MIN_GAIN_DB;
        self.meter.peak_db = MIN_GAIN_DB;
    }

    pub(crate) fn group(&self) -> usize {
        self.id / (CHANNEL_COUNT / GROUP_COUNT)
    }

    pub(crate) fn is_visually_dimmed_by_solo(&self, solo_active: bool) -> bool {
        solo_active && !self.flags.solo
    }
}

fn default_gain(channel: usize) -> f32 {
    -5.0 - (channel % 8) as f32 * 1.4 - (channel / 8) as f32 * 0.8
}

fn default_pan(channel: usize) -> f32 {
    const PANS: [f32; 8] = [-0.55, -0.28, -0.08, 0.0, 0.10, 0.24, 0.42, 0.58];
    PANS[channel % PANS.len()]
}

fn default_sends(channel: usize) -> [f32; SEND_COUNT] {
    [
        0.14 + (channel % 5) as f32 * 0.035,
        0.08 + (channel % 7) as f32 * 0.025,
        0.05 + (channel % 4) as f32 * 0.045,
    ]
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

pub(crate) fn gain_for_ratio(ratio: f32) -> f32 {
    MIN_GAIN_DB + (MAX_GAIN_DB - MIN_GAIN_DB) * ratio.clamp(0.0, 1.0)
}

fn db_to_linear(db: f32) -> f32 {
    10.0_f32.powf(db.clamp(MIN_GAIN_DB, MAX_GAIN_DB) / 20.0)
}
