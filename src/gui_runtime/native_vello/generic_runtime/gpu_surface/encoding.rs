use super::*;

pub(super) fn uniforms_as_bytes(uniforms: &GpuSurfaceUniforms) -> &[u8] {
    gpu_upload_value_as_bytes(uniforms)
}

pub(super) fn signal_uniforms_as_bytes(uniforms: &SignalUniforms) -> &[u8] {
    gpu_upload_value_as_bytes(uniforms)
}

pub(super) fn summary_buckets_as_f32s(buckets: &[GpuSignalSummaryBucket]) -> Vec<f32> {
    let mut values = Vec::with_capacity(buckets.len().saturating_mul(2));
    for bucket in buckets {
        values.push(bucket.min);
        values.push(bucket.max);
    }
    values
}

pub(super) fn summary_bucket_value_count(buckets: &[GpuSignalSummaryBucket]) -> usize {
    buckets.len().saturating_mul(2)
}

pub(super) fn summary_bucket_bytes(values: &[f32]) -> &[u8] {
    gpu_upload_slice_as_bytes(values)
}

trait GpuUploadData {}

impl GpuUploadData for GpuSurfaceUniforms {}
impl GpuUploadData for SignalUniforms {}
impl GpuUploadData for f32 {}

fn gpu_upload_value_as_bytes<T: GpuUploadData>(value: &T) -> &[u8] {
    gpu_upload_slice_as_bytes(std::slice::from_ref(value))
}

fn gpu_upload_slice_as_bytes<T: GpuUploadData>(values: &[T]) -> &[u8] {
    let len = std::mem::size_of_val(values);
    let ptr = values.as_ptr().cast::<u8>();
    // SAFETY: `GpuUploadData` is a private marker implemented only for plain
    // old data types used by this GPU-surface module. The returned view is
    // tied to `values`, so it cannot outlive the source storage.
    unsafe { std::slice::from_raw_parts(ptr, len) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_byte_views_match_uniform_sizes() {
        let surface = GpuSurfaceUniforms::default();
        let signal = SignalUniforms::default();

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
    fn summary_bucket_encoding_flattens_min_max_pairs() {
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

        let values = summary_buckets_as_f32s(&buckets);

        assert_eq!(values, vec![-0.25, 0.75, -1.0, 1.0]);
        assert_eq!(summary_bucket_value_count(&buckets), values.len());
        assert_eq!(
            summary_bucket_bytes(&values).len(),
            values.len() * std::mem::size_of::<f32>()
        );
    }
}
