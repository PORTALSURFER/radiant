use super::{SelectedSignalLevel, SignalRenderSource};
use crate::gui_runtime::native_vello::generic_runtime::gpu_surface::gpu_surface_types::{
    SignalBodyCacheKey, SignalUniforms,
};
use crate::runtime::{GpuSignalGainPreview, GpuSurfaceContent};

pub(super) fn signal_uniforms(
    source: &SignalRenderSource,
    selected: &SelectedSignalLevel<'_>,
    body_key: SignalBodyCacheKey,
) -> SignalUniforms {
    let gain_uniforms = signal_gain_preview_uniforms(source.gain_preview);
    SignalUniforms {
        dest: [0.0, 0.0, body_key.width as f32, body_key.height as f32],
        frame_range: [
            source.shape.frame_range[0],
            source.shape.frame_range[1],
            source.shape.frames as f32,
            source.shape.band_count as f32,
        ],
        summary_meta: [
            selected.level.bucket_frames as f32,
            (selected.level.buckets.len() / source.shape.band_count) as f32,
            selected.index as f32,
            0.0,
        ],
        gain_preview_a: gain_uniforms[0],
        gain_preview_b: gain_uniforms[1],
        gain_preview_c: gain_uniforms[2],
        target_size: [body_key.width as f32, body_key.height as f32],
        cursor_ratio: -1.0,
        cursor_width: 1.0,
        cursor_color: [1.0, 1.0, 1.0, 0.92],
    }
}

pub(super) fn signal_gain_preview(content: &GpuSurfaceContent) -> Option<GpuSignalGainPreview> {
    match content {
        GpuSurfaceContent::SignalSummaryBands { gain_preview, .. } => *gain_preview,
        _ => None,
    }
}

fn signal_gain_preview_uniforms(preview: Option<GpuSignalGainPreview>) -> [[f32; 4]; 3] {
    let Some(preview) = preview else {
        return [[0.0; 4]; 3];
    };
    [
        [1.0, preview.start, preview.end, preview.gain],
        [
            preview.fade_in_length,
            preview.fade_in_curve,
            preview.fade_out_length,
            preview.fade_out_curve,
        ],
        [
            preview.fade_in_mute,
            preview.fade_out_mute,
            preview.fade_in_outer_gain,
            preview.fade_out_outer_gain,
        ],
    ]
}

#[cfg(test)]
#[path = "uniforms/tests.rs"]
mod tests;
