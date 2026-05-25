use radiant::prelude::{ListSelectionController, ListSelectionModifiers};

#[path = "model/channel.rs"]
mod channel;

pub(crate) use channel::{MixerChannel, gain_for_ratio, ratio_for_gain, ratio_for_meter_db};

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
