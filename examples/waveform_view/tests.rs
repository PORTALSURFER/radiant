use super::*;
use radiant::{
    layout::LayoutOutput,
    runtime::{GpuSurfaceOverlay, PaintGpuSurface, SurfaceRuntime, TransientOverlayContext},
    theme::ThemeTokens,
    widgets::{Widget, WidgetInput},
};

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

fn synthetic_file(mono_samples: Vec<f32>, sample_rate: u32, channels: usize) -> SignalSource {
    signal_source_from_samples(
        "synthetic-test-waveform".to_owned(),
        sample_rate,
        channels,
        mono_samples,
    )
}
