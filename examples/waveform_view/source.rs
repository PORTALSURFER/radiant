use radiant::runtime::GpuSignalSummary;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::PathBuf,
    sync::Arc,
};

#[path = "source/bands.rs"]
mod bands;
use bands::split_frequency_bands;
pub(super) use bands::{downmix_to_mono, interleaved_band_samples};

#[path = "source/summary.rs"]
mod summary;
pub(super) use summary::WaveformBand;
#[cfg(test)]
pub(super) use summary::{WaveformSummary, band_stats};

#[path = "source/viewport.rs"]
mod viewport;
pub(super) use viewport::WaveformViewport;

#[cfg(test)]
#[path = "source/raster.rs"]
mod raster;
#[cfg(test)]
pub(super) use raster::render_waveform_image;

const WAVEFORM_PATH_ENV_VAR: &str = "RADIANT_WAVEFORM_PATH";
pub(super) const MIN_VISIBLE_FRAMES: usize = 256;
pub(super) const BAND_COUNT: usize = 4;
const SYNTHETIC_SAMPLE_RATE: u32 = 48_000;
const SYNTHETIC_SECONDS: usize = 4;

#[derive(Clone, Debug)]
pub(super) struct WaveformFile {
    pub(super) path: PathBuf,
    pub(super) sample_rate: u32,
    pub(super) channels: usize,
    pub(super) frames: usize,
    #[cfg(test)]
    pub(super) mono_samples: Vec<f32>,
    #[cfg(test)]
    pub(super) mono_summary: WaveformSummary,
    #[cfg(test)]
    pub(super) bands: [WaveformBand; BAND_COUNT],
    pub(super) gpu_signal_summary: Arc<GpuSignalSummary>,
}

impl WaveformFile {
    pub(super) fn path_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.path.hash(&mut hasher);
        self.frames.hash(&mut hasher);
        self.sample_rate.hash(&mut hasher);
        hasher.finish()
    }
}

pub(super) fn resolve_sample_path() -> Option<PathBuf> {
    std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .or_else(|| std::env::var_os(WAVEFORM_PATH_ENV_VAR).map(PathBuf::from))
}

pub(super) fn load_waveform_source(path: Option<PathBuf>) -> Result<WaveformFile, String> {
    match path {
        Some(path) => load_waveform_file(path),
        None => Ok(synthetic_waveform_file()),
    }
}

fn synthetic_waveform_file() -> WaveformFile {
    let frames = SYNTHETIC_SAMPLE_RATE as usize * SYNTHETIC_SECONDS;
    let samples = (0..frames)
        .map(|frame| {
            let t = frame as f32 / SYNTHETIC_SAMPLE_RATE as f32;
            let envelope = (1.0 - t / SYNTHETIC_SECONDS as f32).clamp(0.18, 1.0);
            let low = (std::f32::consts::TAU * 72.0 * t).sin() * 0.48;
            let mid = (std::f32::consts::TAU * 220.0 * t).sin() * 0.24;
            let high = (std::f32::consts::TAU * 1_760.0 * t).sin() * 0.1;
            ((low + mid + high) * envelope).clamp(-1.0, 1.0)
        })
        .collect::<Vec<_>>();
    waveform_file_from_mono_samples(
        PathBuf::from("synthetic-waveform"),
        SYNTHETIC_SAMPLE_RATE,
        1,
        samples,
    )
}

fn load_waveform_file(path: PathBuf) -> Result<WaveformFile, String> {
    let mut reader =
        hound::WavReader::open(&path).map_err(|err| format!("failed to open WAV: {err}"))?;
    let spec = reader.spec();
    let channels = usize::from(spec.channels).max(1);
    let samples = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .map(|sample| {
                sample
                    .map(|value| value.clamp(-1.0, 1.0))
                    .map_err(|err| format!("failed to read float sample: {err}"))
            })
            .collect::<Result<Vec<_>, _>>()?,
        hound::SampleFormat::Int if spec.bits_per_sample <= 16 => {
            let max =
                ((1_i32 << (u32::from(spec.bits_per_sample).saturating_sub(1))) - 1).max(1) as f32;
            reader
                .samples::<i16>()
                .map(|sample| {
                    sample
                        .map(|value| (f32::from(value) / max).clamp(-1.0, 1.0))
                        .map_err(|err| format!("failed to read integer sample: {err}"))
                })
                .collect::<Result<Vec<_>, _>>()?
        }
        hound::SampleFormat::Int => {
            let max =
                ((1_i64 << (u32::from(spec.bits_per_sample).saturating_sub(1))) - 1).max(1) as f32;
            reader
                .samples::<i32>()
                .map(|sample| {
                    sample
                        .map(|value| ((value as f32) / max).clamp(-1.0, 1.0))
                        .map_err(|err| format!("failed to read integer sample: {err}"))
                })
                .collect::<Result<Vec<_>, _>>()?
        }
    };
    if samples.is_empty() {
        return Err(String::from("WAV contains no samples"));
    }

    let frames = samples.len() / channels;
    let mono_samples = downmix_to_mono(&samples, channels, frames);
    if mono_samples.is_empty() {
        return Err(String::from("WAV contains no complete frames"));
    }
    let bands = split_frequency_bands(&mono_samples, spec.sample_rate);
    let gpu_signal_samples = interleaved_band_samples(&bands);
    let gpu_signal_summary = Arc::new(GpuSignalSummary::from_interleaved_samples(
        &gpu_signal_samples,
        frames,
        BAND_COUNT,
    ));
    #[cfg(test)]
    let mono_summary = WaveformSummary::from_samples(&mono_samples);

    Ok(WaveformFile {
        path,
        sample_rate: spec.sample_rate,
        channels,
        frames,
        #[cfg(test)]
        mono_samples,
        #[cfg(test)]
        mono_summary,
        #[cfg(test)]
        bands,
        gpu_signal_summary,
    })
}

pub(super) fn waveform_file_from_mono_samples(
    path: PathBuf,
    sample_rate: u32,
    channels: usize,
    mono_samples: Vec<f32>,
) -> WaveformFile {
    let bands = split_frequency_bands(&mono_samples, sample_rate);
    let gpu_signal_samples = interleaved_band_samples(&bands);
    let gpu_signal_summary = Arc::new(GpuSignalSummary::from_interleaved_samples(
        &gpu_signal_samples,
        mono_samples.len(),
        BAND_COUNT,
    ));
    WaveformFile {
        path,
        sample_rate,
        channels,
        frames: mono_samples.len(),
        #[cfg(test)]
        mono_summary: WaveformSummary::from_samples(&mono_samples),
        #[cfg(test)]
        bands,
        #[cfg(test)]
        mono_samples,
        gpu_signal_summary,
    }
}
