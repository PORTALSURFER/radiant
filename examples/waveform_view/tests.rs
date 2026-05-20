use super::*;
use radiant::{
    layout::LayoutOutput,
    runtime::{
        GpuSurfaceOverlay, PaintGpuSurface, RuntimeBridge, SurfaceRuntime, TransientOverlayContext,
    },
    theme::ThemeTokens,
    widgets::{Widget, WidgetInput},
};
use std::path::PathBuf;

#[path = "tests/gpu_projection.rs"]
mod gpu_projection;
#[path = "tests/interaction.rs"]
mod interaction;
#[path = "tests/rendering.rs"]
mod rendering;
#[path = "tests/runtime.rs"]
mod runtime;
#[path = "tests/source.rs"]
mod source;

fn synthetic_file(mono_samples: Vec<f32>, sample_rate: u32, channels: usize) -> WaveformFile {
    waveform_file_from_mono_samples(
        PathBuf::from("synthetic-test-waveform"),
        sample_rate,
        channels,
        mono_samples,
    )
}
