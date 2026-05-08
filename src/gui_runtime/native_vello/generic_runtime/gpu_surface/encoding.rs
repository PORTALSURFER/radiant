use super::*;

pub(super) fn uniforms_as_bytes(uniforms: &GpuSurfaceUniforms) -> &[u8] {
    let len = std::mem::size_of::<GpuSurfaceUniforms>();
    let ptr = std::ptr::from_ref(uniforms).cast::<u8>();
    // SAFETY: `GpuSurfaceUniforms` is a plain repr(C) float-only uniform block.
    unsafe { std::slice::from_raw_parts(ptr, len) }
}

pub(super) fn signal_uniforms_as_bytes(uniforms: &SignalUniforms) -> &[u8] {
    let len = std::mem::size_of::<SignalUniforms>();
    let ptr = std::ptr::from_ref(uniforms).cast::<u8>();
    // SAFETY: `SignalUniforms` is a plain repr(C) float-only uniform block.
    unsafe { std::slice::from_raw_parts(ptr, len) }
}

fn f32s_as_bytes(values: &[f32]) -> &[u8] {
    let len = std::mem::size_of_val(values);
    let ptr = values.as_ptr().cast::<u8>();
    // SAFETY: `f32` samples are plain data and the byte view does not outlive `values`.
    unsafe { std::slice::from_raw_parts(ptr, len) }
}

pub(super) fn summary_buckets_as_f32s(buckets: &[GpuSignalSummaryBucket]) -> Vec<f32> {
    let mut values = Vec::with_capacity(buckets.len().saturating_mul(2));
    for bucket in buckets {
        values.push(bucket.min);
        values.push(bucket.max);
    }
    values
}

pub(super) fn summary_bucket_bytes(values: &[f32]) -> &[u8] {
    f32s_as_bytes(values)
}
