use radiant::runtime::GpuSignalSummary;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::PathBuf,
    sync::Arc,
};

#[path = "source/bands.rs"]
mod bands;
#[cfg(test)]
pub(super) use bands::{downmix_to_mono, interleaved_band_samples};

#[path = "source/loading.rs"]
mod loading;
#[cfg(test)]
pub(super) use loading::waveform_file_from_mono_samples;
pub(super) use loading::{load_waveform_source, resolve_sample_path};

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

pub(super) const MIN_VISIBLE_FRAMES: usize = 256;
pub(super) const BAND_COUNT: usize = 4;

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
