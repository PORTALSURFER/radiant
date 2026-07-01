use super::gpu_surface_types::{GpuSurfaceUniforms, SignalUniforms};
use crate::gui_runtime::native_vello::generic_runtime::gpu_upload_bytes::{
    GpuUploadBytes, upload_slice_as_bytes, upload_value_as_bytes,
};
use crate::runtime::GpuSignalSummaryBucket;

pub(super) fn uniforms_as_bytes(uniforms: &GpuSurfaceUniforms) -> &[u8] {
    upload_value_as_bytes(uniforms)
}

pub(super) fn signal_uniforms_as_bytes(uniforms: &SignalUniforms) -> &[u8] {
    upload_value_as_bytes(uniforms)
}

pub(super) fn summary_bucket_value_count(buckets: &[GpuSignalSummaryBucket]) -> usize {
    buckets.len().saturating_mul(2)
}

pub(super) fn summary_bucket_bytes(buckets: &[GpuSignalSummaryBucket]) -> &[u8] {
    upload_slice_as_bytes(buckets)
}

unsafe impl GpuUploadBytes for GpuSurfaceUniforms {}
unsafe impl GpuUploadBytes for SignalUniforms {}
unsafe impl GpuUploadBytes for GpuSignalSummaryBucket {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_byte_views_match_uniform_sizes() {
        let surface = GpuSurfaceUniforms::default();
        let signal = SignalUniforms::default();

        assert_eq!(std::mem::size_of::<GpuSurfaceUniforms>(), 240);
        assert_eq!(std::mem::size_of::<SignalUniforms>(), 144);
        assert_eq!(
            uniforms_as_bytes(&surface).len(),
            std::mem::size_of::<GpuSurfaceUniforms>()
        );
        assert_eq!(
            signal_uniforms_as_bytes(&signal).len(),
            std::mem::size_of::<SignalUniforms>()
        );
    }

    #[test]
    fn summary_bucket_encoding_exposes_min_max_pairs_without_flattening() {
        let buckets = [
            GpuSignalSummaryBucket {
                min: -0.25,
                max: 0.75,
            },
            GpuSignalSummaryBucket {
                min: -1.0,
                max: 1.0,
            },
        ];

        let bytes = summary_bucket_bytes(&buckets);
        let values = bytes
            .chunks_exact(std::mem::size_of::<f32>())
            .map(|chunk| f32::from_ne_bytes(chunk.try_into().expect("f32 byte chunk")))
            .collect::<Vec<_>>();

        assert_eq!(values, vec![-0.25, 0.75, -1.0, 1.0]);
        assert_eq!(summary_bucket_value_count(&buckets), values.len());
        assert_eq!(bytes.len(), values.len() * std::mem::size_of::<f32>());
    }
}
