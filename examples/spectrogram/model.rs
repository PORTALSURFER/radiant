use std::collections::VecDeque;

pub(super) const COLUMNS: usize = 96;
pub(super) const BINS: usize = 48;
pub(super) const MIN_FREQ_HZ: f32 = 40.0;
pub(super) const MAX_FREQ_HZ: f32 = 18_000.0;
pub(super) const DATA_SOURCE_NOTE: &str = "without_dsp";

#[derive(Clone, Debug)]
pub(super) struct SpectrogramState {
    pub(super) running: bool,
    pub(super) frame: u64,
    pub(super) intensity: f32,
    pub(super) speed: u32,
    pub(super) columns: VecDeque<SpectralColumn>,
}

impl Default for SpectrogramState {
    fn default() -> Self {
        let mut state = Self {
            running: true,
            frame: 0,
            intensity: 0.82,
            speed: 2,
            columns: VecDeque::with_capacity(COLUMNS),
        };
        for _ in 0..COLUMNS {
            state.push_next_column();
        }
        state
    }
}

impl SpectrogramState {
    pub(super) fn tick(&mut self) {
        if !self.running {
            return;
        }
        for _ in 0..self.speed {
            self.push_next_column();
        }
    }

    pub(super) fn reset(&mut self) {
        self.frame = 0;
        self.columns.clear();
        for _ in 0..COLUMNS {
            self.push_next_column();
        }
    }

    pub(super) fn status(&self) -> String {
        let transport = if self.running { "running" } else { "paused" };
        format!(
            "{transport} | frame {} | speed {}x | synthetic GUI data",
            self.frame, self.speed
        )
    }

    fn push_next_column(&mut self) {
        self.frame = self.frame.saturating_add(1);
        if self.columns.len() == COLUMNS {
            self.columns.pop_front();
        }
        self.columns
            .push_back(generate_spectral_column(self.frame, self.intensity));
    }

    fn adjust_intensity(&mut self, delta: f32) {
        self.intensity = (self.intensity + delta).clamp(0.35, 1.35);
        self.reset();
    }

    fn cycle_speed(&mut self) {
        self.speed = match self.speed {
            1 => 2,
            2 => 4,
            _ => 1,
        };
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct SpectralColumn {
    pub(super) bins: Vec<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SpectrogramMessage {
    Frame,
    ToggleRun,
    Reset,
    IncreaseIntensity,
    DecreaseIntensity,
    CycleSpeed,
}

pub(super) fn update(state: &mut SpectrogramState, message: SpectrogramMessage) {
    match message {
        SpectrogramMessage::Frame => state.tick(),
        SpectrogramMessage::ToggleRun => {
            state.running = !state.running;
        }
        SpectrogramMessage::Reset => {
            state.running = false;
            state.reset();
        }
        SpectrogramMessage::IncreaseIntensity => state.adjust_intensity(0.08),
        SpectrogramMessage::DecreaseIntensity => state.adjust_intensity(-0.08),
        SpectrogramMessage::CycleSpeed => state.cycle_speed(),
    }
}

pub(super) fn generate_spectral_column(frame: u64, intensity: f32) -> SpectralColumn {
    let mut bins = Vec::with_capacity(BINS);
    let frame_phase = frame as f32 * 0.043;
    let low_sweep = 0.20 + 0.14 * (frame_phase * 0.71).sin();
    let high_sweep = 0.68 + 0.18 * (frame_phase * 0.43).cos();

    for bin in 0..BINS {
        let ratio = bin as f32 / (BINS - 1) as f32;
        bins.push(spectral_bin_energy(
            frame, bin as u64, ratio, low_sweep, high_sweep, intensity,
        ));
    }

    SpectralColumn { bins }
}

fn spectral_bin_energy(
    frame: u64,
    bin: u64,
    ratio: f32,
    low_sweep: f32,
    high_sweep: f32,
    intensity: f32,
) -> f32 {
    let low_band = gaussian(ratio, low_sweep, 0.055);
    let high_band = gaussian(ratio, high_sweep, 0.075) * 0.72;
    let frame_phase = frame as f32 * 0.043;
    let harmonic = ((ratio * 18.0 + frame_phase * 2.4).sin() * 0.5 + 0.5) * 0.24;
    let noise = deterministic_noise(frame, bin) * 0.22;
    let rolloff = (1.0 - ratio * 0.42).max(0.28);

    ((low_band + high_band + harmonic + noise) * rolloff * intensity).clamp(0.0, 1.0)
}

fn gaussian(value: f32, center: f32, width: f32) -> f32 {
    let delta = value - center;
    (-(delta * delta) / (2.0 * width * width)).exp()
}

fn deterministic_noise(frame: u64, bin: u64) -> f32 {
    let mut value = frame
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(bin.wrapping_mul(0xBF58_476D_1CE4_E5B9));
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^= value >> 31;
    ((value >> 40) as f32) / ((1_u64 << 24) as f32)
}
